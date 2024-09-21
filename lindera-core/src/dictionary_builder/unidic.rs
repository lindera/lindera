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
const DETAILED_USERDIC_FIELDS_NUM: usize = 21;
const COMPRESS_ALGORITHM: Algorithm = Algorithm::Deflate;
const UNK_FIELDS_NUM: usize = 10;

pub struct UnidicBuilder {}

impl UnidicBuilder {
    pub fn new() -> Self {
        UnidicBuilder {}
    }
}

impl Default for UnidicBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryBuilder for UnidicBuilder {
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
            .unk_fields_num(UNK_FIELDS_NUM)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        DictBuilderOptions::default()
            .flexible_csv(false)
            .compress_algorithm(COMPRESS_ALGORITHM)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        CostMatrixBuilderOptions::default()
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
            .flexible_csv(false)
            .simple_userdic_details_handler(Some(Box::new(|row: &StringRecord| {
                Ok(vec![
                    row[1].to_string(), //Major POS classification
                    "*".to_string(),    // Middle POS classification
                    "*".to_string(),    // Small POS classification
                    "*".to_string(),    // Fine POS classification
                    "*".to_string(),    // Conjugation form
                    "*".to_string(),    // Conjugation type
                    row[2].to_string(), //Lexeme reading
                    "*".to_string(),    // Lexeme
                    "*".to_string(),    // Orthography appearance type
                    "*".to_string(),    // Pronunciation appearance type
                    "*".to_string(),    // Orthography basic type
                    "*".to_string(),    // Pronunciation basic type
                    "*".to_string(),    // Word type
                    "*".to_string(),    // Prefix of a word form
                    "*".to_string(),    // Prefix of a word type
                    "*".to_string(),    // Suffix of a word form
                    "*".to_string(),    // Suffix of a word type
                ])
            })))
            .builder()
            .unwrap()
            .build(input_file)
    }
}
