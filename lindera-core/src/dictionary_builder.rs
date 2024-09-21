pub mod cc_cedict;
pub mod chardef;
pub mod cost_matrix;
pub mod dict;
pub mod ipadic;
pub mod ipadic_neologd;
pub mod ko_dic;
pub mod unidic;
pub mod unk;
pub mod user_dict;
pub mod utils;

pub use chardef::CharDefBuilderOptions;
pub use cost_matrix::CostMatrixBuilderOptions;
pub use dict::DictBuilderOptions;
pub use unk::UnkBuilderOptions;
pub use user_dict::{build_user_dictionary, UserDictBuilderOptions};

use std::path::Path;

use crate::dictionary::character_definition::CharacterDefinitions;
use crate::dictionary::UserDictionary;
use crate::LinderaResult;

pub trait DictionaryBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_user_dictionary(&self, input_path: &Path, output_path: &Path) -> LinderaResult<()>;
    fn build_chardef(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinitions>;
    fn build_unk(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinitions,
        output_dir: &Path,
    ) -> LinderaResult<()>;
    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()>;
    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary>;
}
