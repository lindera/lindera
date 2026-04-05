use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::util::read_file;

/// Loader for character definition data from disk files.
pub struct CharacterDefinitionLoader {}

impl CharacterDefinitionLoader {
    /// Load character definition from a file in the specified directory.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Path to the directory containing char_def.bin.
    ///
    /// # Returns
    ///
    /// A `CharacterDefinition` loaded from the file.
    pub fn load(input_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let raw_data = read_file(input_dir.join("char_def.bin").as_path())?;

        let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
        aligned_data.extend_from_slice(&raw_data);

        CharacterDefinition::load(&aligned_data)
    }
}
