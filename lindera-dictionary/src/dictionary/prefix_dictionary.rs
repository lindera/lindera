//! The prefix dictionary: surface forms mapped to their `WordEntry` records.
//!
//! The system dictionary ([`PrefixDictionary`]) is a char-wise double-array
//! trie (built with crawdad) that is **walked in place** over the serialized
//! bytes of `dict.trie` -- no deserialization, no owned node array. The trie
//! maps a surface form to its key ordinal; `dict.valsidx` (a `u32` prefix sum)
//! turns that ordinal into a run of records inside `dict.vals`.
//!
//! The user dictionary ([`UserPrefixDictionary`]) still uses a daachorse
//! Aho-Corasick automaton, because prebuilt user-dictionary `.bin` files embed
//! its serialized form inside an rkyv archive; changing it would invalidate
//! every `.bin` in the wild for little gain (user dictionaries are small).

use std::collections::BTreeMap;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use daachorse::DoubleArrayAhoCorasick;
use rkyv::rancor::{Fallible, Source};
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Place, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::{LinderaResult, error::LinderaErrorKind, util::Data, viterbi::WordEntry};

/// Match structure for common prefix iterator compatibility
#[derive(Debug, Clone)]
pub struct Match {
    /// Which word matched.
    pub word_idx: WordIdx,
    /// Match length in characters (the number of `chars` consumed).
    pub end_char: usize,
}

/// Identifies a word by its id within the dictionary it came from.
#[derive(Debug, Clone, Copy)]
pub struct WordIdx {
    /// The word id.
    pub word_id: u32,
}

impl WordIdx {
    /// Wraps a raw word id.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The word id to wrap.
    ///
    /// # Returns
    ///
    /// The wrapped id.
    pub fn new(word_id: u32) -> Self {
        Self { word_id }
    }
}

/// Mask selecting the index/value bits of a crawdad node's `base`/`check`;
/// the top bit is the leaf flag (`base`) / has-leaf flag (`check`).
const OFFSET_MASK: u32 = 0x7fff_ffff;

/// The code-mapper table value marking a character with no code assigned.
///
/// Also used by [`PrefixDictionary::map_code_or_invalid`] callers (the
/// lattice's per-sentence code buffer) to mark unmapped characters.
pub(crate) const INVALID_CODE: u32 = u32::MAX;

/// Byte offset where the code-mapper table starts inside `dict.trie`
/// (after the `u32` table length).
const TABLE_START: usize = 4;

/// Byte length of one serialized trie node (`base: u32` + `check: u32`).
const NODE_LEN_BYTES: usize = 8;

/// The system prefix dictionary: a serialized crawdad trie walked in place.
///
/// Serialized trie layout (crawdad 0.3, `Trie::serialize_to_vec`):
///
/// ```text
/// [table_len: u32][table: 4*table_len][alphabet_size: u32][node_len: u32][nodes: 8*node_len]
/// ```
///
/// This layout is not a documented contract of the crawdad crate, so the
/// dependency is pinned exactly and a round-trip test
/// (`trie_view_matches_crawdad_search`) fails loudly if it ever changes.
///
/// All node and table reads go through checked slicing (`get`) and
/// alignment-agnostic `u32` reads, so malformed bytes yield "no match" or a
/// load-time error -- never a panic or undefined behaviour. That is what
/// retired the O(n) validating pass and the `deserialize_unchecked` path the
/// daachorse representation needed.
#[derive(Clone)]
pub struct PrefixDictionary {
    /// The serialized trie (`dict.trie`), walked in place.
    trie_data: Data,
    /// Number of code-mapper table entries (indexable code points).
    table_len: usize,
    /// Byte offset of the node array inside `trie_data`.
    nodes_start: usize,
    /// Prefix sum over per-surface entry counts (`dict.valsidx`): `u32` LE
    /// records, one per trie key plus a trailing sentinel, in units of
    /// [`WordEntry::SERIALIZED_LEN`]-byte records inside `vals_data`.
    vals_idx: Data,
    /// The values file (`dict.vals`): `WordEntry` records back to back.
    pub vals_data: Data,
    /// Byte offsets into `words_data`, one `u32` per word id.
    pub words_idx_data: Data,
    /// Word detail records.
    pub words_data: Data,
    /// The root node's raw `base`, cached at load so every common-prefix
    /// search starts with the current node's `base` already in hand and the
    /// walk reads exactly one node per character step (#942). A trie with no
    /// nodes stores a leaf-flagged value that ends any walk immediately.
    root_base: u32,
}

impl PrefixDictionary {
    /// Serializes `word_entry_map` into `dict.trie` and `dict.valsidx` bytes.
    ///
    /// This is the build-time half of the dictionary: it is the only place
    /// that runs crawdad itself. The caller is responsible for writing
    /// `dict.vals` from the same map in the same iteration order (which a
    /// `BTreeMap` fixes), so the prefix sum built here indexes it correctly.
    ///
    /// # Arguments
    ///
    /// * `word_entry_map` - Surface form to word entries, sorted and
    ///   deduplicated by the `BTreeMap`.
    ///
    /// # Returns
    ///
    /// `(trie_bytes, vals_idx_bytes)`, or an error if a surface is empty or
    /// contains NUL (which crawdad reserves as its end marker), or if the
    /// trie build fails.
    pub fn serialize_trie(
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<(Vec<u8>, Vec<u8>)> {
        if word_entry_map.is_empty() {
            // crawdad rejects an empty key set, so emit a minimal image the
            // view treats as "matches nothing": an empty code table (every
            // character maps to no code) and zero nodes.
            let mut trie_bytes = Vec::with_capacity(12);
            trie_bytes.extend_from_slice(&0u32.to_le_bytes()); // table_len
            trie_bytes.extend_from_slice(&0u32.to_le_bytes()); // alphabet_size
            trie_bytes.extend_from_slice(&0u32.to_le_bytes()); // node_len
            return Ok((trie_bytes, 0u32.to_le_bytes().to_vec()));
        }

        let mut keys: Vec<&str> = Vec::with_capacity(word_entry_map.len());
        let mut offsets: Vec<u32> = Vec::with_capacity(word_entry_map.len() + 1);
        let mut acc: u32 = 0;

        for (surface, entries) in word_entry_map {
            // Skipping would silently desynchronize the prefix sum from the
            // separately-written dict.vals, so a bad surface is an error.
            // (An empty surface cannot get here anyway: the CSV parser drops
            // empty fields before the map is built.)
            if surface.is_empty() || surface.contains('\0') {
                return Err(LinderaErrorKind::Build.with_error(anyhow::anyhow!(
                    "surface {surface:?} cannot be stored in the trie (empty or contains NUL)"
                )));
            }
            keys.push(surface.as_str());
            offsets.push(acc);
            acc = acc.checked_add(entries.len() as u32).ok_or_else(|| {
                LinderaErrorKind::Build.with_error(anyhow::anyhow!("entry offset overflowed u32"))
            })?;
        }
        // Sentinel so the run for the last key is `idx[n-1]..idx[n]`.
        offsets.push(acc);

        let trie = crawdad::Trie::from_keys(keys.iter().copied()).map_err(|err| {
            LinderaErrorKind::Build.with_error(anyhow::anyhow!("crawdad trie build failed: {err}"))
        })?;
        let trie_bytes = trie.serialize_to_vec();

        let mut idx_bytes = Vec::with_capacity(offsets.len() * 4);
        for offset in &offsets {
            idx_bytes
                .write_u32::<LittleEndian>(*offset)
                .map_err(|err| {
                    LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to encode values index")
                })?;
        }

        Ok((trie_bytes, idx_bytes))
    }

    /// Builds an in-memory dictionary from surface forms and entries.
    ///
    /// Serializes the values file alongside the trie so the result is
    /// self-consistent. Used by the trainer and by tests; the production path
    /// loads previously-built files via [`PrefixDictionary::load`] instead.
    ///
    /// # Arguments
    ///
    /// * `word_entry_map` - Surface form to word entries.
    ///
    /// # Returns
    ///
    /// A dictionary answering prefix queries over the map, with empty word
    /// detail data.
    pub fn from_word_entry_map(
        word_entry_map: &BTreeMap<String, Vec<WordEntry>>,
    ) -> LinderaResult<Self> {
        let (trie_bytes, idx_bytes) = Self::serialize_trie(word_entry_map)?;

        let mut vals_bytes = Vec::with_capacity(word_entry_map.len() * WordEntry::SERIALIZED_LEN);
        for entries in word_entry_map.values() {
            for entry in entries {
                entry.serialize(&mut vals_bytes).map_err(|err| {
                    LinderaErrorKind::Serialize
                        .with_error(anyhow::anyhow!(err))
                        .add_context("Failed to serialize word entry")
                })?;
            }
        }

        Self::load(trie_bytes, idx_bytes, vals_bytes, Vec::new(), Vec::new())
    }

    /// Load a `PrefixDictionary` from raw binary data.
    ///
    /// Performs the O(1) structural check on the trie header: both length
    /// headers must be consistent with the buffer, which is what stops a
    /// crafted file from requesting an enormous allocation or walking out of
    /// bounds. No O(n) node scan is needed -- every access during search is
    /// bounds-checked, so a malformed node yields "no match" rather than a
    /// panic.
    ///
    /// # Arguments
    ///
    /// * `trie_data` - Contents of `dict.trie`.
    /// * `vals_idx` - Contents of `dict.valsidx`.
    /// * `vals_data` - Contents of `dict.vals`.
    /// * `words_idx_data` - Contents of `dict.wordsidx`.
    /// * `words_data` - Contents of `dict.words`.
    ///
    /// # Returns
    ///
    /// A `PrefixDictionary`, or an error if the trie or index headers are
    /// inconsistent with their buffers.
    pub fn load(
        trie_data: impl Into<Data>,
        vals_idx: impl Into<Data>,
        vals_data: impl Into<Data>,
        words_idx_data: impl Into<Data>,
        words_data: impl Into<Data>,
    ) -> LinderaResult<PrefixDictionary> {
        let trie_data = trie_data.into();
        let vals_idx = vals_idx.into();

        if trie_data.len() < TABLE_START {
            return Err(LinderaErrorKind::Deserialize
                .with_error(anyhow::anyhow!("dict.trie is too short for a trie header")));
        }
        let table_len = LittleEndian::read_u32(&trie_data[0..4]) as usize;
        // TABLE_START (table_len) + table + 4 (alphabet_size) + 4 (node_len)
        let nodes_start = table_len
            .checked_mul(4)
            .and_then(|table_bytes| table_bytes.checked_add(TABLE_START + 8))
            .ok_or_else(implausible_size)?;
        if nodes_start > trie_data.len() {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "dict.trie declares a {table_len}-entry code table that exceeds the file"
            )));
        }
        let node_len = LittleEndian::read_u32(&trie_data[nodes_start - 4..nodes_start]) as usize;
        let expected = node_len
            .checked_mul(NODE_LEN_BYTES)
            .and_then(|node_bytes| node_bytes.checked_add(nodes_start))
            .ok_or_else(implausible_size)?;
        if expected != trie_data.len() {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "dict.trie declares {node_len} nodes ({expected} bytes) but the file is {} bytes",
                trie_data.len()
            )));
        }

        if vals_idx.len() % 4 != 0 {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "dict.valsidx length {} is not a whole number of u32 records",
                vals_idx.len()
            )));
        }

        // Cache the root node's raw base (validated above: when node_len > 0
        // the node array starts at nodes_start and is fully inside the file).
        let root_base = if node_len > 0 {
            LittleEndian::read_u32(&trie_data[nodes_start..nodes_start + 4])
        } else {
            // Leaf-flagged poison: TrieWalk::advance ends immediately.
            !OFFSET_MASK
        };

        Ok(PrefixDictionary {
            trie_data,
            table_len,
            nodes_start,
            vals_idx,
            vals_data: vals_data.into(),
            words_idx_data: words_idx_data.into(),
            words_data: words_data.into(),
            root_base,
        })
    }

    /// Reads node `idx`'s `(base, check)` pair, raw (flag bits included).
    ///
    /// # Arguments
    ///
    /// * `idx` - Node index.
    ///
    /// # Returns
    ///
    /// The raw pair, or `None` when `idx` is out of range -- which for a
    /// well-formed trie never happens, and for a malformed one safely ends
    /// the search.
    #[inline(always)]
    fn node(&self, idx: u32) -> Option<(u32, u32)> {
        let off = self.nodes_start + (idx as usize) * NODE_LEN_BYTES;
        let bytes = self.trie_data.get(off..off + NODE_LEN_BYTES)?;
        Some((
            LittleEndian::read_u32(&bytes[0..4]),
            LittleEndian::read_u32(&bytes[4..8]),
        ))
    }

    /// Maps a character to its trie code.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to map.
    ///
    /// # Returns
    ///
    /// The mapped code, or `None` when the character appears in no key.
    #[inline(always)]
    fn map_code(&self, c: char) -> Option<u32> {
        let ord = c as usize;
        if ord >= self.table_len {
            return None;
        }
        let off = TABLE_START + ord * 4;
        let code = LittleEndian::read_u32(self.trie_data.get(off..off + 4)?);
        (code != INVALID_CODE).then_some(code)
    }

    /// Maps a character to its trie code, using [`INVALID_CODE`] for
    /// characters that appear in no key.
    ///
    /// The lattice fills a per-sentence code buffer with this once per
    /// character, so overlapping prefix walks stop re-querying the code
    /// table for the same character (#942).
    ///
    /// # Arguments
    ///
    /// * `c` - The character to map.
    ///
    /// # Returns
    ///
    /// The mapped code, or [`INVALID_CODE`].
    #[inline(always)]
    pub(crate) fn map_code_or_invalid(&self, c: char) -> u32 {
        self.map_code(c).unwrap_or(INVALID_CODE)
    }

    /// Looks up the values run for trie key ordinal `key_ord`.
    ///
    /// # Arguments
    ///
    /// * `key_ord` - The trie value: the key's ordinal in sorted key order.
    ///
    /// # Returns
    ///
    /// The key's serialized `WordEntry` records, or `None` if either index is
    /// out of range (malformed input).
    #[inline(always)]
    fn entry_bytes(&self, key_ord: u32) -> Option<&[u8]> {
        let i = key_ord as usize * 4;
        let idx = self.vals_idx.get(i..i + 8)?;
        let start = LittleEndian::read_u32(&idx[0..4]) as usize;
        let end = LittleEndian::read_u32(&idx[4..8]) as usize;
        self.vals_data
            .get(start * WordEntry::SERIALIZED_LEN..end * WordEntry::SERIALIZED_LEN)
    }

    /// Returns the word entries for every key that is a prefix of `chars`.
    ///
    /// This is the tokenizer's hot path: one call per lattice-reachable
    /// position, yielding matches in ascending length order.
    ///
    /// # Arguments
    ///
    /// * `chars` - The sentence suffix starting at the query position.
    ///
    /// # Returns
    ///
    /// An iterator of `(serialized WordEntry records, chars consumed)`.
    #[inline]
    pub fn common_prefix_search<'a, 'b>(&'a self, chars: &'b [char]) -> CommonPrefixSearch<'a, 'b> {
        CommonPrefixSearch {
            dict: self,
            chars,
            pos: 0,
            walk: TrieWalk::at_root(self),
        }
    }

    /// [`Self::common_prefix_search`] over pre-mapped trie codes.
    ///
    /// The lattice maps every sentence character through
    /// [`Self::map_code_or_invalid`] once (in the same pass that fills its
    /// other per-char buffers) and hands each start position's suffix here,
    /// so overlapping walks skip the per-step code-table probe (#942).
    ///
    /// # Arguments
    ///
    /// * `codes` - Codes for the sentence suffix starting at the query
    ///   position, with [`INVALID_CODE`] marking unmapped characters.
    ///
    /// # Returns
    ///
    /// An iterator of `(serialized WordEntry records, chars consumed)`.
    #[inline]
    pub(crate) fn common_prefix_search_codes<'a, 'b>(
        &'a self,
        codes: &'b [u32],
    ) -> CommonPrefixSearchCodes<'a, 'b> {
        CommonPrefixSearchCodes {
            dict: self,
            codes,
            pos: 0,
            walk: TrieWalk::at_root(self),
        }
    }

    /// Returns the entries whose surface is a prefix of `s`, with byte-level
    /// end offsets.
    ///
    /// # Arguments
    ///
    /// * `s` - The query string.
    ///
    /// # Returns
    ///
    /// An iterator of `(end byte offset in s, entry)` pairs.
    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        // Match lengths come back in characters; convert through the byte
        // offset of each char boundary.
        let boundaries: Vec<usize> = s
            .char_indices()
            .map(|(byte_offset, _)| byte_offset)
            .chain(std::iter::once(s.len()))
            .collect();
        let chars: Vec<char> = s.chars().collect();
        let mut results: Vec<(usize, WordEntry)> = Vec::new();
        for (entries, end_char) in self.common_prefix_search_owned(&chars) {
            let end_byte = boundaries[end_char];
            for chunk in entries.chunks_exact(WordEntry::SERIALIZED_LEN) {
                results.push((end_byte, WordEntry::deserialize(chunk, true)));
            }
        }
        results.into_iter()
    }

    /// [`Self::common_prefix_search`] over a temporary char buffer, collecting
    /// eagerly so the buffer does not need to outlive the iterator.
    ///
    /// # Arguments
    ///
    /// * `chars` - The query characters.
    ///
    /// # Returns
    ///
    /// The collected `(entry bytes, chars consumed)` pairs.
    fn common_prefix_search_owned(&self, chars: &[char]) -> Vec<(&[u8], usize)> {
        self.common_prefix_search(chars).collect()
    }

    /// Find `WordEntry`s with surface
    ///
    /// # Arguments
    ///
    /// * `surface` - The exact surface form to look up.
    ///
    /// # Returns
    ///
    /// All entries whose surface equals `surface`.
    pub fn find_surface(&self, surface: &str) -> Vec<WordEntry> {
        self.find_surface_iter(surface).collect()
    }

    /// Find `WordEntry`s with surface using lazy evaluation
    /// This iterator-based approach reduces memory allocations
    ///
    /// # Arguments
    ///
    /// * `surface` - The exact surface form to look up.
    ///
    /// # Returns
    ///
    /// An iterator over the matching entries.
    pub fn find_surface_iter<'a>(
        &'a self,
        surface: &'a str,
    ) -> impl Iterator<Item = WordEntry> + 'a {
        let chars: Vec<char> = surface.chars().collect();
        let char_count = chars.len();
        let mut entries: Vec<WordEntry> = Vec::new();
        for (bytes, end_char) in self.common_prefix_search_owned(&chars) {
            if end_char == char_count {
                for chunk in bytes.chunks_exact(WordEntry::SERIALIZED_LEN) {
                    entries.push(WordEntry::deserialize(chunk, true));
                }
            }
        }
        entries.into_iter()
    }

    /// Common prefix iterator using character array input.
    ///
    /// `end_char` counts characters consumed from `suffix` -- the natural
    /// unit for callers that index `&[char]` (the trainer's lattice does
    /// `pos + end_char`). The retired daachorse implementation returned byte
    /// lengths here, which silently misplaced edges for any non-ASCII text.
    ///
    /// # Arguments
    ///
    /// * `suffix` - The sentence suffix to match prefixes of.
    ///
    /// # Returns
    ///
    /// Matches for every dictionary key that is a prefix of `suffix`.
    pub fn common_prefix_iterator(&self, suffix: &[char]) -> Vec<Match> {
        let mut matches = Vec::new();
        for (bytes, end_char) in self.common_prefix_search(suffix) {
            for chunk in bytes.chunks_exact(WordEntry::SERIALIZED_LEN) {
                let word_entry = WordEntry::deserialize(chunk, true);
                matches.push(Match {
                    word_idx: WordIdx::new(word_entry.word_id().id()),
                    end_char,
                });
            }
        }
        matches
    }
}

/// Walk state shared by the common-prefix search iterators: the current node
/// index and its cached raw `base`, loaded when the node was entered so each
/// step reads exactly one node (#942 removed the per-step re-read of the
/// node the previous step had already fetched).
struct TrieWalk {
    /// Current trie node.
    node_idx: u32,
    /// The current node's raw `base` (leaf flag included).
    node_base: u32,
}

impl TrieWalk {
    /// Creates a walk positioned at the trie root.
    ///
    /// # 引数
    ///
    /// * `dict` - The dictionary whose root to start from.
    ///
    /// # 戻り値
    ///
    /// A walk at node 0 with the load-time-cached root `base`.
    #[inline(always)]
    fn at_root(dict: &PrefixDictionary) -> TrieWalk {
        TrieWalk {
            node_idx: 0,
            node_base: dict.root_base,
        }
    }

    /// Follows the transition labeled `mc` from the current node.
    ///
    /// # 引数
    ///
    /// * `dict` - The dictionary being walked.
    /// * `mc` - The mapped code of the next character.
    ///
    /// # 戻り値
    ///
    /// The child's raw `(base, check)` pair, or `None` when the walk ends
    /// (the current node is a leaf, the transition does not exist, or the
    /// bytes are malformed).
    #[inline(always)]
    fn advance(&mut self, dict: &PrefixDictionary, mc: u32) -> Option<(u32, u32)> {
        // A leaf stores its value in `base`, not a child offset, so it
        // has no children and the walk ends.
        if self.node_base & !OFFSET_MASK != 0 {
            return None;
        }
        let child_idx = (self.node_base & OFFSET_MASK) ^ mc;
        let (child_base, child_check) = dict.node(child_idx)?;
        if child_check & OFFSET_MASK != self.node_idx {
            return None;
        }
        self.node_idx = child_idx;
        self.node_base = child_base;
        Some((child_base, child_check))
    }
}

/// What one walk step produced, computed from the child's raw pair.
enum StepOutput<'a> {
    /// No key ends here; keep walking.
    Continue,
    /// Malformed bytes; end the search.
    Stop,
    /// A key ends here with these serialized `WordEntry` records.
    Yield(&'a [u8]),
}

/// Resolves a just-entered node's `(base, check)` pair to its step output.
///
/// # 引数
///
/// * `dict` - The dictionary being walked.
/// * `child_base` - The entered node's raw `base`.
/// * `child_check` - The entered node's raw `check`.
///
/// # 戻り値
///
/// Whether the step yields a key's entries, continues, or stops.
#[inline(always)]
fn step_output<'a>(
    dict: &'a PrefixDictionary,
    child_base: u32,
    child_check: u32,
) -> StepOutput<'a> {
    if child_base & !OFFSET_MASK != 0 {
        // The node itself is a leaf: its base is the value.
        return match dict.entry_bytes(child_base & OFFSET_MASK) {
            Some(entries) => StepOutput::Yield(entries),
            None => StepOutput::Stop,
        };
    }
    if child_check & !OFFSET_MASK != 0 {
        // The node has a leaf child reached by the end marker, whose
        // code is 0, so the child index is just the base.
        let leaf_idx = child_base & OFFSET_MASK;
        return match dict
            .node(leaf_idx)
            .and_then(|(leaf_base, _)| dict.entry_bytes(leaf_base & OFFSET_MASK))
        {
            Some(entries) => StepOutput::Yield(entries),
            None => StepOutput::Stop,
        };
    }
    StepOutput::Continue
}

/// Iterator over the dictionary keys that are prefixes of a query.
///
/// Mirrors crawdad 0.3's `CommonPrefixSearchIter` step for step, but walks
/// the serialized bytes directly with bounds-checked reads.
pub struct CommonPrefixSearch<'a, 'b> {
    /// The dictionary being searched.
    dict: &'a PrefixDictionary,
    /// The query characters.
    chars: &'b [char],
    /// Characters consumed so far.
    pos: usize,
    /// Walk state (current node and its cached `base`).
    walk: TrieWalk,
}

impl<'a> Iterator for CommonPrefixSearch<'a, '_> {
    type Item = (&'a [u8], usize);

    /// Advances to the next key that is a prefix of the query.
    ///
    /// # Returns
    ///
    /// The key's serialized `WordEntry` records and the number of characters
    /// consumed, or `None` when no further prefix matches.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.chars.len() {
            let mc = self.dict.map_code(self.chars[self.pos])?;
            let (child_base, child_check) = self.walk.advance(self.dict, mc)?;
            self.pos += 1;

            match step_output(self.dict, child_base, child_check) {
                StepOutput::Continue => {}
                StepOutput::Stop => return None,
                StepOutput::Yield(entries) => return Some((entries, self.pos)),
            }
        }
        None
    }
}

/// [`CommonPrefixSearch`] over pre-mapped trie codes instead of characters.
///
/// See [`PrefixDictionary::common_prefix_search_codes`].
pub(crate) struct CommonPrefixSearchCodes<'a, 'b> {
    /// The dictionary being searched.
    dict: &'a PrefixDictionary,
    /// The query's pre-mapped codes ([`INVALID_CODE`] = unmapped).
    codes: &'b [u32],
    /// Codes consumed so far.
    pos: usize,
    /// Walk state (current node and its cached `base`).
    walk: TrieWalk,
}

impl<'a> Iterator for CommonPrefixSearchCodes<'a, '_> {
    type Item = (&'a [u8], usize);

    /// Advances to the next key that is a prefix of the query.
    ///
    /// # Returns
    ///
    /// The key's serialized `WordEntry` records and the number of codes
    /// consumed, or `None` when no further prefix matches.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.codes.len() {
            let mc = self.codes[self.pos];
            if mc == INVALID_CODE {
                return None;
            }
            let (child_base, child_check) = self.walk.advance(self.dict, mc)?;
            self.pos += 1;

            match step_output(self.dict, child_base, child_check) {
                StepOutput::Continue => {}
                StepOutput::Stop => return None,
                StepOutput::Yield(entries) => return Some((entries, self.pos)),
            }
        }
        None
    }
}

/// Builds the "declared size is implausible" load error.
///
/// # Returns
///
/// A deserialize error describing an arithmetic overflow in the headers.
fn implausible_size() -> crate::error::LinderaError {
    LinderaErrorKind::Deserialize
        .with_error(anyhow::anyhow!("dict.trie declares an implausible size"))
}

/// rkyv adapter storing a daachorse automaton as its serialized bytes.
pub struct DoubleArrayArchiver;

impl ArchiveWith<DoubleArrayAhoCorasick<u32>> for DoubleArrayArchiver {
    type Archived = rkyv::vec::ArchivedVec<u8>;
    type Resolver = rkyv::vec::VecResolver;

    /// Resolves the archived byte vector for the automaton.
    fn resolve_with(
        field: &DoubleArrayAhoCorasick<u32>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let bytes = field.serialize();
        rkyv::vec::ArchivedVec::resolve_from_slice(&bytes, resolver, out);
    }
}

impl<S: Fallible + rkyv::ser::Writer + rkyv::ser::Allocator + ?Sized>
    SerializeWith<DoubleArrayAhoCorasick<u32>, S> for DoubleArrayArchiver
{
    /// Serializes the automaton as a byte vector.
    fn serialize_with(
        field: &DoubleArrayAhoCorasick<u32>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let bytes = field.serialize();
        rkyv::vec::ArchivedVec::serialize_from_slice(&bytes, serializer)
    }
}

impl<D: Fallible<Error: Source> + ?Sized>
    DeserializeWith<rkyv::vec::ArchivedVec<u8>, DoubleArrayAhoCorasick<u32>, D>
    for DoubleArrayArchiver
{
    /// Deserialize the archived byte vector into a `DoubleArrayAhoCorasick`.
    ///
    /// # Returns
    ///
    /// The deserialized `DoubleArrayAhoCorasick`, or an error if deserialization fails.
    fn deserialize_with(
        archived: &rkyv::vec::ArchivedVec<u8>,
        _deserializer: &mut D,
    ) -> Result<DoubleArrayAhoCorasick<u32>, D::Error> {
        let (da, _) = DoubleArrayAhoCorasick::deserialize(archived.as_slice()).map_err(|err| {
            D::Error::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ))
        })?;
        Ok(da)
    }
}

/// serde adapter storing a daachorse automaton as its serialized bytes.
mod double_array_serde {
    use daachorse::DoubleArrayAhoCorasick;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serializes the automaton as bytes.
    ///
    /// # Arguments
    ///
    /// * `da` - The automaton to serialize.
    /// * `serializer` - The serde serializer.
    ///
    /// # Returns
    ///
    /// The serializer's output.
    pub fn serialize<S>(da: &DoubleArrayAhoCorasick<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = da.serialize();
        serializer.serialize_bytes(&bytes)
    }

    /// Deserializes an automaton from bytes, validating them.
    ///
    /// # Arguments
    ///
    /// * `deserializer` - The serde deserializer.
    ///
    /// # Returns
    ///
    /// The automaton, or an error for malformed bytes.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DoubleArrayAhoCorasick<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let (da, _) = DoubleArrayAhoCorasick::deserialize(&bytes)
            .map_err(|err| serde::de::Error::custom(err.to_string()))?;
        Ok(da)
    }
}

/// The user prefix dictionary: a daachorse Aho-Corasick automaton.
///
/// The field sequence is byte-for-byte the sequence the pre-v6
/// `PrefixDictionary` archived (`da`, `vals_data`, `words_idx_data`,
/// `words_data`, `is_system`). rkyv 0.8 archives structurally, without type
/// names, so keeping the sequence is what lets every previously-built user
/// dictionary `.bin` keep loading. Do not add, remove or reorder fields
/// without bumping the dictionary format version and rebuilding the committed
/// `.bin` fixtures.
#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct UserPrefixDictionary {
    /// The Aho-Corasick automaton over surface forms.
    #[serde(with = "self::double_array_serde")]
    #[rkyv(with = DoubleArrayArchiver)]
    pub da: DoubleArrayAhoCorasick<u32>,
    /// The values file: `WordEntry` records back to back.
    pub vals_data: Data,
    /// Byte offsets into `words_data`, one `u32` per word id.
    pub words_idx_data: Data,
    /// Word detail records.
    pub words_data: Data,
    /// Always `false`; retained because the archived field sequence must not
    /// change (see the type-level comment).
    pub is_system: bool,
}

impl UserPrefixDictionary {
    /// Decode the `(offset, count)` pair stored in the double-array value.
    ///
    /// The word-id offset lives in the high 24 bits and the per-surface
    /// variant count in the low 8 bits (up to 255 variants). The legacy 5-bit
    /// encoding was retired in v4.0.0; user `.bin` files built with v3 must
    /// be rebuilt from CSV.
    ///
    /// # Arguments
    ///
    /// * `val` - The packed automaton value.
    ///
    /// # Returns
    ///
    /// The `(offset, count)` pair.
    #[inline]
    pub fn decode_val(&self, val: u32) -> (u32, u32) {
        (val >> 8u32, val & ((1u32 << 8) - 1u32))
    }

    /// Load a `UserPrefixDictionary` from raw binary data.
    ///
    /// The automaton bytes are always run through daachorse's validating
    /// deserializer: user dictionaries come from the filesystem or from
    /// callers, never from this crate's own build pipeline, so there is no
    /// trusted path.
    ///
    /// # Arguments
    ///
    /// * `da_data` - Serialized automaton bytes.
    /// * `vals_data` - Values data bytes.
    /// * `words_idx_data` - Word index data bytes.
    /// * `words_data` - Words data bytes.
    ///
    /// # Returns
    ///
    /// A `UserPrefixDictionary`, or an error if deserialization fails.
    pub fn load(
        da_data: impl Into<Data>,
        vals_data: impl Into<Data>,
        words_idx_data: impl Into<Data>,
        words_data: impl Into<Data>,
    ) -> LinderaResult<UserPrefixDictionary> {
        let da_bytes = da_data.into();
        let da = DoubleArrayAhoCorasick::deserialize(&da_bytes[..])
            .map_err(|err| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
            })?
            .0;

        Ok(UserPrefixDictionary {
            da,
            vals_data: vals_data.into(),
            words_idx_data: words_idx_data.into(),
            words_data: words_data.into(),
            is_system: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use daachorse::DoubleArrayAhoCorasickBuilder;

    use super::*;
    use crate::viterbi::{LexType, WordId};

    fn entry(word_id: u32, cost: i16) -> WordEntry {
        WordEntry::new(WordId::new(LexType::System, word_id), cost, 0, 0)
    }

    fn sample_map() -> BTreeMap<String, Vec<WordEntry>> {
        let mut map = BTreeMap::new();
        map.insert("世界".to_string(), vec![entry(0, 10)]);
        map.insert("世界中".to_string(), vec![entry(1, 20), entry(2, 30)]);
        map.insert("世論調査".to_string(), vec![entry(3, 40)]);
        map.insert("統計調査".to_string(), vec![entry(4, 50)]);
        map
    }

    /// The load-bearing round-trip: the in-place view must return exactly
    /// what crawdad's own search returns on the same serialized bytes. This
    /// is the test that fails loudly if crawdad's byte layout ever changes.
    #[test]
    fn trie_view_matches_crawdad_search() {
        let map = sample_map();
        let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
        let reference = crawdad::Trie::from_keys(keys.iter().copied()).unwrap();

        let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();

        for haystack in ["世界中で世論調査", "統計調査だ", "無関係な文", "世", ""]
        {
            let chars: Vec<char> = haystack.chars().collect();
            for start in 0..=chars.len() {
                let expected: Vec<(u32, usize)> = reference
                    .common_prefix_search(chars[start..].iter().copied())
                    .collect();
                let actual: Vec<(usize, usize)> = dict
                    .common_prefix_search(&chars[start..])
                    .map(|(bytes, end)| (bytes.len() / WordEntry::SERIALIZED_LEN, end))
                    .collect();

                assert_eq!(actual.len(), expected.len(), "at {haystack:?}[{start}..]");
                for ((key_ord, exp_end), (n_entries, act_end)) in expected.iter().zip(actual.iter())
                {
                    assert_eq!(exp_end, act_end);
                    // The run length must match the map's entry count for the
                    // key crawdad says matched.
                    let surface: String = chars[start..start + exp_end].iter().collect();
                    assert_eq!(
                        map[&surface].len(),
                        *n_entries,
                        "run length for key ordinal {key_ord}"
                    );
                }
            }
        }
    }

    #[test]
    fn common_prefix_search_codes_matches_chars() {
        let dict = PrefixDictionary::from_word_entry_map(&sample_map()).unwrap();

        // Includes unmapped characters (ASCII, kana absent from the keys)
        // to exercise the INVALID_CODE early-out.
        for haystack in ["世界中で世論調査", "統計調査だ", "a世界", "世", ""] {
            let chars: Vec<char> = haystack.chars().collect();
            let codes: Vec<u32> = chars.iter().map(|&c| dict.map_code_or_invalid(c)).collect();
            for start in 0..=chars.len() {
                let expected: Vec<(&[u8], usize)> =
                    dict.common_prefix_search(&chars[start..]).collect();
                let actual: Vec<(&[u8], usize)> =
                    dict.common_prefix_search_codes(&codes[start..]).collect();
                assert_eq!(expected, actual, "at {haystack:?}[{start}..]");
            }
        }
    }

    #[test]
    fn find_surface_returns_all_variants() {
        let dict = PrefixDictionary::from_word_entry_map(&sample_map()).unwrap();

        let entries = dict.find_surface("世界中");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].word_cost(), 20);
        assert_eq!(entries[1].word_cost(), 30);

        assert!(dict.find_surface("世論").is_empty());
        assert!(dict.find_surface("未知語").is_empty());
    }

    #[test]
    fn prefix_returns_byte_offsets() {
        let dict = PrefixDictionary::from_word_entry_map(&sample_map()).unwrap();

        let results: Vec<(usize, WordEntry)> = dict.prefix("世界中で").collect();
        // "世界" (6 bytes) and then "世界中" (9 bytes), three entries total.
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 6);
        assert_eq!(results[1].0, 9);
        assert_eq!(results[2].0, 9);
    }

    #[test]
    fn common_prefix_iterator_counts_characters() {
        let dict = PrefixDictionary::from_word_entry_map(&sample_map()).unwrap();

        let chars: Vec<char> = "世界中".chars().collect();
        let matches = dict.common_prefix_iterator(&chars);
        // "世界" consumes 2 chars, "世界中" consumes 3 -- in characters, not
        // bytes (the retired implementation returned 6 and 9 here).
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].end_char, 2);
        assert_eq!(matches[1].end_char, 3);
        assert_eq!(matches[2].end_char, 3);
    }

    #[test]
    fn load_rejects_truncated_trie() {
        let map = sample_map();
        let (trie_bytes, idx_bytes) = PrefixDictionary::serialize_trie(&map).unwrap();

        let mut truncated = trie_bytes.clone();
        truncated.truncate(trie_bytes.len() - 3);
        assert!(
            PrefixDictionary::load(
                truncated,
                idx_bytes.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new()
            )
            .is_err()
        );

        assert!(
            PrefixDictionary::load(vec![0u8; 2], idx_bytes, Vec::new(), Vec::new(), Vec::new())
                .is_err()
        );
    }

    #[test]
    fn load_rejects_absurd_table_length() {
        // A table length claiming more entries than the file could hold must
        // be rejected up front, not fed to an allocator.
        let mut data = Vec::new();
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&[0u8; 16]);
        assert!(
            PrefixDictionary::load(data, Vec::new(), Vec::new(), Vec::new(), Vec::new()).is_err()
        );
    }

    #[test]
    fn corrupted_nodes_yield_no_matches_without_panicking() {
        let map = sample_map();
        let (trie_bytes, idx_bytes) = PrefixDictionary::serialize_trie(&map).unwrap();

        // Flip bytes throughout the node array; every lookup must stay
        // panic-free (bounds-checked reads make garbage "no match").
        for step in [1usize, 3, 7, 13] {
            let mut corrupted = trie_bytes.clone();
            let start = corrupted.len().saturating_sub(200);
            let len = corrupted.len();
            for i in (start..len).step_by(step) {
                corrupted[i] ^= 0xa5;
            }
            if let Ok(dict) = PrefixDictionary::load(
                corrupted,
                idx_bytes.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ) {
                let chars: Vec<char> = "世界中で統計調査".chars().collect();
                for start in 0..chars.len() {
                    for _ in dict.common_prefix_search(&chars[start..]) {}
                }
            }
        }
    }

    #[test]
    fn serialize_trie_rejects_nul_in_surface() {
        let mut map = BTreeMap::new();
        map.insert("a\0b".to_string(), vec![entry(0, 0)]);
        assert!(PrefixDictionary::serialize_trie(&map).is_err());
    }

    #[test]
    fn empty_dictionary_matches_nothing() {
        let dict = PrefixDictionary::from_word_entry_map(&BTreeMap::new()).unwrap();
        let chars: Vec<char> = "何か".chars().collect();
        assert_eq!(dict.common_prefix_search(&chars).count(), 0);
    }

    #[test]
    fn user_dictionary_load_validates_da_bytes() {
        let keyset: Vec<(&[u8], u32)> = vec![(b"a", 0), (b"ab", 1), (b"b", 2)];
        let da = DoubleArrayAhoCorasickBuilder::new()
            .build_with_values(keyset)
            .unwrap();
        let da_bytes = da.serialize();

        let dict = UserPrefixDictionary::load(
            da_bytes.clone(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
        )
        .unwrap();
        assert_eq!(dict.da.find_overlapping_iter("ab").count(), 3);

        // Truncated bytes must be rejected by the validating deserializer,
        // not panic or read out of bounds.
        let mut truncated = da_bytes;
        truncated.truncate(4);
        assert!(
            UserPrefixDictionary::load(
                truncated,
                Vec::<u8>::new(),
                Vec::<u8>::new(),
                Vec::<u8>::new()
            )
            .is_err()
        );
    }
}
