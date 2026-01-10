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
    pub left_index: u16, // Index in the previous position's vector

    pub start_index: u32,
    pub stop_index: u32,

    pub kanji_only: bool,
}

impl Edge {
    pub fn num_chars(&self) -> usize {
        (self.stop_index - self.start_index) as usize / 3
    }
}

#[derive(Clone, Default)]
pub struct Lattice {
    capacity: usize,
    ends_at: Vec<Vec<Edge>>, // Now stores edges directly
    char_info_buffer: Vec<CharData>,
    categories_buffer: Vec<CategoryId>,
    char_category_cache: Vec<Vec<CategoryId>>,
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
            left_index: u16::MAX,
            start_index: start as u32,
            stop_index: stop as u32,
            path_cost: i32::MAX,
            kanji_only,
        }
    }

    pub fn clear(&mut self) {
        for edge_vec in &mut self.ends_at {
            edge_vec.clear();
        }
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
        if self.capacity <= text_len {
            self.capacity = text_len;
            self.ends_at.resize(text_len + 1, Vec::new());
        }
        for vec in &mut self.ends_at {
            vec.clear();
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

        let mut start_edge = Edge::default();
        start_edge.path_cost = 0;
        start_edge.left_index = u16::MAX;
        self.ends_at[0].push(start_edge);

        // Index of the last character of unknown word
        let mut unknown_word_end: Option<usize> = None;

        for char_idx in 0..self.char_info_buffer.len() - 1 {
            let start = self.char_info_buffer[char_idx].byte_offset as usize;

            // No arc is ending here.
            // No need to check if a valid word starts here.
            if self.ends_at[start].is_empty() {
                continue;
            }

            let suffix = &text[start..];

            let mut found: bool = false;

            // Lookup user dictionary
            if user_dict.is_some() {
                let dict = user_dict.as_ref().unwrap();
                for (prefix_len, word_entry) in dict.prefix(suffix) {
                    let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                    let edge = Self::create_edge(
                        EdgeType::KNOWN,
                        word_entry,
                        start,
                        start + prefix_len,
                        kanji_only,
                    );
                    self.add_edge_in_lattice(edge, cost_matrix, search_mode);
                    found = true;
                }
            }

            // Check all word starting at start, using the double array, like we would use
            // a prefix trie, and populate the lattice with as many edges
            for (prefix_len, word_entry) in dict.prefix(suffix) {
                let kanji_only = self.is_kanji_all(char_idx, prefix_len);
                let edge = Self::create_edge(
                    EdgeType::KNOWN,
                    word_entry,
                    start,
                    start + prefix_len,
                    kanji_only,
                );
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
            let mut eos_edge = Edge::default();
            eos_edge.start_index = len as u32;
            eos_edge.stop_index = len as u32;
            // Calculate cost for EOS
            let left_edges = &self.ends_at[len];
            let mut best_cost = i32::MAX;
            let mut best_left = None;
            let right_left_id = 0; // EOS default left_id

            for (i, left_edge) in left_edges.iter().enumerate() {
                let left_right_id = left_edge.word_entry.right_id();
                let conn_cost = cost_matrix.cost(left_right_id, right_left_id);
                let path_cost = left_edge.path_cost.saturating_add(conn_cost);
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
    fn add_edge_in_lattice(
        &mut self,
        mut edge: Edge,
        cost_matrix: &ConnectionCostMatrix,
        mode: &Mode,
    ) {
        let start_index = edge.start_index as usize;
        let stop_index = edge.stop_index as usize;

        let left_edges = &self.ends_at[start_index];
        if left_edges.is_empty() {
            return;
        }

        let mut best_cost = i32::MAX;
        let mut best_left = None;
        let right_left_id = edge.word_entry.left_id();

        for (i, left_edge) in left_edges.iter().enumerate() {
            let left_right_id = left_edge.word_entry.right_id();
            let conn_cost = cost_matrix.cost(left_right_id, right_left_id);
            let penalty = mode.penalty_cost(left_edge);
            let total_cost = left_edge
                .path_cost
                .saturating_add(conn_cost)
                .saturating_add(penalty);

            if total_cost < best_cost {
                best_cost = total_cost;
                best_left = Some(i as u16);
            }
        }

        if let Some(best_left_idx) = best_left {
            edge.path_cost = best_cost.saturating_add(edge.word_entry.word_cost as i32);
            edge.left_index = best_left_idx;
            self.ends_at[stop_index].push(edge);
        }
    }

    pub fn tokens_offset(&self) -> Vec<(usize, WordId)> {
        let mut offsets = Vec::new();

        if self.ends_at.is_empty() {
            return offsets;
        }

        let mut last_idx = self.ends_at.len() - 1;
        while last_idx > 0 && self.ends_at[last_idx].is_empty() {
            last_idx -= 1;
        }

        if self.ends_at[last_idx].is_empty() {
            return offsets;
        }

        let idx = self.ends_at[last_idx].len() - 1;
        let mut edge = &self.ends_at[last_idx][idx];

        if edge.left_index == u16::MAX {
            return offsets;
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
