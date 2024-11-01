use std::path::Path;

use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::util::read_file;
use crate::LinderaResult;

pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    pub fn load(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let da_data = read_file(input_dir.join("dict.da").as_path())?;
        let vals_data = read_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = read_file(input_dir.join("dict.words").as_path())?;

        Ok(PrefixDictionary::load(
            da_data.as_slice(),
            vals_data.as_slice(),
            words_idx_data.as_slice(),
            words_data.as_slice(),
        ))
    }
}
