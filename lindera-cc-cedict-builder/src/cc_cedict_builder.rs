use std::{
    fs,
    fs::File,
    io::{self, Write},
    path::Path,
};

use lindera_dictionary_builder::{
    CharDefBuilderOptions, CostMatrixBuilderOptions, DictBuilderOptions, UnkBuilderOptions,
    UserDictBuilderOptions,
};

#[cfg(feature = "compress")]
use lindera_compress::compress;
use lindera_core::{
    character_definition::CharacterDefinitions, dictionary::UserDictionary,
    dictionary_builder::DictionaryBuilder, error::LinderaErrorKind, LinderaResult,
};
use lindera_decompress::Algorithm;

const SIMPLE_USERDIC_FIELDS_NUM: usize = 3;
const SIMPLE_WORD_COST: i16 = -10000;
const SIMPLE_CONTEXT_ID: u16 = 0;
const DETAILED_USERDIC_FIELDS_NUM: usize = 12;
const COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;

pub struct CcCedictBuilder {}

impl CcCedictBuilder {
    const UNK_FIELDS_NUM: usize = 10;

    pub fn new() -> Self {
        CcCedictBuilder {}
    }
}

impl Default for CcCedictBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryBuilder for CcCedictBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let chardef = self.build_chardef(input_dir, output_dir).unwrap();
        self.build_unk(input_dir, &chardef, output_dir).unwrap();
        self.build_dict(input_dir, output_dir).unwrap();
        self.build_cost_matrix(input_dir, output_dir).unwrap();

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
        CharDefBuilderOptions::default()
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_unk(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinitions,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        UnkBuilderOptions::default()
            .compress_algorithm(COMPRESS_ALGORITHM)
            .unk_fields_num(Self::UNK_FIELDS_NUM)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        DictBuilderOptions::default()
            .flexible_csv(true)
            .encoding("UTF-8")
            .compress_algorithm(COMPRESS_ALGORITHM)
            .normalize_details(false)
            .skip_invalid_cost_or_id(true)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        let matrix_data_path = input_dir.join("matrix.def");
        CostMatrixBuilderOptions::default()
            .encoding("UTF-8")
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
            .flexible_csv(false)
            .simple_userdic_details_handler(Box::new(|row| {
                Ok(vec![
                    row[1].to_string(), // POS
                    "*".to_string(),    // POS subcategory 1
                    "*".to_string(),    // POS subcategory 2
                    "*".to_string(),    // POS subcategory 3
                    row[2].to_string(), // pinyin
                    "*".to_string(),    // traditional
                    "*".to_string(),    // simplified
                    "*".to_string(),    // definition
                ])
            }))
            .builder()
            .unwrap()
            .build(input_file)
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
