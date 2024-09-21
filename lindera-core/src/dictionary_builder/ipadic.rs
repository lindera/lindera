use std::fs;
use std::path::Path;

use csv::StringRecord;

use crate::decompress::Algorithm;
use crate::dictionary::character_definition::CharacterDefinitions;
use crate::dictionary::UserDictionary;
use crate::dictionary_builder::DictionaryBuilder;
use crate::dictionary_builder::{
    build_user_dictionary, CharDefBuilderOptions, CostMatrixBuilderOptions, DictBuilderOptions,
    UnkBuilderOptions, UserDictBuilderOptions,
};
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

const SIMPLE_USERDIC_FIELDS_NUM: usize = 3;
const SIMPLE_WORD_COST: i16 = -10000;
const SIMPLE_CONTEXT_ID: u16 = 0;
const DETAILED_USERDIC_FIELDS_NUM: usize = 13;
const COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;
const UNK_FIELDS_NUM: usize = 11;
const ENCODING: &str = "EUC-JP";

pub struct IpadicBuilder {}

impl IpadicBuilder {
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
        let user_dict = self.build_user_dict(input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    fn build_chardef(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinitions> {
        CharDefBuilderOptions::default()
            .encoding(ENCODING)
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
            .encoding(ENCODING)
            .compress_algorithm(COMPRESS_ALGORITHM)
            .unk_fields_num(UNK_FIELDS_NUM)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        DictBuilderOptions::default()
            .flexible_csv(false)
            .encoding(ENCODING)
            .compress_algorithm(COMPRESS_ALGORITHM)
            .normalize_details(true)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        CostMatrixBuilderOptions::default()
            .encoding(ENCODING)
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        UserDictBuilderOptions::default()
            .simple_userdic_fields_num(SIMPLE_USERDIC_FIELDS_NUM)
            .detailed_userdic_fields_num(DETAILED_USERDIC_FIELDS_NUM)
            .simple_word_cost(SIMPLE_WORD_COST)
            .simple_context_id(SIMPLE_CONTEXT_ID)
            .flexible_csv(true)
            .simple_userdic_details_handler(Some(Box::new(|row: &StringRecord| {
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
            })))
            .builder()
            .unwrap()
            .build(input_file)
    }
}
