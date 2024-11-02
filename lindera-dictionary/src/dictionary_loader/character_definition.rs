use std::path::Path;

#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::character_definition::CharacterDefinition;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;
use crate::LinderaResult;

pub struct CharacterDefinitionLoader {}

impl CharacterDefinitionLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let mut data = read_file(input_dir.join("char_def.bin").as_path())?;

        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        CharacterDefinition::load(data.as_slice())
    }
}
