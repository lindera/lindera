use std::path::Path;

#[cfg(feature = "compress")]
use crate::decompress::decompress;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
#[cfg(feature = "compress")]
use crate::error::LinderaErrorKind;
use crate::util::read_file;
use crate::LinderaResult;

pub struct PrefixDictionaryLoader {}

impl PrefixDictionaryLoader {
    #[allow(unused_mut)]
    pub fn load(input_dir: &Path) -> LinderaResult<PrefixDictionary> {
        let mut da_data = read_file(input_dir.join("dict.da").as_path())?;
        let mut vals_data = read_file(input_dir.join("dict.vals").as_path())?;
        let mut words_idx_data = read_file(input_dir.join("dict.wordsidx").as_path())?;
        let mut words_data = read_file(input_dir.join("dict.words").as_path())?;

        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(da_data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            da_data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }
        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(vals_data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            vals_data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }
        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(words_idx_data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            words_idx_data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }
        #[cfg(feature = "compress")]
        {
            let compressed_data = bincode::deserialize_from(words_data.as_slice())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
            words_data = decompress(compressed_data)
                .map_err(|err| LinderaErrorKind::Decompress.with_error(err))?;
        }

        Ok(PrefixDictionary::load(
            da_data.as_slice(),
            vals_data.as_slice(),
            words_idx_data.as_slice(),
            words_data.as_slice(),
        ))
    }
}
