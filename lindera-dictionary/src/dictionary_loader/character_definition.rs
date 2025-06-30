use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::character_definition::CharacterDefinition;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;

pub struct CharacterDefinitionLoader {}

impl CharacterDefinitionLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let mut data = read_file(input_dir.join("char_def.bin").as_path())?;

        #[cfg(feature = "compress")]
        {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(data.as_slice(), bincode::config::legacy())
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err))
                    })?;
            data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        CharacterDefinition::load(data.as_slice())
    }
}
