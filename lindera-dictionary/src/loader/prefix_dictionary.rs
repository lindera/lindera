use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

/// Loader for prefix dictionary data from disk files.
pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    /// Load prefix dictionary from files in the specified directory.
    ///
    /// Reads dict.trie, dict.valsidx, dict.vals, dict.wordsidx and dict.words
    /// and constructs a PrefixDictionary.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing dictionary files.
    ///
    /// # Returns
    ///
    /// A `PrefixDictionary` loaded from the files.
    pub fn load(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let trie_data = read_file(input_dir.join("dict.trie").as_path())?;
        let vals_idx = read_file(input_dir.join("dict.valsidx").as_path())?;
        let vals_data = read_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = read_file(input_dir.join("dict.words").as_path())?;

        PrefixDictionary::load(trie_data, vals_idx, vals_data, words_idx_data, words_data)
    }

    /// Load prefix dictionary using memory-mapped files.
    ///
    /// Every component stays genuinely mmap-backed: the trie is walked in
    /// place over its serialized bytes, and the values/word files are read
    /// lazily at lookup time via `Data::Map`. Loading costs an O(1) header
    /// check, no decode and no anonymous memory.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing dictionary files.
    ///
    /// # Returns
    ///
    /// A `PrefixDictionary` loaded via memory mapping.
    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let trie_data = mmap_file(input_dir.join("dict.trie").as_path())?;
        let vals_idx = mmap_file(input_dir.join("dict.valsidx").as_path())?;
        let vals_data = mmap_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = mmap_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = mmap_file(input_dir.join("dict.words").as_path())?;

        PrefixDictionary::load(trie_data, vals_idx, vals_data, words_idx_data, words_data)
    }
}
