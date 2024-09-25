use std::path::Path;

use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::dictionary_builder::utils::read_file;
use crate::LinderaResult;

pub struct UnknownDictionaryLoader {}

impl UnknownDictionaryLoader {
    pub fn load(&self, input_dir: &Path) -> LinderaResult<UnknownDictionary> {
        let path = input_dir.join("unk.bin");
        let data = read_file(path.as_path())?;

        UnknownDictionary::load(data.as_slice())
    }
}
