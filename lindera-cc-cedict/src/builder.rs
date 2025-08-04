use std::fs;
use std::path::Path;

use csv::StringRecord;
use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::UserDictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary_builder::{
    metadata::MetadataBuilder,
    CharacterDefinitionBuilderOptions, ConnectionCostMatrixBuilderOptions, DictionaryBuilder,
    PrefixDictionaryBuilderOptions, UnknownDictionaryBuilderOptions,
    UserDictionaryBuilderOptions, build_user_dictionary,
};
use lindera_dictionary::error::LinderaErrorKind;

use crate::schema::CcCedictSchema;
use crate::metadata::CcCedictMetadata;

pub struct CcCedictBuilder {
    metadata: Metadata,
}

impl CcCedictBuilder {
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }
}

impl Default for CcCedictBuilder {
    fn default() -> Self {
        Self::new(CcCedictMetadata::default().into())
    }
}

impl DictionaryBuilder for CcCedictBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        self.build_metadata(output_dir)?;
        let chardef = self.build_character_definition(input_dir, output_dir)?;
        self.build_unknown_dictionary(input_dir, output_dir, &chardef)?;
        self.build_prefix_dictionary(input_dir, output_dir)?;
        self.build_connection_cost_matrix(input_dir, output_dir)?;

        Ok(())
    }

    fn build_metadata(&self, output_dir: &Path) -> LinderaResult<()> {
        MetadataBuilder::new().build(&self.metadata, output_dir)
    }

    fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        CharacterDefinitionBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .compress_algorithm(self.metadata.compress_algorithm)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
    ) -> LinderaResult<()> {
        UnknownDictionaryBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .compress_algorithm(self.metadata.compress_algorithm)
            .unk_fields_num(self.metadata.unk_fields_num)
            .builder()
            .unwrap()
            .build(input_dir, chardef, output_dir)
    }

    fn build_prefix_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(true)
            .encoding(self.metadata.encoding.clone())
            .compress_algorithm(self.metadata.compress_algorithm)
            .skip_invalid_cost_or_id(true)
            .schema(CcCedictSchema::default().into())
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
            .encoding(self.metadata.encoding.clone())
            .compress_algorithm(self.metadata.compress_algorithm)
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    fn build_user_dictionary(&self, input_file: &Path, output_file: &Path) -> LinderaResult<()> {
        let user_dict = self.build_user_dict(input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        UserDictionaryBuilderOptions::default()
            .simple_userdic_fields_num(self.metadata.simple_userdic_fields_num)
            .detailed_userdic_fields_num(self.metadata.detailed_userdic_fields_num)
            .simple_word_cost(self.metadata.simple_word_cost)
            .simple_context_id(self.metadata.simple_context_id)
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