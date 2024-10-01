pub mod cc_cedict;
pub mod character_definition;
pub mod connection_cost_matrix;
pub mod ipadic;
pub mod ipadic_neologd;
pub mod ko_dic;
pub mod prefix_dictionary;
pub mod unidic;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::path::Path;

pub use character_definition::CharacterDefinitionBuilderOptions;
pub use connection_cost_matrix::ConnectionCostMatrixBuilderOptions;
pub use prefix_dictionary::PrefixDictionaryBuilderOptions;
pub use unknown_dictionary::UnknownDictionaryBuilderOptions;
pub use user_dictionary::{build_user_dictionary, UserDictionaryBuilderOptions};

use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::UserDictionary;
use crate::LinderaResult;

pub trait DictionaryBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_user_dictionary(&self, input_path: &Path, output_path: &Path) -> LinderaResult<()>;
    fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition>;
    fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinition,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_prefix_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_connection_cost_matrix(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary>;
}
