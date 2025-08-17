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
            .schema(self.metadata.dictionary_schema.clone())
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
        let userdic_schema = self.metadata.user_dictionary_schema.clone();
        let dict_schema = self.metadata.dictionary_schema.clone();
        let default_field_value = self.metadata.default_field_value.clone();

        UserDictionaryBuilderOptions::default()
            .user_dictionary_fields_num(self.metadata.user_dictionary_schema.field_count())
            .dictionary_fields_num(self.metadata.dictionary_schema.field_count())
            .default_word_cost(self.metadata.default_word_cost)
            .default_left_context_id(self.metadata.default_left_context_id)
            .default_right_context_id(self.metadata.default_right_context_id)
            .flexible_csv(self.metadata.flexible_csv)
            .user_dictionary_handler(Some(Box::new(move |row: &StringRecord| {
                // Map user dictionary fields to dictionary schema fields
                let mut result = Vec::new();

                // Skip the first 4 common fields (surface, left_id, right_id, cost)
                for field_name in dict_schema.get_custom_fields() {
                    if let Some(idx) = userdic_schema.get_field_index(field_name) {
                        // If field exists in user dictionary schema, get value from CSV
                        if idx < row.len() {
                            result.push(row[idx].to_string());
                        } else {
                            result.push(default_field_value.clone());
                        }
                    } else {
                        // Field not in user dictionary schema, use default value
                        result.push(default_field_value.clone());
                    }
                }

                Ok(result)
            })))
            .builder()
            .unwrap()
            .build(input_file)
    }
}
