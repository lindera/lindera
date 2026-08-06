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
/// The data is bound to `static`s rather than `const`s deliberately. A `const`
/// body is encoded into the crate's metadata, so a `const` here would put a copy
/// of every dictionary byte into `lib.rmeta` at roughly 4x its size — several
/// hundred megabytes per dictionary crate, re-read by every downstream crate.
/// The data is private to `load()` below and never const-evaluated, so a `static`
/// is all that is needed.
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
        static CHAR_DEFINITION_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/char_def.bin"));
        static CONNECTION_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/matrix.mtx"));
        static DA_DATA: &[u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.da"));
        static VALS_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.vals"));
        static UNKNOWN_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/unk.bin"));
        static WORDS_IDX_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.wordsidx"));
        static WORDS_DATA: &[u8] =
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.words"));
        static METADATA_DATA: &[u8] =
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
                $crate::dictionary::prefix_dictionary::DaTrust::Trusted,
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
                prefix_dictionary: ::std::sync::Arc::new(prefix_dictionary),
                connection_cost_matrix: ::std::sync::Arc::new(connection_cost_matrix),
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
