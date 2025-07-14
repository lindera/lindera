use std::fs;
use std::path::Path;

use csv::StringRecord;

use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::metadata::Metadata;
use crate::dictionary_builder::metadata::MetadataBuilder;
use crate::dictionary_builder::{
    CharacterDefinitionBuilderOptions, ConnectionCostMatrixBuilderOptions, DictionaryBuilder,
    PrefixDictionaryBuilderOptions, UnknownDictionaryBuilderOptions, UserDictionaryBuilderOptions,
    build_user_dictionary,
};
use crate::error::LinderaErrorKind;

pub struct UnidicBuilder {}

impl UnidicBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UnidicBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryBuilder for UnidicBuilder {
    fn build_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        self.build_metadata(metadata, output_dir)?;
        let chardef = self.build_character_definition(metadata, input_dir, output_dir)?;
        self.build_unknown_dictionary(metadata, input_dir, output_dir, &chardef)?;
        self.build_prefix_dictionary(metadata, input_dir, output_dir)?;
        self.build_connection_cost_matrix(metadata, input_dir, output_dir)?;

        Ok(())
    }

    fn build_user_dictionary(
        &self,
        metadata: &Metadata,
        input_file: &Path,
        output_file: &Path,
    ) -> LinderaResult<()> {
        let user_dict = self.build_user_dict(metadata, input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    fn build_metadata(&self, metadata: &Metadata, output_dir: &Path) -> LinderaResult<()> {
        MetadataBuilder::new().build(metadata, output_dir)
    }

    fn build_character_definition(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        CharacterDefinitionBuilderOptions::default()
            .encoding(metadata.encoding.clone())
            .compress_algorithm(metadata.compress_algorithm)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_unknown_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
    ) -> LinderaResult<()> {
        UnknownDictionaryBuilderOptions::default()
            .encoding(metadata.encoding.clone())
            .compress_algorithm(metadata.compress_algorithm)
            .unk_fields_num(metadata.unk_fields_num)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_prefix_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(false)
            .encoding(metadata.encoding.clone())
            .compress_algorithm(metadata.compress_algorithm)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_connection_cost_matrix(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        ConnectionCostMatrixBuilderOptions::default()
            .encoding(metadata.encoding.clone())
            .compress_algorithm(metadata.compress_algorithm)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_user_dict(
        &self,
        metadata: &Metadata,
        input_file: &Path,
    ) -> LinderaResult<UserDictionary> {
        UserDictionaryBuilderOptions::default()
            .simple_userdic_fields_num(metadata.simple_userdic_fields_num)
            .detailed_userdic_fields_num(metadata.detailed_userdic_fields_num)
            .simple_word_cost(metadata.simple_word_cost)
            .simple_context_id(metadata.simple_context_id)
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
