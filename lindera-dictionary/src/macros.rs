//! Shared macros for the per-dictionary crates (`lindera-ipadic`,
//! `lindera-ko-dic`, `lindera-unidic`, `lindera-sudachidict`,
//! `lindera-cc-cedict`, `lindera-jieba`, `lindera-ipadic-neologd`).
//!
//! Each of those crates' `embedded` module used to contain ~90 lines of
//! identical boilerplate that differed only in the dictionary subdirectory
//! name and the loader struct name.
//! [`embedded_dictionary!`](crate::embedded_dictionary) generates that
//! boilerplate from those two inputs.

/// Includes a file's bytes as a 16-byte-aligned `&'static [u8]`.
///
/// `include_bytes!` yields a `[u8; N]`, whose alignment guarantee is 1. The
/// connection cost matrix payload starts at byte 6 of `matrix.mtx`, so a
/// 1-aligned base can leave it at an odd address, which pushes
/// [`crate::dictionary::connection_cost_matrix::ConnectionCostMatrix::load`]
/// into its owning fallback and reintroduces exactly the copy that this crate
/// avoids elsewhere (71.5 MB for UniDic). Wrapping the array in a
/// `#[repr(C, align(16))]` struct raises the `static`'s alignment so the
/// zero-copy path is taken deterministically rather than by the linker's
/// goodwill.
///
/// The data is bound to a `static` rather than a `const`: a `const` body would
/// be encoded into the crate's metadata at roughly 4x its size. A `static`'s
/// initializer is still encoded once, so unlike the other files of
/// [`embedded_dictionary!`](crate::embedded_dictionary), which are returned from private `fn`s, this
/// payload remains in `lib.rmeta`.
///
/// # Arguments
///
/// * `$path` - Path forwarded to `include_bytes!`.
///
/// # Returns
///
/// A `&'static [u8]` whose first byte is 16-byte aligned.
#[macro_export]
macro_rules! include_bytes_aligned {
    ($path:expr) => {{
        /// Raises the alignment of the wrapped bytes to 16.
        #[repr(C, align(16))]
        struct Aligned16<T: ?Sized> {
            /// The included bytes.
            bytes: T,
        }

        static ALIGNED: &Aligned16<[u8]> = &Aligned16 {
            bytes: *::core::include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}

/// Generates the embedded-dictionary loader for a dictionary crate.
///
/// The dictionary data is baked into the binary with `include_bytes!`,
/// reading from the `LINDERA_WORKDIR` directory populated by the crate's
/// build script.
///
/// Each file except `matrix.mtx` is returned from a private `fn` rather than
/// bound to a `const` or a `static`. A `const` body is encoded into the
/// crate's metadata at roughly 4x its size and a `static`'s initializer at 1x,
/// so either puts every dictionary byte into `lib.rmeta`, which every
/// downstream crate reads back. The body of a private, non-generic,
/// non-`#[inline]` `fn` is not encoded. Keep it that way: making these
/// functions `pub` or `load()` `#[inline]` brings the bytes back into the
/// metadata in release builds. `matrix.mtx` needs a const-evaluated aligned
/// wrapper (see [`include_bytes_aligned!`]), so it stays in the metadata.
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
        /// Returns the dictionary's char_def.bin bytes.
        fn char_definition_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/char_def.bin"))
        }

        // Aligned so `ConnectionCostMatrix::load` can view the payload as
        // `[i16]` in place instead of decoding it into an owned buffer.
        static CONNECTION_DATA: &[u8] =
            $crate::include_bytes_aligned!(concat!(env!("LINDERA_WORKDIR"), $dir, "/matrix.mtx"));

        /// Returns the dictionary's dict.trie bytes.
        fn trie_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.trie"))
        }

        /// Returns the dictionary's dict.valsidx bytes.
        fn vals_idx_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.valsidx"))
        }

        /// Returns the dictionary's dict.vals bytes.
        fn vals_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.vals"))
        }

        /// Returns the dictionary's unk.bin bytes.
        fn unknown_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/unk.bin"))
        }

        /// Returns the dictionary's dict.wordsidx bytes.
        fn words_idx_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.wordsidx"))
        }

        /// Returns the dictionary's dict.words bytes.
        fn words_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/dict.words"))
        }

        /// Returns the dictionary's metadata.json bytes.
        fn metadata_data() -> &'static [u8] {
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $dir, "/metadata.json"))
        }

        /// Loads the embedded dictionary from data baked into the binary.
        pub fn load() -> $crate::LinderaResult<$crate::dictionary::Dictionary> {
            let metadata = $crate::dictionary::metadata::Metadata::load(metadata_data())?;
            // Guards against a stale build cache: `include_bytes!` bakes in
            // whatever the build script produced, so a cache directory left
            // behind by another format version must be caught here.
            metadata.validate_format_version()?;
            // The trie is walked in place over these bytes with
            // bounds-checked reads, so unlike the retired daachorse
            // representation there is no unchecked-deserialize fast path to
            // guard -- embedded and filesystem data take the same safe code.
            let prefix_dictionary = $crate::dictionary::prefix_dictionary::PrefixDictionary::load(
                trie_data(),
                vals_idx_data(),
                vals_data(),
                words_idx_data(),
                words_data(),
            )?;
            let connection_cost_matrix =
                $crate::dictionary::connection_cost_matrix::ConnectionCostMatrix::load(
                    CONNECTION_DATA,
                )?;
            let character_definition =
                $crate::dictionary::character_definition::CharacterDefinition::load(
                    char_definition_data(),
                )?;
            let unknown_dictionary =
                $crate::dictionary::unknown_dictionary::UnknownDictionary::load(unknown_data())?;

            Ok($crate::dictionary::Dictionary {
                prefix_dictionary: ::std::sync::Arc::new(prefix_dictionary),
                connection_cost_matrix: ::std::sync::Arc::new(connection_cost_matrix),
                character_definition: ::std::sync::Arc::new(character_definition),
                unknown_dictionary: ::std::sync::Arc::new(unknown_dictionary),
                metadata: ::std::sync::Arc::new(metadata),
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
