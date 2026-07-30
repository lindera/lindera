pub mod character_definition;
pub mod connection_cost_matrix;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::Dictionary;
use crate::error::LinderaErrorKind;

/// Common trait for all dictionary loaders (both external and embedded)
pub trait DictionaryLoader {
    /// Load dictionary from configured location or embedded data
    fn load(&self) -> LinderaResult<Dictionary> {
        Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "This loader does not support load function"
        )))
    }

    /// Load dictionary from a specific path (optional for embedded loaders)
    fn load_from_path(&self, path: &Path) -> LinderaResult<Dictionary> {
        let _ = path;
        Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "This loader does not support load_from_path function"
        )))
    }
}

pub struct FSDictionaryLoader;

impl Default for FSDictionaryLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FSDictionaryLoader {
    pub fn new() -> Self {
        Self
    }

    /// Load a dictionary from a directory, always doing a plain file read.
    ///
    /// # Arguments
    ///
    /// * `dict_path` - Path to the directory containing dictionary files.
    ///
    /// # Returns
    ///
    /// A `Dictionary`, or an error if loading fails.
    pub fn load_from_path<P: AsRef<Path>>(&self, dict_path: P) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path(dict_path.as_ref())
    }

    /// Load a dictionary from a directory, optionally via memory-mapped
    /// reads. See [`Dictionary::load_from_path_with_options`] for exactly
    /// which components `use_mmap` does and does not make lazy.
    ///
    /// # Arguments
    ///
    /// * `dict_path` - Path to the directory containing dictionary files.
    /// * `use_mmap` - Whether to route the connection-cost matrix and
    ///   prefix dictionary through memory-mapped reads.
    ///
    /// # Returns
    ///
    /// A `Dictionary`, or an error if loading fails.
    pub fn load_from_path_with_options<P: AsRef<Path>>(
        &self,
        dict_path: P,
        use_mmap: bool,
    ) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path_with_options(dict_path.as_ref(), use_mmap)
    }
}

impl DictionaryLoader for FSDictionaryLoader {
    fn load_from_path(&self, dict_path: &Path) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path(dict_path)
    }
}
