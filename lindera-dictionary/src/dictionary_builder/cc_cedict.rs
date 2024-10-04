use std::fs;
use std::path::Path;

use csv::StringRecord;

use crate::decompress::Algorithm;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::UserDictionary;
use crate::dictionary_builder::DictionaryBuilder;
use crate::dictionary_builder::{
    build_user_dictionary, CharacterDefinitionBuilderOptions, ConnectionCostMatrixBuilderOptions,
    PrefixDictionaryBuilderOptions, UnknownDictionaryBuilderOptions, UserDictionaryBuilderOptions,
};
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

const SIMPLE_USERDIC_FIELDS_NUM: usize = 3;
const SIMPLE_WORD_COST: i16 = -10000;
const SIMPLE_CONTEXT_ID: u16 = 0;
const DETAILED_USERDIC_FIELDS_NUM: usize = 12;
const COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;
const UNK_FIELDS_NUM: usize = 10;

pub struct CcCedictBuilder {}

impl CcCedictBuilder {
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

        let chardef = self.build_character_definition(input_dir, output_dir)?;
        self.build_unknown_dictionary(input_dir, &chardef, output_dir)?;
        self.build_prefix_dictionary(input_dir, output_dir)?;
        self.build_connection_cost_matrix(input_dir, output_dir)?;

        Ok(())
    }

    fn build_user_dictionary(&self, input_file: &Path, output_file: &Path) -> LinderaResult<()> {
        let user_dict = self.build_user_dict(input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        CharacterDefinitionBuilderOptions::default()
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinition,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        UnknownDictionaryBuilderOptions::default()
            .compress_algorithm(COMPRESS_ALGORITHM)
            .unk_fields_num(UNK_FIELDS_NUM)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_prefix_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(true)
            .compress_algorithm(COMPRESS_ALGORITHM)
            .skip_invalid_cost_or_id(true)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_connection_cost_matrix(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        ConnectionCostMatrixBuilderOptions::default()
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        UserDictionaryBuilderOptions::default()
            .simple_userdic_fields_num(SIMPLE_USERDIC_FIELDS_NUM)
            .detailed_userdic_fields_num(DETAILED_USERDIC_FIELDS_NUM)
            .simple_word_cost(SIMPLE_WORD_COST)
            .simple_context_id(SIMPLE_CONTEXT_ID)
            .flexible_csv(false)
            .simple_userdic_details_handler(Some(Box::new(|row: &StringRecord| {
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
            })))
            .builder()
            .unwrap()
            .build(input_file)
    }
}
