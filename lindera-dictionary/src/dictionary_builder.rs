pub mod character_definition;
pub mod connection_cost_matrix;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::fs;
use std::path::Path;

use csv::StringRecord;

use self::character_definition::CharacterDefinitionBuilderOptions;
use self::connection_cost_matrix::ConnectionCostMatrixBuilderOptions;
use self::metadata::MetadataBuilder;
use self::prefix_dictionary::PrefixDictionaryBuilderOptions;
use self::unknown_dictionary::UnknownDictionaryBuilderOptions;
use self::user_dictionary::{UserDictionaryBuilderOptions, build_user_dictionary};
use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::metadata::Metadata;
use crate::error::LinderaErrorKind;

#[derive(Clone)]
pub struct DictionaryBuilder {
    metadata: Metadata,
}

impl DictionaryBuilder {
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }

    pub fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        self.build_metadata(output_dir)?;
        let chardef = self.build_character_definition(input_dir, output_dir)?;
        self.build_unknown_dictionary(input_dir, output_dir, &chardef)?;
        self.build_prefix_dictionary(input_dir, output_dir)?;
        self.build_connection_cost_matrix(input_dir, output_dir)?;

        Ok(())
    }

    pub fn build_metadata(&self, output_dir: &Path) -> LinderaResult<()> {
        MetadataBuilder::new().build(&self.metadata, output_dir)
    }

    pub fn build_character_definition(
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

    pub fn build_unknown_dictionary(
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

    pub fn build_prefix_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(self.metadata.flexible_csv)
            .encoding(self.metadata.encoding.clone())
            .compress_algorithm(self.metadata.compress_algorithm)
            .skip_invalid_cost_or_id(self.metadata.skip_invalid_cost_or_id)
            .normalize_details(self.metadata.normalize_details)
            .schema(self.metadata.schema.clone())
            .builder()
            .unwrap()
            .build(input_dir, output_dir)
    }

    pub fn build_connection_cost_matrix(
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

    pub fn build_user_dictionary(
        &self,
        input_file: &Path,
        output_file: &Path,
    ) -> LinderaResult<()> {
        let user_dict = self.build_user_dict(input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    pub fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        let indices = self.metadata.userdic_field_indices.clone();
        UserDictionaryBuilderOptions::default()
            .simple_userdic_fields_num(self.metadata.simple_userdic_fields_num)
            .detailed_userdic_fields_num(self.metadata.detailed_userdic_fields_num)
            .simple_word_cost(self.metadata.simple_word_cost)
            .simple_context_id(self.metadata.simple_context_id)
            .flexible_csv(self.metadata.flexible_csv)
            .simple_userdic_details_handler(Some(Box::new(move |row: &StringRecord| {
                Ok(indices
                    .iter()
                    .map(|idx| match idx {
                        Some(i) => row[*i].to_string(),
                        None => "*".to_string(),
                    })
                    .collect())
            })))
            .builder()
            .unwrap()
            .build(input_file)
    }
}
