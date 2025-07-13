use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::metadata::Metadata;
use crate::error::LinderaErrorKind;
#[cfg(feature = "mmap")]
use crate::util::mmap_file;
use crate::util::read_file;

/// MetadataLoader is a loader for reading persisted metadata files.
///
/// # Examples
///
/// ```rust
/// use lindera_dictionary::dictionary_loader::MetadataLoader;
/// use std::path::Path;
///
/// // Normal loading
/// let metadata = MetadataLoader::load(Path::new("/path/to/dictionary"))?;
///
/// // Memory-mapped loading (when mmap feature is enabled)
/// #[cfg(feature = "mmap")]
/// let metadata = MetadataLoader::load_mmap(Path::new("/path/to/dictionary"))?;
/// ```
pub struct MetadataLoader {}

impl MetadataLoader {
    /// Loads metadata file (metadata.json) from the specified directory.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing the metadata file
    ///
    /// # Returns
    ///
    /// The loaded Metadata object, or an error
    ///
    /// # Errors
    ///
    /// Returns an error if file reading fails or deserialization fails.
    pub fn load(input_dir: &Path) -> LinderaResult<Metadata> {
        let data = read_file(input_dir.join("metadata.json").as_path())?;

        let metadata: Metadata = serde_json::from_slice(&data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

        Ok(metadata)
    }

    /// Loads metadata file using memory mapping.
    ///
    /// This method is only available when the "mmap" feature is enabled.
    /// It's useful for efficiently reading large files.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing the metadata file
    ///
    /// # Returns
    ///
    /// The loaded Metadata object, or an error
    #[cfg(feature = "mmap")]
    pub fn load_mmap(input_dir: &Path) -> LinderaResult<Metadata> {
        let data = mmap_file(input_dir.join("metadata.json").as_path())?;

        let metadata: Metadata = serde_json::from_slice(&data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary::metadata::Metadata;
    use crate::dictionary_builder::metadata::MetadataBuilder;

    use super::*;

    #[test]
    fn test_metadata_load() {
        // Note: This is an integration test that uses the actual file system
        // In a real environment, we recommend using the tempfile crate

        let temp_path = std::env::temp_dir().join("lindera_test_metadata");
        std::fs::create_dir_all(&temp_path).unwrap();

        // Create a sample metadata object
        let metadata = Metadata::default();

        // Create test metadata file
        let metadata_builder = MetadataBuilder::new();
        metadata_builder.build(&metadata, &temp_path).unwrap();

        // Load from file
        let loaded_metadata = MetadataLoader::load(&temp_path).unwrap();

        assert_eq!(loaded_metadata.encoding, "UTF-8");
        assert_eq!(loaded_metadata.simple_word_cost, -10000);
        assert_eq!(loaded_metadata.simple_context_id, 0);
        assert_eq!(loaded_metadata.simple_userdic_fields_num, 3);
        assert_eq!(loaded_metadata.detailed_userdic_fields_num, 13);
        assert_eq!(loaded_metadata.unk_fields_num, 11);
        // Cleanup
        std::fs::remove_dir_all(&temp_path).ok();
    }

    #[test]
    fn test_metadata_load_nonexistent_file() {
        let temp_path = std::env::temp_dir().join("lindera_test_nonexistent");

        // Try to load non-existent file (should return error)
        let result = MetadataLoader::load(&temp_path);
        assert!(result.is_err());
    }
}
