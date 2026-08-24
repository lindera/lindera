use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use daachorse::DoubleArrayAhoCorasickBuilder;
use log::debug;

use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::prefix_dictionary::UserPrefixDictionary;
use crate::error::LinderaErrorKind;
use crate::viterbi::WordEntry;

type StringRecordProcessor = Option<Box<dyn Fn(&StringRecord) -> LinderaResult<Vec<String>>>>;

pub struct UserDictionaryBuilder {
    user_dictionary_fields_num: usize,
    dictionary_fields_num: usize,
    default_word_cost: i16,
    default_left_context_id: u16,
    default_right_context_id: u16,
    flexible_csv: bool,
    user_dictionary_handler: StringRecordProcessor,
}

/// Options for [`UserDictionaryBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible. Setters take `self` by value to keep the
/// original owned-builder calling style.
#[derive(Default)]
pub struct UserDictionaryBuilderOptions {
    user_dictionary_fields_num: Option<usize>,
    dictionary_fields_num: Option<usize>,
    default_word_cost: Option<i16>,
    default_left_context_id: Option<u16>,
    default_right_context_id: Option<u16>,
    flexible_csv: Option<bool>,
    user_dictionary_handler: StringRecordProcessor,
}

impl UserDictionaryBuilderOptions {
    pub fn user_dictionary_fields_num(mut self, value: usize) -> Self {
        self.user_dictionary_fields_num = Some(value);
        self
    }

    pub fn dictionary_fields_num(mut self, value: usize) -> Self {
        self.dictionary_fields_num = Some(value);
        self
    }

    pub fn default_word_cost(mut self, value: i16) -> Self {
        self.default_word_cost = Some(value);
        self
    }

    pub fn default_left_context_id(mut self, value: u16) -> Self {
        self.default_left_context_id = Some(value);
        self
    }

    pub fn default_right_context_id(mut self, value: u16) -> Self {
        self.default_right_context_id = Some(value);
        self
    }

    pub fn flexible_csv(mut self, value: bool) -> Self {
        self.flexible_csv = Some(value);
        self
    }

    pub fn user_dictionary_handler(mut self, value: StringRecordProcessor) -> Self {
        self.user_dictionary_handler = value;
        self
    }

    pub fn builder(self) -> UserDictionaryBuilder {
        UserDictionaryBuilder {
            user_dictionary_fields_num: self.user_dictionary_fields_num.unwrap_or(3),
            dictionary_fields_num: self.dictionary_fields_num.unwrap_or(12),
            default_word_cost: self.default_word_cost.unwrap_or(-10000),
            default_left_context_id: self.default_left_context_id.unwrap_or(0),
            default_right_context_id: self.default_right_context_id.unwrap_or(0),
            flexible_csv: self.flexible_csv.unwrap_or(true),
            user_dictionary_handler: self.user_dictionary_handler,
        }
    }
}

impl UserDictionaryBuilder {
    /// Builds a user dictionary from a CSV file on disk.
    ///
    /// # 引数
    ///
    /// * `input_file` - Path of the user dictionary CSV file.
    ///
    /// # 戻り値
    ///
    /// The built user dictionary.
    pub fn build(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        debug!("reading {input_file:?}");

        let file = File::open(input_file).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to open user dictionary CSV file: {input_file:?}"
                ))
        })?;
        self.build_from_reader(file)
    }

    /// Builds a user dictionary from CSV content supplied by a reader.
    ///
    /// This is the filesystem-free entry point: WebAssembly callers feed it
    /// bytes obtained in JavaScript (fetch, file inputs, OPFS), and
    /// [`Self::build`] delegates here after opening its file (#972).
    /// The content must be UTF-8.
    ///
    /// # 引数
    ///
    /// * `reader` - Read source of the user dictionary CSV content.
    ///
    /// # 戻り値
    ///
    /// The built user dictionary.
    pub fn build_from_reader<R: Read>(&self, reader: R) -> LinderaResult<UserDictionary> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(self.flexible_csv)
            .from_reader(reader);

        let mut rows: Vec<StringRecord> = vec![];
        for (line_num, result) in rdr.records().enumerate() {
            let record = result.map_err(|err| {
                LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to parse CSV record at line {}",
                        line_num + 1,
                    ))
            })?;
            rows.push(record);
        }

        // Classify row arity before anything indexes into a row: with
        // `flexible` CSV parsing a 1- or 2-field row would otherwise reach
        // the cost/context-id column accesses below and panic out of bounds
        // -- and a panic aborts the module on wasm32, where this builder is
        // fed untrusted browser input (#972). Runs before the sort so the
        // reported row number matches the input order.
        for (row_id, row) in rows.iter().enumerate() {
            if row.len() != self.user_dictionary_fields_num
                && row.len() < self.dictionary_fields_num
            {
                return Err(LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!(
                        "user dictionary should be a CSV with {} or {}+ fields",
                        self.user_dictionary_fields_num,
                        self.dictionary_fields_num
                    ))
                    .add_context(format!(
                        "Row {} has {} fields (surface: '{}')",
                        row_id + 1,
                        row.len(),
                        row.get(0).unwrap_or("<empty>")
                    )));
            }
        }

        // The cached variant computes the allocating surface key once per row
        // instead of once per comparison.
        rows.sort_by_cached_key(|row| row[0].to_string());

        let mut word_entry_map: BTreeMap<String, Vec<WordEntry>> = BTreeMap::new();

        for (row_id, row) in rows.iter().enumerate() {
            let surface = row[0].to_string();
            let word_cost = if row.len() == self.user_dictionary_fields_num {
                self.default_word_cost
            } else {
                row[3].parse::<i16>().map_err(|_err| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("failed to parse word cost"))
                        .add_context(format!(
                            "Invalid word cost '{}' at row {} (surface: '{}')",
                            &row[3],
                            row_id + 1,
                            &row[0]
                        ))
                })?
            };
            let (left_id, right_id) = if row.len() == self.user_dictionary_fields_num {
                (self.default_left_context_id, self.default_right_context_id)
            } else {
                (
                    row[1].parse::<u16>().map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse left context id"))
                            .add_context(format!(
                                "Invalid left context ID '{}' at row {} (surface: '{}')",
                                &row[1],
                                row_id + 1,
                                &row[0]
                            ))
                    })?,
                    row[2].parse::<u16>().map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse right context id"))
                            .add_context(format!(
                                "Invalid right context ID '{}' at row {} (surface: '{}')",
                                &row[2],
                                row_id + 1,
                                &row[0]
                            ))
                    })?,
                )
            };

            word_entry_map
                .entry(surface)
                .or_default()
                .push(WordEntry::new(
                    crate::viterbi::WordId::new(crate::viterbi::LexType::User, row_id as u32),
                    word_cost,
                    left_id,
                    right_id,
                ));
        }

        let mut words_data = Vec::<u8>::new();
        let mut words_idx_data = Vec::<u8>::new();
        for (row_id, row) in rows.iter().enumerate() {
            let word_detail = if row.len() == self.user_dictionary_fields_num {
                if let Some(handler) = &self.user_dictionary_handler {
                    handler(row)?
                } else {
                    row.iter()
                        .skip(1)
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                }
            } else if row.len() >= self.dictionary_fields_num {
                let mut tmp_word_detail = Vec::new();
                for item in row.iter().skip(4) {
                    tmp_word_detail.push(item.to_string());
                }
                tmp_word_detail
            } else {
                return Err(LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!(
                        "user dictionary should be a CSV with {} or {}+ fields",
                        self.user_dictionary_fields_num,
                        self.dictionary_fields_num
                    ))
                    .add_context(format!(
                        "Row {} has {} fields (surface: '{}')",
                        row_id + 1,
                        row.len(),
                        row.get(0).unwrap_or("<empty>")
                    )));
            };

            let offset = words_data.len();
            words_idx_data
                .write_u32::<LittleEndian>(offset as u32)
                .map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to write word offset to user dictionary words index")
                })?;

            // Store word details as null-separated string (like main dictionary)
            let joined_details = word_detail.join("\0");
            let joined_details_len = u32::try_from(joined_details.len()).map_err(|err| {
                LinderaErrorKind::Serialize
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Word details length too large: {} bytes for word '{}'",
                        joined_details.len(),
                        row.get(0).unwrap_or("<unknown>")
                    ))
            })?;

            words_data
                .write_u32::<LittleEndian>(joined_details_len)
                .map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context(
                            "Failed to write word details length to user dictionary words data",
                        )
                })?;
            words_data
                .write_all(joined_details.as_bytes())
                .map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to write word details to user dictionary words data")
                })?;
        }

        let mut id = 0u32;

        // building double array trie
        let mut keyset: Vec<(&[u8], u32)> = vec![];
        for (key, word_entries) in &word_entry_map {
            let len = word_entries.len() as u32;
            // 24bit for word ID, 8bit for variant count (up to 255 per surface),
            // matching the system dictionary encoding. The legacy 5-bit user
            // encoding (max 31 variants) was retired in v4.0.0; user dictionary
            // `.bin` files built with v3 must be rebuilt from their CSV source.
            let val = crate::builder::prefix_dictionary::pack_entry_value(key, id, len)?;
            keyset.push((key.as_bytes(), val));
            id += len;
        }
        let da_bytes = DoubleArrayAhoCorasickBuilder::new()
            .build_with_values(keyset)
            .map_err(|err| {
                LinderaErrorKind::Build
                    .with_error(anyhow::anyhow!(err))
                    .add_context("Failed to build DoubleArray for user dictionary")
            })?
            .serialize();

        // building values
        let mut vals_data = Vec::<u8>::new();
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry.serialize(&mut vals_data).map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to serialize user dictionary word entry (id: {})",
                            word_entry.word_id().id()
                        ))
                })?;
            }
        }

        let dict = UserPrefixDictionary::load(da_bytes, vals_data, words_idx_data, words_data)?;

        Ok(UserDictionary { dict })
    }
}

pub fn build_user_dictionary(user_dict: UserDictionary, output_file: &Path) -> LinderaResult<()> {
    let parent_dir = match output_file.parent() {
        Some(parent_dir) => parent_dir,
        None => {
            return Err(LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(
                    "failed to get parent directory of output file"
                ))
                .add_context(format!("Invalid output file path: {output_file:?}")));
        }
    };
    fs::create_dir_all(parent_dir).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to create parent directory: {parent_dir:?}"))
    })?;

    let mut wtr = io::BufWriter::new(File::create(output_file).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to create user dictionary output file: {output_file:?}"
            ))
    })?);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&user_dict).map_err(|err| {
        LinderaErrorKind::Serialize
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to serialize user dictionary to file: {output_file:?}"
            ))
    })?;
    wtr.write_all(&bytes).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to write user dictionary to file: {output_file:?}"
            ))
    })?;
    wtr.flush().map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to flush user dictionary output file: {output_file:?}"
            ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::builder::DictionaryBuilder;
    use crate::dictionary::metadata::Metadata;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources/user_dict")
            .join(name)
    }

    fn flexible_builder() -> DictionaryBuilder {
        let metadata = Metadata {
            flexible_csv: true,
            ..Metadata::default()
        };
        DictionaryBuilder::new(metadata)
    }

    /// The reader-based build must produce a byte-identical dictionary to the
    /// path-based build over the same CSV (#972).
    #[test]
    fn reader_and_path_builds_are_identical() {
        let path = fixture_path("ipadic_simple_userdic.csv");
        let builder = flexible_builder();

        let from_path = match builder.build_user_dict(&path) {
            Ok(dict) => dict,
            Err(err) => panic!("path build failed: {err}"),
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to read fixture: {err}"),
        };
        let from_reader = match builder.build_user_dict_from_reader(bytes.as_slice()) {
            Ok(dict) => dict,
            Err(err) => panic!("reader build failed: {err}"),
        };

        let path_bytes = match rkyv::to_bytes::<rkyv::rancor::Error>(&from_path) {
            Ok(bytes) => bytes,
            Err(err) => panic!("serialization failed: {err}"),
        };
        let reader_bytes = match rkyv::to_bytes::<rkyv::rancor::Error>(&from_reader) {
            Ok(bytes) => bytes,
            Err(err) => panic!("serialization failed: {err}"),
        };
        assert_eq!(path_bytes.as_slice(), reader_bytes.as_slice());
    }

    /// A CSV mixing simple (3-field) and detailed (13-field) rows builds
    /// under flexible parsing. First test to exercise the mixed fixture.
    #[test]
    fn mixed_simple_and_detailed_rows_build() {
        let bytes = match std::fs::read(fixture_path("ipadic_mixed_userdic.csv")) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to read fixture: {err}"),
        };
        let dict = match flexible_builder().build_user_dict_from_reader(bytes.as_slice()) {
            Ok(dict) => dict,
            Err(err) => panic!("mixed build failed: {err}"),
        };
        assert!(!dict.dict.vals_data.is_empty());
    }

    /// Rows shorter than the simple format must produce an error, not an
    /// out-of-bounds panic: with flexible CSV parsing such rows previously
    /// reached the cost-column access before any arity check, and on wasm32
    /// a panic aborts the module (#972).
    #[test]
    fn short_rows_error_instead_of_panicking() {
        for csv in ["東京\n", "東京,1288\n"] {
            let result = flexible_builder().build_user_dict_from_reader(csv.as_bytes());
            let err = match result {
                Ok(_) => panic!("{csv:?} must not build"),
                Err(err) => format!("{err:?}"),
            };
            assert!(
                err.contains("user dictionary should be a CSV with 3 or 13+ fields"),
                "unexpected error for {csv:?}: {err}"
            );
        }
    }
}
