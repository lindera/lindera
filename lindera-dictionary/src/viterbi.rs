use std::io;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::prefix_dictionary::{PrefixDictionary, UserPrefixDictionary};
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::mode::Mode;

/// Type of lexicon containing the word
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]

pub enum LexType {
    /// System dictionary (base dictionary)
    #[default]
    System,
    /// User dictionary (additional vocabulary)
    User,
    /// Unknown words (OOV handling)
    Unknown,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    RkyvDeserialize,
    RkyvSerialize,
)]

pub struct WordId {
    /// Numeric identifier of the word within its lexicon.
    id: u32,
    /// Whether the word originates from the system dictionary.
    is_system: bool,
    /// Lexicon type the word belongs to.
    lex_type: LexType,
}

impl WordId {
    /// Creates a new WordId with specified lexicon type
    pub fn new(lex_type: LexType, id: u32) -> Self {
        WordId {
            id,
            is_system: matches!(lex_type, LexType::System),
            lex_type,
        }
    }

    /// Returns the numeric identifier of the word within its lexicon.
    ///
    /// # 戻り値
    ///
    /// The lexicon-local word id.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns `true` when the word is an unknown-word entry.
    #[inline]
    pub fn is_unknown(&self) -> bool {
        matches!(self.lex_type, LexType::Unknown)
    }

    /// Returns `true` when the word originates from the system dictionary.
    #[inline]
    pub fn is_system(&self) -> bool {
        self.is_system
    }

    /// Returns the lexicon type of the word.
    #[inline]
    pub fn lex_type(&self) -> LexType {
        self.lex_type
    }
}

impl Default for WordId {
    fn default() -> Self {
        WordId {
            id: u32::MAX,
            is_system: true,
            lex_type: LexType::System,
        }
    }
}

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]

pub struct WordEntry {
    /// Word id identifying this entry in the dictionary.
    word_id: WordId,
    /// Emission (word) cost of the entry.
    word_cost: i16,
    /// Left context id used by the connection matrix.
    left_id: u16,
    /// Right context id used by the connection matrix.
    right_id: u16,
}

impl WordEntry {
    /// Length in bytes of the serialized representation.
    pub(crate) const SERIALIZED_LEN: usize = 10;

    /// Creates a new word entry from its raw components.
    ///
    /// # 引数
    ///
    /// * `word_id` - The word id identifying the entry.
    /// * `word_cost` - The emission cost of the word.
    /// * `left_id` - The left context id.
    /// * `right_id` - The right context id.
    #[inline]
    pub fn new(word_id: WordId, word_cost: i16, left_id: u16, right_id: u16) -> Self {
        WordEntry {
            word_id,
            word_cost,
            left_id,
            right_id,
        }
    }

    /// Returns the word id of this entry.
    #[inline]
    pub fn word_id(&self) -> WordId {
        self.word_id
    }

    /// Returns the emission (word) cost of this entry.
    #[inline]
    pub fn word_cost(&self) -> i16 {
        self.word_cost
    }

    /// Returns the left context id, widened to `u32`.
    #[inline]
    pub fn left_id(&self) -> u32 {
        self.left_id as u32
    }

    /// Returns the right context id, widened to `u32`.
    #[inline]
    pub fn right_id(&self) -> u32 {
        self.right_id as u32
    }

    /// Serializes this entry into `wtr` in little-endian byte order.
    pub(crate) fn serialize<W: io::Write>(&self, wtr: &mut W) -> io::Result<()> {
        wtr.write_u32::<LittleEndian>(self.word_id.id)?;
        wtr.write_i16::<LittleEndian>(self.word_cost)?;
        wtr.write_u16::<LittleEndian>(self.left_id)?;
        wtr.write_u16::<LittleEndian>(self.right_id)?;
        Ok(())
    }

    /// Deserializes a word entry from `data`.
    pub(crate) fn deserialize(data: &[u8], is_system_entry: bool) -> WordEntry {
        let word_id = WordId::new(
            if is_system_entry {
                LexType::System
            } else {
                LexType::User
            },
            LittleEndian::read_u32(&data[0..4]),
        );
        let word_cost = LittleEndian::read_i16(&data[4..6]);
        let left_id = LittleEndian::read_u16(&data[6..8]);
        let right_id = LittleEndian::read_u16(&data[8..10]);
        WordEntry {
            word_id,
            word_cost,
            left_id,
            right_id,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Edge {
    /// Word entry backing this edge.
    word_entry: WordEntry,

    /// Best forward path cost reaching this edge.
    path_cost: i32,
    /// Index of the chosen left edge in the previous position's vector.
    left_index: u16,

    /// Start byte position of the edge.
    start_index: u32,
    /// Stop byte position of the edge.
    stop_index: u32,

    /// Whether the edge surface consists solely of kanji.
    kanji_only: bool,
}

impl Edge {
    /// Returns the number of characters spanned by this edge.
    pub fn num_chars(&self) -> usize {
        (self.stop_index - self.start_index) as usize / 3
    }

    /// Returns the word entry backing this edge.
    #[inline]
    pub(crate) fn word_entry(&self) -> &WordEntry {
        &self.word_entry
    }

    /// Returns the best forward path cost reaching this edge.
    #[inline]
    pub(crate) fn path_cost(&self) -> i32 {
        self.path_cost
    }

    /// Returns the index of the chosen left edge in the previous position.
    #[inline]
    pub(crate) fn left_index(&self) -> u16 {
        self.left_index
    }

    /// Returns the start byte position of this edge.
    #[inline]
    pub(crate) fn start_index(&self) -> u32 {
        self.start_index
    }

    /// Returns the stop byte position of this edge.
    #[inline]
    pub(crate) fn stop_index(&self) -> u32 {
        self.stop_index
    }

    /// Returns whether the edge surface consists solely of kanji.
    #[inline]
    pub(crate) fn kanji_only(&self) -> bool {
        self.kanji_only
    }
}

/// Records a transition from a left edge to the current edge.
/// Used in N-Best mode to store all predecessor transitions
/// (not just the best one as in 1-best).
#[derive(Clone, Debug)]
pub struct PathEntry {
    /// Index of this edge in ends_at[stop_index]
    edge_index: u16,
    /// Byte position where the left edge ends (= this edge's start_index)
    left_pos: u32,
    /// Index of the left edge in ends_at[left_pos]
    left_index: u16,
    /// Total forward cost: left_edge.path_cost + conn_cost + penalty_cost
    cost: i32,
}

impl PathEntry {
    /// Returns the index of this edge in `ends_at[stop_index]`.
    #[inline]
    pub(crate) fn edge_index(&self) -> u16 {
        self.edge_index
    }

    /// Returns the byte position where the left edge ends.
    #[inline]
    pub(crate) fn left_pos(&self) -> u32 {
        self.left_pos
    }

    /// Returns the index of the left edge in `ends_at[left_pos]`.
    #[inline]
    pub(crate) fn left_index(&self) -> u16 {
        self.left_index
    }

    /// Returns the total forward cost of this transition.
    #[inline]
    pub(crate) fn cost(&self) -> i32 {
        self.cost
    }
}

#[derive(Clone, Default)]
pub struct Lattice {
    capacity: usize,
    ends_at: Vec<Vec<Edge>>, // Now stores edges directly
    char_info_buffer: Vec<CharData>,
    categories_buffer: Vec<CategoryId>,

    // N-Best fields (only populated when set_text_nbest is called)
    all_paths: Vec<Vec<PathEntry>>,
    nbest_capacity: usize,
    /// The text length (in bytes) of the last set_text/set_text_nbest call
    last_text_len: usize,

    // Scratch buffers for the Aho-Corasick match pre-scan in set_text/set_text_nbest.
    // Reused across calls (like the fields above) instead of being reallocated per
    // call, since set_text runs once per sentence rather than once per document.
    /// Linked-list head table: matches_head[start_idx] -> index into
    /// matches_store; u32::MAX terminates a list (#880 shrank the element
    /// widths to halve the per-sentence refill and walk traffic).
    matches_head: Vec<u32>,
    /// Linked-list node pool: (match end offset, word entry, next node index).
    matches_store: Vec<(u32, WordEntry, u32)>,
    /// The sentence's characters, materialized once per call because the
    /// char-wise trie consumes `&[char]` (byte offsets come from
    /// `char_info_buffer`, which is built in the same pass).
    chars_buf: Vec<char>,
    /// Per-position system-dictionary matches (match end byte offset, word
    /// entry), buffered so they can be replayed in reverse discovery order --
    /// the order the retired whole-text pre-scan's head-inserted list drained
    /// in, which decides the winner among equal-cost edges.
    sys_matches: Vec<(u32, WordEntry)>,
}

/// Upper bound applied to every stored `path_cost` so the relaxation loops
/// can use plain addition: one connection cost plus one penalty per step is
/// at most 2 * 32,767, which cannot overflow from this clamp.
const PATH_COST_CLAMP: i32 = i32::MAX - 131_072;

#[derive(Clone, Copy, Debug, Default)]
struct CharData {
    byte_offset: u32,
    is_kanji: bool,
    categories_start: u32,
    categories_len: u16,
    kanji_run_byte_len: u32,
}

#[inline]
pub fn is_kanji(c: char) -> bool {
    let c = c as u32;
    // CJK Unified Ideographs (4E00-9FAF) and Extension A (3400-4DBF)
    (0x4E00..=0x9FAF).contains(&c) || (0x3400..=0x4DBF).contains(&c)
}

impl Lattice {
    /// Helper method to create an edge efficiently
    #[inline]
    fn create_edge(word_entry: WordEntry, start: usize, stop: usize, kanji_only: bool) -> Edge {
        Edge {
            word_entry,
            left_index: u16::MAX,
            start_index: start as u32,
            stop_index: stop as u32,
            path_cost: i32::MAX,
            kanji_only,
        }
    }

    pub fn clear(&mut self) {
        // Only slots up to the previous sentence's length can hold entries:
        // every `ends_at`/`all_paths` write in set_text/set_text_nbest
        // targets an index <= that call's text length (BOS at 0, edges at
        // stop_index <= len, EOS at len), which `set_capacity` recorded in
        // `last_text_len`, and every slot past it was left empty by the
        // previous clear(). Walking only this prefix keeps clear() O(previous
        // sentence) instead of O(historical max capacity), which matters
        // once one long sentence has grown the lattice (#877).
        let bound = self.last_text_len + 1;
        for edge_vec in self.ends_at.iter_mut().take(bound) {
            edge_vec.clear();
        }
        debug_assert!(
            self.ends_at.iter().skip(bound).all(|v| v.is_empty()),
            "ends_at slot beyond last_text_len must be empty"
        );
        for path_vec in self.all_paths.iter_mut().take(bound) {
            path_vec.clear();
        }
        debug_assert!(
            self.all_paths.iter().skip(bound).all(|v| v.is_empty()),
            "all_paths slot beyond last_text_len must be empty"
        );
        self.char_info_buffer.clear();
        self.categories_buffer.clear();
    }

    #[inline]
    fn is_kanji_all(&self, char_idx: usize, byte_len: usize) -> bool {
        self.char_info_buffer[char_idx].kanji_run_byte_len >= byte_len as u32
    }

    #[inline]
    fn get_cached_category(&self, char_idx: usize, category_ord: usize) -> CategoryId {
        let char_data = &self.char_info_buffer[char_idx];
        self.categories_buffer[char_data.categories_start as usize + category_ord]
    }

    fn set_capacity(&mut self, text_len: usize) {
        self.clear();
        self.last_text_len = text_len;
        if self.capacity <= text_len {
            self.capacity = text_len;
            // Pre-size newly-grown slots (like Vibrato's reset_vec) to
            // avoid a couple of small reallocations the first time a busy
            // position accumulates several edges. `resize_with` is required
            // here: `resize` fills new slots with clones of its template
            // value, and cloning an empty Vec allocates capacity 0, so only
            // the moved-in last slot would actually be pre-sized (#827).
            self.ends_at
                .resize_with(text_len + 1, || Vec::with_capacity(16));
        }
    }

    fn set_capacity_nbest(&mut self, text_len: usize) {
        self.set_capacity(text_len);
        if self.nbest_capacity <= text_len {
            self.nbest_capacity = text_len;
            self.all_paths.resize(text_len + 1, Vec::new());
        }
    }

    /// Returns the lattice's current slot capacity: the largest sentence
    /// length (in bytes) whose `ends_at` slots are already allocated.
    ///
    /// # 戻り値
    ///
    /// The capacity in bytes. `0` for a fresh lattice.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Shrinks the internal buffers down to what a sentence of `text_len`
    /// bytes needs, releasing memory retained after processing a long
    /// sentence.
    ///
    /// The lattice grows monotonically (`set_capacity` never shrinks), so a
    /// single long sentence pins its worst-case allocation for the lifetime
    /// of the lattice. Long-lived holders (e.g. a reusable worker) can call
    /// this to bound retention. This is never called on the hot path:
    /// `clear()`/`set_text` stay shrink-free (#877/#884).
    ///
    /// Invariants preserved:
    /// - Every remaining `ends_at` slot keeps a capacity of at least 16, the
    ///   pre-size that avoids first-growth reallocations (#827/#841).
    /// - `clear()` runs first, so all slots are empty and the
    ///   `last_text_len` bound (#877) stays valid after truncation.
    ///
    /// # 引数
    ///
    /// * `text_len` - Target sentence length in bytes; buffers are reduced
    ///   to what a sentence of this length requires. Buffers already at or
    ///   below the target are left untouched.
    pub fn shrink_to(&mut self, text_len: usize) {
        self.clear();
        let slots = text_len + 1;
        if self.capacity > text_len {
            self.ends_at.truncate(slots);
            self.ends_at.shrink_to(slots);
            for slot in &mut self.ends_at {
                // Keep the per-slot pre-size intact (#841); only release
                // growth beyond it.
                slot.shrink_to(16);
            }
            self.capacity = text_len;
        }
        if self.nbest_capacity > text_len {
            self.all_paths.truncate(slots);
            self.all_paths.shrink_to(slots);
            for paths in &mut self.all_paths {
                paths.shrink_to(0);
            }
            self.nbest_capacity = text_len;
        }
        // All slots are empty after clear() + truncate, so lowering the
        // clear()/backtrace bound is safe (its debug_asserts hold trivially).
        self.last_text_len = self.last_text_len.min(text_len);
        // Scratch buffers are sized per sentence content, not per slot; the
        // bounds below are heuristics (roughly: categories per char, matches
        // per start position), not correctness requirements — set_text
        // regrows them on demand.
        self.char_info_buffer.shrink_to(text_len);
        self.categories_buffer.shrink_to(4 * text_len);
        self.matches_head.shrink_to(slots);
        self.matches_store.shrink_to(8 * slots);
        self.chars_buf.shrink_to(text_len);
        self.sys_matches.shrink_to(64);
    }

    /// Fills the per-sentence character buffers (`char_info_buffer`,
    /// `categories_buffer`, `chars_buf`) from `text`, appends the
    /// end-of-text sentinel, and precomputes the kanji run lengths consumed
    /// by the Decompose-mode penalty.
    ///
    /// Shared by `set_text` and `set_text_nbest` so the two hot paths cannot
    /// drift apart.
    ///
    /// # 引数
    ///
    /// * `char_definitions` - Character category definitions.
    /// * `text` - The sentence to prepare buffers for.
    fn prepare_char_buffers(&mut self, char_definitions: &CharacterDefinition, text: &str) {
        let len = text.len();
        self.char_info_buffer.clear();
        self.categories_buffer.clear();
        self.chars_buf.clear();

        for (byte_offset, c) in text.char_indices() {
            let categories_start = self.categories_buffer.len() as u32;

            // Category lookup is O(1) for BMP codepoints via the flat table
            // built at dictionary load (#878), so no per-lattice cache is
            // needed.
            let categories = char_definitions.lookup_categories(c);
            for &category in categories {
                self.categories_buffer.push(category);
            }

            let categories_len = (self.categories_buffer.len() as u32 - categories_start) as u16;

            self.char_info_buffer.push(CharData {
                byte_offset: byte_offset as u32,
                is_kanji: is_kanji(c),
                categories_start,
                categories_len,
                kanji_run_byte_len: 0,
            });
            self.chars_buf.push(c);
        }
        // Sentinel for end of text
        self.char_info_buffer.push(CharData {
            byte_offset: len as u32,
            is_kanji: false,
            categories_start: 0,
            categories_len: 0,
            kanji_run_byte_len: 0,
        });

        // Pre-calculate Kanji run lengths (backwards)
        for i in (0..self.char_info_buffer.len() - 1).rev() {
            if self.char_info_buffer[i].is_kanji {
                let next_byte_offset = self.char_info_buffer[i + 1].byte_offset;
                let char_byte_len = next_byte_offset - self.char_info_buffer[i].byte_offset;
                self.char_info_buffer[i].kanji_run_byte_len =
                    char_byte_len + self.char_info_buffer[i + 1].kanji_run_byte_len;
            } else {
                self.char_info_buffer[i].kanji_run_byte_len = 0;
            }
        }
    }

    #[inline(never)]
    // Forward Viterbi implementation:
    // Constructs the lattice and calculates the path costs simultaneously.
    // This improves performance by avoiding a separate lattice traversal pass.
    #[allow(clippy::too_many_arguments)]
    pub fn set_text(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&UserPrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        text: &str,
        search_mode: &Mode,
    ) {
        let len = text.len();
        self.set_capacity(len);

        // Pre-calculate character information for the text
        self.prepare_char_buffers(char_definitions, text);

        let start_edge = Edge {
            path_cost: 0,
            left_index: u16::MAX,
            ..Default::default()
        };
        self.ends_at[0].push(start_edge);

        // Index of the last character of unknown word
        let mut unknown_word_end: Option<usize> = None;

        // Pre-scan text with Aho-Corasick to report all matches
        // Optimization: Use flat vectors instead of Vec<Vec<_>> to avoid many small allocations.
        // Linked list structure: matches_head[start_idx] -> index in matches_store
        // Buffers are Lattice fields reused across calls; refill matches_head (its
        // contents are meaningful, unlike ends_at's empty-Vec slots) and clear
        // matches_store (a plain append-only pool).
        // The pool now holds only user-dictionary matches: the system
        // dictionary is searched per lattice-reachable position instead of
        // pre-scanned (#882). daachorse's API shape still forces a whole-text
        // scan for the user automaton, so its matches keep the linked list;
        // with no user dictionary the head table stays empty and the drain
        // below is skipped by its `start < matches_head.len()` guard.
        self.matches_head.clear();
        self.matches_store.clear();

        // User dictionary scan
        if let Some(ud) = user_dict {
            self.matches_head.resize(len + 1, u32::MAX);
            let ud_vals: &[u8] = &ud.vals_data;
            for m in ud.da.find_overlapping_iter(text) {
                let start = m.start();
                let (offset, count) = ud.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

                if start < self.matches_head.len() {
                    let avail = ud_vals.len().saturating_sub(offset_bytes);
                    let n = (count as usize).min(avail / WordEntry::SERIALIZED_LEN);
                    let block =
                        &ud_vals[offset_bytes..offset_bytes + n * WordEntry::SERIALIZED_LEN];
                    let end = m.end() as u32;
                    for chunk in block.chunks_exact(WordEntry::SERIALIZED_LEN) {
                        let entry = WordEntry::deserialize(chunk, false);
                        let next = self.matches_head[start];
                        self.matches_head[start] = self.matches_store.len() as u32;
                        self.matches_store.push((end, entry, next));
                    }
                }
            }
        }

        for char_idx in 0..self.char_info_buffer.len() - 1 {
            let start = self.char_info_buffer[char_idx].byte_offset as usize;

            // No arc is ending here.
            // No need to check if a valid word starts here.
            if self.ends_at[start].is_empty() {
                continue;
            }

            let mut found: bool = false;

            // Drain user-dictionary matches (reverse discovery order, from
            // the head-inserted list).
            if start < self.matches_head.len() {
                let mut match_idx = self.matches_head[start];
                while match_idx != u32::MAX {
                    let (end, word_entry, next) = self.matches_store[match_idx as usize];

                    let prefix_len = end as usize - start;
                    let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                    let edge = Self::create_edge(
                        word_entry, // WordEntry is Copy
                        start,
                        end as usize,
                        kanji_only,
                    );
                    self.add_edge_in_lattice(edge, cost_matrix, search_mode);
                    found = true;

                    match_idx = next;
                }
            }

            // System dictionary: per-position common-prefix search over the
            // in-place trie, run only at lattice-reachable positions (the
            // gate above). Matches are buffered and replayed in reverse so
            // equal-cost tie-breaks keep choosing the same winner as the
            // retired whole-text pre-scan; user matches were drained first
            // for the same reason (the old shared list held them on top).
            self.sys_matches.clear();
            {
                let suffix = &self.chars_buf[char_idx..];
                for (entries, end_char_offset) in dict.common_prefix_search(suffix) {
                    let end_char_idx = char_idx + end_char_offset;
                    let end = self.char_info_buffer[end_char_idx].byte_offset;
                    for chunk in entries.chunks_exact(WordEntry::SERIALIZED_LEN) {
                        self.sys_matches
                            .push((end, WordEntry::deserialize(chunk, true)));
                    }
                }
            }
            for i in (0..self.sys_matches.len()).rev() {
                let (end, word_entry) = self.sys_matches[i];
                let end = end as usize;
                let prefix_len = end - start;
                let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                let edge = Self::create_edge(word_entry, start, end, kanji_only);
                self.add_edge_in_lattice(edge, cost_matrix, search_mode);
                found = true;
            }

            // In the case of normal mode, it doesn't process unknown word greedily.
            if (search_mode.is_search()
                || unknown_word_end.map(|index| index <= start).unwrap_or(true))
                && char_idx < self.char_info_buffer.len() - 1
            {
                let num_categories = self.char_info_buffer[char_idx].categories_len as usize;
                for category_ord in 0..num_categories {
                    let category = self.get_cached_category(char_idx, category_ord);
                    unknown_word_end = self.process_unknown_word(
                        char_definitions,
                        unknown_dictionary,
                        cost_matrix,
                        search_mode,
                        category,
                        category_ord,
                        unknown_word_end,
                        start,
                        char_idx,
                        found,
                    );
                }
            }
        }

        // Connect EOS
        if !self.ends_at[len].is_empty() {
            let mut eos_edge = Edge {
                start_index: len as u32,
                stop_index: len as u32,
                ..Default::default()
            };
            // Calculate cost for EOS with the row hoisted (#880).
            let left_edges = &self.ends_at[len];
            let mut best_cost = i32::MAX;
            let mut best_left = None;
            let cost_row = cost_matrix.row(0); // EOS default left_id

            for (i, left_edge) in left_edges.iter().enumerate() {
                let path_cost =
                    left_edge.path_cost + cost_row[left_edge.word_entry.right_id() as usize] as i32;
                if path_cost < best_cost {
                    best_cost = path_cost;
                    best_left = Some(i as u16);
                }
            }
            if let Some(left_idx) = best_left {
                eos_edge.left_index = left_idx;
                eos_edge.path_cost = best_cost;
                self.ends_at[len].push(eos_edge);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_unknown_word(
        &mut self,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        category: CategoryId,
        category_ord: usize,
        unknown_word_index: Option<usize>,
        start: usize,
        char_idx: usize,
        found: bool,
    ) -> Option<usize> {
        let mut unknown_word_num_chars: usize = 0;
        let category_data = char_definitions.lookup_definition(category);
        if category_data.invoke || !found {
            unknown_word_num_chars = 1;
            if category_data.group {
                for i in 1.. {
                    let next_idx = char_idx + i;
                    if next_idx >= self.char_info_buffer.len() - 1 {
                        break;
                    }
                    let num_categories = self.char_info_buffer[next_idx].categories_len as usize;
                    let mut found_cat = false;
                    if category_ord < num_categories {
                        let cat = self.get_cached_category(next_idx, category_ord);
                        if cat == category {
                            unknown_word_num_chars += 1;
                            found_cat = true;
                        }
                    }
                    if !found_cat {
                        break;
                    }
                }
            }
        }
        if unknown_word_num_chars > 0 {
            let byte_end_offset =
                self.char_info_buffer[char_idx + unknown_word_num_chars].byte_offset;
            let byte_len = byte_end_offset as usize - start;

            // Check Kanji status using pre-calculated buffer
            let kanji_only = self.is_kanji_all(char_idx, byte_len);

            for &word_id in unknown_dictionary.lookup_word_ids(category) {
                let word_entry = unknown_dictionary.word_entry(word_id);
                let edge = Self::create_edge(word_entry, start, start + byte_len, kanji_only);
                self.add_edge_in_lattice(edge, cost_matrix, search_mode);
            }
            return Some(start + byte_len);
        }
        unknown_word_index
    }

    // Adds an edge to the lattice and calculates the minimum cost to reach it.
    fn add_edge_in_lattice(
        &mut self,
        mut edge: Edge,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
    ) {
        let start_index = edge.start_index as usize;
        let stop_index = edge.stop_index as usize;
        let right_left_id = edge.word_entry.left_id();

        if self.ends_at[start_index].is_empty() {
            return;
        }

        let mut best_cost = i32::MAX;
        let mut best_left = None;

        match mode {
            Mode::Normal => {
                // Matrix row hoisted out of the loop; the plain additions
                // cannot overflow thanks to PATH_COST_CLAMP (#880).
                let left_edges = &self.ends_at[start_index];
                let cost_row = cost_matrix.row(right_left_id);
                for (i, left_edge) in left_edges.iter().enumerate() {
                    let conn_cost = cost_row[left_edge.word_entry.right_id() as usize] as i32;
                    let total_cost = left_edge.path_cost + conn_cost;

                    if total_cost < best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u16);
                    }
                }
            }
            Mode::Decompose(penalty) => {
                let left_edges = &self.ends_at[start_index];
                for (i, left_edge) in left_edges.iter().enumerate() {
                    let left_right_id = left_edge.word_entry.right_id();
                    let conn_cost = cost_matrix.cost(left_right_id, right_left_id);
                    let penalty_cost = penalty.penalty(left_edge);
                    let total_cost = left_edge
                        .path_cost
                        .saturating_add(conn_cost)
                        .saturating_add(penalty_cost);

                    if total_cost < best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u16);
                    }
                }
            }
        }

        if let Some(best_left_idx) = best_left {
            edge.path_cost = best_cost
                .saturating_add(edge.word_entry.word_cost as i32)
                .min(PATH_COST_CLAMP);
            edge.left_index = best_left_idx;
            self.ends_at[stop_index].push(edge);
        }
    }

    /// Backtraces the best path and returns `(start_byte_offset, word_id)`
    /// pairs for each token in reading order (BOS/EOS excluded).
    ///
    /// # Returns
    ///
    /// A freshly allocated offsets vector; empty when the lattice holds no
    /// complete path. Prefer [`Lattice::tokens_offset_into`] in per-sentence
    /// loops to reuse one allocation across sentences.
    pub fn tokens_offset(&self) -> Vec<(usize, WordId)> {
        let mut offsets = Vec::new();
        self.tokens_offset_into(&mut offsets);
        offsets
    }

    /// Backtraces the best path into a caller-provided buffer, clearing it
    /// first, so the allocation can be reused across sentences.
    ///
    /// # Arguments
    ///
    /// * `offsets` - The buffer to fill with `(start_byte_offset, word_id)`
    ///   pairs in reading order (BOS/EOS excluded). Cleared on entry; left
    ///   empty when the lattice holds no complete path.
    pub fn tokens_offset_into(&self, offsets: &mut Vec<(usize, WordId)>) {
        offsets.clear();

        if self.ends_at.is_empty() {
            return;
        }

        // The EOS edge, when present, sits at `ends_at[last_text_len]`
        // (see set_text), and every slot past it is always empty, so the
        // backward scan starts there rather than at the historical
        // capacity end (#877).
        let mut last_idx = self.last_text_len.min(self.ends_at.len() - 1);
        while last_idx > 0 && self.ends_at[last_idx].is_empty() {
            last_idx -= 1;
        }

        if self.ends_at[last_idx].is_empty() {
            return;
        }

        let idx = self.ends_at[last_idx].len() - 1;
        let mut edge = &self.ends_at[last_idx][idx];

        if edge.left_index == u16::MAX {
            return;
        }

        loop {
            if edge.left_index == u16::MAX {
                break;
            }

            offsets.push((edge.start_index as usize, edge.word_entry.word_id));

            let left_idx = edge.left_index as usize;
            let start_idx = edge.start_index as usize;

            edge = &self.ends_at[start_idx][left_idx];
        }

        offsets.reverse();
        offsets.pop(); // Remove EOS
    }

    // --- N-Best support ---

    /// Returns the text length (in bytes) from the last set_text/set_text_nbest call.
    pub fn text_len(&self) -> usize {
        self.last_text_len
    }

    /// Returns the edges at a given byte position.
    pub fn edges_at(&self, byte_pos: usize) -> &[Edge] {
        &self.ends_at[byte_pos]
    }

    /// Returns the N-Best path entries at a given byte position.
    pub fn paths_at(&self, byte_pos: usize) -> &[PathEntry] {
        if byte_pos < self.all_paths.len() {
            &self.all_paths[byte_pos]
        } else {
            &[]
        }
    }

    /// Adds an edge to the lattice, recording ALL predecessor transitions for N-Best.
    fn add_edge_in_lattice_nbest(
        &mut self,
        mut edge: Edge,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
    ) {
        let start_index = edge.start_index as usize;
        let stop_index = edge.stop_index as usize;
        let right_left_id = edge.word_entry.left_id();

        if self.ends_at[start_index].is_empty() {
            return;
        }

        let mut best_cost = i32::MAX;
        let mut best_left = None;

        // The edge_index of the new edge being added
        let new_edge_index = self.ends_at[stop_index].len() as u16;

        match mode {
            Mode::Normal => {
                // Same hoisted-row scan as add_edge_in_lattice (#880).
                let cost_row = cost_matrix.row(right_left_id);
                for i in 0..self.ends_at[start_index].len() {
                    let left_edge = &self.ends_at[start_index][i];
                    let total_cost = left_edge.path_cost
                        + cost_row[left_edge.word_entry.right_id() as usize] as i32;

                    // Record ALL transitions for N-Best
                    self.all_paths[stop_index].push(PathEntry {
                        edge_index: new_edge_index,
                        left_pos: start_index as u32,
                        left_index: i as u16,
                        cost: total_cost,
                    });

                    if total_cost < best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u16);
                    }
                }
            }
            Mode::Decompose(penalty) => {
                for i in 0..self.ends_at[start_index].len() {
                    let left_edge = &self.ends_at[start_index][i];
                    let left_right_id = left_edge.word_entry.right_id();
                    let conn_cost = cost_matrix.cost(left_right_id, right_left_id);
                    let penalty_cost = penalty.penalty(left_edge);
                    let total_cost = left_edge
                        .path_cost
                        .saturating_add(conn_cost)
                        .saturating_add(penalty_cost);

                    // Record ALL transitions for N-Best
                    self.all_paths[stop_index].push(PathEntry {
                        edge_index: new_edge_index,
                        left_pos: start_index as u32,
                        left_index: i as u16,
                        cost: total_cost,
                    });

                    if total_cost < best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u16);
                    }
                }
            }
        }

        if let Some(best_left_idx) = best_left {
            edge.path_cost = best_cost
                .saturating_add(edge.word_entry.word_cost as i32)
                .min(PATH_COST_CLAMP);
            edge.left_index = best_left_idx;
            self.ends_at[stop_index].push(edge);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_unknown_word_nbest(
        &mut self,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        category: CategoryId,
        category_ord: usize,
        unknown_word_index: Option<usize>,
        start: usize,
        char_idx: usize,
        found: bool,
    ) -> Option<usize> {
        let mut unknown_word_num_chars: usize = 0;
        let category_data = char_definitions.lookup_definition(category);
        if category_data.invoke || !found {
            unknown_word_num_chars = 1;
            if category_data.group {
                for i in 1.. {
                    let next_idx = char_idx + i;
                    if next_idx >= self.char_info_buffer.len() - 1 {
                        break;
                    }
                    let num_categories = self.char_info_buffer[next_idx].categories_len as usize;
                    let mut found_cat = false;
                    if category_ord < num_categories {
                        let cat = self.get_cached_category(next_idx, category_ord);
                        if cat == category {
                            unknown_word_num_chars += 1;
                            found_cat = true;
                        }
                    }
                    if !found_cat {
                        break;
                    }
                }
            }
        }
        if unknown_word_num_chars > 0 {
            let byte_end_offset =
                self.char_info_buffer[char_idx + unknown_word_num_chars].byte_offset;
            let byte_len = byte_end_offset as usize - start;

            let kanji_only = self.is_kanji_all(char_idx, byte_len);

            for &word_id in unknown_dictionary.lookup_word_ids(category) {
                let word_entry = unknown_dictionary.word_entry(word_id);
                let edge = Self::create_edge(word_entry, start, start + byte_len, kanji_only);
                self.add_edge_in_lattice_nbest(edge, cost_matrix, search_mode);
            }
            return Some(start + byte_len);
        }
        unknown_word_index
    }

    /// Forward Viterbi implementation for N-Best mode.
    /// Same as set_text() but records ALL predecessor transitions in all_paths.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_text_nbest(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&UserPrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        text: &str,
        search_mode: &Mode,
    ) {
        let len = text.len();
        self.set_capacity_nbest(len);

        // Pre-calculate character information for the text
        self.prepare_char_buffers(char_definitions, text);

        let start_edge = Edge {
            path_cost: 0,
            left_index: u16::MAX,
            ..Default::default()
        };
        self.ends_at[0].push(start_edge);

        let mut unknown_word_end: Option<usize> = None;

        // Pre-scan text with Aho-Corasick
        // Buffers are Lattice fields reused across calls; refill matches_head (its
        // contents are meaningful, unlike ends_at's empty-Vec slots) and clear
        // matches_store (a plain append-only pool).
        // The pool now holds only user-dictionary matches; see set_text.
        self.matches_head.clear();
        self.matches_store.clear();

        // User dictionary scan
        if let Some(ud) = user_dict {
            self.matches_head.resize(len + 1, u32::MAX);
            let ud_vals: &[u8] = &ud.vals_data;
            for m in ud.da.find_overlapping_iter(text) {
                let start = m.start();
                let (offset, count) = ud.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

                if start < self.matches_head.len() {
                    let avail = ud_vals.len().saturating_sub(offset_bytes);
                    let n = (count as usize).min(avail / WordEntry::SERIALIZED_LEN);
                    let block =
                        &ud_vals[offset_bytes..offset_bytes + n * WordEntry::SERIALIZED_LEN];
                    let end = m.end() as u32;
                    for chunk in block.chunks_exact(WordEntry::SERIALIZED_LEN) {
                        let entry = WordEntry::deserialize(chunk, false);
                        let next = self.matches_head[start];
                        self.matches_head[start] = self.matches_store.len() as u32;
                        self.matches_store.push((end, entry, next));
                    }
                }
            }
        }

        for char_idx in 0..self.char_info_buffer.len() - 1 {
            let start = self.char_info_buffer[char_idx].byte_offset as usize;

            if self.ends_at[start].is_empty() {
                continue;
            }

            let mut found: bool = false;

            // Drain user-dictionary matches first; see set_text for why the
            // order matters.
            if start < self.matches_head.len() {
                let mut match_idx = self.matches_head[start];
                while match_idx != u32::MAX {
                    let (end, word_entry, next) = self.matches_store[match_idx as usize];

                    let prefix_len = end as usize - start;
                    let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                    let edge = Self::create_edge(word_entry, start, end as usize, kanji_only);
                    self.add_edge_in_lattice_nbest(edge, cost_matrix, search_mode);
                    found = true;

                    match_idx = next;
                }
            }

            // System dictionary: per-position common-prefix search over the
            // in-place trie, run only at lattice-reachable positions (the
            // gate above). Matches are buffered and replayed in reverse so
            // equal-cost tie-breaks keep choosing the same winner as the
            // retired whole-text pre-scan; user matches were drained first
            // for the same reason (the old shared list held them on top).
            self.sys_matches.clear();
            {
                let suffix = &self.chars_buf[char_idx..];
                for (entries, end_char_offset) in dict.common_prefix_search(suffix) {
                    let end_char_idx = char_idx + end_char_offset;
                    let end = self.char_info_buffer[end_char_idx].byte_offset;
                    for chunk in entries.chunks_exact(WordEntry::SERIALIZED_LEN) {
                        self.sys_matches
                            .push((end, WordEntry::deserialize(chunk, true)));
                    }
                }
            }
            for i in (0..self.sys_matches.len()).rev() {
                let (end, word_entry) = self.sys_matches[i];
                let end = end as usize;
                let prefix_len = end - start;
                let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                let edge = Self::create_edge(word_entry, start, end, kanji_only);
                self.add_edge_in_lattice_nbest(edge, cost_matrix, search_mode);
                found = true;
            }

            if (search_mode.is_search()
                || unknown_word_end.map(|index| index <= start).unwrap_or(true))
                && char_idx < self.char_info_buffer.len() - 1
            {
                let num_categories = self.char_info_buffer[char_idx].categories_len as usize;
                for category_ord in 0..num_categories {
                    let category = self.get_cached_category(char_idx, category_ord);
                    unknown_word_end = self.process_unknown_word_nbest(
                        char_definitions,
                        unknown_dictionary,
                        cost_matrix,
                        search_mode,
                        category,
                        category_ord,
                        unknown_word_end,
                        start,
                        char_idx,
                        found,
                    );
                }
            }
        }

        // Connect EOS with all-path recording
        if !self.ends_at[len].is_empty() {
            let eos_edge_index = self.ends_at[len].len() as u16;
            let mut eos_edge = Edge {
                start_index: len as u32,
                stop_index: len as u32,
                ..Default::default()
            };
            let mut best_cost = i32::MAX;
            let mut best_left = None;
            let cost_row = cost_matrix.row(0); // EOS default left_id

            for i in 0..self.ends_at[len].len() {
                let left_edge = &self.ends_at[len][i];
                let path_cost =
                    left_edge.path_cost + cost_row[left_edge.word_entry.right_id() as usize] as i32;

                // Record all transitions to EOS
                self.all_paths[len].push(PathEntry {
                    edge_index: eos_edge_index,
                    left_pos: len as u32,
                    left_index: i as u16,
                    cost: path_cost,
                });

                if path_cost < best_cost {
                    best_cost = path_cost;
                    best_left = Some(i as u16);
                }
            }
            if let Some(left_idx) = best_left {
                eos_edge.left_index = left_idx;
                eos_edge.path_cost = best_cost;
                self.ends_at[len].push(eos_edge);
            }
        }
    }

    /// Returns the top-N paths through the lattice.
    /// Each result is a (path, cost) pair where path is a Vec of (byte_start, WordId) pairs.
    /// The first result (index 0) is the 1-best path.
    /// If `unique` is true, paths with the same segmentation (same byte_start sequence)
    /// are deduplicated, keeping only the first (lowest cost) variant.
    /// If `cost_threshold` is Some(t), paths whose cost exceeds best_cost + t are discarded.
    /// Requires set_text_nbest() to have been called first.
    pub fn nbest_tokens_offset(
        &self,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> Vec<(Vec<(usize, WordId)>, i64)> {
        use std::collections::HashSet;

        use crate::nbest::NBestGenerator;
        let mut generator = NBestGenerator::new(self);
        let mut results = Vec::with_capacity(n);
        let mut best_cost: Option<i64> = None;

        if unique {
            let mut seen: HashSet<Vec<usize>> = HashSet::new();
            while results.len() < n {
                match generator.next() {
                    Some((path, cost)) => {
                        // Record best cost from first result
                        let bc = *best_cost.get_or_insert(cost);
                        // Skip if cost exceeds threshold
                        if let Some(threshold) = cost_threshold
                            && cost > bc + threshold
                        {
                            break;
                        }
                        let key: Vec<usize> = path.iter().map(|(start, _)| *start).collect();
                        if seen.insert(key) {
                            results.push((path, cost));
                        }
                    }
                    None => break,
                }
            }
        } else {
            while results.len() < n {
                match generator.next() {
                    Some((path, cost)) => {
                        let bc = *best_cost.get_or_insert(cost);
                        if let Some(threshold) = cost_threshold
                            && cost > bc + threshold
                        {
                            break;
                        }
                        results.push((path, cost));
                    }
                    None => break,
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use crate::viterbi::{Edge, Lattice, LexType, WordEntry, WordId};

    /// Builds an edge whose backtrace fields are set explicitly, for
    /// hand-assembled lattices in tests.
    fn test_edge(word_id: u32, start: usize, stop: usize, left_index: u16) -> Edge {
        let mut edge = Lattice::create_edge(
            WordEntry::new(WordId::new(LexType::System, word_id), 0, 0, 0),
            start,
            stop,
            false,
        );
        edge.left_index = left_index;
        edge.path_cost = 0;
        edge
    }

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry =
            WordEntry::new(WordId::new(LexType::System, 1u32), -17i16, 1411u16, 1412u16);
        word_entry.serialize(&mut buffer).unwrap();
        assert_eq!(WordEntry::SERIALIZED_LEN, buffer.len());
        let word_entry2 = WordEntry::deserialize(&buffer[..], true);
        assert_eq!(word_entry, word_entry2);
    }

    /// Regression test for #827: `Vec::resize` clones its template value
    /// into all but the last new slot, and cloning an empty Vec yields
    /// capacity 0, so only the last slot was actually pre-sized. Every
    /// newly-grown `ends_at` slot must get the intended pre-size, both on
    /// the initial growth and on a later, larger growth.
    #[test]
    fn test_set_capacity_presizes_all_new_slots() {
        let mut lattice = Lattice::default();

        lattice.set_capacity(5);
        assert_eq!(lattice.ends_at.len(), 6);
        for (i, slot) in lattice.ends_at.iter().enumerate() {
            assert!(
                slot.capacity() >= 16,
                "slot {} has capacity {} < 16 after initial growth",
                i,
                slot.capacity()
            );
        }

        // Growing an already-used lattice must pre-size the appended slots too.
        lattice.set_capacity(10);
        assert_eq!(lattice.ends_at.len(), 11);
        for (i, slot) in lattice.ends_at.iter().enumerate() {
            assert!(
                slot.capacity() >= 16,
                "slot {} has capacity {} < 16 after second growth",
                i,
                slot.capacity()
            );
        }
    }

    /// Regression test for #877: `clear()` walks only `..=last_text_len`
    /// instead of the historical max capacity, so it must still clear every
    /// slot the previous sentence could have written — including the
    /// boundary slot at exactly `last_text_len` (EOS position).
    #[test]
    fn test_clear_after_shrink_leaves_no_stale_edges() {
        let mut lattice = Lattice::default();

        // Long sentence: capacity grows to 101 slots, writes up to index 100.
        lattice.set_capacity(100);
        lattice.ends_at[0].push(test_edge(1, 0, 0, u16::MAX));
        lattice.ends_at[57].push(test_edge(2, 0, 57, 0));
        lattice.ends_at[100].push(test_edge(3, 57, 100, 0)); // boundary slot

        // Shorter sentence: clear() runs bounded by the previous
        // last_text_len (100), then records the new length.
        lattice.set_capacity(10);
        assert!(
            lattice.ends_at.iter().all(|v| v.is_empty()),
            "stale edges survived a bounded clear"
        );

        // A second shrink exercises the induction step: nothing past the
        // new bound (10) may hold entries, and slots within it are cleared.
        lattice.ends_at[10].push(test_edge(4, 0, 10, 0)); // boundary slot again
        lattice.set_capacity(3);
        assert!(
            lattice.ends_at.iter().all(|v| v.is_empty()),
            "stale edge at the previous boundary slot survived"
        );
    }

    /// Regression test for #877: the `tokens_offset` backward scan starts at
    /// `last_text_len`, which must still find the EOS edge at exactly that
    /// index after the capacity has grown far beyond the current sentence.
    #[test]
    fn test_tokens_offset_finds_eos_at_last_text_len_after_shrink() {
        let mut lattice = Lattice::default();

        // Grow capacity well past the sentence we are about to assemble.
        lattice.set_capacity(100);

        // Hand-assembled best path for a 3-byte sentence:
        // BOS(ends_at[0]) <- token A (0..3) <- EOS(ends_at[3]).
        lattice.set_capacity(3);
        lattice.ends_at[0].push(test_edge(0, 0, 0, u16::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(42, 0, 3, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 3, 0)); // EOS -> token A

        let offsets = lattice.tokens_offset();
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].1, WordId::new(LexType::System, 42));
    }

    /// `shrink_to` must release slots beyond the target while preserving
    /// the #841 per-slot pre-size on the remaining slots, and the lattice
    /// must regrow correctly (pre-sized) afterwards.
    #[test]
    fn test_shrink_to_truncates_and_keeps_presize() {
        let mut lattice = Lattice::default();

        lattice.set_capacity(100);
        lattice.ends_at[0].push(test_edge(1, 0, 0, u16::MAX));
        lattice.ends_at[100].push(test_edge(2, 0, 100, 0));

        lattice.shrink_to(10);
        assert_eq!(lattice.capacity(), 10);
        assert_eq!(lattice.ends_at.len(), 11);
        assert!(
            lattice.ends_at.iter().all(|v| v.is_empty()),
            "shrink_to must clear all slots"
        );
        for (i, slot) in lattice.ends_at.iter().enumerate() {
            assert!(
                slot.capacity() >= 16,
                "slot {} lost its pre-size after shrink_to (capacity {})",
                i,
                slot.capacity()
            );
        }

        // Regrowth after a shrink must pre-size the appended slots again.
        lattice.set_capacity(50);
        assert_eq!(lattice.ends_at.len(), 51);
        for (i, slot) in lattice.ends_at.iter().enumerate() {
            assert!(
                slot.capacity() >= 16,
                "slot {} not pre-sized after regrowth (capacity {})",
                i,
                slot.capacity()
            );
        }
    }

    /// `shrink_to` with a target at or above the current capacity must be a
    /// no-op for the slot vectors (no truncation, no capacity change).
    #[test]
    fn test_shrink_to_noop_when_target_not_smaller() {
        let mut lattice = Lattice::default();
        lattice.set_capacity(5);

        lattice.shrink_to(100);
        assert_eq!(lattice.capacity(), 5);
        assert_eq!(lattice.ends_at.len(), 6);

        lattice.shrink_to(5);
        assert_eq!(lattice.capacity(), 5);
        assert_eq!(lattice.ends_at.len(), 6);

        // A fresh lattice tolerates shrink_to without panicking.
        let mut fresh = Lattice::default();
        fresh.shrink_to(0);
        assert_eq!(fresh.capacity(), 0);
        assert!(fresh.ends_at.is_empty());
    }

    /// A lattice must produce a correct backtrace when used again after
    /// `shrink_to`: the `last_text_len` bound and the EOS scan start must
    /// stay consistent (same guarantee as the #877 regression tests, with a
    /// shrink in between).
    #[test]
    fn test_backtrace_works_after_shrink_to() {
        let mut lattice = Lattice::default();
        lattice.set_capacity(100);
        lattice.ends_at[0].push(test_edge(1, 0, 0, u16::MAX));
        lattice.ends_at[100].push(test_edge(2, 0, 100, 0));

        lattice.shrink_to(10);

        // Hand-assemble a 3-byte sentence path, as in the #877 tests.
        lattice.set_capacity(3);
        lattice.ends_at[0].push(test_edge(0, 0, 0, u16::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(42, 0, 3, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 3, 0)); // EOS -> token A

        let offsets = lattice.tokens_offset();
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].1, WordId::new(LexType::System, 42));

        // clear() after the shrink must leave nothing behind.
        lattice.clear();
        assert!(lattice.ends_at.iter().all(|v| v.is_empty()));
    }

    /// `shrink_to` must also release the N-Best `all_paths` slots.
    #[test]
    fn test_shrink_to_releases_nbest_paths() {
        let mut lattice = Lattice::default();
        lattice.set_capacity_nbest(100);
        assert_eq!(lattice.all_paths.len(), 101);

        lattice.shrink_to(10);
        assert_eq!(lattice.all_paths.len(), 11);
        assert_eq!(lattice.nbest_capacity, 10);
        assert!(lattice.all_paths.iter().all(|v| v.is_empty()));

        // Regrowth of the nbest side after a shrink.
        lattice.set_capacity_nbest(20);
        assert_eq!(lattice.all_paths.len(), 21);
    }

    /// `tokens_offset_into` must clear the caller's buffer and produce the
    /// same result as `tokens_offset`, including on a pathless lattice.
    #[test]
    fn test_tokens_offset_into_matches_tokens_offset() {
        let mut lattice = Lattice::default();
        lattice.set_capacity(3);
        lattice.ends_at[0].push(test_edge(0, 0, 0, u16::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(7, 0, 3, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 3, 0)); // EOS -> token A

        let mut reused = vec![(999usize, WordId::default())]; // stale content
        lattice.tokens_offset_into(&mut reused);
        assert_eq!(reused, lattice.tokens_offset());
        assert_eq!(reused.len(), 1);

        // A cleared (pathless) lattice must leave the reused buffer empty.
        lattice.clear();
        lattice.tokens_offset_into(&mut reused);
        assert!(reused.is_empty());
        assert!(lattice.tokens_offset().is_empty());
    }
}
