use std::path::{Path, PathBuf};
use std::str::FromStr;

use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use url::Url;

#[cfg(feature = "embed-cc-cedict")]
use lindera_cc_cedict::DICTIONARY_NAME as CC_CEDICT_DICTIONARY_NAME;
#[cfg(feature = "embed-cc-cedict")]
use lindera_cc_cedict::embedded::EmbeddedCcCedictLoader;
use lindera_dictionary::loader::DictionaryLoader;
use lindera_dictionary::loader::FSDictionaryLoader;
use lindera_dictionary::loader::user_dictionary::UserDictionaryLoader;
#[cfg(feature = "embed-ipadic")]
use lindera_ipadic::DICTIONARY_NAME as IPADIC_DICTIONARY_NAME;
#[cfg(feature = "embed-ipadic")]
use lindera_ipadic::embedded::EmbeddedIPADICLoader;
#[cfg(feature = "embed-ipadic-neologd")]
use lindera_ipadic_neologd::DICTIONARY_NAME as IPADIC_NEOLOGD_DICTIONARY_NAME;
#[cfg(feature = "embed-ipadic-neologd")]
use lindera_ipadic_neologd::embedded::EmbeddedIPADICNEologdLoader;
#[cfg(feature = "embed-jieba")]
use lindera_jieba::DICTIONARY_NAME as JIEBA_DICTIONARY_NAME;
#[cfg(feature = "embed-jieba")]
use lindera_jieba::embedded::EmbeddedJiebaLoader;
#[cfg(feature = "embed-ko-dic")]
use lindera_ko_dic::DICTIONARY_NAME as KO_DIC_DICTIONARY_NAME;
#[cfg(feature = "embed-ko-dic")]
use lindera_ko_dic::embedded::EmbeddedKoDicLoader;
#[cfg(feature = "embed-sudachidict")]
use lindera_sudachidict::DICTIONARY_NAME as SUDACHIDICT_DICTIONARY_NAME;
#[cfg(feature = "embed-sudachidict")]
use lindera_sudachidict::embedded::EmbeddedSudachiDictLoader;
#[cfg(feature = "train")]
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
pub use lindera_trainer as trainer;
#[cfg(feature = "embed-unidic")]
use lindera_unidic::DICTIONARY_NAME as UNIDIC_DICTIONARY_NAME;
#[cfg(feature = "embed-unidic")]
use lindera_unidic::embedded::EmbeddedUniDicLoader;

use crate::LinderaResult;
use crate::error::{LinderaError, LinderaErrorKind};

// Low-level building blocks of `lindera-dictionary`, exposed inside this
// module so facade users reach them as
// `lindera::dictionary::core::prefix_dictionary::PrefixDictionary` and
// `lindera::dictionary::builder::DictionaryBuilder`. `no_inline` keeps
// rustdoc from duplicating the whole crate under `dictionary`.
#[doc(no_inline)]
pub use lindera_dictionary::dictionary as core;
#[doc(no_inline)]
pub use lindera_dictionary::{builder, error, loader, mode, nbest, space_penalty, util, viterbi};
#[doc(no_inline)]
pub use lindera_dictionary::{embedded_dictionary, include_bytes_aligned};

pub use lindera_dictionary::builder::DictionaryBuilder;
pub use lindera_dictionary::dictionary::metadata::Metadata;
pub use lindera_dictionary::dictionary::schema::{FieldDefinition, FieldType, Schema};
pub use lindera_dictionary::dictionary::{Dictionary, UserDictionary};
pub use lindera_dictionary::viterbi::{Lattice, WordId};

pub type DictionaryConfig = Value;
pub type UserDictionaryConfig = Value;

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryScheme {
    #[cfg(any(
        feature = "embed-ipadic",
        feature = "embed-ipadic-neologd",
        feature = "embed-unidic",
        feature = "embed-sudachidict",
        feature = "embed-ko-dic",
        feature = "embed-cc-cedict",
        feature = "embed-jieba",
    ))]
    #[serde(rename = "embedded")]
    Embedded,
    #[serde(rename = "file")]
    File,
}

impl DictionaryScheme {
    pub fn as_str(&self) -> &str {
        match self {
            #[cfg(any(
                feature = "embed-ipadic",
                feature = "embed-ipadic-neologd",
                feature = "embed-unidic",
                feature = "embed-sudachidict",
                feature = "embed-ko-dic",
                feature = "embed-cc-cedict",
                feature = "embed-jieba",
            ))]
            DictionaryScheme::Embedded => "embedded",
            DictionaryScheme::File => "file",
        }
    }
}

impl FromStr for DictionaryScheme {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryScheme, Self::Err> {
        match input {
            #[cfg(any(
                feature = "embed-ipadic",
                feature = "embed-ipadic-neologd",
                feature = "embed-unidic",
                feature = "embed-sudachidict",
                feature = "embed-ko-dic",
                feature = "embed-cc-cedict",
                feature = "embed-jieba",
            ))]
            "embedded" => Ok(DictionaryScheme::Embedded),
            "file" => Ok(DictionaryScheme::File),
            _ => Err(LinderaErrorKind::Dictionary
                .with_error(anyhow::anyhow!("Invalid dictionary scheme: {input}"))),
        }
    }
}

#[derive(Debug, Clone, EnumIter, Deserialize, Serialize, PartialEq, Eq)]
pub enum DictionaryKind {
    #[cfg(feature = "embed-ipadic")]
    #[serde(rename = "ipadic")]
    IPADIC,
    #[cfg(feature = "embed-ipadic-neologd")]
    #[serde(rename = "ipadic-neologd")]
    IPADICNEologd,
    #[cfg(feature = "embed-unidic")]
    #[serde(rename = "unidic")]
    UniDic,
    #[cfg(feature = "embed-sudachidict")]
    #[serde(rename = "sudachidict")]
    SudachiDict,
    #[cfg(feature = "embed-ko-dic")]
    #[serde(rename = "ko-dic")]
    KoDic,
    #[cfg(feature = "embed-cc-cedict")]
    #[serde(rename = "cc-cedict")]
    CcCedict,
    #[cfg(feature = "embed-jieba")]
    #[serde(rename = "jieba")]
    Jieba,
}

impl DictionaryKind {
    pub fn variants() -> Vec<DictionaryKind> {
        DictionaryKind::iter().collect::<Vec<_>>()
    }

    pub fn contained_variants() -> Vec<DictionaryKind> {
        DictionaryKind::variants()
            .into_iter()
            .filter(|kind| match kind {
                #[cfg(feature = "embed-ipadic")]
                DictionaryKind::IPADIC => cfg!(feature = "embed-ipadic"),
                #[cfg(feature = "embed-ipadic-neologd")]
                DictionaryKind::IPADICNEologd => cfg!(feature = "embed-ipadic-neologd"),
                #[cfg(feature = "embed-unidic")]
                DictionaryKind::UniDic => cfg!(feature = "embed-unidic"),
                #[cfg(feature = "embed-sudachidict")]
                DictionaryKind::SudachiDict => cfg!(feature = "embed-sudachidict"),
                #[cfg(feature = "embed-ko-dic")]
                DictionaryKind::KoDic => cfg!(feature = "embed-ko-dic"),
                #[cfg(feature = "embed-cc-cedict")]
                DictionaryKind::CcCedict => cfg!(feature = "embed-cc-cedict"),
                #[cfg(feature = "embed-jieba")]
                DictionaryKind::Jieba => cfg!(feature = "embed-jieba"),
                #[allow(unreachable_patterns)]
                _ => false,
            })
            .collect::<Vec<_>>()
    }

    pub fn as_str(&self) -> &str {
        match self {
            #[cfg(feature = "embed-ipadic")]
            DictionaryKind::IPADIC => IPADIC_DICTIONARY_NAME,
            #[cfg(feature = "embed-ipadic-neologd")]
            DictionaryKind::IPADICNEologd => IPADIC_NEOLOGD_DICTIONARY_NAME,
            #[cfg(feature = "embed-unidic")]
            DictionaryKind::UniDic => UNIDIC_DICTIONARY_NAME,
            #[cfg(feature = "embed-sudachidict")]
            DictionaryKind::SudachiDict => SUDACHIDICT_DICTIONARY_NAME,
            #[cfg(feature = "embed-ko-dic")]
            DictionaryKind::KoDic => KO_DIC_DICTIONARY_NAME,
            #[cfg(feature = "embed-cc-cedict")]
            DictionaryKind::CcCedict => CC_CEDICT_DICTIONARY_NAME,
            #[cfg(feature = "embed-jieba")]
            DictionaryKind::Jieba => JIEBA_DICTIONARY_NAME,
            #[allow(unreachable_patterns)]
            _ => "",
        }
    }
}

impl FromStr for DictionaryKind {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryKind, Self::Err> {
        // Use if-else chain instead of match because const &str values
        // are interpreted as variable bindings in match patterns.
        #[cfg(feature = "embed-ipadic")]
        if input == IPADIC_DICTIONARY_NAME {
            return Ok(DictionaryKind::IPADIC);
        }
        #[cfg(feature = "embed-ipadic-neologd")]
        if input == IPADIC_NEOLOGD_DICTIONARY_NAME {
            return Ok(DictionaryKind::IPADICNEologd);
        }
        #[cfg(feature = "embed-unidic")]
        if input == UNIDIC_DICTIONARY_NAME {
            return Ok(DictionaryKind::UniDic);
        }
        #[cfg(feature = "embed-sudachidict")]
        if input == SUDACHIDICT_DICTIONARY_NAME {
            return Ok(DictionaryKind::SudachiDict);
        }
        #[cfg(feature = "embed-ko-dic")]
        if input == KO_DIC_DICTIONARY_NAME {
            return Ok(DictionaryKind::KoDic);
        }
        #[cfg(feature = "embed-cc-cedict")]
        if input == CC_CEDICT_DICTIONARY_NAME {
            return Ok(DictionaryKind::CcCedict);
        }
        #[cfg(feature = "embed-jieba")]
        if input == JIEBA_DICTIONARY_NAME {
            return Ok(DictionaryKind::Jieba);
        }
        Err(LinderaErrorKind::Dictionary
            .with_error(anyhow::anyhow!("Invalid dictionary kind: {input}")))
    }
}

/// Resolve the embedded loader for a given dictionary kind.
///
/// # Arguments
///
/// * `dictionary_type` - Which embedded dictionary to load.
///
/// # Returns
///
/// A boxed loader for the requested dictionary kind, or an error if the
/// corresponding `embed-*` feature was not compiled in.
pub fn resolve_embedded_loader(
    dictionary_type: DictionaryKind,
) -> LinderaResult<Box<dyn DictionaryLoader>> {
    match dictionary_type {
        #[cfg(feature = "embed-ipadic")]
        DictionaryKind::IPADIC => Ok(Box::new(EmbeddedIPADICLoader::new())),
        #[cfg(feature = "embed-ipadic-neologd")]
        DictionaryKind::IPADICNEologd => Ok(Box::new(EmbeddedIPADICNEologdLoader::new())),
        #[cfg(feature = "embed-unidic")]
        DictionaryKind::UniDic => Ok(Box::new(EmbeddedUniDicLoader::new())),
        #[cfg(feature = "embed-sudachidict")]
        DictionaryKind::SudachiDict => Ok(Box::new(EmbeddedSudachiDictLoader::new())),
        #[cfg(feature = "embed-ko-dic")]
        DictionaryKind::KoDic => Ok(Box::new(EmbeddedKoDicLoader::new())),
        #[cfg(feature = "embed-cc-cedict")]
        DictionaryKind::CcCedict => Ok(Box::new(EmbeddedCcCedictLoader::new())),
        #[cfg(feature = "embed-jieba")]
        DictionaryKind::Jieba => Ok(Box::new(EmbeddedJiebaLoader::new())),
    }
}

/// Load a dictionary from a filesystem directory, always doing a plain file
/// read (no memory mapping).
///
/// # Arguments
///
/// * `path` - Path to the directory containing dictionary files.
///
/// # Returns
///
/// A `Dictionary`, or an error if loading fails.
pub fn load_fs_dictionary(path: &Path) -> LinderaResult<Dictionary> {
    load_fs_dictionary_with_options(path, cfg!(feature = "mmap"))
}

/// Load a dictionary from a filesystem directory, optionally via
/// memory-mapped reads. See
/// [`lindera_dictionary::dictionary::Dictionary::load_from_path_with_options`]
/// for exactly which components `use_mmap` does and does not make lazy.
///
/// # Arguments
///
/// * `path` - Path to the directory containing dictionary files.
/// * `use_mmap` - Whether to route the connection-cost matrix and prefix
///   dictionary through memory-mapped reads.
///
/// # Returns
///
/// A `Dictionary`, or an error if loading fails.
pub fn load_fs_dictionary_with_options(path: &Path, use_mmap: bool) -> LinderaResult<Dictionary> {
    let loader = FSDictionaryLoader::new();
    loader.load_from_path_with_options(path, use_mmap)
}

/// Load an embedded dictionary of the given kind.
///
/// # Arguments
///
/// * `kind` - Which embedded dictionary to load.
///
/// # Returns
///
/// A `Dictionary`, or an error if the corresponding `embed-*` feature was
/// not compiled in or loading otherwise fails.
pub fn load_embedded_dictionary(kind: DictionaryKind) -> LinderaResult<Dictionary> {
    let loader = resolve_embedded_loader(kind)?;
    loader
        .load()
        .map_err(|e| LinderaErrorKind::NotFound.with_error(e))
}

/// Load a dictionary from a URI (`embedded://<kind>`, `file://<path>`) or a
/// bare filesystem path, always doing a plain file read for filesystem
/// dictionaries (no memory mapping).
///
/// # Arguments
///
/// * `uri` - A dictionary URI or bare filesystem path.
///
/// # Returns
///
/// A `Dictionary`, or an error if the URI is invalid or loading fails.
pub fn load_dictionary(uri: &str) -> LinderaResult<Dictionary> {
    load_dictionary_with_options(uri, cfg!(feature = "mmap"))
}

/// Load a dictionary from a URI (`embedded://<kind>`, `file://<path>`) or a
/// bare filesystem path, optionally via memory-mapped reads for filesystem
/// dictionaries. `use_mmap` is silently ignored for `embedded://` URIs,
/// since embedded dictionary data is already a static, zero-copy byte slice
/// baked into the binary.
///
/// # Arguments
///
/// * `uri` - A dictionary URI or bare filesystem path.
/// * `use_mmap` - Whether to route filesystem-loaded dictionaries' connection
///   cost matrix and prefix dictionary through memory-mapped reads.
///
/// # Returns
///
/// A `Dictionary`, or an error if the URI is invalid or loading fails.
pub fn load_dictionary_with_options(uri: &str, use_mmap: bool) -> LinderaResult<Dictionary> {
    // Try to parse as URI first, but only if it looks like a URI
    // (contains "://" or starts with known schemes)
    if uri.contains("://") {
        match Url::parse(uri) {
            Ok(parsed_uri) => {
                // Parse the URI and return the appropriate dictionary
                let scheme = DictionaryScheme::from_str(parsed_uri.scheme()).map_err(|err| {
                    LinderaErrorKind::Dictionary
                        .with_error(anyhow::anyhow!("Invalid dictionary scheme: {err}"))
                })?;

                match scheme {
                    #[cfg(any(
                        feature = "embed-ipadic",
                        feature = "embed-ipadic-neologd",
                        feature = "embed-unidic",
                        feature = "embed-sudachidict",
                        feature = "embed-ko-dic",
                        feature = "embed-cc-cedict",
                        feature = "embed-jieba",
                    ))]
                    DictionaryScheme::Embedded => {
                        let kind = DictionaryKind::from_str(parsed_uri.host_str().unwrap_or(""))
                            .map_err(|err| LinderaErrorKind::Dictionary.with_error(err))?;

                        // Load the embedded dictionary
                        load_embedded_dictionary(kind)
                    }
                    DictionaryScheme::File => {
                        // Extract path from file:// URL manually
                        let path_str = parsed_uri.path();

                        // Handle Windows paths that might start with /C:/ etc.
                        let path_str =
                            if cfg!(windows) && path_str.len() > 1 && path_str.starts_with('/') {
                                &path_str[1..]
                            } else {
                                path_str
                            };

                        // Decode percent-encoded characters
                        let decoded_path =
                            percent_decode_str(path_str).decode_utf8().map_err(|e| {
                                LinderaErrorKind::Dictionary
                                    .with_error(anyhow::anyhow!("Invalid UTF-8 in path: {e}"))
                            })?;

                        let path = Path::new(decoded_path.as_ref());

                        // Load the file-based dictionary
                        load_fs_dictionary_with_options(path, use_mmap)
                    }
                }
            }
            Err(e) => {
                Err(LinderaErrorKind::Dictionary
                    .with_error(anyhow::anyhow!("Invalid URI format: {e}")))
            }
        }
    } else {
        // Treat it as a file path directly
        let path = Path::new(uri);
        load_fs_dictionary_with_options(path, use_mmap)
    }
}

pub fn load_user_dictionary_from_csv(
    metadata: &Metadata,
    path: &Path,
) -> LinderaResult<UserDictionary> {
    let builder = DictionaryBuilder::new(metadata.clone());
    UserDictionaryLoader::load_from_csv(builder, path)
}

pub fn load_user_dictionary_from_bin(path: &Path) -> LinderaResult<UserDictionary> {
    UserDictionaryLoader::load_from_bin(path)
}

/// Load a user dictionary from a URI (`file://<path>`) or a bare filesystem
/// path, by its extension: a `.csv` is built with `metadata`, and a `.bin`
/// is loaded as `lindera build --user` wrote it.
///
/// A `.csv` loaded this way is not checked against a system dictionary's
/// connection matrix until it is combined with one:
/// [`Segmenter::try_new`](crate::segmenter::Segmenter::try_new) then names
/// an entry outside the matrix by its word id. [`load_user_dictionary_for`]
/// checks a `.csv` as it builds it instead, and names the CSV line and
/// surface.
///
/// # Arguments
///
/// * `uri` - A `file://` URI or a bare filesystem path of a `.csv` or `.bin`
///   user dictionary.
/// * `metadata` - The metadata to build a `.csv` with, normally that of the
///   system dictionary the user dictionary is used with.
///
/// # Returns
///
/// The user dictionary, or an error if the URI or the extension is invalid
/// or the file cannot be loaded or built.
pub fn load_user_dictionary(uri: &str, metadata: &Metadata) -> LinderaResult<UserDictionary> {
    load_user_dictionary_with(uri, || DictionaryBuilder::new(metadata.clone()))
}

/// Load a user dictionary for the system dictionary `dictionary` from a URI
/// (`file://<path>`) or a bare filesystem path, as [`load_user_dictionary`]
/// does with `dictionary`'s metadata, and check a `.csv` against
/// `dictionary`'s connection matrix as it is built.
///
/// A `.csv` is built with [`DictionaryBuilder::with_system_matrix`], so a
/// row whose left or right context ID lies outside the matrix fails with an
/// error that names its CSV line and surface, as
/// `lindera build --user --system-dict` reports it. A `.bin` is loaded as it
/// was built; [`Segmenter::try_new`](crate::segmenter::Segmenter::try_new)
/// checks it when it is combined with `dictionary`, and names an entry
/// outside the matrix by its word id.
///
/// # Arguments
///
/// * `uri` - A `file://` URI or a bare filesystem path of a `.csv` or `.bin`
///   user dictionary.
/// * `dictionary` - The system dictionary the user dictionary is used with.
///
/// # Returns
///
/// The user dictionary, or an error if the URI or the extension is invalid,
/// the file cannot be loaded or built, or a `.csv` row has a context ID
/// outside `dictionary`'s connection matrix.
///
/// # Example
///
/// ```
/// use lindera_segmenter::dictionary::{load_dictionary, load_user_dictionary_for};
/// use lindera_segmenter::mode::Mode;
/// use lindera_segmenter::segmenter::Segmenter;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "embed-ipadic")]
/// # {
/// let dictionary = load_dictionary("embedded://ipadic")?;
/// let user_dictionary = load_user_dictionary_for(
///     "../resources/user_dict/ipadic_simple_userdic.csv",
///     &dictionary,
/// )?;
/// let segmenter = Segmenter::try_new(Mode::Normal, dictionary, Some(user_dictionary))?;
/// # }
/// # Ok(())
/// # }
/// ```
pub fn load_user_dictionary_for(
    uri: &str,
    dictionary: &Dictionary,
) -> LinderaResult<UserDictionary> {
    load_user_dictionary_with(uri, || {
        DictionaryBuilder::new((*dictionary.metadata).clone())
            .with_system_matrix(&dictionary.connection_cost_matrix)
    })
}

/// Resolve a user dictionary URI or path and load the file by its
/// extension, building a `.csv` with the builder that `builder` makes.
///
/// # Arguments
///
/// * `uri` - A `file://` URI or a bare filesystem path of a `.csv` or `.bin`
///   user dictionary.
/// * `builder` - Makes the builder for a `.csv`. It is called only for a
///   `.csv`, so loading a `.bin` copies no metadata.
///
/// # Returns
///
/// The user dictionary, or an error if the URI or the extension is invalid
/// or the file cannot be loaded or built.
fn load_user_dictionary_with(
    uri: &str,
    builder: impl FnOnce() -> DictionaryBuilder,
) -> LinderaResult<UserDictionary> {
    // Try to parse as URI first, but only if it looks like a URI
    // (contains "://" or starts with known schemes)
    let path = if uri.contains("://") {
        match Url::parse(uri) {
            Ok(parsed_uri) => {
                // Parse the URI and return the appropriate dictionary
                let scheme = DictionaryScheme::from_str(parsed_uri.scheme()).map_err(|err| {
                    LinderaErrorKind::Dictionary
                        .with_error(anyhow::anyhow!("Invalid dictionary scheme: {err}"))
                })?;

                match scheme {
                    DictionaryScheme::File => {
                        // Extract path from file:// URL manually
                        let path_str = parsed_uri.path();

                        // Handle Windows paths that might start with /C:/ etc.
                        let path_str =
                            if cfg!(windows) && path_str.len() > 1 && path_str.starts_with('/') {
                                &path_str[1..]
                            } else {
                                path_str
                            };

                        // Decode percent-encoded characters
                        let decoded_path =
                            percent_decode_str(path_str).decode_utf8().map_err(|e| {
                                LinderaErrorKind::Dictionary
                                    .with_error(anyhow::anyhow!("Invalid UTF-8 in path: {e}"))
                            })?;

                        PathBuf::from(decoded_path.as_ref())
                    }
                    #[cfg(any(
                        feature = "embed-ipadic",
                        feature = "embed-ipadic-neologd",
                        feature = "embed-unidic",
                        feature = "embed-sudachidict",
                        feature = "embed-ko-dic",
                        feature = "embed-cc-cedict",
                        feature = "embed-jieba",
                    ))]
                    _ => {
                        // Unsupported dictionary scheme
                        return Err(LinderaErrorKind::Dictionary
                            .with_error(anyhow::anyhow!("Unsupported dictionary scheme")));
                    }
                }
            }
            Err(e) => {
                return Err(LinderaErrorKind::Dictionary
                    .with_error(anyhow::anyhow!("Invalid URI format: {e}")));
            }
        }
    } else {
        // Treat it as a file path directly
        PathBuf::from(uri)
    };

    // extract file extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("Invalid user dictionary source file"))
        })?;

    match extension {
        "csv" => UserDictionaryLoader::load_from_csv(builder(), &path),
        "bin" => load_user_dictionary_from_bin(&path),
        _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
            "Invalid user dictionary source file extension"
        ))),
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    mod ipadic {
        use std::borrow::Cow;
        use std::path::{Path, PathBuf};

        use crate::dictionary::{
            Dictionary, DictionaryBuilder, load_dictionary, load_user_dictionary_for,
        };
        use crate::mode::Mode;
        use crate::segmenter::Segmenter;

        /// Line 1 lies inside IPADIC's 1316 x 1316 connection matrix (1288 is
        /// its 名詞,固有名詞,一般 context ID); line 2 does not. Line 1's surface
        /// sorts first, so line 2 is the user dictionary's entry 1.
        const CSV_LINE_2_OUT_OF_RANGE: &str = "\
とうきょうスカイツリー駅,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ
東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
";

        fn ipadic() -> Dictionary {
            load_dictionary("embedded://ipadic").unwrap()
        }

        fn write_csv(dir: &Path, csv: &str) -> PathBuf {
            let path = dir.join("userdic.csv");
            std::fs::write(&path, csv).unwrap();
            path
        }

        fn bundled_simple_csv() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources/user_dict/ipadic_simple_userdic.csv")
        }

        fn surfaces(segmenter: &Segmenter, text: &str) -> Vec<String> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .into_iter()
                .map(|token| token.surface.to_string())
                .collect()
        }

        /// A `.csv` row outside the system dictionary's matrix fails the load,
        /// and the error names the row's CSV line and surface (#1187).
        #[test]
        fn load_user_dictionary_for_names_the_csv_line_and_surface() {
            let dir = tempfile::tempdir().unwrap();
            let csv = write_csv(dir.path(), CSV_LINE_2_OUT_OF_RANGE);

            let err = match load_user_dictionary_for(csv.to_str().unwrap(), &ipadic()) {
                Ok(_) => panic!("loaded a user dictionary outside the matrix"),
                Err(err) => err.to_string(),
            };
            assert!(
                err.contains(
                    "user dictionary line 2 (surface: '東京スカイツリー'): left context id 60000 \
                     is out of range for the connection matrix (matrix.def size 1316 1316; left \
                     context ids must be below 1316)"
                ),
                "{err}"
            );
        }

        /// A `.csv` inside the matrix loads and segments as the user
        /// dictionary says.
        #[test]
        fn load_user_dictionary_for_loads_a_csv_inside_the_matrix() {
            let dictionary = ipadic();
            let user_dictionary =
                load_user_dictionary_for(bundled_simple_csv().to_str().unwrap(), &dictionary)
                    .unwrap();
            let segmenter =
                Segmenter::try_new(Mode::Normal, dictionary, Some(user_dictionary)).unwrap();

            assert_eq!(
                surfaces(&segmenter, "東京スカイツリー"),
                ["東京スカイツリー"]
            );
        }

        /// A `.bin` is loaded as it was built, without the matrix check;
        /// `Segmenter::try_new` still rejects it, naming the entry by word id.
        #[test]
        fn load_user_dictionary_for_loads_a_bin_as_built() {
            let dictionary = ipadic();
            let dir = tempfile::tempdir().unwrap();
            let csv = write_csv(dir.path(), CSV_LINE_2_OUT_OF_RANGE);
            let bin = dir.path().join("userdic.bin");
            DictionaryBuilder::new((*dictionary.metadata).clone())
                .build_user_dictionary(&csv, &bin)
                .unwrap();

            let user_dictionary =
                load_user_dictionary_for(bin.to_str().unwrap(), &dictionary).unwrap();
            let err = match Segmenter::try_new(Mode::Normal, dictionary, Some(user_dictionary)) {
                Ok(_) => panic!("try_new accepted a user dictionary outside the matrix"),
                Err(err) => err.to_string(),
            };
            assert!(
                err.contains(
                    "user dictionary entry 1 (名詞,固有名詞,一般,*,*,*,東京スカイツリー,\
                     トウキョウスカイツリー,トウキョウスカイツリー): left context id 60000 is out \
                     of range"
                ),
                "{err}"
            );
        }

        /// A `file://` URI resolves to the same file as the bare path.
        #[test]
        fn load_user_dictionary_for_resolves_a_file_uri() {
            let dictionary = ipadic();
            let path = bundled_simple_csv().canonicalize().unwrap();
            let uri = url::Url::from_file_path(&path).unwrap();

            let user_dictionary = load_user_dictionary_for(uri.as_str(), &dictionary).unwrap();
            let segmenter =
                Segmenter::try_new(Mode::Normal, dictionary, Some(user_dictionary)).unwrap();

            assert_eq!(
                surfaces(&segmenter, "東京スカイツリー"),
                ["東京スカイツリー"]
            );
        }
    }
}
