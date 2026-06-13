use std::path::Path;

use crate::LinderaResult;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::util::read_aligned_file;

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
        let aligned_data = read_aligned_file(input_dir.join("char_def.bin").as_path())?;

        CharacterDefinition::load(&aligned_data)
    }
}
