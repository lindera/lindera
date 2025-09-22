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

    pub fn load_from_path<P: AsRef<Path>>(&self, dict_path: P) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path(dict_path.as_ref())
    }
}

impl DictionaryLoader for FSDictionaryLoader {
    fn load_from_path(&self, dict_path: &Path) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path(dict_path)
    }
}
