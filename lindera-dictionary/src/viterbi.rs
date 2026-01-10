use std::io;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
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
    pub id: u32,
    pub is_system: bool,
    pub lex_type: LexType,
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

    pub fn is_unknown(&self) -> bool {
        self.id == u32::MAX || matches!(self.lex_type, LexType::Unknown)
    }

    pub fn is_system(&self) -> bool {
        self.is_system
    }

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
    pub word_id: WordId,
    pub word_cost: i16,
    pub left_id: u16,
    pub right_id: u16,
}

impl WordEntry {
    pub const SERIALIZED_LEN: usize = 10;

    pub fn left_id(&self) -> u32 {
        self.left_id as u32
    }

    pub fn right_id(&self) -> u32 {
        self.right_id as u32
    }

    pub fn serialize<W: io::Write>(&self, wtr: &mut W) -> io::Result<()> {
        wtr.write_u32::<LittleEndian>(self.word_id.id)?;
        wtr.write_i16::<LittleEndian>(self.word_cost)?;
        wtr.write_u16::<LittleEndian>(self.left_id)?;
        wtr.write_u16::<LittleEndian>(self.right_id)?;
        Ok(())
    }

    pub fn deserialize(data: &[u8], is_system_entry: bool) -> WordEntry {
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

#[derive(Clone, Copy, Debug, Default)]
pub enum EdgeType {
    #[default]
    KNOWN,
    UNKNOWN,
    USER,
    INSERTED,
}

#[derive(Default, Clone, Debug)]
pub struct Edge {
    pub edge_type: EdgeType,
    pub word_entry: WordEntry,

    pub path_cost: i32,
    pub left_index: u32, // Index in the previous position's vector OR absolute index in buffer

    pub start_index: u32,
    pub stop_index: u32,

    pub kanji_only: bool,

    // Index of the next edge that ends at the same position (Linked list)
    pub next: Option<usize>,
}

impl Edge {
    pub fn num_chars(&self) -> usize {
        (self.stop_index - self.start_index) as usize / 3
    }
}

#[derive(Clone, Default)]
pub struct Lattice {
    capacity: usize,
    ends_at_indices: Vec<Option<usize>>, // Head index of the linked list for each position

    // SoA (Structure of Arrays) for Edges
    // Hot fields (accessed in forward Viterbi)
    edge_path_costs: Vec<i32>,
    edge_right_ids: Vec<u16>,              // WordEntry.right_id
    edge_next_indices: Vec<Option<usize>>, // Linked list next ptr
    edge_left_ids: Vec<u16>,               // WordEntry.left_id

    // Cold fields (accessed in backtracking / result construction)
    edge_left_indices: Vec<u32>,
    edge_word_costs: Vec<i16>,  // WordEntry.word_cost
    edge_word_ids: Vec<WordId>, // WordEntry.word_id
    edge_start_indices: Vec<u32>,
    edge_stop_indices: Vec<u32>,
    edge_types: Vec<EdgeType>,
    edge_kanji_onlys: Vec<bool>,

    char_info_buffer: Vec<CharData>,
    categories_buffer: Vec<CategoryId>,
    char_category_cache: Vec<Vec<CategoryId>>,
    // (prefix_len, offset, len, is_system)
    indices_buffer: Vec<(usize, usize, usize, bool)>,
    left_path_costs: Vec<i32>,
    left_right_ids: Vec<u32>,
}

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
    fn create_edge(
        edge_type: EdgeType,
        word_entry: WordEntry,
        start: usize,
        stop: usize,
        kanji_only: bool,
    ) -> Edge {
        Edge {
            edge_type,
            word_entry,
            left_index: u32::MAX,
            start_index: start as u32,
            stop_index: stop as u32,
            path_cost: i32::MAX,
            kanji_only,
            next: None,
        }
    }

    pub fn clear(&mut self) {
        // SoA clear
        self.edge_path_costs.clear();
        self.edge_right_ids.clear();
        self.edge_next_indices.clear();
        self.edge_left_ids.clear();

        self.edge_left_indices.clear();
        self.edge_word_costs.clear();
        self.edge_word_ids.clear();
        self.edge_start_indices.clear();
        self.edge_stop_indices.clear();
        self.edge_types.clear();
        self.edge_kanji_onlys.clear();

        for head in &mut self.ends_at_indices {
            *head = None;
        }
        self.char_info_buffer.clear();
        self.categories_buffer.clear();
        self.indices_buffer.clear();
        self.left_path_costs.clear();
        self.left_right_ids.clear();
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
        if self.capacity <= text_len {
            self.capacity = text_len;
            self.ends_at_indices.resize(text_len + 1, None);
        }
    }

    #[inline(never)]
    // Forward Viterbi implementation:
    // Constructs the lattice and calculates the path costs simultaneously.
    // This improves performance by avoiding a separate lattice traversal pass.
    pub fn set_text(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&PrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        cost_matrix: &ConnectionCostMatrix,
        text: &str,
        search_mode: &Mode,
    ) {
        let len = text.len();
        self.set_capacity(len);

        // Pre-calculate character information for the text
        self.char_info_buffer.clear();
        self.categories_buffer.clear();

        if self.char_category_cache.is_empty() {
            self.char_category_cache.resize(256, Vec::new());
        }

        for (byte_offset, c) in text.char_indices() {
            let categories_start = self.categories_buffer.len() as u32;

            if (c as u32) < 256 {
                let cached = &mut self.char_category_cache[c as usize];
                if cached.is_empty() {
                    let cats = char_definitions.lookup_categories(c);
                    for &category in cats {
                        cached.push(category);
                    }
                }
                for &category in cached.iter() {
                    self.categories_buffer.push(category);
                }
            } else {
                let categories = char_definitions.lookup_categories(c);
                for &category in categories {
                    self.categories_buffer.push(category);
                }
            }

            let categories_len = (self.categories_buffer.len() as u32 - categories_start) as u16;

            self.char_info_buffer.push(CharData {
                byte_offset: byte_offset as u32,
                is_kanji: is_kanji(c),
                categories_start,
                categories_len,
                kanji_run_byte_len: 0,
            });
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

        // SoA: Push start edge
        let edge_idx = self.edge_path_costs.len();
        self.edge_path_costs.push(0); // path_cost
        self.edge_right_ids.push(0); // right_id for start? WordEntry default right_id is 0? Yes for BOS.
        self.edge_next_indices.push(None);
        self.edge_left_ids.push(0); // left_id for start

        self.edge_left_indices.push(u32::MAX);
        self.edge_word_costs.push(0);
        self.edge_word_ids.push(WordId::new(LexType::System, 0)); // Dummy WordId
        self.edge_start_indices.push(0);
        self.edge_stop_indices.push(0);
        self.edge_types.push(EdgeType::KNOWN); // Dummy EdgeType or EdgeType::KNOWN (default)
        self.edge_kanji_onlys.push(false);

        self.ends_at_indices[0] = Some(edge_idx);

        // Index of the last character of unknown word
        let mut unknown_word_end: Option<usize> = None;

        for char_idx in 0..self.char_info_buffer.len() - 1 {
            let start = self.char_info_buffer[char_idx].byte_offset as usize;

            // No arc is ending here.
            // No need to check if a valid word starts here.
            if self.ends_at_indices[start].is_none() {
                continue;
            }

            // Optimization: Cache left edges information to SoA buffers
            // to improve cache locality for the inner loops of batch processing.
            self.left_path_costs.clear();
            self.left_right_ids.clear();

            let mut curr = self.ends_at_indices[start];
            while let Some(idx) = curr {
                // SoA access
                self.left_path_costs.push(self.edge_path_costs[idx]);
                self.left_right_ids.push(self.edge_right_ids[idx] as u32);
                curr = self.edge_next_indices[idx];
            }
            // Reverse the collected buffers so that indices match the iteration order (newest first)?
            // Actually, the linked list is LIFO (stack-like), so the latest pushed edge is the head.
            // The original implementation using Vec::push maintained insertion order (FIFO-ish or deterministic).
            // However, typical Viterbi implementations iterate edges in arbitrary order as long as all are visited.
            // But verify if index mapping matters (best_left index).
            // `best_left` index refers to the index in the temporary buffer (left_path_costs/left_right_ids).
            // Since we reconstruct the buffers, `best_left` will point to an index in these buffers.
            // When we retrieve the edge later, `left_index` should perhaps be an index into `edges_buffer` directly?
            // Currently `left_index` is `u16`. If we use direct index, we need `usize`.
            // Let's keep `left_index` as "index within the gathered edges at this position",
            // but wait, `left_index` is used for backtracking.
            // In backtracking, we need to find the specific edge.
            // If `left_index` is just an index in the `ends_at` vector, it works because we can randomly access `ends_at[pos][left_index]`.
            // With linked list, random access is O(N).
            // CRITICAL: We need O(1) random access for backtracking!

            // Re-evaluating Design:
            // If backtracking relies on `left_index` as an offset, Linked List makes backtracking slow (O(N) search).
            // Unless we change `left_index` to be the absolute index in `edges_buffer`.
            // `Edge.left_index` is `u16`. `edges_buffer` can be large (>> 65536).
            // This is a problem. The original code assumed # of edges at one position < 65536.
            // But global edges count can be large.

            // Solution:
            // Change `left_index` to `u32` (or usize) and store absolute index in `edges_buffer`.
            // This allows O(1) backtracking.

            let suffix = &text[start..];
            self.indices_buffer.clear();

            // Lookup user dictionary
            let user_dict_ref = user_dict.as_ref();
            if let Some(udict) = user_dict_ref {
                for (prefix_len, offset, len) in udict.prefix_indices(suffix) {
                    self.indices_buffer.push((prefix_len, offset, len, false));
                }
            }

            // Lookup system dictionary
            for (prefix_len, offset, len) in dict.prefix_indices(suffix) {
                self.indices_buffer.push((prefix_len, offset, len, true));
            }

            let mut found = false;
            if !self.indices_buffer.is_empty() {
                // Use std::mem::take to avoid clone()
                let mut indices = std::mem::take(&mut self.indices_buffer);
                self.add_edges_in_lattice_batched(
                    &indices,
                    dict,
                    user_dict.as_deref(),
                    char_idx,
                    start,
                    cost_matrix,
                    search_mode,
                );
                found = true;
                // Move it back and clear for next position
                indices.clear();
                self.indices_buffer = indices;
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
        if self.ends_at_indices[len].is_some() {
            // Calculate cost for EOS
            let mut best_cost = i32::MAX;
            let mut best_left_idx = u32::MAX; // Changed to absolute index
            let right_left_id = 0; // EOS default left_id

            let mut curr = self.ends_at_indices[len];
            while let Some(idx) = curr {
                // SoA access
                let left_right_id = self.edge_right_ids[idx];
                let left_path_cost = self.edge_path_costs[idx];
                let left_next = self.edge_next_indices[idx];

                let conn_cost = cost_matrix.cost(left_right_id as u32, right_left_id);
                let path_cost = left_path_cost.saturating_add(conn_cost);
                if path_cost < best_cost {
                    best_cost = path_cost;
                    best_left_idx = idx as u32;
                }
                curr = left_next;
            }
            if best_left_idx != u32::MAX {
                // SoA: Push EOS edge
                let edge_idx = self.edge_path_costs.len();

                self.edge_path_costs.push(best_cost);
                self.edge_right_ids.push(0); // EOS right_id
                self.edge_next_indices.push(self.ends_at_indices[len]);
                self.edge_left_ids.push(0); // EOS left_id

                self.edge_left_indices.push(best_left_idx);
                self.edge_word_costs.push(0);
                self.edge_word_ids.push(WordId::new(LexType::System, 0)); // EOS WordID
                self.edge_start_indices.push(len as u32);
                self.edge_stop_indices.push(len as u32);
                self.edge_types.push(EdgeType::KNOWN);
                self.edge_kanji_onlys.push(false);

                self.ends_at_indices[len] = Some(edge_idx);
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
                // Create minimal edge info for passing to add_edge_in_lattice?
                // add_edge_in_lattice takes Edge, which is now temporary.
                // We need to construct Edge.
                let edge = Self::create_edge(
                    EdgeType::UNKNOWN,
                    word_entry,
                    start,
                    start + byte_len,
                    kanji_only,
                );
                self.add_edge_in_lattice(edge, cost_matrix, search_mode);
            }
            return Some(start + byte_len);
        }
        unknown_word_index
    }

    // Adds an edge to the lattice and calculates the minimum cost to reach it.
    fn add_edge_in_lattice(&mut self, edge: Edge, cost_matrix: &ConnectionCostMatrix, mode: &Mode) {
        let start_index = edge.start_index as usize;
        let stop_index = edge.stop_index as usize;

        if self.ends_at_indices[start_index].is_none() {
            return;
        }

        let mut best_cost = i32::MAX;
        let mut best_left_idx = u32::MAX;
        let right_left_id = edge.word_entry.left_id();

        let mut curr = self.ends_at_indices[start_index];
        while let Some(idx) = curr {
            // SoA access
            let left_right_id = self.edge_right_ids[idx];
            let left_path_cost = self.edge_path_costs[idx];
            let left_next = self.edge_next_indices[idx];

            // For penalty calculation, we need to reconstruct partial Edge info or change penalty_cost signature.
            // Mode::Normal penalty is always 0. Mode::Decompose uses edge_type/wordvec.
            // Assumption: penalty_cost only needs edge_type or similar.
            // Let's check Mode::penalty_cost implementation if possible.
            // Assuming we need to support penalty, we access cold fields if strictly necessary, or optimize Mode.
            // For now, access cold fields carefully or skip if Mode unused.
            // But penalty_cost takes &Edge. We might need to construct a temporary Edge or update penalty_cost.
            // To be safe and compatible, let's reconstruct minimal Edge for penalty.

            // Reconstruct minimal reference for cache efficiency? No, penalty logic is complex.
            // Let's access cold fields for penalty calculation if mode requires it.
            let penalty = if mode.is_search() {
                // Decompose mode needs full edge info usually?
                // Let's reconstruct Edge just for compatibility for now.
                // This is slow but Decompose is not the benchmark target.
                let left_edge = Edge {
                    edge_type: self.edge_types[idx],
                    word_entry: WordEntry {
                        word_id: self.edge_word_ids[idx],
                        word_cost: self.edge_word_costs[idx],
                        left_id: self.edge_left_ids[idx],
                        right_id: left_right_id,
                    },
                    left_index: self.edge_left_indices[idx],
                    start_index: self.edge_start_indices[idx],
                    stop_index: self.edge_stop_indices[idx],
                    path_cost: left_path_cost,
                    kanji_only: self.edge_kanji_onlys[idx],
                    next: left_next,
                };
                mode.penalty_cost(&left_edge)
            } else {
                0
            };

            let conn_cost = cost_matrix.cost(left_right_id as u32, right_left_id as u32);
            let total_cost = left_path_cost
                .saturating_add(conn_cost)
                .saturating_add(penalty);

            if total_cost < best_cost {
                best_cost = total_cost;
                best_left_idx = idx as u32;
            }
            curr = left_next;
        }

        if best_left_idx != u32::MAX {
            let path_cost = best_cost.saturating_add(edge.word_entry.word_cost as i32);

            // Push to SoA vectors
            let edge_idx = self.edge_path_costs.len();

            // Hot fields
            self.edge_path_costs.push(path_cost);
            self.edge_right_ids.push(edge.word_entry.right_id);
            self.edge_next_indices
                .push(self.ends_at_indices[stop_index]);
            self.edge_left_ids.push(edge.word_entry.left_id);

            // Cold fields
            self.edge_left_indices.push(best_left_idx);
            self.edge_word_costs.push(edge.word_entry.word_cost);
            self.edge_word_ids.push(edge.word_entry.word_id);
            self.edge_start_indices.push(edge.start_index);
            self.edge_stop_indices.push(edge.stop_index);
            self.edge_types.push(edge.edge_type);
            self.edge_kanji_onlys.push(edge.kanji_only);

            self.ends_at_indices[stop_index] = Some(edge_idx);
        }
    }

    // Adds multiple edges to the lattice and calculates the minimum cost to reach them.
    // This method is optimized to process words in batches.
    fn add_edges_in_lattice_batched(
        &mut self,
        words_indices: &[(usize, usize, usize, bool)], // prefix_len, offset, len, is_system
        dict: &PrefixDictionary,
        user_dict: Option<&PrefixDictionary>,
        char_idx: usize,
        start_index: usize,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
    ) {
        if self.left_path_costs.is_empty() {
            return;
        }

        match mode {
            Mode::Normal => {
                let forward_size = cost_matrix.forward_size;
                let costs_data = &cost_matrix.costs_data;

                // Pre-calculate slices for system dictionary
                let (sys_c_pre, sys_c_slice, sys_c_post) =
                    unsafe { dict.vals_costs_data.align_to::<i16>() };
                let (sys_l_pre, sys_l_slice, sys_l_post) =
                    unsafe { dict.vals_left_ids_data.align_to::<u16>() };
                let (sys_r_pre, sys_r_slice, sys_r_post) =
                    unsafe { dict.vals_right_ids_data.align_to::<u16>() };
                let sys_use_fast = sys_c_pre.is_empty()
                    && sys_c_post.is_empty()
                    && !sys_c_slice.is_empty()
                    && sys_l_pre.is_empty()
                    && sys_l_post.is_empty()
                    && !sys_l_slice.is_empty()
                    && sys_r_pre.is_empty()
                    && sys_r_post.is_empty()
                    && !sys_r_slice.is_empty();

                // Pre-calculate slices for user dictionary if it exists
                let (user_slices, user_use_fast) = if let Some(udict) = user_dict {
                    let (u_c_pre, u_c_slice, u_c_post) =
                        unsafe { udict.vals_costs_data.align_to::<i16>() };
                    let (u_l_pre, u_l_slice, u_l_post) =
                        unsafe { udict.vals_left_ids_data.align_to::<u16>() };
                    let (u_r_pre, u_r_slice, u_r_post) =
                        unsafe { udict.vals_right_ids_data.align_to::<u16>() };
                    let u_fast = u_c_pre.is_empty()
                        && u_c_post.is_empty()
                        && !u_c_slice.is_empty()
                        && u_l_pre.is_empty()
                        && u_l_post.is_empty()
                        && !u_l_slice.is_empty()
                        && u_r_pre.is_empty()
                        && u_r_post.is_empty()
                        && !u_r_slice.is_empty();
                    (Some((u_c_slice, u_l_slice, u_r_slice)), u_fast)
                } else {
                    (None, false)
                };

                for &(prefix_len, offset, len, is_system) in words_indices {
                    let active_dict = if is_system { dict } else { user_dict.unwrap() };
                    let word_ids_data = &active_dict.vals_word_ids_data;

                    let use_fast_path = if is_system {
                        sys_use_fast
                    } else {
                        user_use_fast
                    };

                    if use_fast_path {
                        let (costs, lefts, rights) = if is_system {
                            (
                                &sys_c_slice[offset..offset + len],
                                &sys_l_slice[offset..offset + len],
                                &sys_r_slice[offset..offset + len],
                            )
                        } else {
                            let (u_c, u_l, u_r) = user_slices.unwrap();
                            (
                                &u_c[offset..offset + len],
                                &u_l[offset..offset + len],
                                &u_r[offset..offset + len],
                            )
                        };

                        for i in 0..len {
                            let word_cost = costs[i];
                            let left_id = lefts[i];
                            let right_id = rights[i];

                            let right_left_id = left_id as u32;
                            let mut best_cost = i32::MAX;
                            let mut best_left_idx = u32::MAX;
                            let base_cost_offset = (right_left_id * forward_size) as usize;

                            let mut j = 0;
                            let limit = self.left_path_costs.len();
                            while j + 8 <= limit {
                                let c0 = self.left_path_costs[j].saturating_add(
                                    costs_data[self.left_right_ids[j] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c1 = self.left_path_costs[j + 1].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 1] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c2 = self.left_path_costs[j + 2].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 2] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c3 = self.left_path_costs[j + 3].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 3] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c4 = self.left_path_costs[j + 4].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 4] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c5 = self.left_path_costs[j + 5].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 5] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c6 = self.left_path_costs[j + 6].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 6] as usize + base_cost_offset]
                                        as i32,
                                );
                                let c7 = self.left_path_costs[j + 7].saturating_add(
                                    costs_data
                                        [self.left_right_ids[j + 7] as usize + base_cost_offset]
                                        as i32,
                                );

                                if c0 < best_cost {
                                    best_cost = c0;
                                    best_left_idx = j as u32;
                                }
                                if c1 < best_cost {
                                    best_cost = c1;
                                    best_left_idx = (j + 1) as u32;
                                }
                                if c2 < best_cost {
                                    best_cost = c2;
                                    best_left_idx = (j + 2) as u32;
                                }
                                if c3 < best_cost {
                                    best_cost = c3;
                                    best_left_idx = (j + 3) as u32;
                                }
                                if c4 < best_cost {
                                    best_cost = c4;
                                    best_left_idx = (j + 4) as u32;
                                }
                                if c5 < best_cost {
                                    best_cost = c5;
                                    best_left_idx = (j + 5) as u32;
                                }
                                if c6 < best_cost {
                                    best_cost = c6;
                                    best_left_idx = (j + 6) as u32;
                                }
                                if c7 < best_cost {
                                    best_cost = c7;
                                    best_left_idx = (j + 7) as u32;
                                }
                                j += 8;
                            }
                            while j < limit {
                                let total_cost = self.left_path_costs[j].saturating_add(
                                    costs_data[self.left_right_ids[j] as usize + base_cost_offset]
                                        as i32,
                                );
                                if total_cost < best_cost {
                                    best_cost = total_cost;
                                    best_left_idx = j as u32;
                                }
                                j += 1;
                            }

                            if best_left_idx != u32::MAX {
                                let idx = offset + i;
                                let word_id_val =
                                    LittleEndian::read_u32(&word_ids_data[idx * 4..idx * 4 + 4]);
                                let stop_index = start_index + prefix_len;
                                let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                                let word_id = WordId::new(
                                    if is_system {
                                        LexType::System
                                    } else {
                                        LexType::User
                                    },
                                    word_id_val,
                                );
                                let path_cost = best_cost.saturating_add(word_cost as i32);

                                // Push to SoA vectors
                                let edge_idx = self.edge_path_costs.len();

                                // Hot fields
                                self.edge_path_costs.push(path_cost);
                                self.edge_right_ids.push(right_id);
                                self.edge_next_indices
                                    .push(self.ends_at_indices[stop_index]);
                                self.edge_left_ids.push(left_id);

                                // Cold fields
                                self.edge_left_indices.push(best_left_idx);
                                self.edge_word_costs.push(word_cost);
                                self.edge_word_ids.push(word_id);
                                self.edge_start_indices.push(start_index as u32);
                                self.edge_stop_indices.push(stop_index as u32);
                                self.edge_types.push(EdgeType::KNOWN);
                                self.edge_kanji_onlys.push(kanji_only);

                                self.ends_at_indices[stop_index] = Some(edge_idx);
                            }
                        }
                    } else {
                        // Fallback path
                        let word_costs = &active_dict.vals_costs_data;
                        let left_ids = &active_dict.vals_left_ids_data;
                        let right_ids = &active_dict.vals_right_ids_data;
                        for i in 0..len {
                            let idx = offset + i;
                            let word_cost = LittleEndian::read_i16(&word_costs[idx * 2..]);
                            let left_id = LittleEndian::read_u16(&left_ids[idx * 2..]);
                            let right_id = LittleEndian::read_u16(&right_ids[idx * 2..]);

                            let mut best_cost = i32::MAX;
                            let mut best_left_idx = u32::MAX;
                            let base_cost_offset = (left_id as u32 * forward_size) as usize;

                            for (j, &lp) in self.left_path_costs.iter().enumerate() {
                                let total_cost = lp.saturating_add(
                                    costs_data[self.left_right_ids[j] as usize + base_cost_offset]
                                        as i32,
                                );
                                if total_cost < best_cost {
                                    best_cost = total_cost;
                                    best_left_idx = j as u32;
                                }
                            }

                            if best_left_idx != u32::MAX {
                                let path_cost = best_cost.saturating_add(word_cost as i32);

                                // Prepare variables for SoA push
                                let stop_index = start_index + prefix_len;
                                let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                                let word_id_val =
                                    LittleEndian::read_u32(&word_ids_data[idx * 4..idx * 4 + 4]);
                                let word_id = WordId::new(
                                    if is_system {
                                        LexType::System
                                    } else {
                                        LexType::User
                                    },
                                    word_id_val,
                                );

                                // Push to SoA vectors
                                let edge_idx = self.edge_path_costs.len();

                                // Hot fields
                                self.edge_path_costs.push(path_cost);
                                self.edge_right_ids.push(right_id);
                                self.edge_next_indices
                                    .push(self.ends_at_indices[stop_index]);
                                self.edge_left_ids.push(left_id);

                                // Cold fields
                                self.edge_left_indices.push(best_left_idx);
                                self.edge_word_costs.push(word_cost);
                                self.edge_word_ids.push(word_id);
                                self.edge_start_indices.push(start_index as u32);
                                self.edge_stop_indices.push(stop_index as u32);
                                self.edge_types.push(EdgeType::KNOWN);
                                self.edge_kanji_onlys.push(kanji_only);

                                self.ends_at_indices[stop_index] = Some(edge_idx);
                            }
                        }
                    }
                }
            }
            _ => {
                for &(prefix_len, offset, len, is_system) in words_indices {
                    let active_dict = if is_system { dict } else { user_dict.unwrap() };
                    let word_costs = &active_dict.vals_costs_data;
                    let left_ids = &active_dict.vals_left_ids_data;
                    let right_ids = &active_dict.vals_right_ids_data;
                    let word_ids = &active_dict.vals_word_ids_data;

                    for i in 0..len {
                        let idx = offset + i;
                        let word_cost = LittleEndian::read_i16(&word_costs[idx * 2..]);
                        let left_id = LittleEndian::read_u16(&left_ids[idx * 2..]);
                        let right_id = LittleEndian::read_u16(&right_ids[idx * 2..]);
                        let word_id_val = LittleEndian::read_u32(&word_ids[idx * 4..]);
                        let word_id = WordId::new(
                            if is_system {
                                LexType::System
                            } else {
                                LexType::User
                            },
                            word_id_val,
                        );
                        let word_entry = WordEntry {
                            word_id,
                            word_cost,
                            left_id,
                            right_id,
                        };
                        let edge = Self::create_edge(
                            EdgeType::KNOWN,
                            word_entry,
                            start_index,
                            start_index + prefix_len,
                            self.is_kanji_all(char_idx, prefix_len),
                        );
                        self.add_edge_in_lattice(edge, cost_matrix, mode);
                    }
                }
            }
        }
    }

    pub fn tokens_offset(&self) -> Vec<(usize, WordId)> {
        let mut offsets = Vec::new();

        if self.ends_at_indices.is_empty() {
            return offsets;
        }

        let mut last_idx = self.ends_at_indices.len() - 1;
        while last_idx > 0 && self.ends_at_indices[last_idx].is_none() {
            last_idx -= 1;
        }

        if self.ends_at_indices[last_idx].is_none() {
            return offsets;
        }

        let mut current_idx = self.ends_at_indices[last_idx].unwrap();
        // Since we insert at the head, the latest inserted edge (best path to EOS) is at the head.

        // Ensure left_index is absolute index in edges_buffer
        // Note: left_index was u16::MAX, now u32::MAX
        if self.edge_left_indices[current_idx] == u32::MAX {
            return offsets;
        }

        loop {
            if self.edge_left_indices[current_idx] == u32::MAX {
                break;
            }

            let start_index = self.edge_start_indices[current_idx];
            let word_id = self.edge_word_ids[current_idx];
            offsets.push((start_index as usize, word_id));

            let left_idx = self.edge_left_indices[current_idx] as usize;
            current_idx = left_idx;
        }

        offsets.reverse();
        offsets.pop(); // Remove EOS

        offsets
    }
}

#[cfg(test)]
mod tests {
    use crate::viterbi::{LexType, WordEntry, WordId};

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry = WordEntry {
            word_id: WordId {
                id: 1u32,
                is_system: true,
                lex_type: LexType::System,
            },
            word_cost: -17i16,
            left_id: 1411u16,
            right_id: 1412u16,
        };
        word_entry.serialize(&mut buffer).unwrap();
        assert_eq!(WordEntry::SERIALIZED_LEN, buffer.len());
        let word_entry2 = WordEntry::deserialize(&buffer[..], true);
        assert_eq!(word_entry, word_entry2);
    }
}
