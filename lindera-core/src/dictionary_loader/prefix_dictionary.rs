use std::path::Path;

use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::dictionary_builder::utils::read_file;
use crate::LinderaResult;

pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    pub fn load(&self, input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let da_data = read_file(input_dir.join("da.bin").as_path())?;
        let vals_data = read_file(input_dir.join("vals.bin").as_path())?;
        let words_idx_data = read_file(input_dir.join("words_idx.bin").as_path())?;
        let words_data = read_file(input_dir.join("words.bin").as_path())?;

        Ok(PrefixDictionary::from_static_slice(
            da_data.as_slice(),
            vals_data.as_slice(),
            words_idx_data.as_slice(),
            words_data.as_slice(),
        ))
    }
}
