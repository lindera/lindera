use std::path::Path;

use crate::character_definition::CharacterDefinitions;
use crate::user_dictionary::UserDictionary;
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
