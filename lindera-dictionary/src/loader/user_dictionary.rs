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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::UserDictionaryLoader;

    /// Every checked-in prebuilt user dictionary fixture must stay loadable.
    /// A serialization format change in a dependency (e.g. the daachorse 4.0
    /// automaton format change) invalidates these binaries; this test makes
    /// such breakage visible for all fixtures, not just the ones used by
    /// other tests.
    #[test]
    fn test_load_all_prebuilt_bin_fixtures() {
        let user_dict_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict");

        for name in [
            "ipadic_simple_userdic.bin",
            "unidic_simple_userdic.bin",
            "cc-cedict_simple_userdic.bin",
            "jieba_simple_userdic.bin",
            "ko-dic_simple_userdic.bin",
        ] {
            let result = UserDictionaryLoader::load_from_bin(user_dict_dir.join(name));
            assert!(
                result.is_ok(),
                "failed to load prebuilt user dictionary fixture {name}: {:?}",
                result.err()
            );
        }
    }
}
