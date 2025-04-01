use std::path::Path;

#[cfg(feature = "compress")]
use bincode::config::standard;
#[cfg(feature = "compress")]
use bincode::serde::decode_from_slice;

#[cfg(feature = "compress")]
use crate::decompress::decompress;
#[cfg(feature = "compress")]
use crate::decompress::CompressedData;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;
use crate::LinderaResult;

pub struct UnknownDictionaryLoader {}

impl UnknownDictionaryLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<UnknownDictionary> {
        let mut data = read_file(input_dir.join("unk.bin").as_path())?;

        #[cfg(feature = "compress")]
        {
            let (compressed_data, _): (CompressedData, usize) =
                decode_from_slice(&data, standard())
                    .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        UnknownDictionary::load(data.as_slice())
    }
}
