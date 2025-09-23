use std::path::Path;

use crate::LinderaResult;
use crate::builder::DictionaryBuilder;
use crate::dictionary::UserDictionary;
use crate::util::read_file;

/// Loader for user dictionaries with support for different input formats
pub struct UserDictionaryLoader;

impl UserDictionaryLoader {
    /// Load user dictionary from a binary (.bin) file
    pub fn load_from_bin<P: AsRef<Path>>(path: P) -> LinderaResult<UserDictionary> {
        let data = read_file(path.as_ref())?;
        UserDictionary::load(&data)
    }

    /// Load user dictionary from a CSV file
    /// Requires a DictionaryBuilder to build the user dictionary from CSV format
    pub fn load_from_csv<P: AsRef<Path>>(
        builder: DictionaryBuilder,
        path: P,
    ) -> LinderaResult<UserDictionary> {
        builder.build_user_dict(path.as_ref())
    }
}
