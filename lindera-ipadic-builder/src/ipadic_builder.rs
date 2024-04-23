use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    u32,
};

use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use encoding_rs::EUC_JP;
use encoding_rs_io::DecodeReaderBytesBuilder;
use glob::glob;
use lindera_dictionary_builder::{
    CostMatrixBuilder, CostMatrixBuilderOptions, UserDictBuilderOptions,
};
use log::debug;
use yada::builder::DoubleArrayBuilder;

#[cfg(feature = "compress")]
use lindera_compress::compress;
use lindera_core::{
    character_definition::{CharacterDefinitions, CharacterDefinitionsBuilder},
    dictionary::UserDictionary,
    dictionary_builder::DictionaryBuilder,
    error::LinderaErrorKind,
    file_util::read_euc_file,
    unknown_dictionary::parse_unk,
    word_entry::{WordEntry, WordId},
    LinderaResult,
};
use lindera_decompress::Algorithm;

const SIMPLE_USERDIC_FIELDS_NUM: usize = 3;
const SIMPLE_WORD_COST: i16 = -10000;
const SIMPLE_CONTEXT_ID: u16 = 0;
const DETAILED_USERDIC_FIELDS_NUM: usize = 13;
const COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;

pub struct IpadicBuilder {}

impl IpadicBuilder {
    const UNK_FIELDS_NUM: usize = 11;

    pub fn new() -> Self {
        IpadicBuilder {}
    }
}

impl Default for IpadicBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryBuilder for IpadicBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let chardef = self.build_chardef(input_dir, output_dir)?;
        self.build_unk(input_dir, &chardef, output_dir)?;
        self.build_dict(input_dir, output_dir)?;
        self.build_cost_matrix(input_dir, output_dir)?;

        Ok(())
    }

    fn build_user_dictionary(&self, input_file: &Path, output_file: &Path) -> LinderaResult<()> {
        let parent_dir = match output_file.parent() {
            Some(parent_dir) => parent_dir,
            None => {
                return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                    "failed to get parent directory of output file"
                )))
            }
        };
        fs::create_dir_all(parent_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let user_dict = self.build_user_dict(input_file)?;

        let mut wtr = io::BufWriter::new(
            File::create(output_file)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        bincode::serialize_into(&mut wtr, &user_dict)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        wtr.flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_chardef(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinitions> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {:?}", char_def_path);

        let char_def = read_euc_file(&char_def_path)?;
        let mut char_definitions_builder = CharacterDefinitionsBuilder::default();
        char_definitions_builder.parse(&char_def)?;
        let char_definitions = char_definitions_builder.build();

        let mut chardef_buffer = Vec::new();
        bincode::serialize_into(&mut chardef_buffer, &char_definitions)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;

        let wtr_chardef_path = output_dir.join(Path::new("char_def.bin"));
        let mut wtr_chardef = io::BufWriter::new(
            File::create(wtr_chardef_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        compress_write(&chardef_buffer, COMPRESS_ALGORITHM, &mut wtr_chardef)?;

        wtr_chardef
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(char_definitions)
    }

    fn build_unk(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinitions,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        let unk_data_path = input_dir.join("unk.def");
        debug!("reading {:?}", unk_data_path);

        let unk_data = read_euc_file(&unk_data_path)?;
        let unknown_dictionary = parse_unk(chardef.categories(), &unk_data, Self::UNK_FIELDS_NUM)?;

        let mut unk_buffer = Vec::new();
        bincode::serialize_into(&mut unk_buffer, &unknown_dictionary)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;

        let wtr_unk_path = output_dir.join(Path::new("unk.bin"));
        let mut wtr_unk = io::BufWriter::new(
            File::create(wtr_unk_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        compress_write(&unk_buffer, COMPRESS_ALGORITHM, &mut wtr_unk)?;
        wtr_unk
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
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
                    }
                }
                Err(err) => return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(err))),
            }
        }

        let mut rows: Vec<StringRecord> = vec![];
        for filename in filenames {
            debug!("reading {:?}", filename);

            let file = File::open(filename)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
            let transcoded = DecodeReaderBytesBuilder::new()
                .encoding(Some(EUC_JP))
                .build(file);

            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(transcoded);

            for result in rdr.records() {
                let record = result
                    .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;
                rows.push(record);
            }
        }

        let mut normalized_rows: Vec<Vec<String>> = Vec::new();
        for row in rows {
            let mut normalized_row: Vec<String> = Vec::new();
            for column in row.iter() {
                // yeah for EUC_JP and ambiguous unicode 8012 vs 8013
                // same bullshit as above between for 12316 vs 65374
                normalized_row.push(column.to_string().replace('―', "—").replace('～', "〜"));
            }
            normalized_rows.push(normalized_row);
        }

        normalized_rows.sort_by_key(|row| row.first().map(|s| s.to_string()));

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

        for (row_id, row) in normalized_rows.iter().enumerate() {
            word_entry_map
                .entry(row[0].to_string())
                .or_default()
                .push(WordEntry {
                    word_id: WordId(row_id as u32, true),
                    word_cost: i16::from_str(row[3].trim()).map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse word_cost"))
                    })?,
                    left_id: u16::from_str(row[1].trim()).map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse cost_id"))
                    })?,
                    right_id: u16::from_str(row[2].trim()).map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse cost_id"))
                    })?,
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
        for row in normalized_rows.iter() {
            let offset = words_buffer.len();
            words_idx_buffer
                .write_u32::<LittleEndian>(offset as u32)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

            let mut word_details = Vec::new();
            for item in row.iter().skip(4) {
                word_details.push(item.as_str());
            }
            let joined_details = word_details.join("\0");
            let joined_details_len = u32::try_from(joined_details.as_bytes().len())
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            words_buffer
                .write_u32::<LittleEndian>(joined_details_len)
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            words_buffer
                .write_all(joined_details.as_bytes())
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        }

        compress_write(&words_buffer, COMPRESS_ALGORITHM, &mut wtr_words)?;
        compress_write(&words_idx_buffer, COMPRESS_ALGORITHM, &mut wtr_words_idx)?;

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

        compress_write(&da_bytes, COMPRESS_ALGORITHM, &mut wtr_da)?;

        let mut vals_buffer = Vec::new();
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry
                    .serialize(&mut vals_buffer)
                    .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            }
        }

        compress_write(&vals_buffer, COMPRESS_ALGORITHM, &mut wtr_vals)?;

        wtr_vals
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        let matrix_data_path = input_dir.join("matrix.def");
        CostMatrixBuilderOptions::default()
            .encoding("EUC-JP")
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(&matrix_data_path, output_dir)
    }

    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        UserDictBuilderOptions::default()
            .simple_userdic_fields_num(SIMPLE_USERDIC_FIELDS_NUM)
            .detailed_userdic_fields_num(DETAILED_USERDIC_FIELDS_NUM)
            .simple_word_cost(SIMPLE_WORD_COST)
            .simple_context_id(SIMPLE_CONTEXT_ID)
            .flexible_csv(true)
            .simple_userdic_details_handler(|row| {
                Ok(vec![
                    row[1].to_string(), // POS
                    "*".to_string(),    // POS subcategory 1
                    "*".to_string(),    // POS subcategory 2
                    "*".to_string(),    // POS subcategory 3
                    "*".to_string(),    // Conjugation type
                    "*".to_string(),    // Conjugation form
                    row[0].to_string(), // Base form
                    row[2].to_string(), // Reading
                    "*".to_string(),    // Pronunciation
                ])
            })
            .builder()
            .unwrap()
            .build(input_file)
    }
}

fn user_dict_row_to_details(row: &[String]) -> LinderaResult<Vec<String>> {
    if row.len() == SIMPLE_USERDIC_FIELDS_NUM {
        Ok(vec![
            row[1].to_string(), // POS
            "*".to_string(),    // POS subcategory 1
            "*".to_string(),    // POS subcategory 2
            "*".to_string(),    // POS subcategory 3
            "*".to_string(),    // Conjugation type
            "*".to_string(),    // Conjugation form
            row[0].to_string(), // Base form
            row[2].to_string(), // Reading
            "*".to_string(),    // Pronunciation
        ])
    } else if row.len() >= DETAILED_USERDIC_FIELDS_NUM {
        let mut tmp_word_detail = Vec::new();
        for item in row.iter().skip(4) {
            tmp_word_detail.push(item.to_string());
        }
        Ok(tmp_word_detail)
    } else {
        Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "user dictionary should be a CSV with {} or {}+ fields",
            SIMPLE_USERDIC_FIELDS_NUM,
            DETAILED_USERDIC_FIELDS_NUM
        )))
    }
}

#[cfg(feature = "compress")]
fn compress_write<W: Write>(
    buffer: &[u8],
    algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    let compressed = compress(buffer, algorithm)
        .map_err(|err| LinderaErrorKind::Compress.with_error(anyhow::anyhow!(err)))?;
    bincode::serialize_into(writer, &compressed)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    Ok(())
}

#[cfg(not(feature = "compress"))]
fn compress_write<W: Write>(
    buffer: &[u8],
    _algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    writer
        .write_all(buffer)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    Ok(())
}
