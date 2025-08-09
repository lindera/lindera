pub mod character_definition;
pub mod connection_cost_matrix;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unknown_dictionary;

use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::Dictionary;
use crate::error::LinderaErrorKind;

/// Common trait for all dictionary loaders (both external and embedded)
pub trait DictionaryLoader {
    /// Load dictionary from default location or embedded data
    fn load(&self) -> LinderaResult<Dictionary>;

    /// Load dictionary from a specific path (optional for embedded loaders)
    fn load_from_path(&self, path: &Path) -> LinderaResult<Dictionary> {
        // Default implementation for embedded loaders that don't support path loading
        let _ = path;
        Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "This loader does not support loading from a specific path"
        )))
    }
}

pub struct StandardDictionaryLoader {
    dictionary_name: String,
    search_paths: Vec<String>,
    env_var_name: String,
}

impl StandardDictionaryLoader {
    pub fn new(dictionary_name: String, search_paths: Vec<String>, env_var_name: String) -> Self {
        Self {
            dictionary_name,
            search_paths,
            env_var_name,
        }
    }

    pub fn load_from_path<P: AsRef<Path>>(&self, dict_path: P) -> LinderaResult<Dictionary> {
        Dictionary::load_from_path(dict_path.as_ref())
    }

    pub fn load(&self) -> LinderaResult<Dictionary> {
        // Search for dictionary in common locations
        for path in &self.search_paths {
            let dict_path = Path::new(path);
            if dict_path.exists() && dict_path.is_dir() {
                return self.load_from_path(dict_path);
            }
        }

        // If environment variable is set, use that
        if let Ok(dict_path) = std::env::var(&self.env_var_name) {
            let path = Path::new(&dict_path);
            if path.exists() {
                return self.load_from_path(path);
            }
        }

        Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "{} dictionary not found. Please set {} environment variable or place dictionary files in one of these locations: {}",
            self.dictionary_name, self.env_var_name, self.search_paths.join(", ")
        )))
    }

    pub fn load_user_dictionary<P: AsRef<Path>>(
        &self,
        input_file: P,
    ) -> LinderaResult<crate::dictionary::UserDictionary> {
        let data = crate::util::read_file(input_file.as_ref())?;
        crate::dictionary::UserDictionary::load(&data)
    }
}

impl DictionaryLoader for StandardDictionaryLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        // Search for dictionary in common locations
        for path in &self.search_paths {
            let dict_path = Path::new(path);
            if dict_path.exists() && dict_path.is_dir() {
                return self.load_from_path(dict_path);
            }
        }

        // If environment variable is set, use that
        if let Ok(dict_path) = std::env::var(&self.env_var_name) {
            let path = Path::new(&dict_path);
            if path.exists() {
                return self.load_from_path(path);
            }
        }

        Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
            "{} dictionary not found. Please set {} environment variable or place dictionary files in one of these locations: {}",
            self.dictionary_name, self.env_var_name, self.search_paths.join(", ")
        )))
    }

    fn load_from_path(&self, dict_path: &Path) -> LinderaResult<Dictionary> {
        // StandardDictionaryLoader always uses the default (non-mmap) loading
        // Users can control mmap usage through config or explicit API calls
        Dictionary::load_from_path(dict_path)
    }
}
