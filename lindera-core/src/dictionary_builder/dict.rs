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
use log::{debug, warn};
use yada::builder::DoubleArrayBuilder;

use crate::decompress::Algorithm;
use crate::dictionary::word_entry::{WordEntry, WordId};
use crate::dictionary_builder::utils::compress_write;
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

#[derive(Builder, Debug)]
#[builder(name = "DictBuilderOptions")]
#[builder(build_fn(name = "builder"))]
pub struct DictBuilder {
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
}

impl DictBuilder {
    pub fn build(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        let pattern = if let Some(path) = input_dir.to_str() {
            format!("{}/*.csv", path)
        } else {
            return Err(
                LinderaErrorKind::Io.with_error(anyhow::anyhow!("Failed to convert path to &str."))
            );
        };

        let mut filenames: Vec<PathBuf> = Vec::new();
        for entry in
            glob(&pattern).map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?
        {
            match entry {
                Ok(path) => {
                    if let Some(filename) = path.file_name() {
                        filenames.push(Path::new(input_dir).join(filename));
                    } else {
                        return Err(LinderaErrorKind::Io
                            .with_error(anyhow::anyhow!("failed to get filename")));
                    };
                }
                Err(err) => return Err(LinderaErrorKind::Content.with_error(anyhow!(err))),
            }
        }

        let encoding = Encoding::for_label_no_replacement(self.encoding.as_bytes());
        let encoding = encoding.ok_or_else(|| {
            LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", self.encoding))
        })?;

        let mut rows: Vec<StringRecord> = vec![];
        for filename in filenames {
            debug!("reading {:?}", filename);

            let file = File::open(filename)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
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
                let record =
                    result.map_err(|err| LinderaErrorKind::Content.with_error(anyhow!(err)))?;
                rows.push(record);
            }
        }

        if self.normalize_details {
            rows.sort_by_key(|row| normalize(&row[0]));
        } else {
            rows.sort_by(|a, b| a[0].cmp(&b[0]))
        }

        let wtr_da_path = output_dir.join(Path::new("dict.da"));
        let mut wtr_da = io::BufWriter::new(
            File::create(wtr_da_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let wtr_vals_path = output_dir.join(Path::new("dict.vals"));
        let mut wtr_vals = io::BufWriter::new(
            File::create(wtr_vals_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let mut word_entry_map: BTreeMap<String, Vec<WordEntry>> = BTreeMap::new();

        for (row_id, row) in rows.iter().enumerate() {
            let word_cost = match i16::from_str(row[3].trim()) {
                Ok(wc) => wc,
                Err(_err) => {
                    if self.skip_invalid_cost_or_id {
                        warn!("failed to parse word_cost: {:?}", row);
                        continue;
                    } else {
                        return Err(LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse word_cost")));
                    }
                }
            };
            let left_id = match u16::from_str(row[1].trim()) {
                Ok(lid) => lid,
                Err(_err) => {
                    if self.skip_invalid_cost_or_id {
                        warn!("failed to parse left_id: {:?}", row);
                        continue;
                    } else {
                        return Err(LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse left_id")));
                    }
                }
            };
            let right_id = match u16::from_str(row[2].trim()) {
                Ok(rid) => rid,
                Err(_err) => {
                    if self.skip_invalid_cost_or_id {
                        warn!("failed to parse right_id: {:?}", row);
                        continue;
                    } else {
                        return Err(LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse right_id")));
                    }
                }
            };
            let key = if self.normalize_details {
                normalize(&row[0])
            } else {
                row[0].to_string()
            };
            word_entry_map.entry(key).or_default().push(WordEntry {
                word_id: WordId(row_id as u32, true),
                word_cost,
                left_id,
                right_id,
            });
        }

        let wtr_words_path = output_dir.join(Path::new("dict.words"));
        let mut wtr_words = io::BufWriter::new(
            File::create(wtr_words_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let wtr_words_idx_path = output_dir.join(Path::new("dict.wordsidx"));
        let mut wtr_words_idx = io::BufWriter::new(
            File::create(wtr_words_idx_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let mut words_buffer = Vec::new();
        let mut words_idx_buffer = Vec::new();
        for row in rows.iter() {
            let offset = words_buffer.len();
            words_idx_buffer
                .write_u32::<LittleEndian>(offset as u32)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

            let joined_details = if self.normalize_details {
                row.iter()
                    .skip(4)
                    .map(normalize)
                    .collect::<Vec<String>>()
                    .join("\0")
            } else {
                row.iter().skip(4).collect::<Vec<&str>>().join("\0")
            };
            let joined_details_len = u32::try_from(joined_details.as_bytes().len())
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            words_buffer
                .write_u32::<LittleEndian>(joined_details_len)
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            words_buffer
                .write_all(joined_details.as_bytes())
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        }

        compress_write(&words_buffer, self.compress_algorithm, &mut wtr_words)?;
        compress_write(
            &words_idx_buffer,
            self.compress_algorithm,
            &mut wtr_words_idx,
        )?;

        wtr_words
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        wtr_words_idx
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let mut id = 0u32;

        let mut keyset: Vec<(&[u8], u32)> = vec![];
        for (key, word_entries) in &word_entry_map {
            let len = word_entries.len() as u32;
            let val = (id << 5) | len; // 27bit for word ID, 5bit for different parts of speech on the same surface.
            keyset.push((key.as_bytes(), val));
            id += len;
        }

        let da_bytes = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        compress_write(&da_bytes, self.compress_algorithm, &mut wtr_da)?;

        let mut vals_buffer = Vec::new();
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry
                    .serialize(&mut vals_buffer)
                    .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            }
        }

        compress_write(&vals_buffer, self.compress_algorithm, &mut wtr_vals)?;

        wtr_vals
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }
}

fn normalize(text: &str) -> String {
    text.to_string().replace('―', "—").replace('～', "〜")
}
