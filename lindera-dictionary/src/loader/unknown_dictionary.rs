use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::util::read_file;

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
        let raw_data = read_file(input_dir.join("unk.bin").as_path())?;

        let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
        aligned_data.extend_from_slice(&raw_data);

        UnknownDictionary::load(&aligned_data)
    }
}
