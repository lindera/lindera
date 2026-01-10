use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::anyhow;
use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use derive_builder::Builder;
use encoding_rs::{Encoding, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;
use glob::glob;
use log::debug;
use yada::builder::DoubleArrayBuilder;

use crate::LinderaResult;
use crate::decompress::Algorithm;
use crate::dictionary::schema::Schema;
use crate::error::LinderaErrorKind;
use crate::util::compress_write;
use crate::viterbi::WordEntry;

#[derive(Builder)]
#[builder(name = PrefixDictionaryBuilderOptions)]
#[builder(build_fn(name = "builder"))]
pub struct PrefixDictionaryBuilder {
    #[builder(default = "true")]
    flexible_csv: bool,
    /* If set to UTF-8, it can also read UTF-16 files with BOM. */
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    #[builder(default = "Algorithm::Deflate")]
    compress_algorithm: Algorithm,
    #[builder(default = "false")]
    normalize_details: bool,
    #[builder(default = "false")]
    skip_invalid_cost_or_id: bool,
    #[builder(default = "Schema::default()")]
    schema: Schema,
}

impl PrefixDictionaryBuilder {
    /// Create a new builder with the specified schema
    pub fn new(schema: Schema) -> Self {
        Self {
            flexible_csv: true,
            encoding: "UTF-8".into(),
            compress_algorithm: Algorithm::Deflate,
            normalize_details: false,
            skip_invalid_cost_or_id: false,
            schema,
        }
    }

    /// Main method for building the dictionary
    pub fn build(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        // 1. Load CSV data
        let rows = self.load_csv_data(input_dir)?;

        // 2. Build word entry map
        let word_entry_map = self.build_word_entry_map(&rows)?;

        // 3. Write dictionary files
        self.write_dictionary_files(output_dir, &rows, &word_entry_map)?;

        Ok(())
    }

    /// Load data from CSV files
    fn load_csv_data(&self, input_dir: &Path) -> LinderaResult<Vec<StringRecord>> {
        let filenames = self.collect_csv_files(input_dir)?;
        let encoding = self.get_encoding()?;
        let mut rows = self.read_csv_files(&filenames, encoding)?;

        // Sort dictionary entries by the first column (word)
        // Change sorting method based on normalization settings
        if self.normalize_details {
            // Sort after normalizing characters (―→—, ～→〜)
            rows.sort_by_key(|row| normalize(&row[0]));
        } else {
            // Sort using original strings directly
            rows.sort_by(|a, b| a[0].cmp(&b[0]))
        }

        Ok(rows)
    }

    /// Collect .csv file paths from input directory
    fn collect_csv_files(&self, input_dir: &Path) -> LinderaResult<Vec<PathBuf>> {
        let pattern = if let Some(path) = input_dir.to_str() {
            format!("{path}/*.csv")
        } else {
            return Err(LinderaErrorKind::Io
                .with_error(anyhow::anyhow!("Failed to convert path to &str."))
                .add_context(format!(
                    "Input directory path contains invalid characters: {input_dir:?}"
                )));
        };

        let mut filenames: Vec<PathBuf> = Vec::new();
        for entry in glob(&pattern).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!("Failed to glob CSV files with pattern: {pattern}"))
        })? {
            match entry {
                Ok(path) => {
                    if let Some(filename) = path.file_name() {
                        filenames.push(Path::new(input_dir).join(filename));
                    } else {
                        return Err(LinderaErrorKind::Io
                            .with_error(anyhow::anyhow!("failed to get filename"))
                            .add_context(format!("Invalid filename in path: {path:?}")));
                    };
                }
                Err(err) => {
                    return Err(LinderaErrorKind::Content
                        .with_error(anyhow!(err))
                        .add_context(format!(
                            "Failed to process glob entry with pattern: {pattern}"
                        )));
                }
            }
        }

        Ok(filenames)
    }

    /// Get encoding configuration
    fn get_encoding(&self) -> LinderaResult<&'static Encoding> {
        let encoding = Encoding::for_label_no_replacement(self.encoding.as_bytes());
        encoding.ok_or_else(|| {
            LinderaErrorKind::Decode
                .with_error(anyhow!("Invalid encoding: {}", self.encoding))
                .add_context("Failed to get encoding for CSV file reading")
        })
    }

    /// Read CSV files
    fn read_csv_files(
        &self,
        filenames: &[PathBuf],
        encoding: &'static Encoding,
    ) -> LinderaResult<Vec<StringRecord>> {
        let mut rows: Vec<StringRecord> = vec![];

        for filename in filenames {
            debug!("reading {filename:?}");

            let file = File::open(filename).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!("Failed to open CSV file: {filename:?}"))
            })?;
            let reader: Box<dyn Read> = if encoding == UTF_8 {
                Box::new(file)
            } else {
                Box::new(
                    DecodeReaderBytesBuilder::new()
                        .encoding(Some(encoding))
                        .build(file),
                )
            };
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(self.flexible_csv)
                .from_reader(reader);

            for result in rdr.records() {
                let record = result.map_err(|err| {
                    LinderaErrorKind::Content
                        .with_error(anyhow!(err))
                        .add_context(format!("Failed to parse CSV record in file: {filename:?}"))
                })?;
                rows.push(record);
            }
        }

        Ok(rows)
    }

    /// Build word entry map
    fn build_word_entry_map(
        &self,
        rows: &[StringRecord],
    ) -> LinderaResult<BTreeMap<String, Vec<WordEntry>>> {
        let mut word_entry_map: BTreeMap<String, Vec<WordEntry>> = BTreeMap::new();

        for (row_id, row) in rows.iter().enumerate() {
            let word_cost = self.parse_word_cost(row)?;
            let left_id = self.parse_left_id(row)?;
            let right_id = self.parse_right_id(row)?;

            // Skip if any value is invalid
            if word_cost.is_none() || left_id.is_none() || right_id.is_none() {
                continue;
            }

            let key = if self.normalize_details {
                if let Some(surface) = self.get_field_value(row, "surface")? {
                    normalize(&surface)
                } else {
                    continue;
                }
            } else if let Some(surface) = self.get_field_value(row, "surface")? {
                surface
            } else {
                continue;
            };

            word_entry_map.entry(key).or_default().push(WordEntry {
                word_id: crate::viterbi::WordId::new(
                    crate::viterbi::LexType::System,
                    row_id as u32,
                ),
                word_cost: word_cost.unwrap(),
                left_id: left_id.unwrap(),
                right_id: right_id.unwrap(),
            });
        }

        Ok(word_entry_map)
    }

    /// Get field value by name
    fn get_field_value(
        &self,
        row: &StringRecord,
        field_name: &str,
    ) -> LinderaResult<Option<String>> {
        if let Some(index) = self.schema.get_field_index(field_name) {
            if index >= row.len() {
                return Ok(None);
            }

            let value = row[index].trim();
            Ok(if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            })
        } else {
            Ok(None)
        }
    }

    /// Parse word cost using schema
    fn parse_word_cost(&self, row: &StringRecord) -> LinderaResult<Option<i16>> {
        let cost_str = self.get_field_value(row, "cost")?;
        match cost_str {
            Some(s) => match i16::from_str(&s) {
                Ok(cost) => Ok(Some(cost)),
                Err(_) => {
                    if self.skip_invalid_cost_or_id {
                        Ok(None)
                    } else {
                        Err(LinderaErrorKind::Content
                            .with_error(anyhow!("Invalid cost value: {s}")))
                    }
                }
            },
            None => Ok(None),
        }
    }

    /// Parse left ID using schema
    fn parse_left_id(&self, row: &StringRecord) -> LinderaResult<Option<u16>> {
        let left_id_str = self.get_field_value(row, "left_context_id")?;
        match left_id_str {
            Some(s) => match u16::from_str(&s) {
                Ok(id) => Ok(Some(id)),
                Err(_) => {
                    if self.skip_invalid_cost_or_id {
                        Ok(None)
                    } else {
                        Err(LinderaErrorKind::Content
                            .with_error(anyhow!("Invalid left context ID: {s}")))
                    }
                }
            },
            None => Ok(None),
        }
    }

    /// Parse right ID using schema
    fn parse_right_id(&self, row: &StringRecord) -> LinderaResult<Option<u16>> {
        let right_id_str = self.get_field_value(row, "right_context_id")?;
        match right_id_str {
            Some(s) => match u16::from_str(&s) {
                Ok(id) => Ok(Some(id)),
                Err(_) => {
                    if self.skip_invalid_cost_or_id {
                        Ok(None)
                    } else {
                        Err(LinderaErrorKind::Content
                            .with_error(anyhow!("Invalid right context ID: {s}")))
                    }
                }
            },
            None => Ok(None),
        }
    }

    /// Write dictionary files
    fn write_dictionary_files(
        &self,
        output_dir: &Path,
        rows: &[StringRecord],
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<()> {
        // Write dict.words and dict.wordsidx
        self.write_words_files(output_dir, rows)?;

        // Write dict.da
        self.write_double_array_file(output_dir, word_entry_map)?;

        // Write dict.vals
        self.write_values_file(output_dir, word_entry_map)?;

        Ok(())
    }

    /// Write word detail files (dict.words, dict.wordsidx)
    fn write_words_files(&self, output_dir: &Path, rows: &[StringRecord]) -> LinderaResult<()> {
        let mut dict_words_buffer = Vec::new();
        let mut dict_wordsidx_buffer = Vec::new();

        for row in rows.iter() {
            let offset = dict_words_buffer.len();
            dict_wordsidx_buffer
                .write_u32::<LittleEndian>(offset as u32)
                .map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to write word index offset to dict.wordsidx buffer")
                })?;

            // Create word details from the row data (5th column and beyond)
            let joined_details = if self.normalize_details {
                row.iter()
                    .skip(4)
                    .map(normalize)
                    .collect::<Vec<String>>()
                    .join("\0")
            } else {
                row.iter().skip(4).collect::<Vec<&str>>().join("\0")
            };
            let joined_details_len = u32::try_from(joined_details.len()).map_err(|err| {
                LinderaErrorKind::Serialize
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Word details length too large: {} bytes",
                        joined_details.len()
                    ))
            })?;

            // Write to dict.words buffer
            dict_words_buffer
                .write_u32::<LittleEndian>(joined_details_len)
                .map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to write word details length to dict.words buffer")
                })?;
            dict_words_buffer
                .write_all(joined_details.as_bytes())
                .map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to write word details to dict.words buffer")
                })?;
        }

        // Write dict.words file
        let dict_words_path = output_dir.join(Path::new("dict.words"));
        let mut dict_words_writer =
            io::BufWriter::new(File::create(&dict_words_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.words file: {dict_words_path:?}"
                    ))
            })?);

        compress_write(
            &dict_words_buffer,
            self.compress_algorithm,
            &mut dict_words_writer,
        )?;

        dict_words_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.words file: {dict_words_path:?}"
                ))
        })?;

        // Write dict.wordsidx file
        let dict_wordsidx_path = output_dir.join(Path::new("dict.wordsidx"));
        let mut dict_wordsidx_writer =
            io::BufWriter::new(File::create(&dict_wordsidx_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.wordsidx file: {dict_wordsidx_path:?}"
                    ))
            })?);

        compress_write(
            &dict_wordsidx_buffer,
            self.compress_algorithm,
            &mut dict_wordsidx_writer,
        )?;

        dict_wordsidx_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.wordsidx file: {dict_wordsidx_path:?}"
                ))
        })?;

        Ok(())
    }

    /// Write double array file (dict.da)
    fn write_double_array_file(
        &self,
        output_dir: &Path,
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<()> {
        let mut id = 0u32;
        let mut keyset: Vec<(&[u8], u32)> = vec![];

        for (key, word_entries) in word_entry_map {
            let len = word_entries.len() as u32;
            let val = (id << 5) | len; // 27bit for word ID, 5bit for different parts of speech on the same surface.
            keyset.push((key.as_bytes(), val));
            id += len;
        }

        let dict_da_buffer = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Build
                .with_error(anyhow::anyhow!("DoubleArray build error."))
                .add_context(format!(
                    "Failed to build DoubleArray with {} keys for prefix dictionary",
                    keyset.len()
                ))
        })?;

        let dict_da_path = output_dir.join(Path::new("dict.da"));
        let mut dict_da_writer =
            io::BufWriter::new(File::create(&dict_da_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!("Failed to create dict.da file: {dict_da_path:?}"))
            })?);

        compress_write(
            &dict_da_buffer,
            self.compress_algorithm,
            &mut dict_da_writer,
        )?;

        Ok(())
    }

    /// Write values file (dict.vals.cost, dict.vals.left, dict.vals.right, dict.vals.idx)
    fn write_values_file(
        &self,
        output_dir: &Path,
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<()> {
        let mut costs_buffer = Vec::new();
        let mut left_ids_buffer = Vec::new();
        let mut right_ids_buffer = Vec::new();
        let mut word_ids_buffer = Vec::new();

        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                costs_buffer
                    .write_i16::<LittleEndian>(word_entry.word_cost)
                    .map_err(|err| {
                        LinderaErrorKind::Serialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context(format!(
                                "Failed to serialize word cost (id: {})",
                                word_entry.word_id.id
                            ))
                    })?;
                left_ids_buffer
                    .write_u16::<LittleEndian>(word_entry.left_id)
                    .map_err(|err| {
                        LinderaErrorKind::Serialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context(format!(
                                "Failed to serialize left id (id: {})",
                                word_entry.word_id.id
                            ))
                    })?;
                right_ids_buffer
                    .write_u16::<LittleEndian>(word_entry.right_id)
                    .map_err(|err| {
                        LinderaErrorKind::Serialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context(format!(
                                "Failed to serialize right id (id: {})",
                                word_entry.word_id.id
                            ))
                    })?;
                word_ids_buffer
                    .write_u32::<LittleEndian>(word_entry.word_id.id)
                    .map_err(|err| {
                        LinderaErrorKind::Serialize
                            .with_error(anyhow::anyhow!(err))
                            .add_context(format!(
                                "Failed to serialize word id (id: {})",
                                word_entry.word_id.id
                            ))
                    })?;
            }
        }

        // Write dict.vals.cost
        let dict_vals_cost_path = output_dir.join(Path::new("dict.vals.cost"));
        let mut dict_vals_cost_writer =
            io::BufWriter::new(File::create(&dict_vals_cost_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.vals.cost file: {dict_vals_cost_path:?}"
                    ))
            })?);
        compress_write(
            &costs_buffer,
            self.compress_algorithm,
            &mut dict_vals_cost_writer,
        )?;
        dict_vals_cost_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.vals.cost file: {dict_vals_cost_path:?}"
                ))
        })?;

        // Write dict.vals.left
        let dict_vals_left_path = output_dir.join(Path::new("dict.vals.left"));
        let mut dict_vals_left_writer =
            io::BufWriter::new(File::create(&dict_vals_left_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.vals.left file: {dict_vals_left_path:?}"
                    ))
            })?);
        compress_write(
            &left_ids_buffer,
            self.compress_algorithm,
            &mut dict_vals_left_writer,
        )?;
        dict_vals_left_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.vals.left file: {dict_vals_left_path:?}"
                ))
        })?;

        // Write dict.vals.right
        let dict_vals_right_path = output_dir.join(Path::new("dict.vals.right"));
        let mut dict_vals_right_writer =
            io::BufWriter::new(File::create(&dict_vals_right_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.vals.right file: {dict_vals_right_path:?}"
                    ))
            })?);
        compress_write(
            &right_ids_buffer,
            self.compress_algorithm,
            &mut dict_vals_right_writer,
        )?;
        dict_vals_right_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.vals.right file: {dict_vals_right_path:?}"
                ))
        })?;

        // Write dict.vals.idx
        let dict_vals_idx_path = output_dir.join(Path::new("dict.vals.idx"));
        let mut dict_vals_idx_writer =
            io::BufWriter::new(File::create(&dict_vals_idx_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.vals.idx file: {dict_vals_idx_path:?}"
                    ))
            })?);
        compress_write(
            &word_ids_buffer,
            self.compress_algorithm,
            &mut dict_vals_idx_writer,
        )?;
        dict_vals_idx_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.vals.idx file: {dict_vals_idx_path:?}"
                ))
        })?;

        Ok(())
    }
}

fn normalize(text: &str) -> String {
    text.to_string().replace('―', "—").replace('～', "〜")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::schema::Schema;
    use csv::StringRecord;

    #[test]
    fn test_new_with_schema() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema.clone());

        // Schema no longer has name field
        // Schema no longer has version field
        assert!(builder.flexible_csv);
        assert_eq!(builder.encoding, "UTF-8");
        assert!(!builder.normalize_details);
        assert!(!builder.skip_invalid_cost_or_id);
    }

    #[test]
    fn test_get_common_field_value_empty() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "",    // Empty surface
            "123", // LeftContextId
            "456", // RightContextId
            "789", // Cost
        ]);

        let surface = builder.get_field_value(&record, "surface").unwrap();
        assert_eq!(surface, None);
    }

    #[test]
    fn test_get_common_field_value_out_of_bounds() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "surface_form", // Surface only
        ]);

        let left_id = builder.get_field_value(&record, "left_context_id").unwrap();
        assert_eq!(left_id, None);
    }

    #[test]
    fn test_parse_word_cost() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "surface_form", // Surface
            "123",          // LeftContextId
            "456",          // RightContextId
            "789",          // Cost
        ]);

        let cost = builder.parse_word_cost(&record).unwrap();
        assert_eq!(cost, Some(789));
    }

    #[test]
    fn test_parse_word_cost_invalid() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "surface_form", // Surface
            "123",          // LeftContextId
            "456",          // RightContextId
            "invalid",      // Invalid cost
        ]);

        let result = builder.parse_word_cost(&record);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_word_cost_skip_invalid() {
        let schema = Schema::default();
        let mut builder = PrefixDictionaryBuilder::new(schema);
        builder.skip_invalid_cost_or_id = true;

        let record = StringRecord::from(vec![
            "surface_form", // Surface
            "123",          // LeftContextId
            "456",          // RightContextId
            "invalid",      // Invalid cost
        ]);

        let cost = builder.parse_word_cost(&record).unwrap();
        assert_eq!(cost, None);
    }

    #[test]
    fn test_parse_left_id() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "surface_form", // Surface
            "123",          // LeftContextId
            "456",          // RightContextId
            "789",          // Cost
        ]);

        let left_id = builder.parse_left_id(&record).unwrap();
        assert_eq!(left_id, Some(123));
    }

    #[test]
    fn test_parse_right_id() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "surface_form", // Surface
            "123",          // LeftContextId
            "456",          // RightContextId
            "789",          // Cost
        ]);

        let right_id = builder.parse_right_id(&record).unwrap();
        assert_eq!(right_id, Some(456));
    }

    #[test]
    fn test_normalize_function() {
        assert_eq!(normalize("test―text"), "test—text");
        assert_eq!(normalize("test～text"), "test〜text");
        assert_eq!(normalize("test―text～more"), "test—text〜more");
        assert_eq!(normalize("normal text"), "normal text");
    }

    #[test]
    fn test_get_encoding() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let encoding = builder.get_encoding().unwrap();
        assert_eq!(encoding.name(), "UTF-8");
    }

    #[test]
    fn test_get_encoding_invalid() {
        let schema = Schema::default();
        let mut builder = PrefixDictionaryBuilder::new(schema);
        builder.encoding = "INVALID-ENCODING".into();

        let result = builder.get_encoding();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_common_field_value() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema);

        let record = StringRecord::from(vec![
            "word", // Surface
            "123",  // LeftContextId
            "456",  // RightContextId
            "789",  // Cost
            "名詞", // MajorPos
        ]);

        // Test common fields
        assert_eq!(
            builder.get_field_value(&record, "surface").unwrap(),
            Some("word".to_string())
        );
        assert_eq!(
            builder.get_field_value(&record, "left_context_id").unwrap(),
            Some("123".to_string())
        );
        assert_eq!(
            builder
                .get_field_value(&record, "right_context_id")
                .unwrap(),
            Some("456".to_string())
        );
        assert_eq!(
            builder.get_field_value(&record, "cost").unwrap(),
            Some("789".to_string())
        );

        // Test case where field is out of bounds - should return None, not an error
        let short_record = StringRecord::from(vec!["word", "123"]);
        assert_eq!(
            builder.get_field_value(&short_record, "cost").unwrap(),
            None
        );
    }
}
