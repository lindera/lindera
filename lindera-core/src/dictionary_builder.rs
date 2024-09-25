pub mod cc_cedict;
pub mod character_definition;
pub mod connection_cost_matrix;
pub mod ipadic;
pub mod ipadic_neologd;
pub mod ko_dic;
pub mod prefix_dictionary;
pub mod unidic;
pub mod unknown_dictionary;
pub mod user_dict;
pub mod utils;

pub use character_definition::CharDefBuilderOptions;
pub use connection_cost_matrix::CostMatrixBuilderOptions;
pub use prefix_dictionary::DictBuilderOptions;
pub use unknown_dictionary::UnkBuilderOptions;
pub use user_dict::{build_user_dictionary, UserDictBuilderOptions};

use std::path::Path;

use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::UserDictionary;
use crate::LinderaResult;

pub trait DictionaryBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_user_dictionary(&self, input_path: &Path, output_path: &Path) -> LinderaResult<()>;
    fn build_chardef(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition>;
    fn build_unk(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinition,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary>;
}
