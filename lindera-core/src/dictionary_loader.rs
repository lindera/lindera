pub mod character_definition;
pub mod connection_cost_matrix;
pub mod prefix_dictionary;
pub mod unknown_dictionary;

use std::path::Path;

use crate::dictionary::{Dictionary, UserDictionary};
use crate::LinderaResult;

pub trait DictionaryLoader {
    fn load_dictionary(&self, input_dir: &Path) -> LinderaResult<Dictionary>;
    fn load_user_dictionary(&self, input_file: &Path) -> LinderaResult<UserDictionary>;
}
