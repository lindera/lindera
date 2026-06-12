//! Shared macros for the per-dictionary crates (`lindera-ipadic`,
//! `lindera-ko-dic`, `lindera-unidic`, `lindera-cc-cedict`, `lindera-jieba`,
//! `lindera-ipadic-neologd`).
//!
//! Each of those crates' `embedded` module used to contain ~90 lines of
//! identical boilerplate that differed only in the dictionary subdirectory
//! name and the loader struct name. [`embedded_dictionary!`] generates that
//! boilerplate from those two inputs.

/// Generates the embedded-dictionary loader for a dictionary crate.
///
/// The dictionary data is baked into the binary with `include_bytes!`,
/// reading from the `LINDERA_WORKDIR` directory populated by the crate's
/// build script.
///
/// * `$dir` — the dictionary subdirectory inside `LINDERA_WORKDIR`
///   (e.g. `"/lindera-ipadic"`).
/// * `$loader` — the public loader struct name (e.g. `EmbeddedIPADICLoader`).
///
/// # Example
///
/// ```ignore
/// lindera_dictionary::embedded_dictionary!("/lindera-ipadic", EmbeddedIPADICLoader);
/// ```
#[macro_export]
macro_rules! embedded_dictionary {
    ($dir:literal, $loader:ident) => {
        const CHAR_DEFINITION_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/char_def.bin"));
        const CONNECTION_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/matrix.mtx"));
        const DA_DATA: &[u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.da"));
        const VALS_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.vals"));
        const UNKNOWN_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/unk.bin"));
        const WORDS_IDX_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.wordsidx"));
        const WORDS_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.words"));
        const METADATA_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/metadata.json"));

        /// Loads the embedded dictionary from data baked into the binary.
        pub fn load() -> $crate::LinderaResult<$crate::dictionary::Dictionary> {
            let metadata = $crate::dictionary::metadata::Metadata::load(METADATA_DATA)?;
            let prefix_dictionary = $crate::dictionary::prefix_dictionary::PrefixDictionary::load(
                DA_DATA,
                VALS_DATA,
                WORDS_IDX_DATA,
                WORDS_DATA,
                true,
            )?;
            let connection_cost_matrix =
                $crate::dictionary::connection_cost_matrix::ConnectionCostMatrix::load(
                    CONNECTION_DATA,
                )?;
            let character_definition =
                $crate::dictionary::character_definition::CharacterDefinition::load(
                    CHAR_DEFINITION_DATA,
                )?;
            let unknown_dictionary =
                $crate::dictionary::unknown_dictionary::UnknownDictionary::load(UNKNOWN_DATA)?;

            Ok($crate::dictionary::Dictionary {
                prefix_dictionary,
                connection_cost_matrix,
                character_definition,
                unknown_dictionary,
                metadata,
            })
        }

        /// Loader that returns the dictionary embedded in the binary.
        pub struct $loader;

        impl Default for $loader {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $loader {
            pub fn new() -> Self {
                Self
            }
        }

        impl $crate::loader::DictionaryLoader for $loader {
            fn load(&self) -> $crate::LinderaResult<$crate::dictionary::Dictionary> {
                load()
            }
        }
    };
}
