pub mod character_definition;
pub mod connection_cost_matrix;
pub mod context_id_map;
pub mod metadata;
pub mod prefix_dictionary;
pub mod schema;
pub mod unknown_dictionary;

use std::fs;
use std::path::Path;
use std::str;
use std::sync::Arc;

use byteorder::{ByteOrder, LittleEndian};
use once_cell::sync::Lazy;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::builder::connection_cost_matrix::MatrixSize;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::metadata::Metadata;
use crate::dictionary::prefix_dictionary::{PrefixDictionary, UserPrefixDictionary};
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::error::LinderaErrorKind;
use crate::loader::character_definition::CharacterDefinitionLoader;
use crate::loader::connection_cost_matrix::ConnectionCostMatrixLoader;
use crate::loader::metadata::MetadataLoader;
use crate::loader::prefix_dictionary::PrefixDictionaryLoader;
use crate::loader::unknown_dictionary::UnknownDictionaryLoader;
use crate::util::{Data, detail_field_count, joined_details_at, words_idx_offset};
use crate::viterbi::WordEntry;

/// The single field the unknown-word sentinel consists of.
///
/// Kept as a constant so [`UNK`] and [`DetailFields::unk`] cannot drift apart.
const UNK_FIELD: &str = "UNK";

pub static UNK: Lazy<Vec<&str>> = Lazy::new(|| vec![UNK_FIELD]);

/// The detail fields of one dictionary entry, borrowed from the dictionary's
/// own bytes.
///
/// Yielding borrows rather than a `Vec` is what lets a caller materialize an
/// entry's details in a single allocation: the fields are handed out straight
/// from the packed record, with no intermediate collection (#966). The
/// iterator is [`ExactSizeIterator`], so a caller can size its buffer exactly
/// before consuming anything.
///
/// Obtained from [`Dictionary::word_details_iter`],
/// [`Dictionary::unknown_word_details_iter`],
/// [`UserDictionary::word_details_iter`], or
/// [`UnknownDictionary::word_details_iter`].
pub struct DetailFields<'a> {
    /// The remaining fields, split from the entry's NUL-joined blob.
    inner: str::Split<'a, char>,
    /// How many fields `inner` has yet to yield, for `ExactSizeIterator`.
    remaining: usize,
}

impl<'a> DetailFields<'a> {
    /// Builds the field list of a validated NUL-joined blob.
    ///
    /// # Arguments
    ///
    /// * `joined` - The entry's joined fields, already UTF-8 validated.
    ///
    /// # Returns
    ///
    /// The fields in schema order; always at least one, since an empty blob
    /// splits into a single empty field.
    #[inline]
    fn from_joined(joined: &'a str) -> Self {
        Self {
            inner: joined.split('\0'),
            remaining: detail_field_count(joined.as_bytes()),
        }
    }

    /// The unknown-word sentinel: a single `"UNK"` field.
    ///
    /// This is what every accessor falls back to for a malformed entry. It is
    /// built by splitting the sentinel itself, so the fallback runs through
    /// the same machinery as a real entry instead of needing its own variant.
    ///
    /// # Returns
    ///
    /// A one-field iterator yielding `"UNK"`.
    //
    // Returns `Self`, not `DetailFields<'static>`: `str::Split` makes this
    // type invariant over `'a`, so a `'static` value would not coerce to a
    // shorter lifetime. The `&'static str` constant coerces to `&'a str`
    // before construction instead, which needs no variance.
    #[inline]
    pub fn unk() -> Self {
        Self::from_joined(UNK_FIELD)
    }

    /// An entry with no fields at all.
    ///
    /// Distinct from [`DetailFields::unk`]: this is what
    /// [`Dictionary::word_details_iter`] yields for an out-of-range word id,
    /// where the allocating accessor has always returned an empty vector.
    ///
    /// # Returns
    ///
    /// An iterator that yields nothing.
    //
    // Returns `Self` for the same invariance reason as `unk`.
    #[inline]
    pub fn empty() -> Self {
        Self {
            inner: "".split('\0'),
            remaining: 0,
        }
    }
}

impl<'a> Iterator for DetailFields<'a> {
    type Item = &'a str;

    /// Yields the next detail field.
    ///
    /// # Returns
    ///
    /// The next field in schema order, or `None` once all are consumed.
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.inner.next()
    }

    /// Reports the exact number of fields left, so collecting into a `Vec`
    /// reserves in one shot.
    ///
    /// # Returns
    ///
    /// `(remaining, Some(remaining))`.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for DetailFields<'_> {}

/// Looks up one entry in a packed dictionary's detail records.
///
/// Shared by [`Dictionary`] and [`UserDictionary`], whose storage layout is
/// identical and whose only behavioural difference is what a word id with no
/// slot in the index yields.
///
/// # Arguments
///
/// * `words_idx_data` - The word-id index table.
/// * `words_data` - The packed detail records.
/// * `word_id` - The word id to look up.
/// * `missing` - Builds the fallback for a word id with no slot in the index.
///
/// # Returns
///
/// The entry's fields; `missing()` when the id has no slot, and
/// [`DetailFields::unk`] when the entry itself is malformed.
#[inline]
fn packed_details_iter<'a>(
    words_idx_data: &'a [u8],
    words_data: &'a [u8],
    word_id: usize,
    missing: fn() -> DetailFields<'a>,
) -> DetailFields<'a> {
    let Some(offset) = words_idx_offset(words_idx_data, word_id) else {
        return missing();
    };
    // A malformed entry always falls back to the sentinel in both
    // dictionaries; only a word id with no slot at all is subject to
    // `missing`.
    match joined_details_at(words_data, offset) {
        Some(joined) => DetailFields::from_joined(joined),
        None => DetailFields::unk(),
    }
}

/// `prefix_dictionary` and `connection_cost_matrix` are `Arc`-wrapped so that
/// `Dictionary::clone()` is O(1) regardless of load method (embedded, mmap,
/// or plain filesystem read) -- these two components dominate a dictionary's
/// memory footprint (tens to hundreds of MB), and nothing in this codebase
/// mutates them after construction.
#[derive(Clone)]
pub struct Dictionary {
    pub prefix_dictionary: Arc<PrefixDictionary>,
    pub connection_cost_matrix: Arc<ConnectionCostMatrix>,
    pub character_definition: Arc<CharacterDefinition>,
    pub unknown_dictionary: Arc<UnknownDictionary>,
    pub metadata: Arc<Metadata>,
}

impl Dictionary {
    /// Retrieve the detail fields (POS, etc.) for an unknown word entry.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The unknown-word entry id.
    ///
    /// # Returns
    ///
    /// A freshly allocated vector of the entry's fields, or the [`UNK`]
    /// sentinel when the id is out of range or the entry is malformed. Prefer
    /// [`Dictionary::unknown_word_details_iter`] on per-token paths, which
    /// yields the same fields without allocating.
    pub fn unknown_word_details(&self, word_id: usize) -> Vec<&str> {
        self.unknown_word_details_iter(word_id).collect()
    }

    /// Yields the detail fields of an unknown-word entry, borrowed from the
    /// dictionary's own bytes.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The unknown-word entry id.
    ///
    /// # Returns
    ///
    /// The entry's fields, or the [`UNK`] sentinel when the id is out of
    /// range or the entry is malformed -- the fallback
    /// [`Dictionary::unknown_word_details`] has always applied.
    #[inline]
    pub fn unknown_word_details_iter<'a>(&'a self, word_id: usize) -> DetailFields<'a> {
        self.unknown_dictionary
            .word_details_iter(word_id as u32)
            .unwrap_or_else(DetailFields::unk)
    }

    /// Retrieve the detail fields (POS, etc.) for a system dictionary entry.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The system word id.
    ///
    /// # Returns
    ///
    /// A freshly allocated vector of the entry's fields; empty when `word_id`
    /// is out of range, and the [`UNK`] sentinel when the entry is malformed.
    /// Prefer [`Dictionary::word_details_iter`] on per-token paths, which
    /// yields the same fields without allocating.
    pub fn word_details(&self, word_id: usize) -> Vec<&str> {
        self.word_details_iter(word_id).collect()
    }

    /// Yields the detail fields of a system dictionary entry, borrowed from
    /// the dictionary's own bytes.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The system word id.
    ///
    /// # Returns
    ///
    /// The entry's fields; [`DetailFields::empty`] when `word_id` is out of
    /// range, and [`DetailFields::unk`] when the entry is malformed. Those
    /// two fallbacks differ, and both are what
    /// [`Dictionary::word_details`] has always returned.
    #[inline]
    pub fn word_details_iter<'a>(&'a self, word_id: usize) -> DetailFields<'a> {
        packed_details_iter(
            &self.prefix_dictionary.words_idx_data,
            &self.prefix_dictionary.words_data,
            word_id,
            DetailFields::empty,
        )
    }

    /// Load dictionary from a directory containing dictionary files.
    ///
    /// When the `mmap` feature is compiled in, the connection-cost matrix
    /// and word list are routed through memory-mapped reads by default
    /// (#879); use [`Dictionary::load_from_path_with_options`] with
    /// `use_mmap = false` to force eager reads.
    pub fn load_from_path(dict_path: &Path) -> LinderaResult<Self> {
        Self::load_from_path_with_options(dict_path, cfg!(feature = "mmap"))
    }

    /// Load dictionary from a directory with options
    ///
    /// `use_mmap` (when the `mmap` feature is enabled) routes
    /// `connection_cost_matrix` and `prefix_dictionary` through memory-mapped
    /// reads instead of plain file reads. What that buys differs per
    /// component:
    ///
    /// - `ConnectionCostMatrix` reads its costs **in place**, with no copy and
    ///   no anonymous memory: `matrix.mtx` already stores the values in the
    ///   in-memory layout, and an mmap base is page-aligned, so the payload
    ///   can be viewed as `[i16]` directly. Loading it is O(1) and the pages
    ///   are faulted in lazily during tokenization. (Costs are also borrowed
    ///   from a plain read's buffer whenever it happens to be `i16`-aligned;
    ///   the alignment is only *guaranteed* under mmap and for embedded data.)
    /// - `PrefixDictionary`'s `vals_data`/`words_idx_data`/`words_data` are
    ///   likewise mmap-backed and read lazily at lookup time.
    /// - `PrefixDictionary`'s double-array trie (`da`) is still eagerly
    ///   deserialized into owned daachorse structures, so for that component
    ///   `use_mmap` only avoids the initial file-read syscall/allocation.
    ///
    /// `metadata`, `character_definition` and `unknown_dictionary` are always
    /// plain-read regardless of this flag. Separately, `Dictionary::clone()`
    /// is O(1) regardless of `use_mmap`, since
    /// `prefix_dictionary`/`connection_cost_matrix` are `Arc`-wrapped.
    pub fn load_from_path_with_options(dict_path: &Path, use_mmap: bool) -> LinderaResult<Self> {
        // Verify that the dictionary directory exists
        if !dict_path.exists() {
            return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "Dictionary path does not exist: {}",
                dict_path.display()
            )));
        }

        if !dict_path.is_dir() {
            return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "Dictionary path is not a directory: {}",
                dict_path.display()
            )));
        }

        // Load each component from the dictionary directory. The format check
        // comes first: the remaining artifacts are headerless raw arrays, so a
        // stale dictionary decodes into garbage rather than failing, and the
        // error would surface far from its cause.
        let metadata = MetadataLoader::load(dict_path)?;
        metadata.validate_format_version()?;

        let character_definition = CharacterDefinitionLoader::load(dict_path)?;

        let connection_cost_matrix = {
            #[cfg(feature = "mmap")]
            if use_mmap {
                ConnectionCostMatrixLoader::load_mmap(dict_path)?
            } else {
                ConnectionCostMatrixLoader::load(dict_path)?
            }
            #[cfg(not(feature = "mmap"))]
            ConnectionCostMatrixLoader::load(dict_path)?
        };

        let prefix_dictionary = {
            #[cfg(feature = "mmap")]
            if use_mmap {
                PrefixDictionaryLoader::load_mmap(dict_path)?
            } else {
                PrefixDictionaryLoader::load(dict_path)?
            }
            #[cfg(not(feature = "mmap"))]
            PrefixDictionaryLoader::load(dict_path)?
        };

        let unknown_dictionary = UnknownDictionaryLoader::load(dict_path)?;

        Ok(Dictionary {
            prefix_dictionary: Arc::new(prefix_dictionary),
            connection_cost_matrix: Arc::new(connection_cost_matrix),
            character_definition: Arc::new(character_definition),
            unknown_dictionary: Arc::new(unknown_dictionary),
            metadata: Arc::new(metadata),
        })
    }

    /// Save dictionary to a directory
    pub fn save_to_path(&self, dict_path: &Path) -> LinderaResult<()> {
        // Create directory if it doesn't exist
        fs::create_dir_all(dict_path)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        // For now, we'll implement this as needed
        // This would require implementing save methods for each component
        todo!("Dictionary saving will be implemented when needed")
    }
}

/// `dict` archives with the exact field sequence the pre-v6
/// `PrefixDictionary` used, which is what keeps previously-built user
/// dictionary `.bin` files loading across the v6 system-dictionary format
/// break -- rkyv 0.8 archives structurally, without type names. See
/// [`UserPrefixDictionary`]'s type-level comment before touching either type.
#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct UserDictionary {
    pub dict: UserPrefixDictionary,
}

impl UserDictionary {
    /// Relabel this dictionary's context IDs with a system dictionary's permutation.
    ///
    /// User dictionaries are always compiled in the *original* context-ID space, which
    /// keeps a built `.bin` portable across remapped and un-remapped system
    /// dictionaries. When one is attached to a system dictionary built with
    /// `connection_id_mapping`, its `left_id`/`right_id` must be moved into the same
    /// space, or every connection cost it participates in would address the wrong
    /// matrix cell — silently, since the IDs stay in range.
    ///
    /// Entries live in `vals_data` as a flat 10-byte (`WordEntry::SERIALIZED_LEN`) stride
    /// with `left_id` at offset 6 and `right_id` at offset 8 (little endian), so this
    /// rewrites those two `u16`s in place. IDs outside the permutation are left
    /// untouched, matching the builder's behaviour for malformed IDs.
    ///
    /// # Arguments
    ///
    /// * `map` - The permutation persisted in the system dictionary's metadata.
    pub fn remap_context_ids(&mut self, map: &ContextIdMap) {
        const LEFT_ID_OFFSET: usize = 6;
        const RIGHT_ID_OFFSET: usize = 8;

        let mut vals = self.dict.vals_data.to_vec();
        for entry in vals.as_chunks_mut::<{ WordEntry::SERIALIZED_LEN }>().0 {
            let left = LittleEndian::read_u16(&entry[LEFT_ID_OFFSET..][..2]);
            let right = LittleEndian::read_u16(&entry[RIGHT_ID_OFFSET..][..2]);
            LittleEndian::write_u16(&mut entry[LEFT_ID_OFFSET..][..2], map.map_left(left));
            LittleEndian::write_u16(&mut entry[RIGHT_ID_OFFSET..][..2], map.map_right(right));
        }
        self.dict.vals_data = Data::Vec(vals);
    }

    /// Checks that every entry's context IDs address a cell of a system
    /// dictionary's connection matrix.
    ///
    /// A user dictionary is built without the system dictionary it will be
    /// used with (`lindera build --user` reads only `metadata.json`), so an
    /// entry whose left or right context ID lies outside that dictionary's
    /// matrix builds, and then indexes past the matrix, a panic, when it
    /// matches the input. MeCab rejects such an entry when it compiles the
    /// user dictionary and records the matrix size in the result; a Lindera
    /// user dictionary has no room for the size, so its entries are checked
    /// when the two dictionaries are combined instead (`Segmenter::try_new`
    /// in `lindera-segmenter`).
    ///
    /// The bounds are those `lindera build` applies to a system dictionary:
    /// a left context ID must be below the matrix's backward size and a
    /// right context ID below its forward size. Call this before
    /// [`Self::remap_context_ids`], while the IDs are still the ones written
    /// in the user dictionary; the remap leaves an out-of-range ID unchanged,
    /// so the result would be the same, but the error would show remapped
    /// IDs.
    ///
    /// # Arguments
    ///
    /// * `matrix` - The connection matrix of the system dictionary.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every entry is in range, or a
    /// [`LinderaErrorKind::Content`] error for the first entry that is not.
    /// The error names the entry by its word id and detail fields, since the
    /// surface is kept only in the automaton, and gives the offending ID and
    /// the matrix size.
    pub fn check_context_ids(&self, matrix: &ConnectionCostMatrix) -> LinderaResult<()> {
        let matrix_size = MatrixSize {
            forward_size: matrix.forward_size,
            backward_size: matrix.backward_size,
        };
        for chunk in self
            .dict
            .vals_data
            .as_chunks::<{ WordEntry::SERIALIZED_LEN }>()
            .0
        {
            let entry = WordEntry::deserialize(chunk, false);
            if let Err(message) = matrix_size.check_context_ids(entry.left_id(), entry.right_id()) {
                let word_id = entry.word_id().id();
                let details = self
                    .word_details_iter(word_id as usize)
                    .collect::<Vec<_>>()
                    .join(",");
                return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "user dictionary entry {word_id} ({details}): {message}"
                )));
            }
        }
        Ok(())
    }

    pub fn load(user_dict_data: &[u8]) -> LinderaResult<UserDictionary> {
        let mut aligned = rkyv::util::AlignedVec::<16>::new();
        aligned.extend_from_slice(user_dict_data);
        rkyv::from_bytes::<UserDictionary, rkyv::rancor::Error>(&aligned).map_err(|err| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
        })
    }

    /// Retrieve the detail fields (POS, etc.) for a user dictionary entry.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The user-dictionary word id.
    ///
    /// # Returns
    ///
    /// A freshly allocated vector of the entry's fields, or the [`UNK`]
    /// sentinel when the id is out of range or the entry is malformed --
    /// note this differs from [`Dictionary::word_details`], which returns an
    /// empty vector for an out-of-range id. Prefer
    /// [`UserDictionary::word_details_iter`] on per-token paths, which yields
    /// the same fields without allocating.
    pub fn word_details(&self, word_id: usize) -> Vec<&str> {
        self.word_details_iter(word_id).collect()
    }

    /// Yields the detail fields of a user dictionary entry, borrowed from the
    /// dictionary's own bytes.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The user-dictionary word id.
    ///
    /// # Returns
    ///
    /// The entry's fields, or [`DetailFields::unk`] when the id is out of
    /// range or the entry is malformed. Unlike
    /// [`Dictionary::word_details_iter`], an out-of-range id yields the
    /// sentinel rather than nothing; that divergence is pre-existing.
    #[inline]
    pub fn word_details_iter<'a>(&'a self, word_id: usize) -> DetailFields<'a> {
        packed_details_iter(
            &self.dict.words_idx_data,
            &self.dict.words_data,
            word_id,
            DetailFields::unk,
        )
    }
}

#[cfg(test)]
mod tests {
    use daachorse::DoubleArrayAhoCorasickBuilder;

    use super::{DetailFields, UNK, UserDictionary};
    use crate::builder::user_dictionary::UserDictionaryBuilderOptions;
    use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
    use crate::dictionary::prefix_dictionary::UserPrefixDictionary;

    /// Builds a user dictionary whose words blob holds `entries`, each
    /// encoded as a 4-byte LE length followed by its NUL-joined fields --
    /// the layout `builder::user_dictionary` writes.
    fn user_dictionary(entries: &[&[&str]]) -> UserDictionary {
        let mut words_idx_data = Vec::new();
        let mut words_data = Vec::new();
        for fields in entries {
            words_idx_data.extend_from_slice(&(words_data.len() as u32).to_le_bytes());
            let joined = fields.join("\0");
            words_data.extend_from_slice(&(joined.len() as u32).to_le_bytes());
            words_data.extend_from_slice(joined.as_bytes());
        }
        // The automaton is irrelevant to `word_details`, but the type needs a
        // real one; a single dummy key keeps the build valid.
        let da = match DoubleArrayAhoCorasickBuilder::new().build_with_values([("x", 0u32)]) {
            Ok(da) => da,
            Err(err) => panic!("failed to build test automaton: {err}"),
        };
        UserDictionary {
            dict: UserPrefixDictionary {
                da,
                vals_data: Vec::new().into(),
                words_idx_data: words_idx_data.into(),
                words_data: words_data.into(),
                is_system: false,
            },
        }
    }

    /// Fields come back in order with the exact count that was written.
    #[test]
    fn user_word_details_returns_fields_in_order() {
        let dict = user_dictionary(&[&["カスタム名詞", "*", "リンデラ"], &["動詞", "自立", "*"]]);

        assert_eq!(dict.word_details(0), vec!["カスタム名詞", "*", "リンデラ"]);
        assert_eq!(dict.word_details(1), vec!["動詞", "自立", "*"]);
    }

    /// A single-field entry has no separator, so the capacity must still be 1.
    #[test]
    fn user_word_details_handles_single_field() {
        let dict = user_dictionary(&[&["ONLY"]]);
        assert_eq!(dict.word_details(0), vec!["ONLY"]);
    }

    /// Empty fields are preserved rather than collapsed.
    #[test]
    fn user_word_details_preserves_empty_fields() {
        let dict = user_dictionary(&[&["", "a", "", "b", ""]]);
        assert_eq!(dict.word_details(0), vec!["", "a", "", "b", ""]);
    }

    /// Out-of-range ids fall back to the `UNK` sentinel -- note this differs
    /// from `Dictionary::word_details`, which returns an empty vector.
    #[test]
    fn user_word_details_out_of_range_returns_unk() {
        let dict = user_dictionary(&[&["名詞", "一般"]]);
        assert_eq!(dict.word_details(1), UNK.to_vec());
        assert_eq!(dict.word_details(usize::MAX / 4), UNK.to_vec());
    }

    /// Invalid UTF-8 falls back to the `UNK` sentinel rather than panicking.
    #[test]
    fn user_word_details_invalid_utf8_returns_unk() {
        let mut dict = user_dictionary(&[&["ok", "fields"]]);
        let words_data: &[u8] = &dict.dict.words_data;
        let mut bytes = words_data.to_vec();
        let last = bytes.len() - 1;
        bytes[last] = 0xff;
        dict.dict.words_data = bytes.into();
        assert_eq!(dict.word_details(0), UNK.to_vec());
    }

    /// The iterator and the allocating accessor must agree field for field --
    /// the latter is now implemented as `collect()` over the former, so this
    /// pins that the delegation did not change what callers see.
    #[test]
    fn user_word_details_iter_matches_word_details() {
        let dict = user_dictionary(&[
            &["カスタム名詞", "*", "リンデラ"],
            &["動詞", "自立", "*"],
            &["ONLY"],
        ]);

        for word_id in 0..3 {
            let via_iter: Vec<&str> = dict.word_details_iter(word_id).collect();
            assert_eq!(via_iter, dict.word_details(word_id), "word_id {word_id}");
        }
    }

    /// `DetailFields` reports its length before anything is consumed, which
    /// is what lets a caller size its buffer in one shot. If this stopped
    /// being exact, `Token::ensure_details` would silently start
    /// reallocating.
    #[test]
    fn user_word_details_iter_reports_an_exact_length() {
        let dict = user_dictionary(&[&["a", "b", "c"], &["only"]]);

        let mut fields = dict.word_details_iter(0);
        assert_eq!(fields.len(), 3);
        assert_eq!(fields.next(), Some("a"));
        assert_eq!(fields.len(), 2, "len must track consumption");
        assert_eq!(fields.count(), 2);

        assert_eq!(dict.word_details_iter(1).len(), 1);
        // The out-of-range sentinel is one field, not zero.
        assert_eq!(dict.word_details_iter(99).len(), 1);
    }

    /// A word id whose index entry points past `words_data` used to panic on
    /// an unchecked slice. It now falls back to the sentinel, matching what
    /// `UnknownDictionary` has always done and the repo's no-panic rule.
    #[test]
    fn user_word_details_corrupt_offset_falls_back_instead_of_panicking() {
        let mut dict = user_dictionary(&[&["名詞", "一般"]]);

        // Point word id 0's index entry far past the end of `words_data`.
        let idx: &[u8] = &dict.dict.words_idx_data;
        let mut idx_bytes = idx.to_vec();
        let words_len: &[u8] = &dict.dict.words_data;
        let past_end = (words_len.len() as u32) + 1_000;
        idx_bytes[0..4].copy_from_slice(&past_end.to_le_bytes());
        dict.dict.words_idx_data = idx_bytes.into();

        assert_eq!(dict.word_details(0), UNK.to_vec());
        assert_eq!(dict.word_details_iter(0).collect::<Vec<_>>(), vec!["UNK"]);
    }

    /// Builds a connection matrix of `forward_size` x `backward_size` zero
    /// costs, in the legacy layout `ConnectionCostMatrix::load` still reads.
    fn zero_matrix(forward_size: i16, backward_size: i16) -> ConnectionCostMatrix {
        let mut data = Vec::new();
        data.extend_from_slice(&forward_size.to_le_bytes());
        data.extend_from_slice(&backward_size.to_le_bytes());
        for _ in 0..(forward_size as usize * backward_size as usize) {
            data.extend_from_slice(&0i16.to_le_bytes());
        }
        match ConnectionCostMatrix::load(data) {
            Ok(matrix) => matrix,
            Err(err) => panic!("failed to load test matrix: {err}"),
        }
    }

    /// Builds a user dictionary from CSV rows with the builder's defaults:
    /// a 3-field row is a simple entry with context IDs 0, a row of 12 or
    /// more fields a detailed entry with its own IDs.
    fn built_user_dictionary(csv: &str) -> UserDictionary {
        match UserDictionaryBuilderOptions::default()
            .builder()
            .build_from_reader(csv.as_bytes())
        {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to build test user dictionary: {err}"),
        }
    }

    /// IDs up to the last cell of each axis pass: left IDs index the
    /// backward axis (3 here) and right IDs the forward axis (2).
    #[test]
    fn check_context_ids_accepts_ids_inside_the_matrix() {
        let matrix = zero_matrix(2, 3);
        let dict = built_user_dictionary(
            "東京,2,1,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n\
             京都,0,0,0,名詞,固有名詞,地域,一般,*,*,京都,キョウト,キョウト\n\
             大阪,名詞,オオサカ\n",
        );

        assert!(dict.check_context_ids(&matrix).is_ok());
    }

    /// A left ID equal to the backward size is the first one outside the
    /// matrix; the error names the entry, the ID and the matrix size.
    #[test]
    fn check_context_ids_rejects_a_left_id_at_the_backward_size() {
        let matrix = zero_matrix(2, 3);
        let dict = built_user_dictionary(
            "東京,3,1,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n",
        );

        let err = dict.check_context_ids(&matrix).unwrap_err().to_string();
        assert!(
            err.contains(
                "user dictionary entry 0 (名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ): \
                 left context id 3 is out of range for the connection matrix \
                 (matrix.def size 2 3; left context ids must be below 3)"
            ),
            "{err}"
        );
    }

    /// A right ID equal to the forward size is rejected the same way.
    #[test]
    fn check_context_ids_rejects_a_right_id_at_the_forward_size() {
        let matrix = zero_matrix(2, 3);
        let dict = built_user_dictionary(
            "東京,2,2,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n",
        );

        let err = dict.check_context_ids(&matrix).unwrap_err().to_string();
        assert!(err.contains("right context id 2 is out of range"), "{err}");
        assert!(err.contains("right context ids must be below 2"), "{err}");
    }

    /// The error names the offending entry, not the first one: word ids
    /// follow the rows sorted by surface, so `京都` is entry 0 and `東京`
    /// entry 1. The IDs of the issue report (60000) are far outside.
    #[test]
    fn check_context_ids_names_the_offending_entry() {
        let matrix = zero_matrix(2, 3);
        let dict = built_user_dictionary(
            "東京,60000,60000,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n\
             京都,0,0,0,名詞,固有名詞,地域,一般,*,*,京都,キョウト,キョウト\n",
        );

        let err = dict.check_context_ids(&matrix).unwrap_err().to_string();
        assert!(
            err.contains("user dictionary entry 1 (名詞,固有名詞,地域,一般,*,*,東京,"),
            "{err}"
        );
        assert!(err.contains("left context id 60000"), "{err}");
    }

    /// `DetailFields::empty` yields nothing and `DetailFields::unk` yields
    /// exactly the sentinel, and the two are distinct -- `Dictionary` uses
    /// the first for an out-of-range id where `UserDictionary` uses the
    /// second.
    #[test]
    fn detail_fields_constructors() {
        assert_eq!(
            DetailFields::empty().collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
        assert_eq!(DetailFields::empty().len(), 0);
        assert_eq!(DetailFields::unk().collect::<Vec<_>>(), UNK.to_vec());
        assert_eq!(DetailFields::unk().len(), 1);
    }
}
