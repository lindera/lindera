use std::path::Path;

use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary_builder::utils::read_file;
use crate::LinderaResult;

pub struct CharacterDefinitionLoader {}

impl CharacterDefinitionLoader {
    pub fn load(&self, input_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let path = input_dir.join("char_def.bin");
        let data = read_file(path.as_path())?;

        CharacterDefinition::load(data.as_slice())
    }
}
