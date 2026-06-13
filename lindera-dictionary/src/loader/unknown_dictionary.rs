use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::util::read_aligned_file;

/// Loader for unknown dictionary data from disk files.
pub struct UnknownDictionaryLoader {}

impl UnknownDictionaryLoader {
    /// Load unknown dictionary from a file in the specified directory.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing unk.bin.
    ///
    /// # Returns
    ///
    /// An `UnknownDictionary` loaded from the file.
    pub fn load(input_dir: &Path) -> LinderaResult<UnknownDictionary> {
        let aligned_data = read_aligned_file(input_dir.join("unk.bin").as_path())?;

        UnknownDictionary::load(&aligned_data)
    }
}
