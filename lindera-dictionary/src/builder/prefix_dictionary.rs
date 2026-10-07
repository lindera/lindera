use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use encoding_rs::{Encoding, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;
use glob::glob;
use log::debug;

use crate::LinderaResult;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::schema::Schema;
use crate::error::LinderaErrorKind;
use crate::util::write_data;
use crate::viterbi::WordEntry;

pub struct PrefixDictionaryBuilder {
    flexible_csv: bool,
    /* If set to UTF-8, it can also read UTF-16 files with BOM. */
    encoding: Cow<'static, str>,
    skip_invalid_cost_or_id: bool,
    schema: Schema,
    /// Optional connection-cost context-ID remap. When present, each entry's
    /// `left_id`/`right_id` is relabeled via `remap.left`/`remap.right` before the
    /// `WordEntry` is created, matching the remap applied to the connection matrix.
    context_id_remap: Option<Arc<ContextIdMap>>,
}

/// Options for [`PrefixDictionaryBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Default)]
pub struct PrefixDictionaryBuilderOptions {
    flexible_csv: Option<bool>,
    encoding: Option<Cow<'static, str>>,
    skip_invalid_cost_or_id: Option<bool>,
    schema: Option<Schema>,
    context_id_remap: Option<Arc<ContextIdMap>>,
}

impl PrefixDictionaryBuilderOptions {
    pub fn flexible_csv(&mut self, value: bool) -> &mut Self {
        self.flexible_csv = Some(value);
        self
    }

    pub fn encoding(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.encoding = Some(value.into());
        self
    }

    pub fn skip_invalid_cost_or_id(&mut self, value: bool) -> &mut Self {
        self.skip_invalid_cost_or_id = Some(value);
        self
    }

    pub fn schema(&mut self, value: Schema) -> &mut Self {
        self.schema = Some(value);
        self
    }

    pub fn context_id_remap(&mut self, value: Option<Arc<ContextIdMap>>) -> &mut Self {
        self.context_id_remap = value;
        self
    }

    pub fn builder(&self) -> PrefixDictionaryBuilder {
        PrefixDictionaryBuilder {
            flexible_csv: self.flexible_csv.unwrap_or(true),
            encoding: self.encoding.clone().unwrap_or_else(|| "UTF-8".into()),
            skip_invalid_cost_or_id: self.skip_invalid_cost_or_id.unwrap_or(false),
            schema: self.schema.clone().unwrap_or_default(),
            context_id_remap: self.context_id_remap.clone(),
        }
    }
}

impl PrefixDictionaryBuilder {
    /// Create a new builder with the specified schema
    pub fn new(schema: Schema) -> Self {
        Self {
            flexible_csv: true,
            encoding: "UTF-8".into(),
            skip_invalid_cost_or_id: false,
            schema,
            context_id_remap: None,
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

        // Sort dictionary entries by the surface as written. The sort is
        // stable, so entries sharing a surface keep their input order.
        rows.sort_by(|a, b| a[0].cmp(&b[0]));

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
            // Skip the row if any of these required fields is missing or invalid.
            let (Some(word_cost), Some(left_id), Some(right_id)) = (
                self.parse_word_cost(row)?,
                self.parse_left_id(row)?,
                self.parse_right_id(row)?,
            ) else {
                continue;
            };

            // The surface is the key verbatim, as in MeCab. Trimming it would
            // drop whitespace entries such as U+3000 (IPADIC, UniDic) and
            // U+0020 (SudachiDict) and merge entries that start or end with
            // whitespace into the trimmed key (#1094). Rewriting characters,
            // as the builder once did with U+2015 and U+FF5E, would store an
            // entry under a spelling the input text never has (#1106).
            let Some(surface) = self.get_surface(row) else {
                continue;
            };

            // Relabel context IDs to match the connection matrix when remapping is
            // enabled. `get().unwrap_or` leaves any out-of-range id untouched (a
            // malformed id fails the matrix build instead of panicking here).
            let (left_id, right_id) = match &self.context_id_remap {
                Some(m) => (
                    m.left.get(left_id as usize).copied().unwrap_or(left_id),
                    m.right.get(right_id as usize).copied().unwrap_or(right_id),
                ),
                None => (left_id, right_id),
            };

            word_entry_map
                .entry(surface.to_string())
                .or_default()
                .push(WordEntry::new(
                    crate::viterbi::WordId::new(crate::viterbi::LexType::System, row_id as u32),
                    word_cost,
                    left_id,
                    right_id,
                ));
        }

        Ok(word_entry_map)
    }

    /// Returns the surface of a row exactly as written in the CSV.
    ///
    /// Unlike [`Self::get_field_value`], the value is not trimmed: a surface
    /// may consist of or start or end with whitespace (IPADIC's U+3000 entry,
    /// SudachiDict's U+0020 entry), as in the user dictionary builder.
    ///
    /// # 引数
    ///
    /// * `row` - The CSV row.
    ///
    /// # 戻り値
    ///
    /// The surface, or `None` when the schema has no surface field, the row
    /// is too short, or the surface is empty.
    fn get_surface<'a>(&self, row: &'a StringRecord) -> Option<&'a str> {
        let index = self.schema.get_field_index("surface")?;
        row.get(index).filter(|value| !value.is_empty())
    }

    /// Returns a numeric field (cost or a context id) by name, trimmed.
    ///
    /// Whitespace around a number is tolerated, consistently with the
    /// context id remapping (`context_id_remap.rs`). Do not use this for the
    /// surface, which must be kept verbatim (see [`Self::get_surface`]).
    ///
    /// # 引数
    ///
    /// * `row` - The CSV row.
    /// * `field_name` - The schema field name.
    ///
    /// # 戻り値
    ///
    /// The trimmed value, or `None` when the field is unknown, missing or
    /// blank.
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

        // Write dict.trie and dict.valsidx
        self.write_trie_files(output_dir, word_entry_map)?;

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

            // Create word details from the row data (5th column and beyond),
            // kept as written in the CSV like the surface.
            let joined_details = row.iter().skip(4).collect::<Vec<&str>>().join("\0");
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

        write_data(&dict_words_buffer, &mut dict_words_writer)?;

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

        write_data(&dict_wordsidx_buffer, &mut dict_wordsidx_writer)?;

        dict_wordsidx_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.wordsidx file: {dict_wordsidx_path:?}"
                ))
        })?;

        Ok(())
    }

    /// Write the trie files (dict.trie, dict.valsidx).
    ///
    /// The trie's value for a surface is its key ordinal, and dict.valsidx
    /// maps that ordinal to a run of records inside dict.vals -- which
    /// [`Self::write_values_file`] writes from the same map in the same
    /// iteration order, keeping the two consistent by construction.
    ///
    /// # Arguments
    ///
    /// * `output_dir` - Directory to write the two files into.
    /// * `word_entry_map` - Surface form to word entries.
    ///
    /// # Returns
    ///
    /// Unit on success, or an error if the trie build or a write fails.
    fn write_trie_files(
        &self,
        output_dir: &Path,
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<()> {
        let (trie_bytes, idx_bytes) =
            crate::dictionary::prefix_dictionary::PrefixDictionary::serialize_trie(word_entry_map)?;

        for (name, bytes) in [("dict.trie", &trie_bytes), ("dict.valsidx", &idx_bytes)] {
            let path = output_dir.join(name);
            let mut writer = io::BufWriter::new(File::create(&path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!("Failed to create {name} file: {path:?}"))
            })?);
            write_data(bytes, &mut writer)?;
            // An explicit flush so a write error surfaces here instead of
            // being swallowed by BufWriter's drop.
            writer.flush().map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!("Failed to flush {name} file: {path:?}"))
            })?;
        }

        Ok(())
    }

    /// Write values file (dict.vals)
    fn write_values_file(
        &self,
        output_dir: &Path,
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<()> {
        let mut dict_vals_buffer = Vec::new();
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry.serialize(&mut dict_vals_buffer).map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to serialize word entry (id: {})",
                            word_entry.word_id().id()
                        ))
                })?;
            }
        }

        let dict_vals_path = output_dir.join(Path::new("dict.vals"));
        let mut dict_vals_writer =
            io::BufWriter::new(File::create(&dict_vals_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create dict.vals file: {dict_vals_path:?}"
                    ))
            })?);

        write_data(&dict_vals_buffer, &mut dict_vals_writer)?;

        dict_vals_writer.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush dict.vals file: {dict_vals_path:?}"
                ))
        })?;

        Ok(())
    }
}

/// Maximum number of word entries that can share one surface form, imposed by
/// the 8-bit count field of the packed prefix-dictionary value.
pub(crate) const MAX_ENTRIES_PER_SURFACE: u32 = 0xff;

/// Maximum entry offset representable by the 24-bit offset field of the packed
/// prefix-dictionary value.
pub(crate) const MAX_ENTRY_OFFSET: u32 = (1 << 24) - 1;

/// Packs a prefix-dictionary match value as `(offset << 8) | count`.
///
/// Both fields are bounds-checked: exceeding either silently corrupted the
/// value before (the count overflowed into the offset bits, and the offset
/// shifted out of the u32 entirely), producing a dictionary that built without
/// error but tokenized incorrectly.
///
/// # 引数
///
/// * `surface` - The surface form being packed, used for error reporting.
/// * `offset` - Entry offset into the values file, in `WordEntry` records.
/// * `count` - Number of entries sharing this surface form.
///
/// # 戻り値
///
/// The packed value, or an error naming the surface form and the limit it
/// exceeded.
pub(crate) fn pack_entry_value(surface: &str, offset: u32, count: u32) -> LinderaResult<u32> {
    if count > MAX_ENTRIES_PER_SURFACE {
        return Err(LinderaErrorKind::Build.with_error(anyhow::anyhow!(
            "surface form {surface:?} has {count} entries, exceeding the limit of {MAX_ENTRIES_PER_SURFACE} per surface form"
        )));
    }
    if offset > MAX_ENTRY_OFFSET {
        return Err(LinderaErrorKind::Build.with_error(anyhow::anyhow!(
            "entry offset {offset} at surface form {surface:?} exceeds the limit of {MAX_ENTRY_OFFSET}; the dictionary has too many entries"
        )));
    }
    Ok((offset << 8) | count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::schema::Schema;
    use csv::StringRecord;

    #[test]
    fn test_pack_entry_value_within_bounds() {
        // Round-trips through the same decoding the dictionary performs.
        let packed = pack_entry_value("すもも", 361_708, 1).expect("within bounds");
        assert_eq!(packed >> 8, 361_708);
        assert_eq!(packed & 0xff, 1);

        // Both fields at their maximum still pack losslessly.
        let packed = pack_entry_value("x", MAX_ENTRY_OFFSET, MAX_ENTRIES_PER_SURFACE)
            .expect("boundary values are valid");
        assert_eq!(packed >> 8, MAX_ENTRY_OFFSET);
        assert_eq!(packed & 0xff, MAX_ENTRIES_PER_SURFACE);
    }

    #[test]
    fn test_pack_entry_value_rejects_too_many_variants() {
        // 256 variants would overflow the 8-bit count field and corrupt the
        // offset bits above it; this used to build a silently wrong dictionary.
        let err = pack_entry_value("かん", 0, MAX_ENTRIES_PER_SURFACE + 1)
            .expect_err("256 variants must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("かん"), "error should name the surface: {msg}");
        assert!(msg.contains("256"), "error should state the count: {msg}");
    }

    #[test]
    fn test_pack_entry_value_rejects_offset_overflow() {
        // An offset past 24 bits would shift out of the u32 entirely.
        let err = pack_entry_value("x", MAX_ENTRY_OFFSET + 1, 1)
            .expect_err("offset past 24 bits must be rejected");
        assert!(
            err.to_string().contains("too many entries"),
            "error should explain the cause: {err}"
        );
    }

    #[test]
    fn test_new_with_schema() {
        let schema = Schema::default();
        let builder = PrefixDictionaryBuilder::new(schema.clone());

        // Schema no longer has name field
        // Schema no longer has version field
        assert!(builder.flexible_csv);
        assert_eq!(builder.encoding, "UTF-8");
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

        assert_eq!(builder.get_surface(&record), None);
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
        assert_eq!(builder.get_surface(&record), Some("word"));
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

    #[test]
    fn test_get_surface_keeps_whitespace() {
        let builder = PrefixDictionaryBuilder::new(Schema::default());

        for surface in ["\u{3000}", " ", " a", "a ", "\u{3000}a\u{3000}"] {
            let record = StringRecord::from(vec![surface, "1", "1", "100"]);
            assert_eq!(builder.get_surface(&record), Some(surface));
        }
    }

    #[test]
    fn test_build_word_entry_map_keeps_whitespace_surfaces() {
        let builder = PrefixDictionaryBuilder::new(Schema::default());
        let rows = vec![
            StringRecord::from(vec!["\u{3000}", "9", "9", "1287", "記号", "空白"]),
            StringRecord::from(vec![" ", "5967", "5967", "10", "空白"]),
            StringRecord::from(vec!["AA ", "1", "1", "100", "名詞"]),
            StringRecord::from(vec!["AA", "1", "1", "200", "名詞"]),
            // Whitespace around numbers is still accepted.
            StringRecord::from(vec![" 1", " 2 ", "3", " 4", "名詞"]),
            // An empty surface is skipped.
            StringRecord::from(vec!["", "1", "1", "1", "名詞"]),
        ];

        let map = builder.build_word_entry_map(&rows).unwrap();

        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        assert_eq!(keys, vec![" ", " 1", "AA", "AA ", "\u{3000}"]);
        // "AA " no longer takes over "AA".
        assert_eq!(map["AA"].len(), 1);
        assert_eq!(map["AA"][0].word_cost(), 200);
        assert_eq!(map["\u{3000}"][0].word_cost(), 1287);
        assert_eq!(map[" 1"][0].word_cost(), 4);
        assert_eq!(map[" 1"][0].left_id(), 2);
    }

    #[test]
    fn test_build_word_entry_map_keeps_dash_and_tilde_surfaces() {
        let builder = PrefixDictionaryBuilder::new(Schema::default());
        let rows = vec![
            StringRecord::from(vec!["ＣＤ\u{2015}ＲＯＭ", "1", "1", "100", "名詞"]),
            StringRecord::from(vec!["ＣＤ\u{2014}ＲＯＭ", "1", "1", "200", "名詞"]),
            StringRecord::from(vec!["\u{2015}\u{2015}", "2", "2", "300", "記号"]),
            StringRecord::from(vec!["あ\u{ff5e}", "3", "3", "400", "名詞"]),
            StringRecord::from(vec!["あ\u{301c}", "3", "3", "500", "名詞"]),
        ];

        let map = builder.build_word_entry_map(&rows).unwrap();

        // Each spelling is its own key: U+2015 and U+FF5E are no longer
        // rewritten to U+2014 and U+301C, so they neither move to another
        // key nor merge with the entry spelled that way (#1106).
        assert_eq!(map.len(), 5);
        assert_eq!(map["ＣＤ\u{2015}ＲＯＭ"][0].word_cost(), 100);
        assert_eq!(map["ＣＤ\u{2014}ＲＯＭ"][0].word_cost(), 200);
        assert_eq!(map["\u{2015}\u{2015}"][0].word_cost(), 300);
        assert_eq!(map["あ\u{ff5e}"].len(), 1);
        assert_eq!(map["あ\u{ff5e}"][0].word_cost(), 400);
        assert_eq!(map["あ\u{301c}"].len(), 1);
        assert_eq!(map["あ\u{301c}"][0].word_cost(), 500);
    }

    #[test]
    fn test_build_keeps_dash_and_tilde_in_surfaces_and_details() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            input_dir.path().join("lex.csv"),
            "ＣＤ\u{2015}ＲＯＭ,1,1,100,名詞,一般,ＣＤ\u{2015}ＲＯＭ\n\
             あ\u{ff5e},2,2,200,名詞,一般,あ\u{ff5e}\n",
        )
        .unwrap();

        PrefixDictionaryBuilder::new(Schema::default())
            .build(input_dir.path(), output_dir.path())
            .unwrap();

        let read = |name: &str| std::fs::read(output_dir.path().join(name)).unwrap();
        let dict = crate::dictionary::prefix_dictionary::PrefixDictionary::load(
            read("dict.trie"),
            read("dict.valsidx"),
            read("dict.vals"),
            read("dict.wordsidx"),
            read("dict.words"),
        )
        .unwrap();
        let details = |surface: &str| {
            let entries = dict.find_surface(surface);
            assert_eq!(entries.len(), 1, "one entry for {surface}");
            let word_id = entries[0].word_id().id() as usize;
            let offset = crate::util::words_idx_offset(&dict.words_idx_data, word_id).unwrap();
            crate::util::joined_details_at(&dict.words_data, offset)
                .unwrap()
                .to_string()
        };

        // The entries are found under the spelling of the CSV, and only there.
        assert!(dict.find_surface("ＣＤ\u{2014}ＲＯＭ").is_empty());
        assert!(dict.find_surface("あ\u{301c}").is_empty());
        // The detail fields are kept as written too.
        assert_eq!(
            details("ＣＤ\u{2015}ＲＯＭ"),
            "名詞\0一般\0ＣＤ\u{2015}ＲＯＭ"
        );
        assert_eq!(details("あ\u{ff5e}"), "名詞\0一般\0あ\u{ff5e}");
    }
}
