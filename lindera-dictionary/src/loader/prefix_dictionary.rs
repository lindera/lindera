use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::prefix_dictionary::{DaTrust, PrefixDictionary};
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

/// Loader for prefix dictionary data from disk files.
pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    /// Load prefix dictionary from files in the specified directory.
    ///
    /// Reads dict.da, dict.vals, dict.wordsidx, and dict.words files
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
        let da_data = read_file(input_dir.join("dict.da").as_path())?;
        let vals_data = read_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = read_file(input_dir.join("dict.words").as_path())?;

        PrefixDictionary::load(
            da_data,
            vals_data,
            words_idx_data,
            words_data,
            true,
            DaTrust::Untrusted,
        )
    }

    /// Load prefix dictionary using memory-mapped files.
    ///
    /// Note: only `vals_data`/`words_idx_data`/`words_data` end up genuinely
    /// mmap-backed (read lazily at lookup time via `Data::Map`). `da_data` is
    /// still eagerly copied into an owned `DoubleArrayAhoCorasick` by
    /// [`PrefixDictionary::load`] — daachorse has no zero-copy deserialization
    /// API — so mmap only avoids the initial file-read syscall/allocation for
    /// that component, not runtime lazy paging. mmap'd bytes are passed with
    /// [`DaTrust::Untrusted`], the same as a plain read: they can be
    /// corrupted or tampered with just as easily as a regular file.
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
        let da_data = mmap_file(input_dir.join("dict.da").as_path())?;
        let vals_data = mmap_file(input_dir.join("dict.vals").as_path())?;
        let words_idx_data = mmap_file(input_dir.join("dict.wordsidx").as_path())?;
        let words_data = mmap_file(input_dir.join("dict.words").as_path())?;

        PrefixDictionary::load(
            da_data,
            vals_data,
            words_idx_data,
            words_data,
            true,
            DaTrust::Untrusted,
        )
    }
}
