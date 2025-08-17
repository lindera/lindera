use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use derive_builder::Builder;
use log::debug;
use yada::builder::DoubleArrayBuilder;

use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::error::LinderaErrorKind;
use crate::viterbi::{WordEntry, WordId};

type StringRecordProcessor = Option<Box<dyn Fn(&StringRecord) -> LinderaResult<Vec<String>>>>;

#[derive(Builder)]
#[builder(pattern = "owned")]
#[builder(name = UserDictionaryBuilderOptions)]
#[builder(build_fn(name = "builder"))]
pub struct UserDictionaryBuilder {
    #[builder(default = "3")]
    user_dictionary_fields_num: usize,
    #[builder(default = "12")]
    dictionary_fields_num: usize,
    #[builder(default = "-10000")]
    default_word_cost: i16,
    #[builder(default = "0")]
    default_left_context_id: u16,
    #[builder(default = "0")]
    default_right_context_id: u16,
    #[builder(default = "true")]
    flexible_csv: bool,
    #[builder(setter(strip_option), default = "None")]
    user_dictionary_handler: StringRecordProcessor,
}

impl UserDictionaryBuilder {
    pub fn build(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        debug!("reading {input_file:?}");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(self.flexible_csv)
            .from_path(input_file)
            .map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to open user dictionary CSV file: {input_file:?}"
                    ))
            })?;

        let mut rows: Vec<StringRecord> = vec![];
        for (line_num, result) in rdr.records().enumerate() {
            let record = result.map_err(|err| {
                LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to parse CSV record at line {} in file: {:?}",
                        line_num + 1,
                        input_file
                    ))
            })?;
            rows.push(record);
        }
        rows.sort_by_key(|row| row[0].to_string());

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

            word_entry_map.entry(surface).or_default().push(WordEntry {
                word_id: WordId {
                    id: row_id as u32,
                    is_system: false,
                },
                word_cost,
                left_id,
                right_id,
            });
        }

        let mut words_data = Vec::<u8>::new();
        let mut words_idx_data = Vec::<u8>::new();
        for row in rows.iter() {
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
                        rows.iter().position(|r| std::ptr::eq(r, row)).unwrap_or(0) + 1,
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
            let val = (id << 5) | len;
            keyset.push((key.as_bytes(), val));
            id += len;
        }
        let da_bytes = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Build
                .with_error(anyhow::anyhow!("DoubleArray build error."))
                .add_context(format!(
                    "Failed to build DoubleArray with {} keys for user dictionary",
                    keyset.len()
                ))
        })?;

        // building values
        let mut vals_data = Vec::<u8>::new();
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry.serialize(&mut vals_data).map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context(format!(
                            "Failed to serialize user dictionary word entry (id: {})",
                            word_entry.word_id.id
                        ))
                })?;
            }
        }

        let dict = PrefixDictionary::load(da_bytes, vals_data, words_idx_data, words_data, false);

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
    bincode::serde::encode_into_std_write(&user_dict, &mut wtr, bincode::config::legacy())
        .map_err(|err| {
            LinderaErrorKind::Serialize
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to serialize user dictionary to file: {output_file:?}"
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
