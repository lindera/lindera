use std::path::Path;

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;

pub struct UnknownDictionaryLoader {}

impl UnknownDictionaryLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<UnknownDictionary> {
        let mut data = read_file(input_dir.join("unk.bin").as_path())?;

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

        UnknownDictionary::load(data.as_slice())
    }
}
