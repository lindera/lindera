pub mod cc_cedict;
pub mod character_definition;
pub mod connection_cost_matrix;
pub mod ipadic;
pub mod ipadic_neologd;
pub mod ko_dic;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unidic;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::path::Path;

pub use character_definition::CharacterDefinitionBuilderOptions;
pub use connection_cost_matrix::ConnectionCostMatrixBuilderOptions;
pub use prefix_dictionary::PrefixDictionaryBuilderOptions;
pub use unknown_dictionary::UnknownDictionaryBuilderOptions;
pub use user_dictionary::{UserDictionaryBuilderOptions, build_user_dictionary};

// Re-export DictionarySchema from its new location
pub use crate::dictionary::schema::Schema;

use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::metadata::Metadata;

pub trait DictionaryBuilder {
    fn build_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_user_dictionary(
        &self,
        metadata: &Metadata,
        input_path: &Path,
        output_path: &Path,
    ) -> LinderaResult<()>;
    fn build_metadata(&self, metadata: &Metadata, output_dir: &Path) -> LinderaResult<()>;
    fn build_character_definition(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition>;
    fn build_unknown_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
    ) -> LinderaResult<()>;
    fn build_prefix_dictionary(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_connection_cost_matrix(
        &self,
        metadata: &Metadata,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_user_dict(
        &self,
        metadata: &Metadata,
        input_file: &Path,
    ) -> LinderaResult<UserDictionary>;
}
