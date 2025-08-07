pub mod character_definition;
pub mod connection_cost_matrix;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::path::Path;


use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;

pub trait DictionaryBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_metadata(&self, output_dir: &Path) -> LinderaResult<()>;
    fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition>;
    fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
    ) -> LinderaResult<()>;
    fn build_prefix_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_connection_cost_matrix(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_user_dictionary(&self, input_path: &Path, output_path: &Path) -> LinderaResult<()>;
    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary>;
}
