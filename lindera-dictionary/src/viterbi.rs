use std::io;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::prefix_dictionary::{PrefixDictionary, UserPrefixDictionary};
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::mode::{Mode, Penalty};
use crate::space_penalty::SpacePenaltyTable;
use crate::whitespace::WhitespaceClassifier;

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
    ///
    /// Takes a fixed-size array rather than a slice so that the four
    /// fixed-offset reads below are in bounds by construction, with no
    /// runtime length check. Callers split their byte blocks with
    /// [`slice::as_chunks`], which yields exactly this type.
    ///
    /// # Arguments
    ///
    /// * `data` - The serialized entry, exactly [`Self::SERIALIZED_LEN`] bytes.
    /// * `is_system_entry` - Whether the entry comes from the system lexicon
    ///   (as opposed to a user dictionary).
    ///
    /// # Returns
    ///
    /// The decoded word entry.
    pub(crate) fn deserialize(
        data: &[u8; Self::SERIALIZED_LEN],
        is_system_entry: bool,
    ) -> WordEntry {
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

/// One token of a backtraced path: its start and end byte offsets within
/// the sentence and its word id. With whitespace skipped, the end leaves the
/// skipped whitespace out but keeps the whitespace an entry itself ends with
/// (#1108).
pub type TokenOffset = (usize, usize, WordId);

/// One N-best path: its tokens in reading order and its total cost.
pub type NBestPath = (Vec<TokenOffset>, i64);

/// Bit in [`Edge::flags`] marking an edge whose surface is entirely kanji.
const EDGE_FLAG_KANJI_ONLY: u8 = 0b100;

/// Mask over [`Edge::flags`] selecting the lexicon-type bits
/// (0 = System, 1 = User, 2 = Unknown).
const EDGE_LEX_TYPE_MASK: u8 = 0b011;

/// A lattice edge in the packed runtime representation (#943): 24 bytes,
/// deliberately decoupled from the on-disk `WordEntry`/`WordId` types so
/// shrinking it never touches the dictionary format. Positions are stored
/// in characters. The edge's stop position is not stored: it is the index
/// of the `ends_at` slot holding the edge, except for an edge carried over
/// skipped whitespace, which ends where the run starts plus the whitespace
/// its own surface ends with (see `ws_tail`).
#[derive(Clone, Debug)]
pub struct Edge {
    /// Numeric word id within its lexicon (`u32::MAX` for BOS/EOS).
    word_id: u32,
    /// Best forward path cost reaching this edge.
    path_cost: i32,
    /// Index of the chosen left edge in the start slot's vector
    /// (`u32::MAX` = no predecessor, i.e. BOS). A `u32`, as one slot can
    /// hold more than `u16::MAX` edges: the grouped unknown words starting
    /// at the positions of a long run all end at the run's end (#1105).
    left_index: u32,
    /// Left context id (read at insertion time).
    left_id: u16,
    /// Right context id (read by every relaxation scan).
    right_id: u16,
    /// Emission cost (read at insertion time and during nbest A* expansion).
    word_cost: i16,
    /// Start position of the edge, in characters. `set_text` asserts
    /// `n_chars < u16::MAX`, which every Segmenter-mediated path satisfies
    /// via the `MAX_SENTENCE_BYTES` sentence splitting.
    start_char: u16,
    /// Packed lexicon type ([`EDGE_LEX_TYPE_MASK`]) and kanji-only flag
    /// ([`EDGE_FLAG_KANJI_ONLY`]).
    flags: u8,
    /// Distance in characters from the content end of the slot holding the
    /// edge (the start of the whitespace run before it) to the edge's own
    /// end: the whitespace its surface ends with, saturating at `u8::MAX`.
    /// Only `carry_over_whitespace` writes it, so it stays 0 unless
    /// whitespace is skipped (#1108).
    ws_tail: u8,
}

/// Every lattice slot holds a vector of `Edge`s, so its size is a throughput
/// concern: the fields take 22 bytes, which the 4-byte alignment pads to 24,
/// so 2 bytes of padding remain for a future field. Asserted so that a
/// future field cannot silently grow every edge to 28 bytes.
const _: () = assert!(size_of::<Edge>() == 24);

/// Mirrors the previous derived `Default` semantics: a BOS/EOS sentinel
/// carrying the out-of-lexicon word id, System lexicon type, zero costs.
impl Default for Edge {
    /// Returns the BOS/EOS sentinel edge value.
    fn default() -> Self {
        Edge {
            word_id: u32::MAX,
            path_cost: 0,
            left_index: 0,
            left_id: 0,
            right_id: 0,
            word_cost: 0,
            start_char: 0,
            flags: 0, // LexType::System, not kanji-only
            ws_tail: 0,
        }
    }
}

impl Edge {
    /// Packs a lexicon type and the kanji-only flag into the flags byte.
    ///
    /// # 引数
    ///
    /// * `lex_type` - The lexicon type to encode.
    /// * `kanji_only` - Whether the edge surface is entirely kanji.
    ///
    /// # 戻り値
    ///
    /// The packed flags byte.
    #[inline]
    fn pack_flags(lex_type: LexType, kanji_only: bool) -> u8 {
        let lex_bits = match lex_type {
            LexType::System => 0,
            LexType::User => 1,
            LexType::Unknown => 2,
        };
        lex_bits | if kanji_only { EDGE_FLAG_KANJI_ONLY } else { 0 }
    }

    /// Returns the lexicon type encoded in the flags byte.
    #[inline]
    fn lex_type(&self) -> LexType {
        match self.flags & EDGE_LEX_TYPE_MASK {
            0 => LexType::System,
            1 => LexType::User,
            _ => LexType::Unknown,
        }
    }

    /// Reconstructs the word id (with its lexicon type) backing this edge.
    ///
    /// # 戻り値
    ///
    /// The word id; `WordId::new` re-derives the `is_system` flag.
    #[inline]
    pub(crate) fn word_id(&self) -> WordId {
        WordId::new(self.lex_type(), self.word_id)
    }

    /// Returns where the edge ends: the content end of the slot holding it
    /// plus the whitespace its own surface ends with (#1108).
    ///
    /// # 引数
    ///
    /// * `content_end` - `Lattice::content_end` of the slot holding the edge.
    ///
    /// # 戻り値
    ///
    /// The end position, in characters.
    #[inline]
    fn end_char(&self, content_end: usize) -> usize {
        content_end + self.ws_tail as usize
    }

    /// Returns the length of the edge's own span, its trailing whitespace
    /// included and the whitespace skipped after it excluded: what the
    /// Decompose length penalty measures.
    ///
    /// # 引数
    ///
    /// * `content_end` - `Lattice::content_end` of the slot holding the edge.
    ///
    /// # 戻り値
    ///
    /// The length, in characters.
    #[inline]
    fn char_len(&self, content_end: usize) -> usize {
        self.end_char(content_end)
            .saturating_sub(self.start_char as usize)
    }

    /// Returns the emission (word) cost of this edge.
    #[inline]
    pub(crate) fn word_cost(&self) -> i16 {
        self.word_cost
    }

    /// Returns the best forward path cost reaching this edge.
    #[inline]
    pub(crate) fn path_cost(&self) -> i32 {
        self.path_cost
    }

    /// Returns the index of the chosen left edge in the previous position.
    #[inline]
    pub(crate) fn left_index(&self) -> u32 {
        self.left_index
    }

    /// Returns the start position of this edge, in characters.
    #[inline]
    pub(crate) fn start_char(&self) -> u16 {
        self.start_char
    }

    /// Returns the right context id of this edge: the context the edge
    /// after it connects to.
    ///
    /// # 戻り値
    ///
    /// The right context id.
    #[inline]
    pub(crate) fn right_id(&self) -> u16 {
        self.right_id
    }

    /// Returns whether the edge surface consists solely of kanji.
    #[inline]
    pub(crate) fn kanji_only(&self) -> bool {
        self.flags & EDGE_FLAG_KANJI_ONLY != 0
    }
}

/// Records a transition from a left edge to the current edge.
/// Used in N-Best mode to store all predecessor transitions
/// (not just the best one as in 1-best).
#[derive(Clone, Debug)]
pub struct PathEntry {
    /// Index of this edge in ends_at[stop_char]; a `u32` like
    /// `Edge::left_index`, as a slot can hold more than `u16::MAX` edges
    /// (#1105).
    edge_index: u32,
    /// The slot holding the left edge (= this edge's start_char), which is
    /// where the left edge ends unless it was carried over skipped
    /// whitespace. Kept as u32: narrowing would not shrink the struct
    /// (alignment pads it back to 16 bytes) and would only add an
    /// overflow surface.
    left_pos: u32,
    /// Index of the left edge in ends_at[left_pos] (`u32`, see
    /// `edge_index`).
    left_index: u32,
    /// Total forward cost: left_edge.path_cost + conn_cost + penalty_cost
    cost: i32,
}

impl PathEntry {
    /// Returns the index of this edge in `ends_at[stop_char]`.
    #[inline]
    pub(crate) fn edge_index(&self) -> u32 {
        self.edge_index
    }

    /// Returns the character position where the left edge ends.
    #[inline]
    pub(crate) fn left_pos(&self) -> u32 {
        self.left_pos
    }

    /// Returns the index of the left edge in `ends_at[left_pos]`.
    #[inline]
    pub(crate) fn left_index(&self) -> u32 {
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
    /// Largest sentence length (in characters) whose slots are allocated.
    capacity: usize,
    /// The number of BOS edges of the current sentence, set by
    /// `push_bos_edges`; 0 for a lattice built by hand, which counts as the
    /// default single BOS edge (see [`Lattice::bos_index`]).
    bos_len: usize,
    /// Per-character-position edge slots (#943): `ends_at[i]` holds the
    /// edges ending at char position `i`, 0 = BOS, `n_chars` = EOS. With
    /// whitespace skipped it also holds the edges carried over the run
    /// before `i`, which end inside or before the run (`Edge::ws_tail`).
    ends_at: Vec<Vec<Edge>>,
    char_info_buffer: Vec<CharData>,
    categories_buffer: Vec<CategoryId>,

    // N-Best fields (only populated when set_text_nbest is called)
    all_paths: Vec<Vec<PathEntry>>,
    nbest_capacity: usize,
    /// The character count of the last set_text/set_text_nbest call
    last_n_chars: usize,
    /// The largest per-sentence character count observed since the last
    /// `shrink_to`/`take_max_char_len`; auto-shrink accounting for workers.
    max_chars_seen: usize,

    // Scratch buffers for the Aho-Corasick match pre-scan in set_text/set_text_nbest.
    // Reused across calls (like the fields above) instead of being reallocated per
    // call, since set_text runs once per sentence rather than once per document.
    /// Linked-list head table: matches_head[start_char] -> index into
    /// matches_store; u32::MAX terminates a list (#880 shrank the element
    /// widths to halve the per-sentence refill and walk traffic). Nodes are
    /// inserted at the head, a surface's entries in CSV order, so a list
    /// drains each surface's entries last row first, the order they are
    /// added to the lattice in, and the first row wins a tie (#1131, #1135).
    matches_head: Vec<u32>,
    /// Linked-list node pool: (match end char position, word entry, next
    /// node index).
    matches_store: Vec<(u32, WordEntry, u32)>,
    /// The sentence's characters, materialized once per call because the
    /// char-wise trie consumes `&[char]` (byte offsets come from
    /// `char_info_buffer`, which is built in the same pass).
    chars_buf: Vec<char>,
    /// The sentence's characters mapped through the system trie's code
    /// table, one code per char (`INVALID_CODE` = unmapped), filled in the
    /// same pass as `chars_buf` so overlapping prefix walks stop re-probing
    /// the code table for the same character (#942).
    codes_buf: Vec<u32>,
    /// Per-position system-dictionary matches (match end char position,
    /// word entry) in discovery order: shortest surface first, each
    /// surface's entries in CSV order. They are added to the lattice in
    /// reverse, so the first row wins a tie (#1131, #1135).
    /// Buffered because the trie search borrows `codes_buf` while adding an
    /// edge borrows the whole lattice.
    sys_matches: Vec<(u32, WordEntry)>,
    /// The last grouping run found per category ordinal, as `(category,
    /// end)`: the characters from the position that scanned the run up to
    /// `end` (exclusive) all carry `category` at that ordinal. The
    /// unknown-word logic visits every reachable position of a sentence in
    /// increasing order (#1105), so a later position inside the run reads
    /// its length from here instead of scanning again, which keeps the scans
    /// O(n) per ordinal (#944). Cleared per sentence by
    /// `prepare_char_buffers`.
    group_run_ends: Vec<(CategoryId, u32)>,
    /// Decompose penalty cache for the left edges of the position
    /// currently being relaxed; see `add_edge_in_lattice` (#944).
    penalty_cache: Vec<i32>,
    /// N-best lattice only: the cost of the transition from each left edge
    /// of the position being relaxed, in the order of the left edges, as
    /// `relax_nbest` computed them for one incoming context id.
    /// `push_relaxed_nbest` records them for every edge that shares the
    /// relaxation (the lengths of one unknown-word entry).
    transition_costs: Vec<i32>,
    /// The char position `penalty_cache` is valid for (`usize::MAX` =
    /// invalid; reset at the start of every sentence).
    penalty_cache_pos: usize,
    /// Whether the current sentence skips whitespace (see
    /// [`LatticeOptions::skip_whitespace`]); set at the start of every
    /// sentence and read by the Decompose length penalty and the backtraces,
    /// which take an edge carried over a whitespace run to end where its own
    /// surface ends.
    skip_whitespace: bool,
    /// Number of edges `ends_at[last_n_chars]` held when EOS was connected:
    /// the edges a path can end with before EOS, from which the sentence's
    /// exits come (see [`Lattice::exits_into`]). They are the first edges of
    /// that slot; the EOS edge, when pushed, follows them. 0 when no edge
    /// reaches the end of the sentence; reset by `clear()`.
    final_edge_count: usize,
    /// Decompose mode only: the length penalty of each final edge (see
    /// `final_edge_count`), computed by the EOS connection, which is the
    /// penalty such an edge pays whatever edge follows it. Empty in Normal
    /// mode, where the penalty is 0, so Normal sentences never touch it;
    /// cleared by `clear()`.
    exit_penalties: Vec<i32>,
}

/// Upper bound applied to every stored `path_cost` so the relaxation loops
/// can use plain addition: one connection cost plus one penalty per step is
/// at most 2 * 32,767, which cannot overflow from this clamp. Costs that are
/// not bounded by `i16` (the left-space penalty and `extra_cost`, both
/// arbitrary `i32`) are folded in with `saturating_add` instead.
const PATH_COST_CLAMP: i32 = i32::MAX - 131_072;

/// One BOS (beginning-of-sentence) edge of a lattice; see
/// [`LatticeOptions::bos`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BosContext {
    /// Right context id of the BOS edge: the first word of the sentence
    /// pays the connection cost from this context to its own left context
    /// id. The dictionary's BOS context is 0.
    pub right_id: u16,
    /// Initial path cost of the BOS edge, part of the cost of every path
    /// that starts from it. Clamped to `0..=i32::MAX - 131_072` when the
    /// lattice is built, so pass costs relative to the cheapest context.
    pub cost: i32,
}

/// One exit of a sentence: for one right context id, the best path from a
/// BOS edge to a word that ends at the end of the sentence and has that
/// right id, without EOS. Returned by [`Lattice::exits_into`]; its path is
/// [`Lattice::exit_tokens_offset_into`], and every path that ends with that
/// right id is
/// [`NBestGenerator::from_exit`](crate::nbest::NBestGenerator::from_exit).
///
/// A segmenter that cuts a line into sentences uses the exits to carry the
/// context of one sentence over to the next (see [`LatticeOptions::bos`]):
/// the next sentence's lattice continues them as if the line were one
/// lattice, without the EOS connection in between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatticeExit {
    /// Right context id of the last word of the path (of the BOS edge for a
    /// sentence without words).
    right_id: u16,
    /// Cost of the path: the BOS edge's cost, the word costs, the
    /// connections and penalties between them, and the last word's
    /// Decompose length penalty (the penalty it pays whatever follows it),
    /// but not the EOS connection.
    cost: i32,
    /// Index in `ends_at[n_chars]` of the last edge of the path: the
    /// cheapest final edge with this right id, the last one in the slot on
    /// ties (as the EOS connection chooses).
    edge: u32,
}

impl LatticeExit {
    /// Returns the right context id of the last word of the path, the
    /// context the next word connects to (the BOS edge's for a sentence
    /// without words).
    ///
    /// # Returns
    ///
    /// The right context id.
    #[inline]
    pub fn right_id(&self) -> u16 {
        self.right_id
    }

    /// Returns the cost of the path: everything from the BOS edge's cost to
    /// the last word's Decompose length penalty, without the EOS connection.
    ///
    /// # Returns
    ///
    /// The cost; lower is better.
    #[inline]
    pub fn cost(&self) -> i32 {
        self.cost
    }
}

/// Per-sentence options for [`Lattice::set_text_with_options`] and
/// [`Lattice::set_text_nbest_with_options`].
///
/// Bundles the knobs that used to be passed positionally so new ones can be
/// added without changing the `set_text*` signatures. Construct with
/// [`LatticeOptions::new`] and assign the fields you want to change; the
/// struct is `#[non_exhaustive]`, so a literal cannot be written outside
/// this crate.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct LatticeOptions<'a> {
    /// The segmentation mode.
    pub mode: &'a Mode,
    /// Unknown-word grouping cap (MeCab's `max-grouping-size`; `None` =
    /// unbounded).
    pub max_grouping_len: Option<usize>,
    /// Whether to emit the unknown-word length ladder (#945).
    pub unknown_word_ladder: bool,
    /// Left-space penalty table; `None` (the default) adds no penalty. When
    /// set, a candidate whose start position is preceded by a whitespace
    /// character within the sentence has [`SpacePenaltyTable::cost`] added to
    /// its path cost, in every mode and in both the 1-best and the N-best
    /// lattice. "Whitespace" is what [`SpacePenaltyTable::is_space`] accepts:
    /// a character carrying the dictionary's `SPACE` category, or
    /// `char::is_whitespace` for a dictionary that defines no such category.
    pub space_penalty: Option<&'a SpacePenaltyTable>,
    /// MeCab-compatible whitespace handling; `None` (the default) keeps
    /// whitespace in the lattice. When set, no node starts on a character
    /// the classifier accepts (the dictionary's `SPACE` category), and the
    /// edges ending before or inside a run of such characters are carried
    /// over to the end of the run, so the words on either side connect
    /// directly and pay the connection cost the dictionary was trained with,
    /// in every mode and in both the 1-best and the N-best lattice. Whitespace
    /// otherwise becomes a node of its own (a whitespace dictionary entry or
    /// the `SPACE` unknown word) that the words on either side connect to;
    /// ko-dic's connection costs for the `SPACE` unknown word are all zero,
    /// so there each space resets the context.
    ///
    /// A carried edge keeps its start position, and the backtraces return
    /// its real end: the whitespace its own surface ends with (an entry such
    /// as IPADIC-NEologd's `GeForce GTX Titan X` followed by a space) is part
    /// of it, as in MeCab, and the skipped whitespace after it is not.
    /// When `space_penalty` is set as well, this classifier also decides
    /// which characters count as whitespace for the penalty.
    pub skip_whitespace: Option<&'a WhitespaceClassifier>,
    /// The BOS edges the sentence starts from. Empty (the default) means
    /// the single BOS edge of the dictionary: right context id 0, cost 0.
    ///
    /// Otherwise the lattice gets one BOS edge per context, in the 1-best
    /// and the N-best lattice alike. A path's cost includes the cost of the
    /// BOS edge it starts from, so the best path is the best over all
    /// contexts; of paths of equal cost that differ only in their context,
    /// the one from the context that comes first here wins (the edges are
    /// stored in reverse, see [`Lattice::edges_at_char`]). A segmenter uses
    /// this to carry the context of the
    /// previous sentence across a cut: one context per exit of that
    /// sentence (see [`Lattice::exits_into`]). The index of a context in
    /// this slice is the BOS index that [`Lattice::tokens_offset_into`],
    /// [`Lattice::exit_tokens_offset_into`] and
    /// [`NBestGenerator::next_with_bos`](crate::nbest::NBestGenerator::next_with_bos)
    /// return. At most `u16::MAX - 1` contexts.
    ///
    /// [`Lattice::nbest_tokens_offset`] is not meant for several BOS edges:
    /// it does not say which one a path starts from, and `unique` folds
    /// paths that differ only in it.
    pub bos: &'a [BosContext],
}

impl<'a> LatticeOptions<'a> {
    /// Options for `mode` with the defaults `set_text` uses: unbounded
    /// grouping, the length ladder on, no space penalty, whitespace kept in
    /// the lattice, the dictionary's single BOS edge.
    ///
    /// # Arguments
    ///
    /// * `mode` - The segmentation mode.
    ///
    /// # Returns
    ///
    /// The options.
    pub fn new(mode: &'a Mode) -> Self {
        Self {
            mode,
            max_grouping_len: None,
            unknown_word_ladder: true,
            space_penalty: None,
            skip_whitespace: None,
            bos: &[],
        }
    }

    /// Returns the space-penalty table to consult for candidates starting
    /// at `char_idx`: the configured table when the preceding character is
    /// whitespace (`CharData::is_space`, prepared per sentence), `None`
    /// otherwise. This is mecab-ko's `rlength > length` test: a candidate
    /// at the very start of the sentence has no preceding character and is
    /// not penalized.
    ///
    /// # Arguments
    ///
    /// * `char_info` - The sentence's prepared per-character data.
    /// * `char_idx` - The candidate start position, in characters.
    ///
    /// # Returns
    ///
    /// The table to look candidates up in, or `None` for no penalty.
    #[inline]
    fn space_penalty_at(
        &self,
        char_info: &[CharData],
        char_idx: usize,
    ) -> Option<&'a SpacePenaltyTable> {
        let table = self.space_penalty?;
        (char_idx > 0 && char_info[char_idx - 1].is_space).then_some(table)
    }
}

/// Returns the space penalty for `word_entry`, or `0` when no table applies
/// at the current position.
///
/// # Arguments
///
/// * `space_penalty` - The table gated by position (see
///   [`LatticeOptions::space_penalty_at`]).
/// * `word_entry` - The candidate entry.
///
/// # Returns
///
/// The penalty cost.
#[inline]
fn space_penalty_cost(space_penalty: Option<&SpacePenaltyTable>, word_entry: &WordEntry) -> i32 {
    space_penalty.map_or(0, |table| table.cost(word_entry.word_id()))
}

/// Returns the left edge a relaxation keeps, or `None` when there is no
/// transition: no left edge, or a lowest cost saturated at `i32::MAX` (by a
/// Decompose penalty or a space penalty that large). The relaxations keep
/// the last of equal-cost left edges with `<=` (#1135), which also takes a
/// saturated cost; such a transition is still dropped, as when they kept
/// the first one with a strict `<` against the initial `i32::MAX`.
///
/// # Arguments
///
/// * `best_cost` - The lowest transition cost found, `i32::MAX` initially.
/// * `best_left` - The index of the left edge with that cost.
///
/// # Returns
///
/// The index of the left edge, or `None`.
#[inline(always)]
fn connected(best_cost: i32, best_left: Option<u32>) -> Option<u32> {
    best_left.filter(|_| best_cost < i32::MAX)
}

/// Flag bit on `CharData::categories_start` marking that the offset indexes
/// the `CharacterDefinition` flat category pool instead of the lattice's
/// `categories_buffer` (#942). Both index spaces stay far below this bit:
/// the pool offset is at most 24 bits by construction, and the fallback
/// buffer holds at most 255 categories per char of a 32 KiB sentence.
const CATEGORIES_IN_POOL: u32 = 1 << 31;

#[derive(Clone, Copy, Debug, Default)]
struct CharData {
    byte_offset: u32,
    is_kanji: bool,
    /// Whether this character is whitespace (see
    /// `LatticeOptions::skip_whitespace` and `LatticeOptions::space_penalty`);
    /// computed only when one of the two is enabled, its only consumers.
    /// Fits in the struct's existing padding, so `CharData` stays 16 bytes.
    is_space: bool,
    categories_start: u32,
    categories_len: u16,
    /// Length (in characters) of the all-kanji run starting here; computed
    /// only in Decompose mode (its sole consumer is the penalty).
    kanji_run_char_len: u32,
}

/// `CharData` is allocated one per character of every sentence, so its size
/// is a throughput concern rather than a detail: the `is_space` bit added for
/// the space penalty fits in the padding `is_kanji` already left behind.
/// Asserted rather than commented so a future field cannot silently push the
/// per-character buffer to 20 bytes.
const _: () = assert!(size_of::<CharData>() == 16);

#[inline]
pub fn is_kanji(c: char) -> bool {
    let c = c as u32;
    // CJK Unified Ideographs (4E00-9FAF) and Extension A (3400-4DBF)
    (0x4E00..=0x9FAF).contains(&c) || (0x3400..=0x4DBF).contains(&c)
}

impl Lattice {
    /// Packs a decoded `WordEntry` into a runtime edge starting at
    /// `start_char`.
    ///
    /// # 引数
    ///
    /// * `word_entry` - The decoded dictionary entry backing the edge.
    /// * `start_char` - The edge's start position, in characters.
    /// * `kanji_only` - Whether the edge surface is entirely kanji.
    ///
    /// # 戻り値
    ///
    /// An edge ready for `add_edge_in_lattice` (path cost unset).
    #[inline]
    fn create_edge(word_entry: WordEntry, start_char: usize, kanji_only: bool) -> Edge {
        let word_id = word_entry.word_id();
        Edge {
            word_id: word_id.id(),
            path_cost: i32::MAX,
            left_index: u32::MAX,
            left_id: word_entry.left_id,
            right_id: word_entry.right_id,
            word_cost: word_entry.word_cost,
            start_char: start_char as u16,
            flags: Edge::pack_flags(word_id.lex_type(), kanji_only),
            ws_tail: 0,
        }
    }

    pub fn clear(&mut self) {
        // Only slots up to the previous sentence's length can hold entries:
        // every `ends_at`/`all_paths` write in set_text/set_text_nbest
        // targets a char position <= that call's char count (BOS at 0,
        // edges at stop <= n_chars, EOS at n_chars), recorded in
        // `last_n_chars`, and every slot past it was left empty by the
        // previous clear(). Walking only this prefix keeps clear() O(previous
        // sentence) instead of O(historical max capacity), which matters
        // once one long sentence has grown the lattice (#877).
        let bound = self.last_n_chars + 1;
        for edge_vec in self.ends_at.iter_mut().take(bound) {
            edge_vec.clear();
        }
        debug_assert!(
            self.ends_at.iter().skip(bound).all(|v| v.is_empty()),
            "ends_at slot beyond last_n_chars must be empty"
        );
        for path_vec in self.all_paths.iter_mut().take(bound) {
            path_vec.clear();
        }
        debug_assert!(
            self.all_paths.iter().skip(bound).all(|v| v.is_empty()),
            "all_paths slot beyond last_n_chars must be empty"
        );
        self.char_info_buffer.clear();
        self.categories_buffer.clear();
        self.final_edge_count = 0;
        self.exit_penalties.clear();
    }

    /// Returns whether the `num_chars`-character span starting at
    /// `char_idx` consists solely of kanji (always `false` in Normal mode,
    /// whose preparation leaves the run lengths zero).
    ///
    /// # 引数
    ///
    /// * `char_idx` - Start position of the span, in characters.
    /// * `num_chars` - Span length, in characters.
    ///
    /// # 戻り値
    ///
    /// `true` when the whole span lies inside one kanji run.
    #[inline]
    fn is_kanji_all(&self, char_idx: usize, num_chars: usize) -> bool {
        self.char_info_buffer[char_idx].kanji_run_char_len >= num_chars as u32
    }

    /// Returns the `category_ord`-th category of the char at `char_idx`,
    /// reading the `CharacterDefinition` flat pool or the fallback
    /// `categories_buffer` as recorded by `prepare_char_buffers`.
    ///
    /// # 引数
    ///
    /// * `char_definitions` - The definitions whose pool backs the fast path.
    /// * `char_idx` - Character index in `char_info_buffer`.
    /// * `category_ord` - Ordinal within that char's category set.
    ///
    /// # 戻り値
    ///
    /// The category id.
    #[inline]
    fn get_cached_category(
        &self,
        char_definitions: &CharacterDefinition,
        char_idx: usize,
        category_ord: usize,
    ) -> CategoryId {
        let char_data = &self.char_info_buffer[char_idx];
        let idx = (char_data.categories_start & !CATEGORIES_IN_POOL) as usize + category_ord;
        if char_data.categories_start & CATEGORIES_IN_POOL != 0 {
            char_definitions.flat_category(idx)
        } else {
            self.categories_buffer[idx]
        }
    }

    /// Maps a byte offset in the current sentence to its char position.
    ///
    /// Valid only between `prepare_char_buffers` and the next `clear()`.
    /// Match boundaries on valid UTF-8 always land on char boundaries, so
    /// the exact lookup succeeds; the floor fallback keeps malformed input
    /// panic-free.
    ///
    /// # 引数
    ///
    /// * `byte` - Byte offset into the current sentence.
    ///
    /// # 戻り値
    ///
    /// The char position whose `byte_offset` equals (or, for a
    /// non-boundary byte, precedes) `byte`.
    #[inline]
    fn char_index_of_byte(&self, byte: usize) -> usize {
        self.char_info_buffer
            .binary_search_by_key(&(byte as u32), |cd| cd.byte_offset)
            .unwrap_or_else(|insert| insert.saturating_sub(1))
    }

    /// Returns the byte offset of the given char position in the current
    /// sentence (the sentinel at `n_chars` maps to the sentence's byte
    /// length). Valid only between `prepare_char_buffers` and the next
    /// `clear()`.
    ///
    /// # 引数
    ///
    /// * `char_idx` - Char position, `0..=n_chars`.
    ///
    /// # 戻り値
    ///
    /// The byte offset.
    #[inline]
    pub(crate) fn byte_offset_of(&self, char_idx: usize) -> usize {
        debug_assert!(
            char_idx < self.char_info_buffer.len(),
            "char position {char_idx} is outside the prepared sentence"
        );
        self.char_info_buffer[char_idx].byte_offset as usize
    }

    /// Records `n_chars` as the new sentence length and grows the edge
    /// slots to cover it.
    ///
    /// Called after `prepare_char_buffers` (which is what determines
    /// `n_chars`); `clear()` must already have run for the previous
    /// sentence, since it wipes the buffers `prepare` fills.
    ///
    /// # 引数
    ///
    /// * `n_chars` - The current sentence's character count.
    fn record_and_grow(&mut self, n_chars: usize) {
        self.last_n_chars = n_chars;
        self.max_chars_seen = self.max_chars_seen.max(n_chars);
        if self.capacity <= n_chars {
            self.capacity = n_chars;
            // Pre-size newly-grown slots (like Vibrato's reset_vec) to
            // avoid a couple of small reallocations the first time a busy
            // position accumulates several edges. `resize_with` is required
            // here: `resize` fills new slots with clones of its template
            // value, and cloning an empty Vec allocates capacity 0, so only
            // the moved-in last slot would actually be pre-sized (#827).
            self.ends_at
                .resize_with(n_chars + 1, || Vec::with_capacity(16));
        }
    }

    /// Pushes the sentence's BOS edges into `ends_at[0]`: the dictionary's
    /// single BOS edge (right context id 0, cost 0) when `bos` is empty,
    /// otherwise one edge per context, in reverse order, so that the last
    /// of equal-cost left edges, which the relaxations keep, is the first
    /// context (see [`LatticeOptions::bos`], #1135). Every BOS edge has no
    /// predecessor (`left_index == u32::MAX`), which is how the backtraces
    /// recognize it.
    ///
    /// Nothing else ends at position 0, and a carry-over of leading skipped
    /// whitespace moves the BOS edges into a slot no other edge reaches, so
    /// the BOS edges are always the first edges of the slot holding them and
    /// [`Lattice::bos_index`] maps the index of a BOS edge in that slot to the
    /// index of its context in `bos`.
    ///
    /// # Arguments
    ///
    /// * `bos` - The BOS contexts; fewer than `u16::MAX`.
    #[inline]
    fn push_bos_edges(&mut self, bos: &[BosContext]) {
        if bos.is_empty() {
            self.bos_len = 1;
            self.ends_at[0].push(Edge {
                path_cost: 0,
                left_index: u32::MAX,
                ..Default::default()
            });
            return;
        }
        // The documented limit of `LatticeOptions::bos`. The index width no
        // longer needs it: an edge refers to its left edge, a BOS edge
        // included, by a u32 index, u32::MAX meaning none (#1105). It stays
        // so that the contract does not change. A segmenter passes one
        // context per distinct right id among the previous sentence's final
        // edges, so a dictionary with fewer than u16::MAX right ids cannot
        // reach it.
        assert!(
            bos.len() < u16::MAX as usize,
            "set_text: {} BOS contexts, exceeding the u16::MAX-1 limit",
            bos.len()
        );
        self.bos_len = bos.len();
        self.ends_at[0].extend(bos.iter().rev().map(|context| Edge {
            path_cost: context.cost.clamp(0, PATH_COST_CLAMP),
            left_index: u32::MAX,
            right_id: context.right_id,
            ..Default::default()
        }));
    }

    /// [`Self::record_and_grow`] plus growth of the nbest `all_paths` slots.
    ///
    /// # 引数
    ///
    /// * `n_chars` - The current sentence's character count.
    fn record_and_grow_nbest(&mut self, n_chars: usize) {
        self.record_and_grow(n_chars);
        if self.nbest_capacity <= n_chars {
            self.nbest_capacity = n_chars;
            self.all_paths.resize(n_chars + 1, Vec::new());
        }
    }

    /// Clears the previous sentence and sizes the lattice for `n_chars`
    /// characters (test-facing shorthand for the clear + record + grow
    /// sequence `set_text` performs around `prepare_char_buffers`).
    ///
    /// # 引数
    ///
    /// * `n_chars` - The sentence's character count.
    #[cfg(test)]
    fn set_capacity(&mut self, n_chars: usize) {
        self.clear();
        self.record_and_grow(n_chars);
    }

    /// [`Self::set_capacity`] including the nbest `all_paths` slots.
    ///
    /// # 引数
    ///
    /// * `n_chars` - The sentence's character count.
    #[cfg(test)]
    fn set_capacity_nbest(&mut self, n_chars: usize) {
        self.clear();
        self.record_and_grow_nbest(n_chars);
    }

    /// Returns the lattice's current slot capacity: the largest sentence
    /// length (in characters) whose `ends_at` slots are already allocated.
    ///
    /// # 戻り値
    ///
    /// The capacity in characters. `0` for a fresh lattice.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns and resets the largest per-sentence character count seen
    /// since the last call (or the last `shrink_to`). Auto-shrink
    /// accounting for reusable workers.
    ///
    /// # 戻り値
    ///
    /// The observed maximum, `0` when no sentence was processed since.
    pub fn take_max_char_len(&mut self) -> usize {
        std::mem::take(&mut self.max_chars_seen)
    }

    /// Shrinks the internal buffers down to what a sentence of `n_chars`
    /// characters needs, releasing memory retained after processing a long
    /// sentence.
    ///
    /// The lattice grows monotonically (`set_text` never shrinks), so a
    /// single long sentence pins its worst-case allocation for the lifetime
    /// of the lattice. Long-lived holders (e.g. a reusable worker) can call
    /// this to bound retention. This is never called on the hot path:
    /// `clear()`/`set_text` stay shrink-free (#877/#884). A byte length is
    /// a valid (conservative) argument, since a sentence never has more
    /// characters than bytes.
    ///
    /// Invariants preserved:
    /// - Every remaining `ends_at` slot keeps a capacity of at least 16, the
    ///   pre-size that avoids first-growth reallocations (#827/#841).
    /// - `clear()` runs first, so all slots are empty and the
    ///   `last_n_chars` bound (#877) stays valid after truncation.
    ///
    /// # 引数
    ///
    /// * `n_chars` - Target sentence length in characters; buffers are
    ///   reduced to what a sentence of this length requires. Buffers
    ///   already at or below the target are left untouched.
    pub fn shrink_to(&mut self, n_chars: usize) {
        self.clear();
        let slots = n_chars + 1;
        if self.capacity > n_chars {
            self.ends_at.truncate(slots);
            self.ends_at.shrink_to(slots);
            for slot in &mut self.ends_at {
                // Keep the per-slot pre-size intact (#841); only release
                // growth beyond it.
                slot.shrink_to(16);
            }
            self.capacity = n_chars;
        }
        if self.nbest_capacity > n_chars {
            self.all_paths.truncate(slots);
            self.all_paths.shrink_to(slots);
            for paths in &mut self.all_paths {
                paths.shrink_to(0);
            }
            self.nbest_capacity = n_chars;
        }
        // All slots are empty after clear() + truncate, so lowering the
        // clear()/backtrace bound is safe (its debug_asserts hold trivially).
        self.last_n_chars = self.last_n_chars.min(n_chars);
        self.max_chars_seen = 0;
        // Scratch buffers are sized per sentence content, not per slot; the
        // bounds below are heuristics (roughly: categories per char, matches
        // per start position), not correctness requirements — set_text
        // regrows them on demand.
        self.char_info_buffer.shrink_to(slots);
        self.categories_buffer.shrink_to(4 * n_chars);
        self.matches_head.shrink_to(slots);
        self.matches_store.shrink_to(8 * slots);
        self.chars_buf.shrink_to(n_chars);
        self.codes_buf.shrink_to(n_chars);
        self.sys_matches.shrink_to(64);
        self.penalty_cache.shrink_to(64);
        self.transition_costs.shrink_to(64);
        self.exit_penalties.shrink_to(64);
    }

    /// Fills the per-sentence character buffers (`char_info_buffer`,
    /// `categories_buffer`, `chars_buf`) from `text`, appends the
    /// end-of-text sentinel, resets the grouping runs found so far
    /// (`group_run_ends`), and precomputes the kanji run lengths consumed by
    /// the Decompose-mode penalty.
    ///
    /// Shared by `set_text` and `set_text_nbest` so the two hot paths cannot
    /// drift apart.
    ///
    /// # Arguments
    ///
    /// * `dict` - The system prefix dictionary whose code table maps each
    ///   character into `codes_buf`.
    /// * `char_definitions` - Character category definitions.
    /// * `text` - The sentence to prepare buffers for.
    /// * `options` - The lattice options. The kanji-run lengths are only
    ///   computed in Decompose mode, their sole consumer (`Penalty::penalty`
    ///   via `kanji_only`) — in Normal mode they stay zero and every edge
    ///   gets `kanji_only = false`, which the Normal relaxation arms never
    ///   read (#942). Likewise `CharData::is_space` stays `false` unless
    ///   whitespace skipping or the space penalty is enabled, in which case
    ///   each character is classified by the skipping classifier
    ///   ([`WhitespaceClassifier::is_space`]) or else by
    ///   [`SpacePenaltyTable::is_space`].
    fn prepare_char_buffers(
        &mut self,
        dict: &PrefixDictionary,
        char_definitions: &CharacterDefinition,
        text: &str,
        options: &LatticeOptions,
    ) {
        let len = text.len();
        let search_mode = options.mode;
        let needs_kanji_runs = search_mode.is_search();
        // Whitespace classification, hoisted: the skipping classifier, or
        // else the penalty table's. Below U+0100 it is one indexed load; above,
        // it is `false` outright when the category has no member there (every
        // bundled dictionary), and otherwise scans the categories this loop
        // fetches for the character anyway rather than looking them up a
        // second time. A penalty table for a
        // dictionary without a `SPACE` category falls back to
        // `char::is_whitespace`.
        let whitespace = options.skip_whitespace.or_else(|| {
            options
                .space_penalty
                .and_then(SpacePenaltyTable::whitespace)
        });
        let unicode_whitespace = whitespace.is_none() && options.space_penalty.is_some();
        self.char_info_buffer.clear();
        self.categories_buffer.clear();
        self.chars_buf.clear();
        self.codes_buf.clear();
        self.group_run_ends.clear();

        for (byte_offset, c) in text.char_indices() {
            // Category lookup is O(1) for BMP codepoints via the flat table
            // built at dictionary load (#878). On that fast path, store the
            // pool coordinates directly instead of copying the categories
            // into categories_buffer (#942); non-BMP codepoints (and the
            // never-in-practice case of an unbuilt flat table) keep the
            // copy, discriminated by the CATEGORIES_IN_POOL flag.
            let (categories_start, categories_len) =
                match char_definitions.lookup_categories_packed(c) {
                    Some((offset, len)) => (offset | CATEGORIES_IN_POOL, len),
                    None => {
                        let start = self.categories_buffer.len() as u32;
                        for &category in char_definitions.lookup_categories(c) {
                            self.categories_buffer.push(category);
                        }
                        (start, (self.categories_buffer.len() as u32 - start) as u16)
                    }
                };

            let is_space = match whitespace {
                Some(whitespace) if (c as u32) < 256 => whitespace.is_space_below_256(c as u32),
                Some(whitespace) if !whitespace.has_members_above_latin1() => false,
                Some(whitespace) => {
                    let space = whitespace.category();
                    if categories_start & CATEGORIES_IN_POOL != 0 {
                        let offset = (categories_start & !CATEGORIES_IN_POOL) as usize;
                        (offset..offset + categories_len as usize)
                            .any(|idx| char_definitions.flat_category(idx) == space)
                    } else {
                        let start = categories_start as usize;
                        self.categories_buffer[start..start + categories_len as usize]
                            .contains(&space)
                    }
                }
                None => unicode_whitespace && c.is_whitespace(),
            };

            self.char_info_buffer.push(CharData {
                byte_offset: byte_offset as u32,
                is_kanji: needs_kanji_runs && is_kanji(c),
                is_space,
                categories_start,
                categories_len,
                kanji_run_char_len: 0,
            });
            self.chars_buf.push(c);
            self.codes_buf.push(dict.map_code_or_invalid(c));
        }
        // Sentinel for end of text
        self.char_info_buffer.push(CharData {
            byte_offset: len as u32,
            is_kanji: false,
            is_space: false,
            categories_start: 0,
            categories_len: 0,
            kanji_run_char_len: 0,
        });

        // Pre-calculate Kanji run lengths (backwards); skipped in Normal
        // mode, where no consumer reads them (see `needs_kanji_runs`).
        if needs_kanji_runs {
            let n_chars = self.char_info_buffer.len() - 1;
            for i in (0..n_chars).rev() {
                if self.char_info_buffer[i].is_kanji {
                    self.char_info_buffer[i].kanji_run_char_len =
                        1 + self.char_info_buffer[i + 1].kanji_run_char_len;
                } else {
                    self.char_info_buffer[i].kanji_run_char_len = 0;
                }
            }
        }
    }

    /// Returns the grouping run length of `category` at ordinal
    /// `category_ord` from `char_idx` on: how many characters carry that
    /// category at that ordinal. Reads the run found by an earlier position
    /// when `char_idx` lies inside it (see `group_run_ends`), and scans
    /// forward otherwise. Must be called with non-decreasing `char_idx`
    /// within a sentence.
    ///
    /// # Arguments
    ///
    /// * `char_definitions` - Character category definitions.
    /// * `category` - The category of the char at `char_idx` at that ordinal.
    /// * `category_ord` - Ordinal within the char's category set.
    /// * `char_idx` - Start position, in characters.
    ///
    /// # Returns
    ///
    /// The run length, at least 1.
    #[inline]
    fn group_run_len(
        &mut self,
        char_definitions: &CharacterDefinition,
        category: CategoryId,
        category_ord: usize,
        char_idx: usize,
    ) -> usize {
        if let Some(&(cat, end)) = self.group_run_ends.get(category_ord)
            && cat == category
            && char_idx < end as usize
        {
            return end as usize - char_idx;
        }
        let n_chars = self.char_info_buffer.len() - 1;
        let mut end = char_idx + 1;
        while end < n_chars
            && category_ord < self.char_info_buffer[end].categories_len as usize
            && self.get_cached_category(char_definitions, end, category_ord) == category
        {
            end += 1;
        }
        if self.group_run_ends.len() <= category_ord {
            self.group_run_ends
                .resize(category_ord + 1, (CategoryId(usize::MAX), 0));
        }
        self.group_run_ends[category_ord] = (category, end as u32);
        end - char_idx
    }

    /// Forward Viterbi: constructs the lattice and calculates the path costs
    /// simultaneously (avoiding a separate traversal pass), with the
    /// positional option set. Equivalent to
    /// [`Lattice::set_text_with_options`] without a space penalty.
    ///
    /// # Arguments
    ///
    /// See [`Lattice::set_text_with_options`]; `search_mode`,
    /// `max_grouping_len` and `unknown_word_ladder` map onto the
    /// [`LatticeOptions`] fields of the same name.
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
        max_grouping_len: Option<usize>,
        unknown_word_ladder: bool,
    ) {
        let mut options = LatticeOptions::new(search_mode);
        options.max_grouping_len = max_grouping_len;
        options.unknown_word_ladder = unknown_word_ladder;
        self.set_text_with_options(
            dict,
            user_dict,
            char_definitions,
            unknown_dictionary,
            cost_matrix,
            text,
            &options,
        );
    }

    /// Forward Viterbi implementation: constructs the lattice and calculates
    /// the path costs simultaneously. This improves performance by avoiding a
    /// separate lattice traversal pass.
    ///
    /// The candidates are added in the order MeCab processes them: position
    /// by position, and at each position the unknown words, then the system
    /// dictionary's words, then the user dictionary's, each group in reverse
    /// (the entries of one surface last CSV row first). A slot keeps its
    /// edges in the order they were added, and every relaxation keeps the
    /// last of the left edges that tie on cost, as MeCab keeps the first
    /// node of its end-node list, which holds them in reverse (#1135). So of
    /// two paths of equal cost that end at the same position, the one whose
    /// last word starts later wins, as in MeCab. Among entries that share the
    /// surface, the context ids and the cost, the first CSV row wins, as in
    /// MeCab, and a user entry wins a tie with a system entry (MeCab looks
    /// the system dictionary up first and would pick the system entry)
    /// (#1131).
    ///
    /// # Arguments
    ///
    /// * `dict` - The system prefix dictionary.
    /// * `user_dict` - The user prefix dictionary, if any.
    /// * `char_definitions` - Character category definitions.
    /// * `unknown_dictionary` - Unknown-word entries.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `text` - One sentence (fewer than `u16::MAX` characters).
    /// * `options` - Mode, unknown-word knobs, the optional space penalty and
    ///   whitespace skipping, and the BOS edges (see [`LatticeOptions`]).
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_text_with_options(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&UserPrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        text: &str,
        options: &LatticeOptions,
    ) {
        let search_mode = options.mode;
        let max_grouping_len = options.max_grouping_len;
        let unknown_word_ladder = options.unknown_word_ladder;
        // Clear the previous sentence's slots (bounded by its char count),
        // then build the per-char buffers to learn this sentence's length.
        self.clear();
        self.prepare_char_buffers(dict, char_definitions, text, options);
        let n_chars = self.chars_buf.len();
        // Edge stores its start position as u16 (#943). Every
        // Segmenter-mediated path satisfies this via MAX_SENTENCE_BYTES
        // splitting; direct callers must split their input themselves.
        assert!(
            n_chars < u16::MAX as usize,
            "set_text: sentence has {n_chars} characters, exceeding the u16::MAX-1 limit; \
             split the input into sentences (the Segmenter does this automatically)"
        );
        self.record_and_grow(n_chars);
        self.penalty_cache_pos = usize::MAX;
        self.skip_whitespace = options.skip_whitespace.is_some();

        self.push_bos_edges(options.bos);

        // Pre-scan text with Aho-Corasick to report all matches
        // Optimization: Use flat vectors instead of Vec<Vec<_>> to avoid many small allocations.
        // Linked list structure: matches_head[start_char] -> index in matches_store
        // Buffers are Lattice fields reused across calls; refill matches_head (its
        // contents are meaningful, unlike ends_at's empty-Vec slots) and clear
        // matches_store (a plain append-only pool).
        // The pool now holds only user-dictionary matches: the system
        // dictionary is searched per lattice-reachable position instead of
        // pre-scanned (#882). daachorse's API shape still forces a whole-text
        // scan for the user automaton, so its matches keep the linked list;
        // with no user dictionary the head table stays empty and the drain
        // below is skipped by its `char_idx < matches_head.len()` guard.
        self.matches_head.clear();
        self.matches_store.clear();

        // User dictionary scan (byte offsets from daachorse, converted to
        // char positions -- matches on valid UTF-8 always land on char
        // boundaries).
        if let Some(ud) = user_dict {
            self.matches_head.resize(n_chars + 1, u32::MAX);
            let ud_vals: &[u8] = &ud.vals_data;
            for m in ud.da.find_overlapping_iter(text) {
                let start_char = self.char_index_of_byte(m.start());
                let (offset, count) = ud.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

                if start_char < self.matches_head.len() {
                    let avail = ud_vals.len().saturating_sub(offset_bytes);
                    let n = (count as usize).min(avail / WordEntry::SERIALIZED_LEN);
                    let block =
                        &ud_vals[offset_bytes..offset_bytes + n * WordEntry::SERIALIZED_LEN];
                    let end_char = self.char_index_of_byte(m.end()) as u32;
                    let (chunks, _) = block.as_chunks::<{ WordEntry::SERIALIZED_LEN }>();
                    // In CSV order: the list is drained from its head, so
                    // the surface's entries come out last row first, in
                    // processing order (see the method docs).
                    for chunk in chunks {
                        let entry = WordEntry::deserialize(chunk, false);
                        let next = self.matches_head[start_char];
                        self.matches_head[start_char] = self.matches_store.len() as u32;
                        self.matches_store.push((end_char, entry, next));
                    }
                }
            }
        }

        for char_idx in 0..n_chars {
            if self.skip_whitespace {
                // No node starts on whitespace (MeCab skips it); the edges
                // ending before the run that ends here are carried over, so
                // candidates starting here connect to them directly.
                if self.char_info_buffer[char_idx].is_space {
                    continue;
                }
                // Checked here so unspaced text never pays for the call.
                if char_idx > 0 && self.char_info_buffer[char_idx - 1].is_space {
                    self.carry_over_whitespace(char_idx, false);
                }
            }

            // No arc is ending here.
            // No need to check if a valid word starts here.
            if self.ends_at[char_idx].is_empty() {
                continue;
            }

            // Space penalty gate for every candidate starting here: one
            // O(1) check per reachable position, `None` when disabled.
            let space_penalty = options.space_penalty_at(&self.char_info_buffer, char_idx);

            // The system dictionary: per-position common-prefix search over
            // the in-place trie, run only at lattice-reachable positions (the
            // gate above). Searched before anything is added, so that
            // `found` is known to the unknown words, which come first.
            self.sys_matches.clear();
            {
                let suffix = &self.codes_buf[char_idx..];
                for (entries, end_char_offset) in dict.common_prefix_search_codes(suffix) {
                    let end_char = (char_idx + end_char_offset) as u32;
                    for chunk in entries.as_chunks::<{ WordEntry::SERIALIZED_LEN }>().0 {
                        self.sys_matches
                            .push((end_char, WordEntry::deserialize(chunk, true)));
                    }
                }
            }
            // Whether a dictionary word starts here.
            let found = (char_idx < self.matches_head.len()
                && self.matches_head[char_idx] != u32::MAX)
                || !self.sys_matches.is_empty();

            // The candidates in processing order (see the method docs). First
            // the unknown words, at every reachable position, in every mode,
            // as in MeCab (#1105): one may start inside a run that an earlier
            // position grouped, e.g. after a dictionary word ending inside it.
            let num_categories = self.char_info_buffer[char_idx].categories_len as usize;
            for category_ord in (0..num_categories).rev() {
                let category = self.get_cached_category(char_definitions, char_idx, category_ord);
                self.process_unknown_word(
                    char_definitions,
                    unknown_dictionary,
                    cost_matrix,
                    search_mode,
                    max_grouping_len,
                    unknown_word_ladder,
                    space_penalty,
                    category,
                    category_ord,
                    char_idx,
                    found,
                );
            }

            // Then the system dictionary's words, last found first, so a
            // surface's first CSV row wins a tie (see `sys_matches`).
            for i in (0..self.sys_matches.len()).rev() {
                let (end_char, word_entry) = self.sys_matches[i];
                let end_char = end_char as usize;
                let kanji_only = self.is_kanji_all(char_idx, end_char - char_idx);
                let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                let extra_cost = space_penalty_cost(space_penalty, &word_entry);
                self.add_edge_in_lattice(edge, end_char, cost_matrix, search_mode, extra_cost);
            }

            // Then the user dictionary's, last, so a user entry wins a tie
            // with a system entry; each surface's entries last row first
            // (see `matches_head`).
            if char_idx < self.matches_head.len() {
                let mut match_idx = self.matches_head[char_idx];
                while match_idx != u32::MAX {
                    let (end_char, word_entry, next) = self.matches_store[match_idx as usize];

                    let end_char = end_char as usize;
                    let kanji_only = self.is_kanji_all(char_idx, end_char - char_idx);
                    let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                    let extra_cost = space_penalty_cost(space_penalty, &word_entry);
                    self.add_edge_in_lattice(edge, end_char, cost_matrix, search_mode, extra_cost);

                    match_idx = next;
                }
            }
        }

        // Connect EOS, to the words before trailing whitespace when it is
        // skipped (MeCab connects EOS to the last position a word ends at).
        if self.skip_whitespace && n_chars > 0 && self.char_info_buffer[n_chars - 1].is_space {
            self.carry_over_whitespace(n_chars, false);
        }
        if !self.ends_at[n_chars].is_empty() {
            let mut eos_edge = Edge {
                start_char: n_chars as u16,
                ..Default::default()
            };
            let content_end = self.content_end(n_chars);
            // Calculate cost for EOS with the row hoisted (#880).
            let left_edges = &self.ends_at[n_chars];
            // The edges before EOS are the sentence's exits (`exits_into`).
            self.final_edge_count = left_edges.len();
            let mut best_cost = i32::MAX;
            let mut best_left = None;
            let cost_row = cost_matrix.row(0); // EOS default left_id

            for (i, left_edge) in left_edges.iter().enumerate() {
                let path_cost = left_edge.path_cost + cost_row[left_edge.right_id as usize] as i32;
                let path_cost = match search_mode {
                    Mode::Normal => path_cost,
                    Mode::Decompose(penalty) => {
                        let exit_penalty =
                            penalty.penalty(left_edge, left_edge.char_len(content_end));
                        self.exit_penalties.push(exit_penalty);
                        path_cost.saturating_add(exit_penalty)
                    }
                };
                // The last of equal-cost edges wins (see the method docs).
                if path_cost <= best_cost {
                    best_cost = path_cost;
                    best_left = Some(i as u32);
                }
            }
            if let Some(left_idx) = connected(best_cost, best_left) {
                eos_edge.left_index = left_idx;
                eos_edge.path_cost = best_cost;
                self.ends_at[n_chars].push(eos_edge);
            }
        }
    }

    /// Returns the start of the whitespace run that ends right before
    /// `char_idx`, or `char_idx` itself when the preceding character is not
    /// whitespace (or whitespace is not being classified).
    ///
    /// # Arguments
    ///
    /// * `char_idx` - A slot index, `0..=n_chars`.
    ///
    /// # Returns
    ///
    /// The run's start, in characters.
    #[inline]
    fn whitespace_run_start(&self, char_idx: usize) -> usize {
        let mut start = char_idx;
        while start > 0 && self.char_info_buffer[start - 1].is_space {
            start -= 1;
        }
        start
    }

    /// Returns the start of the whitespace run before `char_idx` when
    /// whitespace is skipped, `char_idx` otherwise: where the edges in
    /// `ends_at[char_idx]` end, before the whitespace their own surfaces end
    /// with (`Edge::ws_tail`) is added. Used for the Decompose length
    /// penalty and for the end offsets of the backtraces.
    ///
    /// # Arguments
    ///
    /// * `char_idx` - A slot index, `0..=n_chars`.
    ///
    /// # Returns
    ///
    /// The end position, in characters.
    #[inline]
    fn content_end(&self, char_idx: usize) -> usize {
        if self.skip_whitespace {
            self.whitespace_run_start(char_idx)
        } else {
            char_idx
        }
    }

    /// Returns where `edge`, held in `ends_at[slot]`, ends: the slot itself,
    /// or, when whitespace is skipped, the start of the whitespace run
    /// before the slot plus the whitespace the edge's own surface ends with.
    ///
    /// # Arguments
    ///
    /// * `edge` - An edge of `ends_at[slot]`.
    /// * `slot` - The slot holding `edge`.
    ///
    /// # Returns
    ///
    /// The end position, in bytes within the sentence.
    #[inline]
    pub(crate) fn edge_end_byte(&self, edge: &Edge, slot: usize) -> usize {
        self.byte_offset_of(edge.end_char(self.content_end(slot)))
    }

    /// Carries the edges ending before or inside the whitespace run that
    /// ends at `char_idx` over to `ends_at[char_idx]` (see
    /// [`LatticeOptions::skip_whitespace`]), so candidates starting at
    /// `char_idx` connect to them directly. MeCab does the same by looking
    /// words up past the whitespace from every position a word ends at.
    ///
    /// Carried edges keep their start positions and predecessor links, so
    /// backtracking is unchanged. Nothing starts inside the run, so no edge
    /// refers to an emptied slot.
    ///
    /// The slot ends up in the order the edges end in: the edges carried
    /// from slot `run_start`, then from each later slot of the run, then the
    /// edges already in `ends_at[char_idx]`, which end after the run. The
    /// relaxations keep the last of equal-cost left edges, so the left word
    /// that ends later wins a tie, as in MeCab, which looks the next word up
    /// from every position a word ends at and keeps the node looked up from
    /// the later one (#1135). No edge refers to `ends_at[char_idx]` yet, as
    /// nothing has started there, so the edges can be reordered.
    ///
    /// In the N-best lattice the transitions of the edges move with them,
    /// their edge indexes renumbered; the slot's transitions stay sorted by
    /// edge index, which `nbest.rs` relies on.
    ///
    /// Every edge that ends in the run or right after it gets in
    /// `Edge::ws_tail` the part of the run its own surface covers, so its
    /// end stays known once it sits in `ends_at[char_idx]` (#1108): an edge
    /// moved from slot `k` covers `k - run_start` characters, and an edge
    /// already in `ends_at[char_idx]` covers the whole run, since no edge
    /// starts inside it.
    ///
    /// Callers check that the character before `char_idx` is whitespace, so
    /// the common unspaced position never makes the call.
    ///
    /// # Arguments
    ///
    /// * `char_idx` - The first position after the run (or `n_chars`).
    /// * `nbest` - Whether to move the N-best transitions as well.
    #[inline(never)]
    fn carry_over_whitespace(&mut self, char_idx: usize, nbest: bool) {
        let run_start = self.whitespace_run_start(char_idx);
        // Drained rather than taken so every slot keeps its allocation for
        // the next sentence.
        let (before, after) = self.ends_at.split_at_mut(char_idx);
        let target = &mut after[0];
        let whole_run = u8::try_from(char_idx - run_start).unwrap_or(u8::MAX);
        for edge in target.iter_mut() {
            debug_assert!(
                (edge.start_char as usize) < run_start,
                "an edge starts inside a skipped whitespace run"
            );
            edge.ws_tail = whole_run;
        }
        // The slot's own edges, and their transitions, which go last.
        let own = target.len();
        let own_paths = if nbest {
            self.all_paths[char_idx].len()
        } else {
            0
        };
        // The edges at the run start end right before it and keep 0; skip
        // them, as every word before a space is there (#1108).
        debug_assert!(
            before[run_start].iter().all(|edge| edge.ws_tail == 0),
            "an edge is carried over twice"
        );
        for slot in run_start..char_idx {
            if before[slot].is_empty() {
                continue;
            }
            if slot > run_start {
                let covered = u8::try_from(slot - run_start).unwrap_or(u8::MAX);
                for edge in before[slot].iter_mut() {
                    debug_assert_eq!(edge.ws_tail, 0, "an edge is carried over twice");
                    edge.ws_tail = covered;
                }
            }
            if nbest {
                let offset = target.len() as u32;
                let (paths_before, paths_after) = self.all_paths.split_at_mut(char_idx);
                paths_after[0].extend(paths_before[slot].drain(..).map(|mut path| {
                    path.edge_index += offset;
                    path
                }));
            }
            target.append(&mut before[slot]);
        }
        // Move the slot's own edges, which end after the run, behind the
        // carried ones; usually there are none, as an entry seldom ends with
        // whitespace (#1108).
        let carried = target.len() - own;
        if own > 0 && carried > 0 {
            target.rotate_left(own);
            if nbest {
                let paths = &mut self.all_paths[char_idx];
                for path in &mut paths[..own_paths] {
                    path.edge_index += carried as u32;
                }
                for path in &mut paths[own_paths..] {
                    path.edge_index -= own as u32;
                }
                paths.rotate_left(own_paths);
            }
        }
    }

    /// Emits the unknown-word edges of `category` at `char_idx`: for every
    /// dictionary entry of the category, last entry first (processing order,
    /// see [`Self::set_text_with_options`]), one edge per length in
    /// `lengths`.
    ///
    /// One relaxation per entry serves all its lengths: it depends only on
    /// the start and the entry's left context id (see [`Self::relax`]). Each
    /// length ends in its own slot, so every slot still receives the
    /// entries in the same order, as when each length was emitted on its
    /// own.
    ///
    /// # Arguments
    ///
    /// * `unknown_dictionary` - Source of unknown-word entries.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `search_mode` - The segmentation mode.
    /// * `space_penalty` - The space-penalty table when the position is
    ///   preceded by whitespace (see [`LatticeOptions::space_penalty_at`]).
    /// * `category` - The character category the edges belong to.
    /// * `char_idx` - Start position, in characters.
    /// * `lengths` - The span lengths, in characters, in emission order (see
    ///   [`Self::candidate_lengths`]).
    #[allow(clippy::too_many_arguments)]
    fn emit_unknown_word_edges(
        &mut self,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        space_penalty: Option<&SpacePenaltyTable>,
        category: CategoryId,
        char_idx: usize,
        lengths: impl Iterator<Item = usize> + Clone,
    ) {
        for &word_id in unknown_dictionary.lookup_word_ids(category).iter().rev() {
            let word_entry = unknown_dictionary.word_entry(word_id);
            // No transition: none of this entry's edges is stored, as
            // `add_edge_in_lattice` would store none. Another entry, with
            // another left id, may still connect.
            let Some(best) = self.relax(
                char_idx,
                word_entry.left_id as u32,
                cost_matrix,
                search_mode,
            ) else {
                continue;
            };
            let extra_cost = space_penalty_cost(space_penalty, &word_entry);
            for num_chars in lengths.clone() {
                let kanji_only = self.is_kanji_all(char_idx, num_chars);
                let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                self.push_relaxed(edge, char_idx + num_chars, best, extra_cost);
            }
        }
    }

    /// [`Self::emit_unknown_word_edges`] for the nbest lattice: one
    /// relaxation per entry ([`Self::relax_nbest`]) serves all its lengths,
    /// and [`Self::push_relaxed_nbest`] records its transitions for each
    /// length's edge.
    ///
    /// # Arguments
    ///
    /// See [`Self::emit_unknown_word_edges`].
    #[allow(clippy::too_many_arguments)]
    fn emit_unknown_word_edges_nbest(
        &mut self,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        space_penalty: Option<&SpacePenaltyTable>,
        category: CategoryId,
        char_idx: usize,
        lengths: impl Iterator<Item = usize> + Clone,
    ) {
        for &word_id in unknown_dictionary.lookup_word_ids(category).iter().rev() {
            let word_entry = unknown_dictionary.word_entry(word_id);
            let extra_cost = space_penalty_cost(space_penalty, &word_entry);
            let best = self.relax_nbest(
                char_idx,
                word_entry.left_id as u32,
                cost_matrix,
                search_mode,
                extra_cost,
            );
            for num_chars in lengths.clone() {
                let kanji_only = self.is_kanji_all(char_idx, num_chars);
                let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                self.push_relaxed_nbest(edge, char_idx + num_chars, best);
            }
        }
    }

    /// Returns the lengths of the unknown-word candidates of one category
    /// in emission order: the primary length, then the length ladder
    /// `1..=ladder` without the primary one (see
    /// [`Self::unknown_word_lengths`]).
    ///
    /// # Arguments
    ///
    /// * `primary` - The primary candidate's length, in characters.
    /// * `ladder` - The longest ladder length, in characters (0 = none).
    ///
    /// # Returns
    ///
    /// The lengths, each once.
    #[inline]
    fn candidate_lengths(primary: usize, ladder: usize) -> impl Iterator<Item = usize> + Clone {
        std::iter::once(primary).chain((1..=ladder).filter(move |&len| len != primary))
    }

    /// Returns the unknown-word candidate lengths for one category at
    /// `char_idx`, or `None` when the category creates no candidate there
    /// (it has `invoke` off and a dictionary word starts here).
    ///
    /// The primary candidate covers the grouping run (`group` set) or one
    /// character; `max_grouping_len` turns a run with more than that many
    /// characters beyond the first back into one character (#944). The
    /// length ladder (#945) adds the lengths `1..=ladder` (skipping the
    /// primary one), where `ladder` is the run length capped at the
    /// category's `char.def` `LENGTH`, 0 when the ladder is off. Both read
    /// the run from `group_run_len`, which scans each run once, so the calls
    /// stay O(n) per category ordinal over a sentence (#944).
    ///
    /// # Arguments
    ///
    /// * `char_definitions` - Character category definitions.
    /// * `max_grouping_len` - Unknown-word grouping cap (`None` = unbounded).
    /// * `unknown_word_ladder` - Whether to emit the length ladder.
    /// * `category` - The category of the candidates.
    /// * `category_ord` - Ordinal of `category` within the char's category
    ///   set.
    /// * `char_idx` - Start position, in characters.
    /// * `found` - Whether a dictionary word starts at `char_idx`.
    ///
    /// # Returns
    ///
    /// `Some((primary, ladder))` in characters, or `None`.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn unknown_word_lengths(
        &mut self,
        char_definitions: &CharacterDefinition,
        max_grouping_len: Option<usize>,
        unknown_word_ladder: bool,
        category: CategoryId,
        category_ord: usize,
        char_idx: usize,
        found: bool,
    ) -> Option<(usize, usize)> {
        let category_data = char_definitions.lookup_definition(category);
        if !category_data.invoke && found {
            return None;
        }
        let needs_run = category_data.group || (unknown_word_ladder && category_data.length > 0);
        let run_len = if needs_run {
            self.group_run_len(char_definitions, category, category_ord, char_idx)
        } else {
            1
        };
        let mut primary = 1;
        if category_data.group {
            primary = run_len;
            // MeCab-style cap (#944): a grouped candidate with more than
            // max_grouping_len characters beyond the first is not emitted
            // at this position; the single-char unknown word (plus the
            // ladder and any dictionary words) stands in. Later positions
            // group the remaining tail once it fits the cap. None (default)
            // keeps the grouping unbounded.
            if let Some(cap) = max_grouping_len
                && primary > 1
                && primary - 1 > cap
            {
                primary = 1;
            }
        }
        let ladder = if unknown_word_ladder {
            run_len.min(category_data.length as usize)
        } else {
            0
        };
        Some((primary, ladder))
    }

    /// Adds the unknown-word candidates of one category at `char_idx` to
    /// the 1-best lattice: the primary candidate and the length ladder of
    /// [`Self::unknown_word_lengths`].
    ///
    /// # Arguments
    ///
    /// * `char_definitions` - Character category definitions.
    /// * `unknown_dictionary` - Source of unknown-word entries.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `search_mode` - The segmentation mode.
    /// * `max_grouping_len` - Unknown-word grouping cap (`None` = unbounded).
    /// * `unknown_word_ladder` - Whether to emit the length ladder.
    /// * `space_penalty` - The space-penalty table when the position is
    ///   preceded by whitespace.
    /// * `category` - The category of the candidates.
    /// * `category_ord` - Ordinal of `category` within the char's category
    ///   set.
    /// * `char_idx` - Start position, in characters.
    /// * `found` - Whether a dictionary word starts at `char_idx`.
    #[allow(clippy::too_many_arguments)]
    fn process_unknown_word(
        &mut self,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        max_grouping_len: Option<usize>,
        unknown_word_ladder: bool,
        space_penalty: Option<&SpacePenaltyTable>,
        category: CategoryId,
        category_ord: usize,
        char_idx: usize,
        found: bool,
    ) {
        let Some((primary, ladder)) = self.unknown_word_lengths(
            char_definitions,
            max_grouping_len,
            unknown_word_ladder,
            category,
            category_ord,
            char_idx,
            found,
        ) else {
            return;
        };
        self.emit_unknown_word_edges(
            unknown_dictionary,
            cost_matrix,
            search_mode,
            space_penalty,
            category,
            char_idx,
            Self::candidate_lengths(primary, ladder),
        );
    }

    /// Decompose-mode relaxation body of `add_edge_in_lattice`, outlined
    /// (`inline(never)`) so the Normal-mode hot arm keeps its compact code
    /// layout (#944 measured a ~2% Normal regression from the enlarged
    /// in-function Decompose arm alone).
    ///
    /// # 引数
    ///
    /// * `start_char` - The incoming edge's start position.
    /// * `right_left_id` - The incoming edge's left context id.
    /// * `penalty` - The Decompose penalty configuration.
    /// * `cost_matrix` - The connection cost matrix.
    ///
    /// # 戻り値
    ///
    /// The best `(cost, left index)` over the position's left edges, the
    /// last of equal-cost ones (see [`Self::relax`]).
    #[inline(never)]
    fn relax_decompose(
        &mut self,
        start_char: usize,
        right_left_id: u32,
        penalty: &Penalty,
        cost_matrix: &ConnectionCostMatrix,
    ) -> (i32, Option<u32>) {
        // Penalties depend only on the left edges, and
        // ends_at[start_char] is immutable while the main loop processes
        // this start position (every insertion targets a later slot), so
        // compute them once per position instead of once per (left edge,
        // incoming edge) pair (#944).
        let mut cache = std::mem::take(&mut self.penalty_cache);
        if self.penalty_cache_pos != start_char {
            cache.clear();
            let content_end = self.content_end(start_char);
            cache.extend(self.ends_at[start_char].iter().map(|left_edge| {
                // The left edge's exact char span (#943 fixed the former
                // byte-length/3 approximation): it ends where this edge
                // starts, or, when it was carried over skipped whitespace,
                // where the run starts plus the whitespace its own surface
                // ends with (#1108).
                penalty.penalty(left_edge, left_edge.char_len(content_end))
            }));
            self.penalty_cache_pos = start_char;
        }
        // Matrix row hoisted like the Normal arm (#880/#944); path +
        // connection stays a plain add (bounded by PATH_COST_CLAMP + i16),
        // while the penalty addition keeps saturating_add because the
        // penalty is unbounded.
        let mut best_cost = i32::MAX;
        let mut best_left = None;
        let left_edges = &self.ends_at[start_char];
        let cost_row = cost_matrix.row(right_left_id);
        for (i, left_edge) in left_edges.iter().enumerate() {
            let conn_cost = cost_row[left_edge.right_id as usize] as i32;
            let total_cost = (left_edge.path_cost + conn_cost).saturating_add(cache[i]);

            if total_cost <= best_cost {
                best_cost = total_cost;
                best_left = Some(i as u32);
            }
        }
        self.penalty_cache = cache;
        (best_cost, best_left)
    }

    /// [`Self::relax_decompose`] for the nbest lattice: additionally
    /// leaves the cost of every transition in `costs`.
    ///
    /// # Arguments
    ///
    /// * `start_char` - The incoming edge's start position.
    /// * `right_left_id` - The incoming edge's left context id.
    /// * `penalty` - The Decompose penalty configuration.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `extra_cost` - Position-dependent cost of the incoming edge (the
    ///   space penalty), folded into every transition so the nbest A*
    ///   re-derives the same totals as the forward pass.
    /// * `costs` - Filled with one transition cost per left edge, in their
    ///   order; expected empty.
    ///
    /// # Returns
    ///
    /// The best `(cost, left index)` over the position's left edges,
    /// including `extra_cost`, the last of equal-cost ones (see
    /// [`Self::relax`]).
    #[inline(never)]
    fn relax_decompose_nbest(
        &mut self,
        start_char: usize,
        right_left_id: u32,
        penalty: &Penalty,
        cost_matrix: &ConnectionCostMatrix,
        extra_cost: i32,
        costs: &mut Vec<i32>,
    ) -> (i32, Option<u32>) {
        // Same per-position penalty cache and hoisted row as
        // relax_decompose (#944).
        let mut cache = std::mem::take(&mut self.penalty_cache);
        if self.penalty_cache_pos != start_char {
            cache.clear();
            let content_end = self.content_end(start_char);
            cache.extend(
                self.ends_at[start_char]
                    .iter()
                    .map(|left_edge| penalty.penalty(left_edge, left_edge.char_len(content_end))),
            );
            self.penalty_cache_pos = start_char;
        }
        let mut best_cost = i32::MAX;
        let mut best_left = None;
        let cost_row = cost_matrix.row(right_left_id);
        for (i, &penalty_cost) in cache.iter().enumerate() {
            let left_edge = &self.ends_at[start_char][i];
            let conn_cost = cost_row[left_edge.right_id as usize] as i32;
            let total_cost = (left_edge.path_cost + conn_cost)
                .saturating_add(penalty_cost)
                .saturating_add(extra_cost);

            // Every transition is recorded for N-Best.
            costs.push(total_cost);

            if total_cost <= best_cost {
                best_cost = total_cost;
                best_left = Some(i as u32);
            }
        }
        self.penalty_cache = cache;
        (best_cost, best_left)
    }

    /// Adds an edge ending at char position `stop_char` to the lattice and
    /// calculates the minimum cost to reach it.
    ///
    /// # 引数
    ///
    /// * `edge` - The edge to relax and store (path cost unset).
    /// * `stop_char` - The edge's end position, in characters; this is the
    ///   `ends_at` slot the edge is stored in, though a later whitespace
    ///   carry-over may move it on.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `mode` - The segmentation mode.
    /// * `extra_cost` - Position-dependent cost of this edge (the space
    ///   penalty; `0` when disabled). Independent of the left edge, so it
    ///   is added once after the relaxation scan rather than per candidate.
    fn add_edge_in_lattice(
        &mut self,
        edge: Edge,
        stop_char: usize,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
        extra_cost: i32,
    ) {
        if let Some(best) = self.relax(
            edge.start_char as usize,
            edge.left_id as u32,
            cost_matrix,
            mode,
        ) {
            self.push_relaxed(edge, stop_char, best, extra_cost);
        }
    }

    /// Returns the best `(path cost, left index)` over the left edges of
    /// `start_char` for an edge with left context id `right_left_id`, or
    /// `None` when there is no transition (see [`connected`]). The result does
    /// not depend on the incoming edge otherwise (its length, cost or right
    /// id), so edges that share the start and the left id can share one
    /// relaxation.
    ///
    /// Of the left edges that tie on cost, the last one in the slot wins
    /// (`<=`): the slot holds them in MeCab's processing order, so it is the
    /// one MeCab keeps (see [`Self::set_text_with_options`], #1135). The
    /// comparison compiles to conditional moves, keeping the loop free of
    /// branches that depend on the costs.
    ///
    /// # Arguments
    ///
    /// * `start_char` - The incoming edge's start position.
    /// * `right_left_id` - The incoming edge's left context id.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `mode` - The segmentation mode.
    ///
    /// # Returns
    ///
    /// The best transition, without the incoming edge's own costs.
    #[inline]
    fn relax(
        &mut self,
        start_char: usize,
        right_left_id: u32,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
    ) -> Option<(i32, u32)> {
        if self.ends_at[start_char].is_empty() {
            return None;
        }

        let mut best_cost = i32::MAX;
        let mut best_left = None;

        match mode {
            Mode::Normal => {
                // Matrix row hoisted out of the loop; the plain additions
                // cannot overflow thanks to PATH_COST_CLAMP (#880).
                let left_edges = &self.ends_at[start_char];
                let cost_row = cost_matrix.row(right_left_id);
                for (i, left_edge) in left_edges.iter().enumerate() {
                    let conn_cost = cost_row[left_edge.right_id as usize] as i32;
                    let total_cost = left_edge.path_cost + conn_cost;

                    if total_cost <= best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u32);
                    }
                }
                // Below `i32::MAX`, so `connected` is not needed.
                debug_assert!(best_cost < i32::MAX);
            }
            Mode::Decompose(penalty) => {
                let (cost, left) =
                    self.relax_decompose(start_char, right_left_id, penalty, cost_matrix);
                best_cost = cost;
                best_left = connected(cost, left);
            }
        }

        best_left.map(|left| (best_cost, left))
    }

    /// Stores `edge` in slot `stop_char` with the transition `best` found by
    /// [`Self::relax`].
    ///
    /// # Arguments
    ///
    /// * `edge` - The edge to store (path cost unset).
    /// * `stop_char` - The edge's end position, in characters.
    /// * `best` - The best `(path cost, left index)` of its start.
    /// * `extra_cost` - Position-dependent cost of this edge (the space
    ///   penalty).
    #[inline]
    fn push_relaxed(
        &mut self,
        mut edge: Edge,
        stop_char: usize,
        best: (i32, u32),
        extra_cost: i32,
    ) {
        let (best_cost, best_left) = best;
        edge.path_cost = best_cost
            .saturating_add(extra_cost)
            .saturating_add(edge.word_cost as i32)
            .min(PATH_COST_CLAMP);
        edge.left_index = best_left;
        self.ends_at[stop_char].push(edge);
    }

    /// Backtraces the best path and returns `(start_byte_offset,
    /// end_byte_offset, word_id)` for each token in reading order (BOS/EOS
    /// excluded). With whitespace skipped, a token ends where its own surface
    /// ends: the skipped whitespace after it is left out, the whitespace an
    /// entry itself ends with is not.
    ///
    /// # Returns
    ///
    /// A freshly allocated offsets vector; empty when the lattice holds no
    /// complete path. Prefer [`Lattice::tokens_offset_into`] in per-sentence
    /// loops to reuse one allocation across sentences.
    pub fn tokens_offset(&self) -> Vec<TokenOffset> {
        let mut offsets = Vec::new();
        self.tokens_offset_into(&mut offsets);
        offsets
    }

    /// Backtraces the best path into a caller-provided buffer, clearing it
    /// first, so the allocation can be reused across sentences.
    ///
    /// # Arguments
    ///
    /// * `offsets` - The buffer to fill with `(start_byte_offset,
    ///   end_byte_offset, word_id)` in reading order (BOS/EOS excluded; ends
    ///   as in [`Lattice::tokens_offset`]). Cleared on entry; left empty
    ///   when the lattice holds no complete path.
    ///
    /// # Returns
    ///
    /// The BOS index of the best path: the index in [`LatticeOptions::bos`]
    /// of the context it starts from, always 0 for the default single BOS
    /// edge. `None` when the lattice holds no complete path (no EOS edge).
    pub fn tokens_offset_into(&self, offsets: &mut Vec<TokenOffset>) -> Option<usize> {
        offsets.clear();

        if self.ends_at.is_empty() {
            return None;
        }

        // The EOS edge, when present, sits at `ends_at[last_n_chars]`
        // (see set_text), and every slot past it is always empty, so the
        // backward scan starts there rather than at the historical
        // capacity end (#877).
        let mut last_idx = self.last_n_chars.min(self.ends_at.len() - 1);
        while last_idx > 0 && self.ends_at[last_idx].is_empty() {
            last_idx -= 1;
        }

        let edge = self.ends_at[last_idx].last()?;
        if edge.left_index == u32::MAX {
            return None;
        }
        // The EOS edge is the only edge whose start is the slot holding it.
        // Without one (no complete path), the scan above stops at some
        // other edge, which is dropped like EOS and whose path is still
        // returned, as before; it has no BOS index to report.
        let complete = last_idx == self.last_n_chars && edge.start_char as usize == last_idx;
        let bos = self.backtrace_into(edge.start_char as usize, edge.left_index as usize, offsets);
        complete.then_some(bos)
    }

    /// Appends the path that ends with `ends_at[slot][index]` (that edge
    /// included, BOS excluded) to `offsets` in reading order. The shared
    /// core of [`Lattice::tokens_offset_into`] and
    /// [`Lattice::exit_tokens_offset_into`].
    ///
    /// # Arguments
    ///
    /// * `slot` - The slot holding the last edge of the path.
    /// * `index` - The index of that edge in `ends_at[slot]`; a BOS edge
    ///   gives an empty path.
    /// * `offsets` - An empty buffer to fill (it is reversed in place).
    ///
    /// # Returns
    ///
    /// The BOS index of the path: the index in [`LatticeOptions::bos`] of
    /// the context of the BOS edge it reaches (see [`Lattice::bos_index`]).
    fn backtrace_into(
        &self,
        mut slot: usize,
        mut index: usize,
        offsets: &mut Vec<TokenOffset>,
    ) -> usize {
        debug_assert!(
            offsets.is_empty(),
            "backtrace_into appends to a non-empty buffer"
        );
        loop {
            let edge = &self.ends_at[slot][index];
            if edge.left_index == u32::MAX {
                break;
            }
            let start_char = edge.start_char as usize;
            offsets.push((
                self.byte_offset_of(start_char),
                self.edge_end_byte(edge, slot),
                edge.word_id(),
            ));
            slot = start_char;
            index = edge.left_index as usize;
        }
        offsets.reverse();
        self.bos_index(index)
    }

    /// Maps the index of a BOS edge in the slot holding it to its BOS
    /// index: the BOS edges are the first edges of that slot, in reverse
    /// context order (see `push_bos_edges`). A lattice built without
    /// `push_bos_edges` has the default single BOS edge.
    ///
    /// # Arguments
    ///
    /// * `edge_index` - The index of the BOS edge in its slot.
    ///
    /// # Returns
    ///
    /// The index of its context in [`LatticeOptions::bos`], 0 for the
    /// default single BOS edge.
    #[inline]
    pub(crate) fn bos_index(&self, edge_index: usize) -> usize {
        debug_assert!(
            edge_index < self.bos_len.max(1),
            "not the index of a BOS edge"
        );
        self.bos_len.max(1) - 1 - edge_index
    }

    /// Returns the edges a path can end with before EOS: the first
    /// `final_edge_count` edges of `ends_at[last_n_chars]` (the BOS edges
    /// themselves for a sentence without words).
    ///
    /// # Returns
    ///
    /// The final edges; empty when no edge reaches the end of the sentence.
    pub(crate) fn final_edges(&self) -> &[Edge] {
        self.ends_at
            .get(self.last_n_chars)
            .and_then(|slot| slot.get(..self.final_edge_count))
            .unwrap_or(&[])
    }

    /// Returns the exit penalty of a final edge: its Decompose length
    /// penalty as the EOS connection computed it, 0 in Normal mode.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the edge in [`Lattice::final_edges`].
    ///
    /// # Returns
    ///
    /// The penalty.
    #[inline]
    pub(crate) fn exit_penalty(&self, index: usize) -> i32 {
        self.exit_penalties.get(index).copied().unwrap_or(0)
    }

    /// Collects the exits of the sentence: for each distinct right context
    /// id among the edges that end at the end of the sentence (the BOS
    /// edges for a sentence without words, e.g. one of skipped whitespace),
    /// the best path to such an edge, without the EOS connection (see
    /// [`LatticeExit`]). The cost of the best complete path is the minimum
    /// of `exit.cost()` plus the EOS connection from `exit.right_id()`.
    ///
    /// Works on the 1-best and the N-best lattice. Quadratic in the number
    /// of final edges, which is small: the words ending at one position.
    ///
    /// # Arguments
    ///
    /// * `out` - The buffer to fill, cleared on entry: one exit per right
    ///   id, in the order the right ids first appear among the final edges.
    ///   Left empty when no edge reaches the end of the sentence.
    pub fn exits_into(&self, out: &mut Vec<LatticeExit>) {
        out.clear();
        for (i, edge) in self.final_edges().iter().enumerate() {
            let cost = edge.path_cost.saturating_add(self.exit_penalty(i));
            match out.iter_mut().find(|exit| exit.right_id == edge.right_id) {
                // The last edge of equal cost wins, as in the EOS
                // connection (see `Lattice::relax`).
                Some(exit) => {
                    if cost <= exit.cost {
                        exit.cost = cost;
                        exit.edge = i as u32;
                    }
                }
                None => out.push(LatticeExit {
                    right_id: edge.right_id,
                    cost,
                    edge: i as u32,
                }),
            }
        }
    }

    /// Backtraces the path of an exit into a caller-provided buffer,
    /// clearing it first.
    ///
    /// # Arguments
    ///
    /// * `exit` - An exit of this lattice's current sentence, from
    ///   [`Lattice::exits_into`].
    /// * `offsets` - The buffer to fill with the path's tokens in reading
    ///   order, as [`Lattice::tokens_offset_into`] does; empty for a
    ///   sentence without words.
    ///
    /// # Returns
    ///
    /// The BOS index of the path: the index in [`LatticeOptions::bos`] of
    /// the context it starts from (0 for the default single BOS edge).
    ///
    /// # Panics
    ///
    /// When `exit` does not come from this lattice's current sentence.
    pub fn exit_tokens_offset_into(
        &self,
        exit: &LatticeExit,
        offsets: &mut Vec<TokenOffset>,
    ) -> usize {
        offsets.clear();
        let index = exit.edge as usize;
        assert!(
            index < self.final_edge_count
                && self.ends_at[self.last_n_chars][index].right_id == exit.right_id,
            "exit_tokens_offset_into: the exit does not belong to this lattice's sentence"
        );
        self.backtrace_into(self.last_n_chars, index, offsets)
    }

    // --- N-Best support ---

    /// Returns the character count from the last set_text/set_text_nbest
    /// call.
    ///
    /// # 戻り値
    ///
    /// The sentence length in characters (#943 renamed this from the
    /// byte-denominated `text_len`).
    pub fn char_len(&self) -> usize {
        self.last_n_chars
    }

    /// Returns the edges ending at a given char position.
    ///
    /// # 引数
    ///
    /// * `char_pos` - Char position, `0..=char_len()`.
    ///
    /// # 戻り値
    ///
    /// The edges stored in that slot: those ending there and, with
    /// whitespace skipped, those carried over the whitespace before it
    /// (#943 renamed this from the byte-denominated `edges_at`). They come in
    /// the order the lattice added them in, MeCab's processing order (see
    /// [`Lattice::set_text_with_options`]); the BOS edges come first, in
    /// reverse context order.
    pub fn edges_at_char(&self, char_pos: usize) -> &[Edge] {
        &self.ends_at[char_pos]
    }

    /// Returns the N-Best path entries recorded at a given char position.
    ///
    /// # 引数
    ///
    /// * `char_pos` - Char position, `0..=char_len()`.
    ///
    /// # 戻り値
    ///
    /// The transitions of the edges stored in that slot (#943 renamed this
    /// from the byte-denominated `paths_at`).
    pub fn paths_at_char(&self, char_pos: usize) -> &[PathEntry] {
        if char_pos < self.all_paths.len() {
            &self.all_paths[char_pos]
        } else {
            &[]
        }
    }

    /// Adds an edge ending at `stop_char`, recording ALL predecessor
    /// transitions for N-Best.
    ///
    /// # Arguments
    ///
    /// * `edge` - The edge to relax and store (path cost unset).
    /// * `stop_char` - The edge's end position, in characters; the slot it is
    ///   stored in, though a later whitespace carry-over may move it on.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `mode` - The segmentation mode.
    /// * `extra_cost` - Position-dependent cost of this edge (the space
    ///   penalty; `0` when disabled). Recorded in every `PathEntry` so the
    ///   backward A* (`nbest.rs`) sees it as part of the transition cost.
    fn add_edge_in_lattice_nbest(
        &mut self,
        edge: Edge,
        stop_char: usize,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
        extra_cost: i32,
    ) {
        let best = self.relax_nbest(
            edge.start_char as usize,
            edge.left_id as u32,
            cost_matrix,
            mode,
            extra_cost,
        );
        self.push_relaxed_nbest(edge, stop_char, best);
    }

    /// [`Self::relax`] for the nbest lattice: also leaves the cost of the
    /// transition from every left edge of `start_char` in
    /// `transition_costs`, in the order of the left edges, for
    /// [`Self::push_relaxed_nbest`] to record. Like the best transition,
    /// the costs do not depend on the incoming edge's length, cost or right
    /// id, so edges that share the start, the left id and `extra_cost` can
    /// share one relaxation.
    ///
    /// # Arguments
    ///
    /// * `start_char` - The incoming edge's start position.
    /// * `right_left_id` - The incoming edge's left context id.
    /// * `cost_matrix` - The connection cost matrix.
    /// * `mode` - The segmentation mode.
    /// * `extra_cost` - Position-dependent cost of the incoming edge (the
    ///   space penalty; `0` when disabled), part of every transition cost.
    ///
    /// # Returns
    ///
    /// The best `(path cost, left index)`, `extra_cost` included and the
    /// incoming edge's word cost not; `None` when no edge ends at
    /// `start_char` or no transition cost is below `i32::MAX`.
    #[inline]
    fn relax_nbest(
        &mut self,
        start_char: usize,
        right_left_id: u32,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
        extra_cost: i32,
    ) -> Option<(i32, u32)> {
        let mut costs = std::mem::take(&mut self.transition_costs);
        costs.clear();
        let (best_cost, best_left) = match mode {
            Mode::Normal => {
                // Same hoisted-row scan as add_edge_in_lattice (#880).
                let mut best_cost = i32::MAX;
                let mut best_left = None;
                let cost_row = cost_matrix.row(right_left_id);
                for (i, left_edge) in self.ends_at[start_char].iter().enumerate() {
                    let total_cost = (left_edge.path_cost
                        + cost_row[left_edge.right_id as usize] as i32)
                        .saturating_add(extra_cost);

                    // Every transition is recorded for N-Best.
                    costs.push(total_cost);

                    // The last of equal-cost edges wins (see `relax`).
                    if total_cost <= best_cost {
                        best_cost = total_cost;
                        best_left = Some(i as u32);
                    }
                }
                (best_cost, best_left)
            }
            Mode::Decompose(penalty) => self.relax_decompose_nbest(
                start_char,
                right_left_id,
                penalty,
                cost_matrix,
                extra_cost,
                &mut costs,
            ),
        };
        self.transition_costs = costs;
        connected(best_cost, best_left).map(|left| (best_cost, left))
    }

    /// Records the transitions [`Self::relax_nbest`] left in
    /// `transition_costs` for `edge`, then stores `edge` in slot `stop_char`
    /// with the best transition. The edge's `PathEntry`s are appended in
    /// one go, before any other edge can push into the slot, and carry the
    /// index the edge gets there, so each edge's entries form one
    /// contiguous run in ascending edge order (which `nbest.rs`
    /// binary-searches). Without a best transition the entries are still
    /// recorded but the edge is not stored, as before the relaxation was
    /// shared.
    ///
    /// # Arguments
    ///
    /// * `edge` - The edge to store (path cost unset); it starts where the
    ///   relaxation was computed.
    /// * `stop_char` - The edge's end position, in characters.
    /// * `best` - The best transition [`Self::relax_nbest`] returned.
    #[inline]
    fn push_relaxed_nbest(&mut self, mut edge: Edge, stop_char: usize, best: Option<(i32, u32)>) {
        let edge_index = self.ends_at[stop_char].len() as u32;
        let left_pos = edge.start_char as u32;
        self.all_paths[stop_char].extend(self.transition_costs.iter().enumerate().map(
            |(i, &cost)| PathEntry {
                edge_index,
                left_pos,
                left_index: i as u32,
                cost,
            },
        ));
        if let Some((best_cost, best_left)) = best {
            // `best_cost` already includes `extra_cost` (both arms of
            // `relax_nbest` fold it into the recorded transitions).
            edge.path_cost = best_cost
                .saturating_add(edge.word_cost as i32)
                .min(PATH_COST_CLAMP);
            edge.left_index = best_left;
            self.ends_at[stop_char].push(edge);
        }
    }

    /// [`Self::process_unknown_word`] for the nbest lattice.
    ///
    /// # Arguments
    ///
    /// See [`Self::process_unknown_word`].
    #[allow(clippy::too_many_arguments)]
    fn process_unknown_word_nbest(
        &mut self,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        search_mode: &Mode,
        max_grouping_len: Option<usize>,
        unknown_word_ladder: bool,
        space_penalty: Option<&SpacePenaltyTable>,
        category: CategoryId,
        category_ord: usize,
        char_idx: usize,
        found: bool,
    ) {
        let Some((primary, ladder)) = self.unknown_word_lengths(
            char_definitions,
            max_grouping_len,
            unknown_word_ladder,
            category,
            category_ord,
            char_idx,
            found,
        ) else {
            return;
        };
        self.emit_unknown_word_edges_nbest(
            unknown_dictionary,
            cost_matrix,
            search_mode,
            space_penalty,
            category,
            char_idx,
            Self::candidate_lengths(primary, ladder),
        );
    }

    /// Forward Viterbi for N-Best mode with the positional option set:
    /// [`Lattice::set_text`] that additionally records every predecessor
    /// transition in `all_paths`. Equivalent to
    /// [`Lattice::set_text_nbest_with_options`] without a space penalty.
    ///
    /// # Arguments
    ///
    /// See [`Lattice::set_text`].
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
        max_grouping_len: Option<usize>,
        unknown_word_ladder: bool,
    ) {
        let mut options = LatticeOptions::new(search_mode);
        options.max_grouping_len = max_grouping_len;
        options.unknown_word_ladder = unknown_word_ladder;
        self.set_text_nbest_with_options(
            dict,
            user_dict,
            char_definitions,
            unknown_dictionary,
            cost_matrix,
            text,
            &options,
        );
    }

    /// Forward Viterbi implementation for N-Best mode.
    /// Same as [`Lattice::set_text_with_options`] but records ALL
    /// predecessor transitions in `all_paths`.
    ///
    /// # Arguments
    ///
    /// See [`Lattice::set_text_with_options`].
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_text_nbest_with_options(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&UserPrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        text: &str,
        options: &LatticeOptions,
    ) {
        let search_mode = options.mode;
        let max_grouping_len = options.max_grouping_len;
        let unknown_word_ladder = options.unknown_word_ladder;
        // Same clear -> prepare -> grow sequence as set_text.
        self.clear();
        self.prepare_char_buffers(dict, char_definitions, text, options);
        let n_chars = self.chars_buf.len();
        // See set_text for the u16 start_char bound contract.
        assert!(
            n_chars < u16::MAX as usize,
            "set_text_nbest: sentence has {n_chars} characters, exceeding the u16::MAX-1 limit; \
             split the input into sentences (the Segmenter does this automatically)"
        );
        self.record_and_grow_nbest(n_chars);
        self.penalty_cache_pos = usize::MAX;
        self.skip_whitespace = options.skip_whitespace.is_some();

        self.push_bos_edges(options.bos);

        // Pre-scan text with Aho-Corasick
        // Buffers are Lattice fields reused across calls; refill matches_head (its
        // contents are meaningful, unlike ends_at's empty-Vec slots) and clear
        // matches_store (a plain append-only pool).
        // The pool now holds only user-dictionary matches; see set_text.
        self.matches_head.clear();
        self.matches_store.clear();

        // User dictionary scan (byte offsets converted to char positions;
        // see set_text).
        if let Some(ud) = user_dict {
            self.matches_head.resize(n_chars + 1, u32::MAX);
            let ud_vals: &[u8] = &ud.vals_data;
            for m in ud.da.find_overlapping_iter(text) {
                let start_char = self.char_index_of_byte(m.start());
                let (offset, count) = ud.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

                if start_char < self.matches_head.len() {
                    let avail = ud_vals.len().saturating_sub(offset_bytes);
                    let n = (count as usize).min(avail / WordEntry::SERIALIZED_LEN);
                    let block =
                        &ud_vals[offset_bytes..offset_bytes + n * WordEntry::SERIALIZED_LEN];
                    let end_char = self.char_index_of_byte(m.end()) as u32;
                    let (chunks, _) = block.as_chunks::<{ WordEntry::SERIALIZED_LEN }>();
                    // In CSV order, drained last row first; see set_text.
                    for chunk in chunks {
                        let entry = WordEntry::deserialize(chunk, false);
                        let next = self.matches_head[start_char];
                        self.matches_head[start_char] = self.matches_store.len() as u32;
                        self.matches_store.push((end_char, entry, next));
                    }
                }
            }
        }

        for char_idx in 0..n_chars {
            // Whitespace skipping; see set_text_with_options.
            if self.skip_whitespace {
                if self.char_info_buffer[char_idx].is_space {
                    continue;
                }
                if char_idx > 0 && self.char_info_buffer[char_idx - 1].is_space {
                    self.carry_over_whitespace(char_idx, true);
                }
            }

            if self.ends_at[char_idx].is_empty() {
                continue;
            }

            // Space penalty gate for this position; see set_text_with_options.
            let space_penalty = options.space_penalty_at(&self.char_info_buffer, char_idx);

            // The system dictionary's words, searched first; see
            // set_text_with_options.
            self.sys_matches.clear();
            {
                let suffix = &self.codes_buf[char_idx..];
                for (entries, end_char_offset) in dict.common_prefix_search_codes(suffix) {
                    let end_char = (char_idx + end_char_offset) as u32;
                    for chunk in entries.as_chunks::<{ WordEntry::SERIALIZED_LEN }>().0 {
                        self.sys_matches
                            .push((end_char, WordEntry::deserialize(chunk, true)));
                    }
                }
            }
            // Whether a dictionary word starts here.
            let found = (char_idx < self.matches_head.len()
                && self.matches_head[char_idx] != u32::MAX)
                || !self.sys_matches.is_empty();

            // The candidates in the processing order of set_text_with_options,
            // so ties resolve the same way: the unknown words first.
            let num_categories = self.char_info_buffer[char_idx].categories_len as usize;
            for category_ord in (0..num_categories).rev() {
                let category = self.get_cached_category(char_definitions, char_idx, category_ord);
                self.process_unknown_word_nbest(
                    char_definitions,
                    unknown_dictionary,
                    cost_matrix,
                    search_mode,
                    max_grouping_len,
                    unknown_word_ladder,
                    space_penalty,
                    category,
                    category_ord,
                    char_idx,
                    found,
                );
            }

            // Then the system dictionary's words, last found first.
            for i in (0..self.sys_matches.len()).rev() {
                let (end_char, word_entry) = self.sys_matches[i];
                let end_char = end_char as usize;
                let kanji_only = self.is_kanji_all(char_idx, end_char - char_idx);
                let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                let extra_cost = space_penalty_cost(space_penalty, &word_entry);
                self.add_edge_in_lattice_nbest(
                    edge,
                    end_char,
                    cost_matrix,
                    search_mode,
                    extra_cost,
                );
            }

            // Then the user dictionary's, last.
            if char_idx < self.matches_head.len() {
                let mut match_idx = self.matches_head[char_idx];
                while match_idx != u32::MAX {
                    let (end_char, word_entry, next) = self.matches_store[match_idx as usize];

                    let end_char = end_char as usize;
                    let kanji_only = self.is_kanji_all(char_idx, end_char - char_idx);
                    let edge = Self::create_edge(word_entry, char_idx, kanji_only);
                    let extra_cost = space_penalty_cost(space_penalty, &word_entry);
                    self.add_edge_in_lattice_nbest(
                        edge,
                        end_char,
                        cost_matrix,
                        search_mode,
                        extra_cost,
                    );

                    match_idx = next;
                }
            }
        }

        // Connect EOS with all-path recording; see set_text_with_options for
        // trailing whitespace.
        if self.skip_whitespace && n_chars > 0 && self.char_info_buffer[n_chars - 1].is_space {
            self.carry_over_whitespace(n_chars, true);
        }
        if !self.ends_at[n_chars].is_empty() {
            let content_end = self.content_end(n_chars);
            let eos_edge_index = self.ends_at[n_chars].len() as u32;
            let mut eos_edge = Edge {
                start_char: n_chars as u16,
                ..Default::default()
            };
            let mut best_cost = i32::MAX;
            let mut best_left = None;
            let cost_row = cost_matrix.row(0); // EOS default left_id
            // The edges before EOS are the sentence's exits (`exits_into`).
            self.final_edge_count = self.ends_at[n_chars].len();

            for i in 0..self.ends_at[n_chars].len() {
                let left_edge = &self.ends_at[n_chars][i];
                let path_cost = left_edge.path_cost + cost_row[left_edge.right_id as usize] as i32;
                let path_cost = match search_mode {
                    Mode::Normal => path_cost,
                    Mode::Decompose(penalty) => {
                        let exit_penalty =
                            penalty.penalty(left_edge, left_edge.char_len(content_end));
                        self.exit_penalties.push(exit_penalty);
                        path_cost.saturating_add(exit_penalty)
                    }
                };

                // Record all transitions to EOS
                self.all_paths[n_chars].push(PathEntry {
                    edge_index: eos_edge_index,
                    left_pos: n_chars as u32,
                    left_index: i as u32,
                    cost: path_cost,
                });

                // The last of equal-cost edges wins (see `relax`).
                if path_cost <= best_cost {
                    best_cost = path_cost;
                    best_left = Some(i as u32);
                }
            }
            if let Some(left_idx) = connected(best_cost, best_left) {
                eos_edge.left_index = left_idx;
                eos_edge.path_cost = best_cost;
                self.ends_at[n_chars].push(eos_edge);
            }
        }
    }

    /// Returns the top-N paths through the lattice.
    /// Each result is a (path, cost) pair where path is a Vec of (byte_start, byte_end, WordId)
    /// triples, with the ends of [`Lattice::tokens_offset`].
    /// The first result (index 0) is the 1-best path.
    /// If `unique` is true, paths with the same segmentation (same sequence of
    /// (byte_start, byte_end) pairs) are deduplicated, keeping only the first (lowest cost)
    /// variant.
    /// If `cost_threshold` is Some(t), paths whose cost exceeds best_cost + t are discarded.
    /// Requires set_text_nbest() to have been called first.
    ///
    /// The paths and costs are those of this lattice, i.e. of the one
    /// sentence it was built from, each ending with the EOS connection. The
    /// segmenter uses it for a sentence that is a segment of its own and
    /// combines the lists of an input's segments into whole-input results.
    ///
    /// Meant for the default single BOS edge: with several
    /// ([`LatticeOptions::bos`]) a path's cost includes its BOS edge's cost,
    /// but the result does not say which edge it starts from, and `unique`
    /// folds paths that differ only in it. Use
    /// [`NBestGenerator::next_with_bos`](crate::nbest::NBestGenerator::next_with_bos)
    /// there.
    pub fn nbest_tokens_offset(
        &self,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> Vec<NBestPath> {
        use std::collections::HashSet;

        use crate::nbest::NBestGenerator;
        let mut generator = NBestGenerator::new(self);
        // Grown on demand: `n` comes from the caller (the CLI's `-N`) and
        // can be far larger than the number of paths.
        let mut results = Vec::new();
        let mut best_cost: Option<i64> = None;

        if unique {
            let mut seen: HashSet<Vec<(usize, usize)>> = HashSet::new();
            while results.len() < n {
                match generator.next() {
                    Some((path, cost)) => {
                        // Record best cost from first result
                        let bc = *best_cost.get_or_insert(cost);
                        // Skip if cost exceeds threshold. Compare the
                        // difference: `bc + threshold` overflows for a
                        // threshold close to `i64::MAX`.
                        if let Some(threshold) = cost_threshold
                            && cost.saturating_sub(bc) > threshold
                        {
                            break;
                        }
                        let key: Vec<(usize, usize)> =
                            path.iter().map(|&(start, end, _)| (start, end)).collect();
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
                            && cost.saturating_sub(bc) > threshold
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
    use std::collections::BTreeMap;

    use daachorse::DoubleArrayAhoCorasickBuilder;

    use crate::dictionary::character_definition::{
        CategoryData, CategoryId, CharacterDefinition, LookupTable,
    };
    use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
    use crate::dictionary::prefix_dictionary::{PrefixDictionary, UserPrefixDictionary};
    use crate::dictionary::unknown_dictionary::UnknownDictionary;
    use crate::mode::{Mode, Penalty};
    use crate::nbest::NBestGenerator;
    use crate::viterbi::{
        BosContext, CharData, Edge, Lattice, LatticeExit, LatticeOptions, LexType, NBestPath,
        PATH_COST_CLAMP, PathEntry, TokenOffset, WordEntry, WordId,
    };
    use crate::whitespace::WhitespaceClassifier;

    /// Builds an edge whose backtrace fields are set explicitly, for
    /// hand-assembled lattices in tests. The edge's stop position is the
    /// `ends_at` slot the test pushes it into.
    fn test_edge(word_id: u32, start_char: usize, left_index: u32) -> Edge {
        let mut edge = Lattice::create_edge(
            WordEntry::new(WordId::new(LexType::System, word_id), 0, 0, 0),
            start_char,
            false,
        );
        edge.left_index = left_index;
        edge.path_cost = 0;
        edge
    }

    /// Seeds `char_info_buffer` with an identity char->byte mapping for
    /// `n_chars` characters, so hand-assembled lattices (which never run
    /// `prepare_char_buffers`) can back-convert positions in `tokens_offset`.
    /// Must run after `set_capacity`, whose `clear()` wipes the buffer.
    fn seed_identity_chars(lattice: &mut Lattice, n_chars: usize) {
        lattice.char_info_buffer.clear();
        for i in 0..=n_chars {
            lattice.char_info_buffer.push(CharData {
                byte_offset: i as u32,
                ..Default::default()
            });
        }
    }

    /// #944: the run lengths `group_run_len` caches must reproduce the
    /// forward grouping scan exactly for every (char, ordinal) pair,
    /// including multi-ordinal chars, runs at the sentence end, and
    /// ordinals present on one char but absent on the next, in both modes.
    /// The positions are visited in increasing order, all of them or only
    /// some (as when positions are unreachable or skipped whitespace), and
    /// the cache must not leak from one sentence into the next.
    #[test]
    fn test_group_runs_match_forward_scan() {
        use std::collections::BTreeMap;

        use crate::dictionary::character_definition::{
            CategoryData, CategoryId, CharacterDefinition, LookupTable,
        };
        use crate::dictionary::prefix_dictionary::PrefixDictionary;
        use crate::mode::{Mode, Penalty};

        // 'a'..='c' -> {A}; 'd'..='f' -> {B, A}; everything else -> {C}.
        let mapping = LookupTable::from_fn(vec![0u32, 0x61, 0x64, 0x67], &|c,
                                                                           buf: &mut Vec<
            CategoryId,
        >| {
            if (0x61..0x64).contains(&c) {
                buf.push(CategoryId(0));
            } else if (0x64..0x67).contains(&c) {
                buf.push(CategoryId(1));
                buf.push(CategoryId(0));
            } else {
                buf.push(CategoryId(2));
            }
        });
        let defs = vec![
            CategoryData {
                invoke: true,
                group: true,
                length: 0,
            };
            3
        ];
        let names = vec!["A".into(), "B".into(), "C".into()];
        let chardef = CharacterDefinition::new(defs, names, mapping);

        let mut map = BTreeMap::new();
        map.insert(
            "x".to_string(),
            vec![WordEntry::new(WordId::new(LexType::System, 0), 0, 0, 0)],
        );
        let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();

        let mut lattice = Lattice::default();
        for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
            // Runs crossing category boundaries, a two-ordinal stretch
            // (d-f), and a trailing single char; then a long `A` run that a
            // stale cache would extend into the next sentence's `a`.
            for text in ["aabdddefzza", "aaaaaaaa", "azzz"] {
                // Every position, then every third one.
                for step in [1, 3] {
                    lattice.prepare_char_buffers(
                        &dict,
                        &chardef,
                        text,
                        &LatticeOptions::new(&mode),
                    );
                    let n_chars = lattice.chars_buf.len();
                    assert_eq!(n_chars, text.chars().count());
                    for i in (0..n_chars).step_by(step) {
                        let char_data = lattice.char_info_buffer[i];
                        for ord in 0..char_data.categories_len as usize {
                            let cat = lattice.get_cached_category(&chardef, i, ord);
                            // Reference: the forward scan process_unknown_word
                            // used before #944.
                            let mut expected = 1usize;
                            for j in i + 1..n_chars {
                                let next = lattice.char_info_buffer[j];
                                if ord < next.categories_len as usize
                                    && lattice.get_cached_category(&chardef, j, ord) == cat
                                {
                                    expected += 1;
                                } else {
                                    break;
                                }
                            }
                            assert_eq!(
                                lattice.group_run_len(&chardef, cat, ord, i),
                                expected,
                                "run mismatch in {text:?} at char {i} ordinal {ord} \
                                 (step {step}, {mode:?})"
                            );
                        }
                    }
                }
            }
        }
    }

    /// #1105: a slot can hold more than `u16::MAX` edges. In every mode
    /// each position of a run adds its grouped unknown words to the slot at
    /// the run's end, and a dictionary word per character makes every
    /// position reachable. The cheapest path is one dictionary word
    /// per character, whose last edge is pushed into the final slot after
    /// about 5 * 14,000 grouped candidates. The edges refer to their left
    /// edges by `u32` indices, so the 1-best and the N-best backtraces
    /// still find that path; with `u16` indices the index was silently
    /// truncated and they followed another edge.
    #[test]
    fn test_slot_with_more_than_u16_max_edges() {
        // DEFAULT = 0, X = 1 (`x`), which invokes and groups, with five
        // unknown-word entries.
        let mapping = LookupTable::from_fn(vec![0, 0x78, 0x79], &|c, buf: &mut Vec<CategoryId>| {
            buf.push(CategoryId(usize::from(c == 0x78)))
        });
        let category = |invoke| CategoryData {
            invoke,
            group: true,
            length: 0,
        };
        let char_definition = CharacterDefinition::new(
            vec![category(false), category(true)],
            vec!["DEFAULT".into(), "X".into()],
            mapping,
        );
        let unknown = |id| WordEntry::new(WordId::new(LexType::Unknown, id), 1000, 0, 0);
        let unknown_dictionary = UnknownDictionary {
            category_references: vec![vec![0], (1..6).collect()],
            costs: (0..6).map(unknown).collect(),
            words_idx_data: Vec::new(),
            words_data: Vec::new(),
        };
        let mut map = BTreeMap::new();
        map.insert(
            "x".to_string(),
            vec![WordEntry::new(WordId::new(LexType::System, 0), -10, 0, 0)],
        );
        let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();

        let n = 14_000;
        let text = "x".repeat(n);
        let expected: Vec<TokenOffset> = (0..n)
            .map(|i| (i, i + 1, WordId::new(LexType::System, 0)))
            .collect();
        for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
            let options = LatticeOptions::new(&mode);
            let mut lattice = Lattice::default();

            lattice.set_text_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                &text,
                &options,
            );
            assert!(
                lattice.edges_at_char(n).len() > u16::MAX as usize,
                "{mode:?}"
            );
            assert_eq!(lattice.tokens_offset(), expected, "{mode:?}");

            lattice.set_text_nbest_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                &text,
                &options,
            );
            assert!(
                lattice.edges_at_char(n).len() > u16::MAX as usize,
                "{mode:?}"
            );
            assert_eq!(
                lattice.nbest_tokens_offset(1, false, None),
                vec![(expected.clone(), -10 * n as i64)],
                "{mode:?}"
            );
        }
    }

    /// #1105: one relaxation serves all the lengths of an unknown-word
    /// entry (the grouped run and the length ladder), but every length
    /// keeps an edge of its own: in its own slot, in processing order (the
    /// entries last first, #1135), with its own kanji-only flag (which the
    /// Decompose penalty reads). `一一一x`
    /// is one run of `X`, which groups and has a ladder up to 3, so the
    /// group (4 characters) is not kanji-only and the ladder lengths are.
    /// The N-best lattice must hold the same edges as the 1-best one, each
    /// with one contiguous run of transitions in ascending edge order.
    #[test]
    fn test_unknown_word_lengths_share_relaxation() {
        // DEFAULT = 0; X = 1 (`x` and `一`), which invokes and groups.
        let mapping =
            LookupTable::from_fn(vec![0, 0x78, 0x79, 0x4E00, 0x4E01], &|c,
                                                                        buf: &mut Vec<
                CategoryId,
            >| {
                buf.push(CategoryId(usize::from(c == 0x78 || c == 0x4E00)))
            });
        let char_definition = CharacterDefinition::new(
            vec![
                CategoryData {
                    invoke: false,
                    group: false,
                    length: 0,
                },
                CategoryData {
                    invoke: true,
                    group: true,
                    length: 3,
                },
            ],
            vec!["DEFAULT".into(), "X".into()],
            mapping,
        );
        let unknown = |id, cost| WordEntry::new(WordId::new(LexType::Unknown, id), cost, 0, 0);
        let unknown_dictionary = UnknownDictionary {
            category_references: vec![vec![0], vec![1, 2]],
            costs: vec![unknown(0, 0), unknown(1, 100), unknown(2, 200)],
            words_idx_data: Vec::new(),
            words_data: Vec::new(),
        };
        let mut map = BTreeMap::new();
        map.insert(
            "z".to_string(),
            vec![WordEntry::new(WordId::new(LexType::System, 0), 0, 0, 0)],
        );
        let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();

        let text = "一一一x";
        for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
            let options = LatticeOptions::new(&mode);
            let mut best = Lattice::default();
            best.set_text_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                text,
                &options,
            );
            let mut nbest = Lattice::default();
            nbest.set_text_nbest_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                text,
                &options,
            );

            // The kanji runs are computed in Decompose mode only.
            let kanji = matches!(mode, Mode::Decompose(_));
            let mut expected = Vec::new();
            for (slot, kanji_only) in [(1, kanji), (2, kanji), (3, kanji), (4, false)] {
                for (id, cost) in [(2, 200), (1, 100)] {
                    expected.push((slot, WordId::new(LexType::Unknown, id), kanji_only, cost));
                }
            }
            let from_start = |lattice: &Lattice| -> Vec<(usize, WordId, bool, i32)> {
                (1..=lattice.char_len())
                    .flat_map(|slot| {
                        lattice
                            .edges_at_char(slot)
                            .iter()
                            .filter(|edge| edge.start_char() == 0)
                            .map(move |edge| {
                                (slot, edge.word_id(), edge.kanji_only(), edge.path_cost())
                            })
                    })
                    .collect()
            };
            assert_eq!(from_start(&best), expected, "{mode:?}");
            assert_eq!(
                lattice_snapshot(&nbest).0,
                lattice_snapshot(&best).0,
                "{mode:?}"
            );

            for slot in 1..=nbest.char_len() {
                let indices: Vec<u32> = nbest
                    .paths_at_char(slot)
                    .iter()
                    .map(PathEntry::edge_index)
                    .collect();
                assert!(indices.is_sorted(), "{mode:?} slot {slot}: {indices:?}");
                let mut edges = indices.clone();
                edges.dedup();
                let stored: Vec<u32> = (0..nbest.edges_at_char(slot).len() as u32).collect();
                assert_eq!(edges, stored, "{mode:?} slot {slot}");
            }
        }
    }

    /// #1105: in every mode, an unknown word may start inside a run that
    /// an earlier position grouped, as in MeCab. `「` is a cheap dictionary
    /// word and a `SYMBOL` character, so the group `「⁂⁂` starts at 0; the
    /// best path is still `「` + the group `⁂⁂` that starts at 1, in the
    /// 1-best and the N-best lattice alike.
    #[test]
    fn test_unknown_word_starts_inside_grouped_run() {
        // DEFAULT = 0, SYMBOL = 1 (`「` and `⁂`), which invokes and groups.
        let mapping =
            LookupTable::from_fn(vec![0, 0x2042, 0x2043, 0x300C, 0x300D], &|c,
                                                                            buf: &mut Vec<
                CategoryId,
            >| match c {
                0x2042 | 0x300C => buf.push(CategoryId(1)),
                _ => buf.push(CategoryId(0)),
            });
        let category = |invoke| CategoryData {
            invoke,
            group: true,
            length: 0,
        };
        let char_definition = CharacterDefinition::new(
            vec![category(false), category(true)],
            vec!["DEFAULT".into(), "SYMBOL".into()],
            mapping,
        );
        let unknown = |id| WordEntry::new(WordId::new(LexType::Unknown, id), 1000, 0, 0);
        let unknown_dictionary = UnknownDictionary {
            category_references: vec![vec![0], vec![1]],
            costs: vec![unknown(0), unknown(1)],
            words_idx_data: Vec::new(),
            words_data: Vec::new(),
        };
        let mut map = BTreeMap::new();
        map.insert(
            "「".to_string(),
            vec![WordEntry::new(WordId::new(LexType::System, 0), -2000, 0, 0)],
        );
        let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();

        let text = "「⁂⁂";
        let bracket = (0, 3, WordId::new(LexType::System, 0));
        let symbols = (3, 9, WordId::new(LexType::Unknown, 1));
        for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
            let options = LatticeOptions::new(&mode);
            let mut lattice = Lattice::default();
            lattice.set_text_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                text,
                &options,
            );
            assert_eq!(lattice.tokens_offset(), vec![bracket, symbols], "{mode:?}");

            lattice.set_text_nbest_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                text,
                &options,
            );
            let paths = lattice.nbest_tokens_offset(usize::MAX, false, None);
            // `「` + `⁂⁂` and the group `「⁂⁂` (`SYMBOL` has no length
            // ladder, so no unknown word ends inside the run).
            assert_eq!(paths.len(), 2, "{mode:?}: {paths:?}");
            assert_eq!(paths[0], (vec![bracket, symbols], -1000), "{mode:?}");
        }
    }

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry =
            WordEntry::new(WordId::new(LexType::System, 1u32), -17i16, 1411u16, 1412u16);
        word_entry.serialize(&mut buffer).unwrap();
        assert_eq!(WordEntry::SERIALIZED_LEN, buffer.len());
        let bytes: &[u8; WordEntry::SERIALIZED_LEN] = buffer.as_slice().try_into().unwrap();
        let word_entry2 = WordEntry::deserialize(bytes, true);
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
        lattice.ends_at[0].push(test_edge(1, 0, u32::MAX));
        lattice.ends_at[57].push(test_edge(2, 0, 0));
        lattice.ends_at[100].push(test_edge(3, 57, 0)); // boundary slot

        // Shorter sentence: clear() runs bounded by the previous
        // last_text_len (100), then records the new length.
        lattice.set_capacity(10);
        assert!(
            lattice.ends_at.iter().all(|v| v.is_empty()),
            "stale edges survived a bounded clear"
        );

        // A second shrink exercises the induction step: nothing past the
        // new bound (10) may hold entries, and slots within it are cleared.
        lattice.ends_at[10].push(test_edge(4, 0, 0)); // boundary slot again
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

        // Hand-assembled best path for a 3-char sentence:
        // BOS(ends_at[0]) <- token A (0..3) <- EOS(ends_at[3]).
        lattice.set_capacity(3);
        seed_identity_chars(&mut lattice, 3);
        lattice.ends_at[0].push(test_edge(0, 0, u32::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(42, 0, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 0)); // EOS -> token A

        let offsets = lattice.tokens_offset();
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].2, WordId::new(LexType::System, 42));
    }

    /// `shrink_to` must release slots beyond the target while preserving
    /// the #841 per-slot pre-size on the remaining slots, and the lattice
    /// must regrow correctly (pre-sized) afterwards.
    #[test]
    fn test_shrink_to_truncates_and_keeps_presize() {
        let mut lattice = Lattice::default();

        lattice.set_capacity(100);
        lattice.ends_at[0].push(test_edge(1, 0, u32::MAX));
        lattice.ends_at[100].push(test_edge(2, 0, 0));

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
        lattice.ends_at[0].push(test_edge(1, 0, u32::MAX));
        lattice.ends_at[100].push(test_edge(2, 0, 0));

        lattice.shrink_to(10);

        // Hand-assemble a 3-char sentence path, as in the #877 tests.
        lattice.set_capacity(3);
        seed_identity_chars(&mut lattice, 3);
        lattice.ends_at[0].push(test_edge(0, 0, u32::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(42, 0, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 0)); // EOS -> token A

        let offsets = lattice.tokens_offset();
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(offsets[0].2, WordId::new(LexType::System, 42));

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
        seed_identity_chars(&mut lattice, 3);
        lattice.ends_at[0].push(test_edge(0, 0, u32::MAX)); // BOS
        lattice.ends_at[3].push(test_edge(7, 0, 0)); // token A
        lattice.ends_at[3].push(test_edge(0, 3, 0)); // EOS -> token A

        let mut reused = vec![(999usize, 999usize, WordId::default())]; // stale content
        lattice.tokens_offset_into(&mut reused);
        assert_eq!(reused, lattice.tokens_offset());
        // Without whitespace skipping a token ends at its slot.
        assert_eq!(reused, vec![(0, 3, WordId::new(LexType::System, 7))]);

        // A cleared (pathless) lattice must leave the reused buffer empty.
        lattice.clear();
        lattice.tokens_offset_into(&mut reused);
        assert!(reused.is_empty());
        assert!(lattice.tokens_offset().is_empty());
    }

    /// Hand-assembles a one-character N-Best lattice with two paths: word 1
    /// at cost `a` and word 2 at cost `b`, both spanning the character.
    fn two_path_nbest_lattice(a: i32, b: i32) -> Lattice {
        let mut lattice = Lattice::default();
        lattice.set_capacity_nbest(1);
        seed_identity_chars(&mut lattice, 1);
        lattice.ends_at[0].push(test_edge(0, 0, u32::MAX)); // BOS
        let mut word_a = test_edge(1, 0, 0);
        word_a.path_cost = a;
        let mut word_b = test_edge(2, 0, 0);
        word_b.path_cost = b;
        let mut eos = test_edge(0, 1, 0);
        eos.path_cost = a.min(b);
        lattice.ends_at[1].extend([word_a, word_b, eos]);
        lattice.all_paths[1].extend([
            // word 1 <- BOS, word 2 <- BOS, then EOS <- word 1 and word 2.
            PathEntry {
                edge_index: 0,
                left_pos: 0,
                left_index: 0,
                cost: a,
            },
            PathEntry {
                edge_index: 1,
                left_pos: 0,
                left_index: 0,
                cost: b,
            },
            PathEntry {
                edge_index: 2,
                left_pos: 1,
                left_index: 0,
                cost: a,
            },
            PathEntry {
                edge_index: 2,
                left_pos: 1,
                left_index: 1,
                cost: b,
            },
        ]);
        lattice
    }

    /// `nbest_tokens_offset` must accept a huge `n` and a threshold close to
    /// `i64::MAX` without overflowing, and measure the threshold from the
    /// best path.
    #[test]
    fn test_nbest_tokens_offset_extreme_n_and_threshold() {
        let lattice = two_path_nbest_lattice(3, 5);
        let word = |id| WordId::new(LexType::System, id);

        let results = lattice.nbest_tokens_offset(usize::MAX, false, Some(i64::MAX));
        assert_eq!(
            results,
            vec![(vec![(0, 1, word(1))], 3), (vec![(0, 1, word(2))], 5)]
        );
        // Both paths share one segmentation, so `unique` keeps the best.
        let results = lattice.nbest_tokens_offset(usize::MAX, true, Some(i64::MAX));
        assert_eq!(results, vec![(vec![(0, 1, word(1))], 3)]);

        assert_eq!(lattice.nbest_tokens_offset(10, false, Some(1)).len(), 1);
        assert_eq!(lattice.nbest_tokens_offset(10, false, Some(2)).len(), 2);
        assert!(lattice.nbest_tokens_offset(10, false, Some(-1)).is_empty());
    }

    /// Word id of the system entry `ab` in [`WsFixture`].
    const AB: u32 = 0;
    /// Word id of the system entry `ab ` (`ab` and a trailing U+0020).
    const AB_SPACE: u32 = 1;
    /// Word id of the system entry `cd`.
    const CD: u32 = 2;
    /// Word id of a costlier second `cd` entry, with the span of [`CD`].
    const CD_ALT: u32 = 3;
    /// Word id of the system entry `é ` (a 2-byte character and a space).
    const E_SPACE: u32 = 4;
    /// Word id of the system entry `é　` (`é` and the 3-byte U+3000).
    const E_IDEOGRAPHIC_SPACE: u32 = 5;
    /// Word id of the system entry `a` and 300 spaces, more trailing
    /// whitespace than `Edge::ws_tail` holds.
    const A_LONG: u32 = 6;
    /// Unknown-word id of the `SPACE` category in [`WsFixture`].
    const UNKNOWN_SPACE: u32 = 1;

    /// A tiny dictionary for the whitespace tests of #1108: the entries
    /// above, `DEFAULT`/`SPACE`/`ALPHA` categories, one costly unknown-word
    /// entry per category and free connections, so a path costs the sum of
    /// its word costs.
    struct WsFixture {
        dict: PrefixDictionary,
        char_definition: CharacterDefinition,
        unknown_dictionary: UnknownDictionary,
        cost_matrix: ConnectionCostMatrix,
        classifier: WhitespaceClassifier,
    }

    /// The categories of the test fixtures: DEFAULT = 0, SPACE = 1 (U+0009,
    /// U+0020 and U+3000, a member wider than one byte), ALPHA = 2 (a-z).
    /// No category invokes unknown words where a dictionary word starts,
    /// and none has a length ladder.
    fn test_char_definition() -> CharacterDefinition {
        let mapping = LookupTable::from_fn(
            vec![0, 0x09, 0x0A, 0x20, 0x21, 0x61, 0x7B, 0x3000, 0x3001],
            &|c, buf: &mut Vec<CategoryId>| match c {
                0x09 | 0x20 | 0x3000 => buf.push(CategoryId(1)),
                0x61..=0x7A => buf.push(CategoryId(2)),
                _ => buf.push(CategoryId(0)),
            },
        );
        let categories = vec![
            CategoryData {
                invoke: false,
                group: true,
                length: 0,
            };
            3
        ];
        let names = vec!["DEFAULT".into(), "SPACE".into(), "ALPHA".into()];
        CharacterDefinition::new(categories, names, mapping)
    }

    /// One costly unknown-word entry per category of
    /// [`test_char_definition`].
    fn test_unknown_dictionary() -> UnknownDictionary {
        let unknown = |id| WordEntry::new(WordId::new(LexType::Unknown, id), 5000, 0, 0);
        UnknownDictionary {
            category_references: vec![vec![0], vec![UNKNOWN_SPACE], vec![2]],
            costs: vec![unknown(0), unknown(UNKNOWN_SPACE), unknown(2)],
            words_idx_data: Vec::new(),
            words_data: Vec::new(),
        }
    }

    impl WsFixture {
        fn new() -> Self {
            let char_definition = test_char_definition();

            // `ab ` is far cheaper than `ab`, so it wins wherever a space
            // follows `ab`, even with the Decompose penalty of
            // `test_decompose_penalty_counts_entry_whitespace_only` on it.
            let entry = |id, cost| WordEntry::new(WordId::new(LexType::System, id), cost, 0, 0);
            let mut map = BTreeMap::new();
            map.insert("ab".to_string(), vec![entry(AB, 2000)]);
            map.insert("ab ".to_string(), vec![entry(AB_SPACE, 100)]);
            map.insert("cd".to_string(), vec![entry(CD, 10), entry(CD_ALT, 1000)]);
            map.insert("é ".to_string(), vec![entry(E_SPACE, 100)]);
            map.insert(
                "é\u{3000}".to_string(),
                vec![entry(E_IDEOGRAPHIC_SPACE, 100)],
            );
            map.insert(format!("a{}", " ".repeat(300)), vec![entry(A_LONG, 100)]);
            let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();
            let unknown_dictionary = test_unknown_dictionary();

            // Transposed-format header (-1, one forward id, one backward id)
            // and a single zero cost.
            let cost_matrix =
                ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
            let classifier = WhitespaceClassifier::new(&char_definition).unwrap();

            Self {
                dict,
                char_definition,
                unknown_dictionary,
                cost_matrix,
                classifier,
            }
        }

        fn options<'a>(&'a self, mode: &'a Mode, skip: bool) -> LatticeOptions<'a> {
            let mut options = LatticeOptions::new(mode);
            options.skip_whitespace = skip.then_some(&self.classifier);
            options
        }

        /// Builds the 1-best lattice of `text`.
        fn lattice(&self, text: &str, mode: &Mode, skip: bool) -> Lattice {
            let mut lattice = Lattice::default();
            lattice.set_text_with_options(
                &self.dict,
                &None,
                &self.char_definition,
                &self.unknown_dictionary,
                &self.cost_matrix,
                text,
                &self.options(mode, skip),
            );
            lattice
        }

        /// Builds the N-best lattice of `text`.
        fn nbest_lattice(&self, text: &str, mode: &Mode, skip: bool) -> Lattice {
            let mut lattice = Lattice::default();
            lattice.set_text_nbest_with_options(
                &self.dict,
                &None,
                &self.char_definition,
                &self.unknown_dictionary,
                &self.cost_matrix,
                text,
                &self.options(mode, skip),
            );
            lattice
        }

        /// Returns the 1-best tokens of `text` with whitespace skipped.
        fn skipped(&self, text: &str) -> Vec<TokenOffset> {
            self.lattice(text, &Mode::Normal, true).tokens_offset()
        }

        /// Returns every N-best path of `text` (no dedup, no threshold).
        fn all_paths(&self, text: &str, mode: &Mode, skip: bool) -> Vec<NBestPath> {
            self.nbest_lattice(text, mode, skip)
                .nbest_tokens_offset(usize::MAX, false, None)
        }
    }

    /// Returns the word id of a system entry.
    fn sys(id: u32) -> WordId {
        WordId::new(LexType::System, id)
    }

    /// Returns the cost of the 1-best path: the EOS edge's path cost.
    fn eos_cost(lattice: &Lattice) -> i64 {
        let eos = lattice.edges_at_char(lattice.char_len()).last().unwrap();
        eos.path_cost() as i64
    }

    /// Returns the cost of the N-best path whose tokens are `tokens`.
    fn cost_of(paths: &[NBestPath], tokens: &[TokenOffset]) -> i64 {
        paths
            .iter()
            .find(|(path, _)| path == tokens)
            .map(|&(_, cost)| cost)
            .unwrap_or_else(|| panic!("no path {tokens:?} in {paths:?}"))
    }

    /// #1108: an edge that already ends at the slot after a whitespace run
    /// (`ab ` in `ab cd`) keeps the whitespace it ends with: its token ends
    /// after the space, not where the run starts.
    #[test]
    fn test_skip_whitespace_keeps_entry_whitespace_in_target_slot() {
        let fixture = WsFixture::new();
        assert_eq!(
            fixture.skipped("ab cd"),
            vec![(0, 3, sys(AB_SPACE)), (3, 5, sys(CD))]
        );
    }

    /// #1108: an edge moved from slot `k` inside a longer run (`ab ` in
    /// `ab  cd` and `ab \tcd`) ends after its own space; the rest of the run
    /// is skipped and belongs to no token.
    #[test]
    fn test_skip_whitespace_keeps_entry_whitespace_when_moved() {
        let fixture = WsFixture::new();
        for text in ["ab  cd", "ab \tcd"] {
            assert_eq!(
                fixture.skipped(text),
                vec![(0, 3, sys(AB_SPACE)), (4, 6, sys(CD))],
                "{text:?}"
            );
        }
        assert_eq!(
            fixture.skipped("ab \t cd"),
            vec![(0, 3, sys(AB_SPACE)), (5, 7, sys(CD))]
        );
    }

    /// #1108: leading whitespace carries BOS over to the first character;
    /// the offsets after it stay exact.
    #[test]
    fn test_skip_whitespace_entry_ends_after_leading_whitespace() {
        let fixture = WsFixture::new();
        assert_eq!(
            fixture.skipped("  ab cd"),
            vec![(2, 5, sys(AB_SPACE)), (5, 7, sys(CD))]
        );
        assert_eq!(
            fixture.skipped("\t ab  cd"),
            vec![(2, 5, sys(AB_SPACE)), (6, 8, sys(CD))]
        );
    }

    /// #1108: at the end of the sentence (the carry-over to EOS) the entry
    /// keeps its own space and the whitespace after it is skipped, in the
    /// 1-best and the N-best lattice alike.
    #[test]
    fn test_skip_whitespace_entry_ends_at_end_of_sentence() {
        let fixture = WsFixture::new();
        for text in ["ab ", "ab   ", "ab \t"] {
            assert_eq!(
                fixture.skipped(text),
                vec![(0, 3, sys(AB_SPACE))],
                "{text:?}"
            );
            assert_eq!(
                fixture.all_paths(text, &Mode::Normal, true),
                vec![
                    (vec![(0, 3, sys(AB_SPACE))], 100),
                    (vec![(0, 2, sys(AB))], 2000),
                ],
                "{text:?}"
            );
        }
    }

    /// #1108: `ws_tail` counts characters and the end is converted to bytes
    /// afterwards: right after a 2-byte `é`, and with a 3-byte U+3000 as the
    /// entry's own whitespace and as the skipped whitespace.
    #[test]
    fn test_skip_whitespace_entry_ends_with_multibyte_characters() {
        let fixture = WsFixture::new();

        let text = "é  cd";
        let tokens = fixture.skipped(text);
        assert_eq!(tokens, vec![(0, 3, sys(E_SPACE)), (4, 6, sys(CD))]);
        assert_eq!(&text[tokens[0].0..tokens[0].1], "é ");

        let text = "é\u{3000}\u{3000}cd";
        let tokens = fixture.skipped(text);
        assert_eq!(
            tokens,
            vec![(0, 5, sys(E_IDEOGRAPHIC_SPACE)), (8, 10, sys(CD))]
        );
        assert_eq!(&text[tokens[0].0..tokens[0].1], "é\u{3000}");
    }

    /// #1108: with whitespace kept in the lattice (`skip_whitespace: None`)
    /// nothing writes `ws_tail`, so every token ends where the next one
    /// starts (or at the end of the sentence), as before the fix.
    #[test]
    fn test_kept_whitespace_tokens_are_contiguous() {
        let fixture = WsFixture::new();
        let long = format!("a{}cd", " ".repeat(300));
        let texts = [
            "ab cd",
            "ab  cd",
            "ab \tcd",
            "  ab  cd  ",
            "ab   ",
            "é  cd",
            "é\u{3000}\u{3000}cd",
            long.as_str(),
        ];
        for text in texts {
            let lattice = fixture.lattice(text, &Mode::Normal, false);
            assert!(
                (0..=lattice.char_len())
                    .flat_map(|slot| lattice.edges_at_char(slot))
                    .all(|edge| edge.ws_tail == 0),
                "{text:?}: ws_tail written without skipping"
            );
            let tokens = lattice.tokens_offset();
            assert_eq!(tokens.first().map(|token| token.0), Some(0), "{text:?}");
            for pair in tokens.windows(2) {
                assert_eq!(pair[0].1, pair[1].0, "{text:?}: {tokens:?}");
            }
            assert_eq!(
                tokens.last().map(|token| token.1),
                Some(text.len()),
                "{text:?}"
            );
        }

        // The whitespace is an unknown-word token of its own here: the
        // `SPACE` group, as no entry ends with a tab.
        let unknown_space = WordId::new(LexType::Unknown, UNKNOWN_SPACE);
        assert_eq!(
            fixture
                .lattice("ab\t\tcd", &Mode::Normal, false)
                .tokens_offset(),
            vec![(0, 2, sys(AB)), (2, 4, unknown_space), (4, 6, sys(CD))]
        );
        // With two spaces, an unknown word also starts at the second one,
        // inside the run the `SPACE` group covers (#1105), so `ab ` (100)
        // plus a one-space unknown word (5000) beats `ab` (2000) plus the
        // group (5000). The tokens still tile the text.
        assert_eq!(
            fixture
                .lattice("ab  cd", &Mode::Normal, false)
                .tokens_offset(),
            vec![
                (0, 3, sys(AB_SPACE)),
                (3, 4, unknown_space),
                (4, 6, sys(CD))
            ]
        );
    }

    /// #1108: the N-best backtrace returns the ends `tokens_offset` returns
    /// for rank 1 (and the same cost), and the entry's own ends for the
    /// other ranks.
    #[test]
    fn test_nbest_ends_match_tokens_offset() {
        let fixture = WsFixture::new();
        let long = format!("a{}cd", " ".repeat(300));
        let texts = [
            "ab cd",
            "ab  cd",
            "ab \tcd",
            "  ab cd",
            "  ab  cd ",
            "ab ",
            "ab   ",
            "é  cd",
            "é\u{3000}\u{3000}cd",
            long.as_str(),
        ];
        for text in texts {
            let lattice = fixture.lattice(text, &Mode::Normal, true);
            let paths = fixture.all_paths(text, &Mode::Normal, true);
            assert_eq!(paths[0].0, lattice.tokens_offset(), "{text:?}");
            assert_eq!(paths[0].1, eos_cost(&lattice), "{text:?}");
        }

        assert_eq!(
            fixture.all_paths("  ab  cd ", &Mode::Normal, true),
            vec![
                (vec![(2, 5, sys(AB_SPACE)), (6, 8, sys(CD))], 110),
                (vec![(2, 5, sys(AB_SPACE)), (6, 8, sys(CD_ALT))], 1100),
                (vec![(2, 4, sys(AB)), (6, 8, sys(CD))], 2010),
                (vec![(2, 4, sys(AB)), (6, 8, sys(CD_ALT))], 3000),
            ]
        );
    }

    /// #1108: unique N-best deduplicates by the (start, end) sequence, so
    /// `ab ` + `cd` and `ab` + `cd`, which share their starts, both stay,
    /// while the `CD_ALT` variants with the same spans fold into them.
    #[test]
    fn test_unique_nbest_keeps_paths_differing_only_in_ends() {
        let fixture = WsFixture::new();
        let lattice = fixture.nbest_lattice("ab cd", &Mode::Normal, true);
        assert_eq!(
            lattice.nbest_tokens_offset(usize::MAX, false, None).len(),
            4
        );
        assert_eq!(
            lattice.nbest_tokens_offset(usize::MAX, true, None),
            vec![
                (vec![(0, 3, sys(AB_SPACE)), (3, 5, sys(CD))], 110),
                (vec![(0, 2, sys(AB)), (3, 5, sys(CD))], 2010),
            ]
        );
    }

    /// #1108: the Decompose length penalty measures an edge carried over
    /// skipped whitespace by its own span, trailing whitespace included and
    /// skipped whitespace excluded: `ab ` (3 characters) pays exactly one
    /// step over the threshold of 2 however much whitespace follows it, and
    /// `ab` (2 characters) pays nothing. Checked through the 1-best lattice
    /// (its relaxation and EOS call sites) and the N-best lattice (its own
    /// two).
    #[test]
    fn test_decompose_penalty_counts_entry_whitespace_only() {
        let fixture = WsFixture::new();
        // The kanji threshold must not exceed the other one: `Penalty::penalty`
        // returns 0 for any span up to the kanji threshold before looking at
        // the edge's kind.
        let decompose = Mode::Decompose(Penalty {
            kanji_penalty_length_threshold: 2,
            kanji_penalty_length_penalty: 3000,
            other_penalty_length_threshold: 2,
            other_penalty_length_penalty: 1000,
        });

        // Each text with the span of its `cd`, if any.
        let cases = [
            ("ab cd", Some((3, 5))),
            ("ab   cd", Some((5, 7))),
            ("ab \t cd", Some((5, 7))),
            ("ab ", None),
            ("ab   ", None),
        ];
        for (text, cd) in cases {
            let with_cd = |token: TokenOffset| {
                let mut tokens = vec![token];
                tokens.extend(cd.map(|(start, end)| (start, end, sys(CD))));
                tokens
            };
            let ab_space_path = with_cd((0, 3, sys(AB_SPACE)));
            let ab_path = with_cd((0, 2, sys(AB)));

            // 1-best: `ab ` wins in both modes (100 + 1000 < 2000).
            let normal = fixture.lattice(text, &Mode::Normal, true);
            let decomposed = fixture.lattice(text, &decompose, true);
            assert_eq!(normal.tokens_offset(), ab_space_path, "{text:?}");
            assert_eq!(decomposed.tokens_offset(), ab_space_path, "{text:?}");
            assert_eq!(eos_cost(&decomposed) - eos_cost(&normal), 1000, "{text:?}");

            // N-best: the same path in both modes, and the `ab` path.
            let normal = fixture.all_paths(text, &Mode::Normal, true);
            let decomposed = fixture.all_paths(text, &decompose, true);
            assert_eq!(
                cost_of(&decomposed, &ab_space_path) - cost_of(&normal, &ab_space_path),
                1000,
                "{text:?}"
            );
            assert_eq!(
                cost_of(&decomposed, &ab_path) - cost_of(&normal, &ab_path),
                0,
                "{text:?}"
            );
        }
    }

    /// #1108: `ws_tail` saturates at `u8::MAX`: an entry whose surface ends
    /// with 300 spaces, followed by `cd`, ends 1 + 255 characters after its
    /// start rather than after all 300 (the documented saturation, far
    /// beyond any real entry).
    #[test]
    fn test_skip_whitespace_ws_tail_saturates() {
        let fixture = WsFixture::new();
        let text = format!("a{}cd", " ".repeat(300));
        assert_eq!(
            fixture.skipped(&text),
            vec![(0, 1 + 255, sys(A_LONG)), (301, 303, sys(CD))]
        );
    }

    // --- #1096: several BOS edges, exits, N-best from an exit ---

    /// Number of context ids in [`CtxFixture`]; 0 is BOS/EOS.
    const CTX_IDS: usize = 4;

    /// Connection costs of [`CtxFixture`], `CTX_CONN[right_id][left_id]`:
    /// what a word with left id `left_id` pays after an edge with right id
    /// `right_id`. Row 0 is the default BOS, column 0 is EOS. Every BOS
    /// context prefers another first word, and the EOS connection differs
    /// per right id.
    const CTX_CONN: [[i16; CTX_IDS]; CTX_IDS] = [
        [0, 30, 250, 400],
        [500, 100, -50, 300],
        [40, 200, 80, -20],
        [300, -60, 150, 90],
    ];

    /// The entries of [`CtxFixture`]: (surface, word id, word cost, left id,
    /// right id). Every letter has a one-character entry, so no unknown
    /// word is ever emitted and the lattice holds exactly the paths
    /// [`brute_paths`] enumerates. `d` has two identical entries (a tie),
    /// and `b ` ends with a space.
    const CTX_ENTRIES: &[(&str, u32, i16, u16, u16)] = &[
        ("a", 0, 100, 1, 1),
        ("a", 1, 150, 2, 3),
        ("b", 2, 120, 2, 2),
        ("b", 3, 80, 3, 1),
        ("c", 4, 90, 3, 3),
        ("c", 5, 60, 1, 2),
        ("ab", 6, 200, 1, 2),
        ("bc", 7, 160, 2, 3),
        ("bc", 8, 170, 3, 2),
        ("abc", 9, 260, 1, 3),
        ("ca", 10, 140, 3, 1),
        ("d", 12, 50, 1, 2),
        ("d", 13, 50, 1, 2),
        ("b ", 11, 70, 2, 3),
    ];

    /// The Decompose penalty of the #1096 tests: every entry longer than
    /// one character pays 500 per extra character (none is kanji).
    fn ctx_decompose() -> Mode {
        Mode::Decompose(Penalty {
            kanji_penalty_length_threshold: 1,
            kanji_penalty_length_penalty: 3000,
            other_penalty_length_threshold: 1,
            other_penalty_length_penalty: 500,
        })
    }

    fn bos_ctx(right_id: u16, cost: i32) -> BosContext {
        BosContext { right_id, cost }
    }

    /// The BOS lists of the #1096 tests: the default, its explicit form, and
    /// lists whose best context depends on the text.
    fn ctx_bos_lists() -> Vec<Vec<BosContext>> {
        vec![
            vec![],
            vec![bos_ctx(0, 0)],
            vec![
                bos_ctx(0, 0),
                bos_ctx(1, 50),
                bos_ctx(2, 10),
                bos_ctx(3, 300),
            ],
            vec![bos_ctx(2, 400), bos_ctx(1, 0)],
            vec![bos_ctx(3, 0), bos_ctx(2, 90), bos_ctx(1, 120)],
        ]
    }

    /// Texts without whitespace (built with and without skipping).
    const CTX_TEXTS: &[&str] = &[
        "", "a", "b", "c", "abc", "cab", "bcab", "abcabc", "cbabca", "dab", "abd",
    ];

    /// Texts with whitespace (built with whitespace skipped only).
    const CTX_SPACED_TEXTS: &[&str] = &["   ", " ab c", "ab  ", "b\t c", "ab b c ", "  ca", "ab\t"];

    /// Runs `f` on every (text, mode, skip, BOS list) case of the #1096
    /// tests.
    fn for_each_ctx_case(mut f: impl FnMut(&str, &Mode, bool, &[BosContext])) {
        for mode in [Mode::Normal, ctx_decompose()] {
            for bos_list in ctx_bos_lists() {
                for &text in CTX_TEXTS {
                    for skip in [false, true] {
                        f(text, &mode, skip, &bos_list);
                    }
                }
                for &text in CTX_SPACED_TEXTS {
                    f(text, &mode, true, &bos_list);
                }
            }
        }
    }

    /// A tiny dictionary where context ids matter: the entries and
    /// connection costs above, the categories of [`test_char_definition`].
    struct CtxFixture {
        dict: PrefixDictionary,
        char_definition: CharacterDefinition,
        unknown_dictionary: UnknownDictionary,
        cost_matrix: ConnectionCostMatrix,
        classifier: WhitespaceClassifier,
    }

    impl CtxFixture {
        fn new() -> Self {
            let mut map: BTreeMap<String, Vec<WordEntry>> = BTreeMap::new();
            for &(surface, id, cost, left, right) in CTX_ENTRIES {
                map.entry(surface.to_string())
                    .or_default()
                    .push(WordEntry::new(sys(id), cost, left, right));
            }
            let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();

            // Transposed format: header (-1, forward size, backward size),
            // then `costs[right_id + left_id * forward size]`.
            let mut bytes = Vec::new();
            for value in [-1, CTX_IDS as i16, CTX_IDS as i16] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            for left in 0..CTX_IDS {
                for row in &CTX_CONN {
                    bytes.extend_from_slice(&row[left].to_le_bytes());
                }
            }
            let cost_matrix = ConnectionCostMatrix::load(bytes).unwrap();
            let char_definition = test_char_definition();
            let classifier = WhitespaceClassifier::new(&char_definition).unwrap();
            Self {
                dict,
                char_definition,
                unknown_dictionary: test_unknown_dictionary(),
                cost_matrix,
                classifier,
            }
        }

        fn options<'a>(
            &'a self,
            mode: &'a Mode,
            skip: bool,
            bos: &'a [BosContext],
        ) -> LatticeOptions<'a> {
            let mut options = LatticeOptions::new(mode);
            options.skip_whitespace = skip.then_some(&self.classifier);
            options.bos = bos;
            options
        }

        fn lattice(&self, text: &str, mode: &Mode, skip: bool, bos: &[BosContext]) -> Lattice {
            let mut lattice = Lattice::default();
            lattice.set_text_with_options(
                &self.dict,
                &None,
                &self.char_definition,
                &self.unknown_dictionary,
                &self.cost_matrix,
                text,
                &self.options(mode, skip, bos),
            );
            lattice
        }

        fn nbest_lattice(
            &self,
            text: &str,
            mode: &Mode,
            skip: bool,
            bos: &[BosContext],
        ) -> Lattice {
            let mut lattice = Lattice::default();
            lattice.set_text_nbest_with_options(
                &self.dict,
                &None,
                &self.char_definition,
                &self.unknown_dictionary,
                &self.cost_matrix,
                text,
                &self.options(mode, skip, bos),
            );
            lattice
        }
    }

    /// One path of [`brute_paths`].
    #[derive(Clone, Debug)]
    struct BrutePath {
        bos: usize,
        tokens: Vec<TokenOffset>,
        /// Right id of the last word (of the BOS context without words).
        last_right: u16,
        /// Cost without EOS: what [`LatticeExit::cost`] measures.
        exit_cost: i64,
    }

    impl BrutePath {
        /// Cost with the EOS connection.
        fn eos_cost(&self) -> i64 {
            self.exit_cost + CTX_CONN[self.last_right as usize][0] as i64
        }
    }

    /// Enumerates every path of `text` through the [`CtxFixture`] entries
    /// from every BOS context, by definition: a path starts after leading
    /// whitespace, each word starts where the previous one ends after the
    /// whitespace there, and a word pays its cost, the connection from the
    /// previous right id and the Decompose penalty of the previous word (0
    /// for BOS); the last word's penalty is part of the exit cost. Meant
    /// for texts of `a`-`d`, spaces and tabs, with whitespace skipped (or
    /// absent).
    fn brute_paths(text: &str, bos_list: &[BosContext], mode: &Mode) -> Vec<BrutePath> {
        fn skip_ws(text: &str, mut pos: usize) -> usize {
            while pos < text.len() && matches!(text.as_bytes()[pos], b' ' | b'\t') {
                pos += 1;
            }
            pos
        }

        #[allow(clippy::too_many_arguments)]
        fn walk(
            text: &str,
            mode: &Mode,
            bos: usize,
            pos: usize,
            prev_right: u16,
            prev_penalty: i64,
            cost: i64,
            tokens: &mut Vec<TokenOffset>,
            out: &mut Vec<BrutePath>,
        ) {
            if pos == text.len() {
                out.push(BrutePath {
                    bos,
                    tokens: tokens.clone(),
                    last_right: prev_right,
                    exit_cost: cost + prev_penalty,
                });
                return;
            }
            for &(surface, id, word_cost, left, right) in CTX_ENTRIES {
                if !text[pos..].starts_with(surface) {
                    continue;
                }
                let penalty = match mode {
                    Mode::Normal => 0,
                    Mode::Decompose(penalty) => {
                        let entry = WordEntry::new(sys(id), word_cost, left, right);
                        let edge = Lattice::create_edge(entry, 0, false);
                        penalty.penalty(&edge, surface.chars().count()) as i64
                    }
                };
                let end = pos + surface.len();
                let cost = cost
                    + prev_penalty
                    + CTX_CONN[prev_right as usize][left as usize] as i64
                    + word_cost as i64;
                tokens.push((pos, end, sys(id)));
                walk(
                    text,
                    mode,
                    bos,
                    skip_ws(text, end),
                    right,
                    penalty,
                    cost,
                    tokens,
                    out,
                );
                tokens.pop();
            }
        }

        let default_bos = [bos_ctx(0, 0)];
        let bos_list = if bos_list.is_empty() {
            &default_bos[..]
        } else {
            bos_list
        };
        let mut out = Vec::new();
        for (index, context) in bos_list.iter().enumerate() {
            let cost = context.cost.clamp(0, PATH_COST_CLAMP) as i64;
            let start = skip_ws(text, 0);
            walk(
                text,
                mode,
                index,
                start,
                context.right_id,
                0,
                cost,
                &mut Vec::new(),
                &mut out,
            );
        }
        out
    }

    /// Returns a sortable form of system-word tokens: `WordId` is not `Ord`.
    fn token_keys(tokens: &[TokenOffset]) -> Vec<(usize, usize, u32)> {
        tokens
            .iter()
            .map(|&(start, end, word_id)| {
                assert!(word_id.is_system());
                (start, end, word_id.id())
            })
            .collect()
    }

    /// Returns the 1-best tokens, BOS index and cost of a lattice.
    fn best_path(lattice: &Lattice) -> (Vec<TokenOffset>, Option<usize>, i64) {
        let mut tokens = vec![(9, 9, WordId::default())]; // stale content
        let bos = lattice.tokens_offset_into(&mut tokens);
        (tokens, bos, eos_cost(lattice))
    }

    /// Returns the exits of a lattice.
    fn exits_of(lattice: &Lattice) -> Vec<LatticeExit> {
        let mut exits = vec![LatticeExit {
            right_id: 9,
            cost: 9,
            edge: 9,
        }]; // stale content
        lattice.exits_into(&mut exits);
        exits
    }

    /// Asserts that the 1-best path is one of the cheapest `brute` paths
    /// (with EOS), and the only one when the cheapest is unique.
    fn assert_best_is_brute_min(lattice: &Lattice, brute: &[BrutePath], case: &str) {
        let (tokens, bos, cost) = best_path(lattice);
        let min = brute.iter().map(BrutePath::eos_cost).min().unwrap();
        assert_eq!(cost, min, "{case}");
        let bos = bos.unwrap();
        assert!(
            brute
                .iter()
                .any(|path| path.eos_cost() == min && path.tokens == tokens && path.bos == bos),
            "{case}: {tokens:?} from BOS {bos} is not a cheapest path"
        );
    }

    /// #1096: with several BOS edges, the best path is the best of the
    /// lattices built from each context alone (with no cost, the context's
    /// cost added afterwards), down to the tokens and the BOS index, and
    /// the cheapest path by definition. The fixture makes the context
    /// matter: some text is segmented differently from different contexts,
    /// and some best path starts from a context other than the first.
    #[test]
    fn test_multi_bos_is_best_single_bos() {
        let fixture = CtxFixture::new();
        let mut segmentation_depends_on_bos = false;
        let mut best_bos_not_first = false;
        let mut bos_cost_decides = false;
        for_each_ctx_case(|text, mode, skip, bos_list| {
            if bos_list.len() < 2 {
                return;
            }
            let case = format!("{text:?} {mode:?} skip={skip} {bos_list:?}");
            let lattice = fixture.lattice(text, mode, skip, bos_list);
            let (tokens, bos, cost) = best_path(&lattice);

            let singles: Vec<_> = bos_list
                .iter()
                .map(|context| {
                    let single = fixture.lattice(text, mode, skip, &[bos_ctx(context.right_id, 0)]);
                    let (tokens, bos, cost) = best_path(&single);
                    assert_eq!(bos, Some(0), "{case}");
                    (tokens, cost + context.cost as i64)
                })
                .collect();
            let min = singles.iter().map(|&(_, cost)| cost).min().unwrap();
            assert_eq!(cost, min, "{case}");
            segmentation_depends_on_bos |= singles.iter().any(|(t, _)| *t != singles[0].0);
            let argmin: Vec<usize> = (0..singles.len())
                .filter(|&i| singles[i].1 == min)
                .collect();
            let brute = brute_paths(text, bos_list, mode);
            let unique = brute.iter().filter(|path| path.eos_cost() == min).count() == 1;
            if unique {
                assert_eq!(argmin, vec![bos.unwrap()], "{case}");
                assert_eq!(tokens, singles[argmin[0]].0, "{case}");
                best_bos_not_first |= argmin[0] != 0;
            } else {
                assert!(argmin.contains(&bos.unwrap()), "{case}");
            }
            assert_best_is_brute_min(&lattice, &brute, &case);

            // Whether the BOS costs decide: no context that would win
            // without them wins with them.
            let free: Vec<i64> = singles
                .iter()
                .zip(bos_list)
                .map(|((_, cost), context)| cost - context.cost as i64)
                .collect();
            let free_min = free.iter().copied().min().unwrap();
            bos_cost_decides |= (0..free.len())
                .filter(|&i| free[i] == free_min)
                .all(|i| singles[i].1 != min);
        });
        assert!(segmentation_depends_on_bos);
        assert!(best_bos_not_first);
        assert!(bos_cost_decides);
    }

    /// The fields of a lattice the forward pass writes, for comparing two
    /// lattices slot by slot.
    #[allow(clippy::type_complexity)]
    fn lattice_snapshot(
        lattice: &Lattice,
    ) -> (
        Vec<Vec<(u32, i32, u32, u16, u16, i16, u16, u8, u8)>>,
        Vec<Vec<(u32, u32, u32, i32)>>,
        usize,
        Vec<i32>,
    ) {
        let slots = 0..=lattice.char_len();
        let edges = slots
            .clone()
            .map(|slot| {
                lattice
                    .edges_at_char(slot)
                    .iter()
                    .map(|e| {
                        (
                            e.word_id,
                            e.path_cost,
                            e.left_index,
                            e.left_id,
                            e.right_id,
                            e.word_cost,
                            e.start_char,
                            e.flags,
                            e.ws_tail,
                        )
                    })
                    .collect()
            })
            .collect();
        let paths = slots
            .map(|slot| {
                lattice
                    .paths_at_char(slot)
                    .iter()
                    .map(|p| (p.edge_index, p.left_pos, p.left_index, p.cost))
                    .collect()
            })
            .collect();
        (
            edges,
            paths,
            lattice.final_edge_count,
            lattice.exit_penalties.clone(),
        )
    }

    /// #1096: the default (no BOS context) builds exactly the lattice of
    /// one explicit context with right id 0 and cost 0, edge by edge and
    /// transition by transition, in the 1-best and the N-best lattice, with
    /// both fixtures; its best path is the cheapest path by definition, and
    /// its BOS index is 0 everywhere.
    #[test]
    fn test_default_bos_is_single_bos_zero() {
        let fixture = CtxFixture::new();
        let explicit = [bos_ctx(0, 0)];
        for mode in [Mode::Normal, ctx_decompose()] {
            for (text, skip) in CTX_TEXTS
                .iter()
                .flat_map(|&text| [(text, false), (text, true)])
                .chain(CTX_SPACED_TEXTS.iter().map(|&text| (text, true)))
            {
                let case = format!("{text:?} {mode:?} skip={skip}");
                let default = fixture.lattice(text, &mode, skip, &[]);
                let single = fixture.lattice(text, &mode, skip, &explicit);
                assert_eq!(
                    lattice_snapshot(&default),
                    lattice_snapshot(&single),
                    "{case}"
                );
                assert_eq!(best_path(&default).1, Some(0), "{case}");
                assert_best_is_brute_min(&default, &brute_paths(text, &[], &mode), &case);

                let default = fixture.nbest_lattice(text, &mode, skip, &[]);
                let single = fixture.nbest_lattice(text, &mode, skip, &explicit);
                assert_eq!(
                    lattice_snapshot(&default),
                    lattice_snapshot(&single),
                    "{case}"
                );
                let mut generator = NBestGenerator::new(&default);
                while let Some((_, bos)) = generator.next_with_bos() {
                    assert_eq!(bos, 0, "{case}");
                }
            }
        }

        let ws = WsFixture::new();
        for text in ["ab cd", "  ab  cd ", "ab \t", "", "   "] {
            for mode in [Mode::Normal, ctx_decompose()] {
                for skip in [false, true] {
                    let build = |bos: &[BosContext], nbest: bool| {
                        let mut options = ws.options(&mode, skip);
                        options.bos = bos;
                        let mut lattice = Lattice::default();
                        if nbest {
                            lattice.set_text_nbest_with_options(
                                &ws.dict,
                                &None,
                                &ws.char_definition,
                                &ws.unknown_dictionary,
                                &ws.cost_matrix,
                                text,
                                &options,
                            );
                        } else {
                            lattice.set_text_with_options(
                                &ws.dict,
                                &None,
                                &ws.char_definition,
                                &ws.unknown_dictionary,
                                &ws.cost_matrix,
                                text,
                                &options,
                            );
                        }
                        lattice
                    };
                    for nbest in [false, true] {
                        assert_eq!(
                            lattice_snapshot(&build(&[], nbest)),
                            lattice_snapshot(&build(&explicit, nbest)),
                            "{text:?} {mode:?} skip={skip} nbest={nbest}"
                        );
                    }
                }
            }
        }
    }

    /// #1096: `exits_into` returns, per right id, the cheapest path by
    /// definition among the paths ending with that right id, without the
    /// EOS connection and with the last word's Decompose penalty; in the
    /// order the right ids first appear among the final edges, each with
    /// the last of the cheapest final edges (#1135). `exit_tokens_offset_into`
    /// returns such a path and its BOS index. The best complete path is the
    /// exit minimizing cost plus EOS connection, through the same edge. The
    /// N-best lattice has the same exits.
    #[test]
    fn test_exits_match_brute_force() {
        let fixture = CtxFixture::new();
        for_each_ctx_case(|text, mode, skip, bos_list| {
            let case = format!("{text:?} {mode:?} skip={skip} {bos_list:?}");
            let lattice = fixture.lattice(text, mode, skip, bos_list);
            let brute = brute_paths(text, bos_list, mode);
            let exits = exits_of(&lattice);

            // Per right id, the cheapest path by definition.
            let mut expected: BTreeMap<u16, i64> = BTreeMap::new();
            for path in &brute {
                let cost = expected.entry(path.last_right).or_insert(i64::MAX);
                *cost = (*cost).min(path.exit_cost);
            }
            let actual: BTreeMap<u16, i64> = exits
                .iter()
                .map(|exit| (exit.right_id(), exit.cost() as i64))
                .collect();
            assert_eq!(actual.len(), exits.len(), "{case}: duplicate right id");
            assert_eq!(actual, expected, "{case}");

            // Order and edge choice.
            let finals = lattice.final_edges();
            let mut first_seen: Vec<u16> = Vec::new();
            for edge in finals {
                if !first_seen.contains(&edge.right_id) {
                    first_seen.push(edge.right_id);
                }
            }
            let order: Vec<u16> = exits.iter().map(LatticeExit::right_id).collect();
            assert_eq!(order, first_seen, "{case}");
            for exit in &exits {
                for (i, edge) in finals.iter().enumerate() {
                    if edge.right_id != exit.right_id {
                        continue;
                    }
                    let cost = edge.path_cost + lattice.exit_penalty(i);
                    assert!(cost >= exit.cost, "{case}");
                    assert!(
                        i <= exit.edge as usize || cost > exit.cost,
                        "{case}: a tie must keep the last edge"
                    );
                }
                assert_eq!(finals[exit.edge as usize].right_id, exit.right_id, "{case}");
            }

            // The exits' paths.
            let mut tokens = vec![(9, 9, WordId::default())];
            for exit in &exits {
                let bos = lattice.exit_tokens_offset_into(exit, &mut tokens);
                assert!(
                    brute.iter().any(|path| path.bos == bos
                        && path.tokens == tokens
                        && path.last_right == exit.right_id()
                        && path.exit_cost == exit.cost() as i64),
                    "{case}: exit {exit:?} has path {tokens:?} from BOS {bos}"
                );
            }

            // The best complete path goes through an exit.
            let eos = lattice.edges_at_char(lattice.char_len()).last().unwrap();
            let eos_total = |exit: &LatticeExit| {
                exit.cost() as i64 + CTX_CONN[exit.right_id() as usize][0] as i64
            };
            let min = exits.iter().map(eos_total).min().unwrap();
            assert_eq!(eos.path_cost as i64, min, "{case}");
            let through = exits
                .iter()
                .filter(|exit| eos_total(exit) == min)
                .max_by_key(|exit| exit.edge)
                .unwrap();
            assert_eq!(eos.left_index, through.edge, "{case}");
            let (best_tokens, best_bos, _) = best_path(&lattice);
            let bos = lattice.exit_tokens_offset_into(through, &mut tokens);
            assert_eq!(
                (best_tokens, best_bos),
                (tokens.clone(), Some(bos)),
                "{case}"
            );

            let nbest = fixture.nbest_lattice(text, mode, skip, bos_list);
            assert_eq!(exits_of(&nbest), exits, "{case}");
        });
    }

    /// #1096: `exits_into` and the EOS connection break a tie between two
    /// final edges the same way: `d` has two identical entries, stored last
    /// row first, and both keep the last edge, the first row (#1135).
    #[test]
    fn test_exit_tie_keeps_first_row() {
        let fixture = CtxFixture::new();
        let lattice = fixture.lattice("ad", &Mode::Normal, false, &[]);
        let finals = lattice.final_edges();
        assert_eq!(
            finals.iter().map(Edge::word_id).collect::<Vec<_>>(),
            [sys(13), sys(12)]
        );
        assert_eq!(finals[0].path_cost, finals[1].path_cost);
        let exits = exits_of(&lattice);
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].edge, 1);
        let eos = lattice.edges_at_char(2).last().unwrap();
        assert_eq!(eos.left_index, 1);
        let mut tokens = Vec::new();
        lattice.exit_tokens_offset_into(&exits[0], &mut tokens);
        assert_eq!(tokens, lattice.tokens_offset());
        assert_eq!(tokens.last().map(|token| token.2), Some(sys(12)));
    }

    /// #1096: leading skipped whitespace carries the BOS edges over in
    /// their order (reverse context order, #1135), with their costs and
    /// `ws_tail` 0, and the BOS index still names the context (here not the
    /// first one).
    #[test]
    fn test_leading_whitespace_keeps_bos_indices() {
        let fixture = CtxFixture::new();
        let bos_list = [
            bos_ctx(0, 0),
            bos_ctx(1, 50),
            bos_ctx(2, 10),
            bos_ctx(3, 300),
        ];
        for mode in [Mode::Normal, ctx_decompose()] {
            for text in ["  bc", " \tbc"] {
                let lattice = fixture.lattice(text, &mode, true, &bos_list);
                let carried: Vec<_> = lattice.edges_at_char(2)[..4]
                    .iter()
                    .map(|e| (e.left_index, e.right_id, e.path_cost, e.ws_tail))
                    .collect();
                assert_eq!(
                    carried,
                    vec![
                        (u32::MAX, 3, 300, 0),
                        (u32::MAX, 2, 10, 0),
                        (u32::MAX, 1, 50, 0),
                        (u32::MAX, 0, 0, 0),
                    ]
                );
                let brute = brute_paths(text, &bos_list, &mode);
                let min = brute.iter().map(BrutePath::eos_cost).min().unwrap();
                let winners: Vec<_> = brute.iter().filter(|p| p.eos_cost() == min).collect();
                assert_eq!(winners.len(), 1, "{text:?} {mode:?}");
                assert_ne!(winners[0].bos, 0, "{text:?} {mode:?}");
                let (tokens, bos, cost) = best_path(&lattice);
                assert_eq!(
                    (tokens, bos, cost),
                    (winners[0].tokens.clone(), Some(winners[0].bos), min)
                );

                let nbest = fixture.nbest_lattice(text, &mode, true, &bos_list);
                let mut generator = NBestGenerator::new(&nbest);
                let ((tokens, cost), bos) = generator.next_with_bos().unwrap();
                assert_eq!(
                    (tokens, cost, bos),
                    (winners[0].tokens.clone(), min, winners[0].bos)
                );
            }
        }
    }

    /// #1096: in a sentence without words (only skipped whitespace, or
    /// empty) the BOS edges are the final edges: they are the exits, in
    /// their order (reverse context order, #1135) and with their own costs,
    /// their paths are empty and lead to
    /// themselves, the best path is the empty one from the BOS edge
    /// cheapest with its EOS connection, and the N-best from an exit is one
    /// empty path.
    #[test]
    fn test_sentence_without_words_exits_are_bos_edges() {
        let fixture = CtxFixture::new();
        let bos_list = [
            bos_ctx(0, 0),
            bos_ctx(1, 50),
            bos_ctx(2, 10),
            bos_ctx(3, 300),
        ];
        for mode in [Mode::Normal, ctx_decompose()] {
            for (text, skip) in [("   ", true), (" \t", true), ("", false), ("", true)] {
                let case = format!("{text:?} {mode:?} skip={skip}");
                let lattice = fixture.lattice(text, &mode, skip, &bos_list);
                let exits = exits_of(&lattice);
                let summary: Vec<_> = exits
                    .iter()
                    .map(|exit| (exit.right_id(), exit.cost(), exit.edge))
                    .collect();
                assert_eq!(
                    summary,
                    vec![(3, 300, 0), (2, 10, 1), (1, 50, 2), (0, 0, 3)],
                    "{case}"
                );
                // Context `i` has right id `i` here.
                let mut tokens = vec![(9, 9, WordId::default())];
                for exit in &exits {
                    assert_eq!(
                        lattice.exit_tokens_offset_into(exit, &mut tokens),
                        exit.right_id() as usize
                    );
                    assert!(tokens.is_empty(), "{case}");
                }
                // EOS from right ids 0..3 costs 0, 500, 40 and 300.
                assert_eq!(best_path(&lattice), (Vec::new(), Some(0), 0), "{case}");
                let shifted = [bos_ctx(1, 50), bos_ctx(2, 10)];
                let lattice = fixture.lattice(text, &mode, skip, &shifted);
                assert_eq!(best_path(&lattice), (Vec::new(), Some(1), 50), "{case}");

                let nbest = fixture.nbest_lattice(text, &mode, skip, &bos_list);
                for exit in &exits_of(&nbest) {
                    let mut generator = NBestGenerator::from_exit(&nbest, exit);
                    assert_eq!(
                        generator.next_with_bos(),
                        Some(((Vec::new(), exit.cost() as i64), exit.right_id() as usize)),
                        "{case}"
                    );
                    assert_eq!(generator.next_with_bos(), None, "{case}");
                }
            }
        }

        // The default BOS edge alone is the one exit.
        let lattice = fixture.lattice("  ", &Mode::Normal, true, &[]);
        assert_eq!(
            exits_of(&lattice),
            vec![LatticeExit {
                right_id: 0,
                cost: 0,
                edge: 0
            }]
        );
    }

    /// #1096: the path of an exit ends where its last word's surface ends:
    /// `b ` keeps its space, the skipped whitespace after a word is not
    /// part of it (#1108).
    #[test]
    fn test_exit_tokens_end_with_entry_whitespace() {
        let fixture = CtxFixture::new();
        for text in ["ab  ", "ab \t", "ab "] {
            let lattice = fixture.lattice(text, &Mode::Normal, true, &[]);
            let mut tokens = Vec::new();
            let ends: BTreeMap<u16, usize> = exits_of(&lattice)
                .iter()
                .map(|exit| {
                    lattice.exit_tokens_offset_into(exit, &mut tokens);
                    (exit.right_id(), tokens.last().unwrap().1)
                })
                .collect();
            // `b ` (right id 3) ends after its space; `b`/`ab` (right ids 1
            // and 2) end before the whitespace.
            assert_eq!(ends, BTreeMap::from([(1, 2), (2, 2), (3, 3)]), "{text:?}");
        }
    }

    /// #1096: in Decompose mode an exit's cost includes the last word's
    /// length penalty, the one it pays whatever follows it: in `ab  ` the
    /// exit with right id 3 can only end with `b ` (two characters, 500),
    /// and costs that much more than in Normal mode; the exits ending with
    /// one-character words cost the same in both modes.
    #[test]
    fn test_exit_cost_includes_decompose_penalty() {
        let fixture = CtxFixture::new();
        let text = "ab  ";
        let decomposed = fixture.lattice(text, &ctx_decompose(), true, &[]);
        let normal = fixture.lattice(text, &Mode::Normal, true, &[]);
        let costs = |lattice: &Lattice| -> BTreeMap<u16, i32> {
            exits_of(lattice)
                .iter()
                .map(|exit| (exit.right_id(), exit.cost()))
                .collect()
        };
        let (decomposed_costs, normal_costs) = (costs(&decomposed), costs(&normal));
        assert_eq!(decomposed_costs[&3] - normal_costs[&3], 500);
        assert_eq!(decomposed_costs[&1], normal_costs[&1]);
        let exit = exits_of(&decomposed)
            .into_iter()
            .find(|exit| exit.right_id() == 3)
            .unwrap();
        let mut tokens = Vec::new();
        decomposed.exit_tokens_offset_into(&exit, &mut tokens);
        assert_eq!(tokens.last(), Some(&(1, 3, sys(11))));
        assert_eq!(decomposed.exit_penalty(exit.edge as usize), 500);
        // Normal mode stores no penalties.
        assert!(normal.exit_penalties.is_empty());
        assert_eq!(normal.exit_penalty(0), 0);
    }

    /// #1096: a BOS cost is clamped to `0..=PATH_COST_CLAMP`, so a huge cost
    /// cannot overflow the path costs after it and a negative one counts as
    /// 0.
    #[test]
    fn test_bos_cost_is_clamped() {
        let fixture = CtxFixture::new();
        let lattice = fixture.lattice(
            "abc",
            &Mode::Normal,
            false,
            &[bos_ctx(1, i32::MAX), bos_ctx(2, -7)],
        );
        let costs: Vec<i32> = lattice
            .edges_at_char(0)
            .iter()
            .map(|e| e.path_cost)
            .collect();
        // In reverse context order (see `push_bos_edges`).
        assert_eq!(costs, vec![0, PATH_COST_CLAMP]);
        assert_eq!(best_path(&lattice).1, Some(1));

        for nbest in [false, true] {
            let huge = [bos_ctx(1, i32::MAX)];
            let lattice = if nbest {
                fixture.nbest_lattice("abc", &ctx_decompose(), false, &huge)
            } else {
                fixture.lattice("abc", &ctx_decompose(), false, &huge)
            };
            let (tokens, bos, cost) = best_path(&lattice);
            assert!(!tokens.is_empty());
            assert_eq!(bos, Some(0));
            assert!(cost >= PATH_COST_CLAMP as i64);
            let exits = exits_of(&lattice);
            assert!(!exits.is_empty());
            assert!(exits.iter().all(|exit| exit.cost() >= PATH_COST_CLAMP));
            if nbest {
                let mut generator = NBestGenerator::from_exit(&lattice, &exits[0]);
                assert!(generator.next_with_bos().is_some());
            }
        }
    }

    /// #1096: `NBestGenerator::from_exit` yields exactly the paths ending
    /// with the exit's right id, by definition (without EOS, with the last
    /// word's Decompose penalty), each once with its BOS index, in
    /// ascending order of cost, the BOS cost included; the first costs the
    /// exit's cost. Over all exits, every path appears once.
    #[test]
    fn test_from_exit_matches_brute_force() {
        let fixture = CtxFixture::new();
        for_each_ctx_case(|text, mode, skip, bos_list| {
            let case = format!("{text:?} {mode:?} skip={skip} {bos_list:?}");
            let lattice = fixture.nbest_lattice(text, mode, skip, bos_list);
            let brute = brute_paths(text, bos_list, mode);
            let mut covered = 0;
            for exit in exits_of(&lattice) {
                let mut generator = NBestGenerator::from_exit(&lattice, &exit);
                let mut actual = Vec::new();
                while let Some(((tokens, cost), bos)) = generator.next_with_bos() {
                    actual.push((cost, token_keys(&tokens), bos));
                }
                assert!(
                    actual.windows(2).all(|pair| pair[0].0 <= pair[1].0),
                    "{case}: not in cost order"
                );
                assert_eq!(
                    actual.first().map(|path| path.0),
                    Some(exit.cost() as i64),
                    "{case}"
                );
                let mut expected: Vec<_> = brute
                    .iter()
                    .filter(|path| path.last_right == exit.right_id())
                    .map(|path| (path.exit_cost, token_keys(&path.tokens), path.bos))
                    .collect();
                covered += expected.len();
                actual.sort();
                expected.sort();
                assert_eq!(actual, expected, "{case}: right id {}", exit.right_id());
            }
            assert_eq!(covered, brute.len(), "{case}");
        });
    }

    /// #1096: from EOS, the N-best paths of a lattice with several BOS
    /// edges are exactly the paths by definition (EOS included), each once
    /// with its BOS index, in ascending order of cost, the BOS cost
    /// included; the first one is the 1-best path, also when other paths
    /// cost the same (#1131).
    #[test]
    fn test_next_with_bos_matches_brute_force() {
        let fixture = CtxFixture::new();
        let mut tie_for_first = false;
        for_each_ctx_case(|text, mode, skip, bos_list| {
            let case = format!("{text:?} {mode:?} skip={skip} {bos_list:?}");
            let lattice = fixture.nbest_lattice(text, mode, skip, bos_list);
            let mut generator = NBestGenerator::new(&lattice);
            let mut actual = Vec::new();
            while let Some(((tokens, cost), bos)) = generator.next_with_bos() {
                actual.push((cost, token_keys(&tokens), bos));
            }
            assert!(
                actual.windows(2).all(|pair| pair[0].0 <= pair[1].0),
                "{case}: not in cost order"
            );
            // The brute force cannot tell which of several cheapest paths is
            // the 1-best one; the 1-best lattice does.
            let (tokens, bos, cost) = best_path(&fixture.lattice(text, mode, skip, bos_list));
            assert_eq!(
                actual.first(),
                Some(&(cost, token_keys(&tokens), bos.unwrap())),
                "{case}"
            );
            let mut expected: Vec<_> = brute_paths(text, bos_list, mode)
                .iter()
                .map(|path| (path.eos_cost(), token_keys(&path.tokens), path.bos))
                .collect();
            expected.sort();
            tie_for_first |= expected.len() > 1 && expected[0].0 == expected[1].0;
            actual.sort();
            assert_eq!(actual, expected, "{case}");
        });
        assert!(tie_for_first, "no case has several cheapest paths");
    }

    /// Builds a user dictionary that holds `entries`, in CSV order, under
    /// the one surface `surface`, laid out as the user dictionary builder
    /// does (word ids from 0, no details).
    fn one_surface_user_dictionary(surface: &str, entries: &[WordEntry]) -> UserPrefixDictionary {
        let keyset: Vec<(&[u8], u32)> = vec![(surface.as_bytes(), entries.len() as u32)];
        let da = DoubleArrayAhoCorasickBuilder::new()
            .build_with_values(keyset)
            .unwrap();
        let mut vals = Vec::new();
        for entry in entries {
            entry.serialize(&mut vals).unwrap();
        }
        UserPrefixDictionary::load(da.serialize(), vals, Vec::<u8>::new(), Vec::<u8>::new())
            .unwrap()
    }

    /// #1131: of the entries that tie (same surface, context ids and cost),
    /// the first CSV row wins, as in MeCab: in the 1-best and the N-best
    /// lattice, in both modes, in the system and in the user dictionary. A
    /// user entry wins a tie with a system entry (MeCab would pick the
    /// system entry, as it looks the system dictionary up first).
    #[test]
    fn test_tied_entries_resolve_to_first_row() {
        let char_definition = test_char_definition();
        let unknown_dictionary = test_unknown_dictionary();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
        let entry = |lex_type, id| WordEntry::new(WordId::new(lex_type, id), 100, 0, 0);
        let mut map = BTreeMap::new();
        map.insert(
            "ab".to_string(),
            vec![entry(LexType::System, 0), entry(LexType::System, 1)],
        );
        let system = PrefixDictionary::from_word_entry_map(&map).unwrap();
        let no_system = PrefixDictionary::from_word_entry_map(&BTreeMap::new()).unwrap();
        let user =
            one_surface_user_dictionary("ab", &[entry(LexType::User, 0), entry(LexType::User, 1)]);
        let first_system = WordId::new(LexType::System, 0);
        let first_user = WordId::new(LexType::User, 0);
        let cases = [
            ("system", &system, None, first_system, 2),
            ("user", &no_system, Some(&user), first_user, 2),
            ("system and user", &system, Some(&user), first_user, 4),
        ];
        let decompose = Mode::Decompose(Penalty::default());
        for (name, dict, user_dict, expected, tied) in cases {
            for mode in [&Mode::Normal, &decompose] {
                let case = format!("{name} {mode:?}");
                let options = LatticeOptions::new(mode);
                let mut lattice = Lattice::default();
                lattice.set_text_with_options(
                    dict,
                    &user_dict,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    "ab",
                    &options,
                );
                assert_eq!(lattice.tokens_offset(), vec![(0, 2, expected)], "{case}");

                lattice.set_text_nbest_with_options(
                    dict,
                    &user_dict,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    "ab",
                    &options,
                );
                let paths = lattice.nbest_tokens_offset(usize::MAX, false, None);
                assert_eq!(paths.len(), tied, "{case}");
                assert!(paths.iter().all(|(_, cost)| *cost == 100), "{case}");
                assert_eq!(paths[0].0, vec![(0, 2, expected)], "{case}");
            }
        }
    }

    /// #1131: when many paths tie for the lowest cost, the first N-best
    /// path is still the 1-best path, from EOS and from the exit, in both
    /// modes. `a` has two tied entries and a costlier third one, and `aa`
    /// costs as much as two `a` (its Decompose length penalty included), so
    /// 5, 12 and 29 paths tie in `aa`, `aaa` and `aaaa`, with tied words at
    /// several positions and of different lengths. A queue ordered by cost
    /// alone popped another tied path first in `aaa` and `aaaa`.
    #[test]
    fn test_nbest_first_path_is_best_path_on_ties() {
        let char_definition = test_char_definition();
        let unknown_dictionary = test_unknown_dictionary();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
        let entry = |id, cost| WordEntry::new(sys(id), cost, 0, 0);
        // `aa` pays 100 for its second character in Decompose mode.
        let decompose = Mode::Decompose(Penalty {
            kanji_penalty_length_threshold: 1,
            kanji_penalty_length_penalty: 100,
            other_penalty_length_threshold: 1,
            other_penalty_length_penalty: 100,
        });
        for (mode, aa_cost) in [(Mode::Normal, 200), (decompose, 100)] {
            let mut map = BTreeMap::new();
            map.insert(
                "a".to_string(),
                vec![entry(0, 100), entry(1, 100), entry(2, 150)],
            );
            map.insert("aa".to_string(), vec![entry(3, aa_cost)]);
            let dict = PrefixDictionary::from_word_entry_map(&map).unwrap();
            let options = LatticeOptions::new(&mode);
            for (text, tied) in [("aa", 5), ("aaa", 12), ("aaaa", 29)] {
                let case = format!("{text} {mode:?}");
                let mut lattice = Lattice::default();
                lattice.set_text_with_options(
                    &dict,
                    &None,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    text,
                    &options,
                );
                let best = (lattice.tokens_offset(), eos_cost(&lattice));

                lattice.set_text_nbest_with_options(
                    &dict,
                    &None,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    text,
                    &options,
                );
                let paths = lattice.nbest_tokens_offset(usize::MAX, false, None);
                assert!(
                    paths.windows(2).all(|pair| pair[0].1 <= pair[1].1),
                    "{case}: not in cost order"
                );
                let cheapest = paths.iter().filter(|(_, cost)| *cost == best.1).count();
                assert_eq!(cheapest, tied, "{case}");
                assert!(paths.len() > tied, "{case}");
                assert_eq!(paths[0], best, "{case}");

                let exits = exits_of(&lattice);
                assert_eq!(exits.len(), 1, "{case}");
                let mut tokens = Vec::new();
                lattice.exit_tokens_offset_into(&exits[0], &mut tokens);
                let mut generator = NBestGenerator::from_exit(&lattice, &exits[0]);
                assert_eq!(
                    generator.next(),
                    Some((tokens, exits[0].cost() as i64)),
                    "{case}: from the exit"
                );
            }
        }
    }

    /// Builds a system dictionary with one entry per surface, word ids in
    /// the order of `surfaces`, each costing 100 with context ids 0.
    fn hundred_each(surfaces: &[&str]) -> PrefixDictionary {
        let mut map = BTreeMap::new();
        for (id, surface) in surfaces.iter().enumerate() {
            map.insert(
                surface.to_string(),
                vec![WordEntry::new(sys(id as u32), 100, 0, 0)],
            );
        }
        PrefixDictionary::from_word_entry_map(&map).unwrap()
    }

    /// #1135: of two paths of equal cost that end at the same position, the
    /// one whose last word starts later wins, as in MeCab: `ab|c` over
    /// `a|bc`, whether EOS (`abc`) or the next word (`abcd`) chooses between
    /// them, in the 1-best and the N-best lattice, in both modes, and for
    /// the exit and the N-best search from it. Every word costs 100 and
    /// every connection 0, so the two paths tie.
    #[test]
    fn test_cross_start_tie_keeps_later_start() {
        let char_definition = test_char_definition();
        let unknown_dictionary = test_unknown_dictionary();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
        let dict = hundred_each(&["a", "ab", "bc", "c", "d"]);
        let decompose = Mode::Decompose(Penalty::default());
        let cases = [
            ("abc", vec![(0, 2, sys(1)), (2, 3, sys(3))], 200),
            (
                "abcd",
                vec![(0, 2, sys(1)), (2, 3, sys(3)), (3, 4, sys(4))],
                300,
            ),
        ];
        for mode in [&Mode::Normal, &decompose] {
            for (text, expected, cost) in &cases {
                let case = format!("{text} {mode:?}");
                let options = LatticeOptions::new(mode);
                let mut lattice = Lattice::default();
                lattice.set_text_with_options(
                    &dict,
                    &None,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    text,
                    &options,
                );
                assert_eq!(&lattice.tokens_offset(), expected, "{case}");
                assert_eq!(eos_cost(&lattice), *cost, "{case}");
                let exits = exits_of(&lattice);
                assert_eq!(exits.len(), 1, "{case}");
                let mut tokens = Vec::new();
                lattice.exit_tokens_offset_into(&exits[0], &mut tokens);
                assert_eq!(&tokens, expected, "{case}: the exit");

                lattice.set_text_nbest_with_options(
                    &dict,
                    &None,
                    &char_definition,
                    &unknown_dictionary,
                    &cost_matrix,
                    text,
                    &options,
                );
                let paths = lattice.nbest_tokens_offset(usize::MAX, false, None);
                assert_eq!(paths.len(), 2, "{case}");
                assert_eq!(paths[0], (expected.clone(), *cost), "{case}");
                assert_eq!(paths[1].1, *cost, "{case}: the other path ties");
                let exits = exits_of(&lattice);
                let mut generator = NBestGenerator::from_exit(&lattice, &exits[0]);
                assert_eq!(
                    generator.next().map(|(tokens, _)| tokens).as_ref(),
                    Some(expected),
                    "{case}: from the exit"
                );
            }
        }
    }

    /// #1135: with whitespace skipped, of two left words of equal cost the
    /// one that ends later wins, as in MeCab, which looks the next word up
    /// from every position a word ends at and keeps the node looked up from
    /// the later one: in `a b`, `a ` (an entry that ends with the space)
    /// wins over `a`, which is carried over the space, in the 1-best and the
    /// N-best lattice.
    #[test]
    fn test_cross_end_tie_keeps_later_end_across_skipped_whitespace() {
        let char_definition = test_char_definition();
        let unknown_dictionary = test_unknown_dictionary();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
        let classifier = WhitespaceClassifier::new(&char_definition).unwrap();
        let dict = hundred_each(&["a", "a ", "b"]);
        let expected = vec![(0, 2, sys(1)), (2, 3, sys(2))];
        for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
            let mut options = LatticeOptions::new(&mode);
            options.skip_whitespace = Some(&classifier);
            let mut lattice = Lattice::default();
            lattice.set_text_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                "a b",
                &options,
            );
            assert_eq!(lattice.tokens_offset(), expected, "{mode:?}");
            assert_eq!(eos_cost(&lattice), 200, "{mode:?}");

            lattice.set_text_nbest_with_options(
                &dict,
                &None,
                &char_definition,
                &unknown_dictionary,
                &cost_matrix,
                "a b",
                &options,
            );
            let paths = lattice.nbest_tokens_offset(usize::MAX, false, None);
            assert_eq!(paths.len(), 2, "{mode:?}");
            assert_eq!(paths[0], (expected.clone(), 200), "{mode:?}");
            assert_eq!(paths[1].1, 200, "{mode:?}: the other path ties");
        }
    }

    /// #1135: the relaxations keep the last of equal-cost left edges with
    /// `<=`, but a transition whose cost saturates at `i32::MAX` is still
    /// none, as with the former strict `<`: with a Decompose penalty of
    /// `i32::MAX` on every word, nothing connects after `a`, so `ab` has no
    /// complete path, in the 1-best and the N-best lattice.
    #[test]
    fn test_saturated_transition_does_not_connect() {
        let char_definition = test_char_definition();
        let unknown_dictionary = test_unknown_dictionary();
        let cost_matrix = ConnectionCostMatrix::load(vec![0xff, 0xff, 1, 0, 1, 0, 0, 0]).unwrap();
        let dict = hundred_each(&["a", "b"]);
        let mode = Mode::Decompose(Penalty {
            kanji_penalty_length_threshold: 0,
            kanji_penalty_length_penalty: i32::MAX,
            other_penalty_length_threshold: 0,
            other_penalty_length_penalty: i32::MAX,
        });
        let options = LatticeOptions::new(&mode);
        let mut lattice = Lattice::default();
        lattice.set_text_with_options(
            &dict,
            &None,
            &char_definition,
            &unknown_dictionary,
            &cost_matrix,
            "ab",
            &options,
        );
        assert!(lattice.edges_at_char(2).is_empty());
        let mut offsets = Vec::new();
        assert_eq!(lattice.tokens_offset_into(&mut offsets), None);

        lattice.set_text_nbest_with_options(
            &dict,
            &None,
            &char_definition,
            &unknown_dictionary,
            &cost_matrix,
            "ab",
            &options,
        );
        assert!(lattice.edges_at_char(2).is_empty());
        assert!(lattice.nbest_tokens_offset(10, false, None).is_empty());
    }
}
