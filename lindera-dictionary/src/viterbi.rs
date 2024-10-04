use std::io;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::mode::Mode;

const EOS_NODE: EdgeId = EdgeId(1u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WordId {
    pub id: u32,
    pub is_system: bool,
}

impl WordId {
    pub fn is_unknown(&self) -> bool {
        self.id == u32::MAX
    }
    pub fn is_system(&self) -> bool {
        self.is_system
    }
}

impl Default for WordId {
    fn default() -> Self {
        WordId {
            id: u32::MAX,
            is_system: true,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
        let word_id = WordId {
            id: LittleEndian::read_u32(&data[0..4]),
            is_system: is_system_entry,
        };
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

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct EdgeId(pub u32);

#[derive(Default, Clone, Debug)]
pub struct Edge {
    pub edge_type: EdgeType,
    pub word_entry: WordEntry,

    pub path_cost: i32,
    pub left_edge: Option<EdgeId>,

    pub start_index: u32,
    pub stop_index: u32,

    pub kanji_only: bool,
}

impl Edge {
    // TODO fix em
    pub fn num_chars(&self) -> usize {
        (self.stop_index - self.start_index) as usize / 3
    }
}

#[derive(Clone, Default)]
pub struct Lattice {
    capacity: usize,
    edges: Vec<Edge>,
    starts_at: Vec<Vec<EdgeId>>,
    ends_at: Vec<Vec<EdgeId>>,
}

fn is_kanji(c: char) -> bool {
    let c = c as u32;
    (19968..=40879).contains(&c)
}

fn is_kanji_only(s: &str) -> bool {
    s.chars().all(is_kanji)
}

impl Lattice {
    pub fn clear(&mut self) {
        for edge_vec in &mut self.starts_at {
            edge_vec.clear();
        }
        for edge_vec in &mut self.ends_at {
            edge_vec.clear();
        }
        self.edges.clear()
    }

    fn set_capacity(&mut self, text_len: usize) {
        self.clear();
        if self.capacity < text_len {
            self.capacity = text_len;
            self.edges.clear();
            self.starts_at.resize(text_len + 1, Vec::new());
            self.ends_at.resize(text_len + 1, Vec::new());
        }
    }

    #[inline(never)]
    pub fn set_text(
        &mut self,
        dict: &PrefixDictionary,
        user_dict: &Option<&PrefixDictionary>,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        text: &str,
        search_mode: &Mode,
    ) {
        let len = text.len();
        self.set_capacity(len);

        let start_edge_id = self.add_edge(Edge::default());
        let end_edge_id = self.add_edge(Edge::default());

        assert_eq!(EOS_NODE, end_edge_id);
        self.ends_at[0].push(start_edge_id);
        self.starts_at[len].push(end_edge_id);

        // index of the last character of unknown word
        let mut unknown_word_end: Option<usize> = None;

        for start in 0..len {
            // No arc is ending here.
            // No need to check if a valid word starts here.
            if self.ends_at[start].is_empty() {
                continue;
            }

            let suffix = &text[start..];

            let mut found: bool = false;

            // lookup user dictionary
            if user_dict.is_some() {
                let dict = user_dict.as_ref().unwrap();
                for (prefix_len, word_entry) in dict.prefix(suffix) {
                    let edge = Edge {
                        edge_type: EdgeType::KNOWN,
                        word_entry,
                        left_edge: None,
                        start_index: start as u32,
                        stop_index: (start + prefix_len) as u32,
                        path_cost: i32::MAX,
                        kanji_only: is_kanji_only(&suffix[..prefix_len]),
                    };
                    self.add_edge_in_lattice(edge);
                    found = true;
                }
            }

            // we check all word starting at start, using the double array, like we would use
            // a prefix trie, and populate the lattice with as many edges
            for (prefix_len, word_entry) in dict.prefix(suffix) {
                let edge = Edge {
                    edge_type: EdgeType::KNOWN,
                    word_entry,
                    left_edge: None,
                    start_index: start as u32,
                    stop_index: (start + prefix_len) as u32,
                    path_cost: i32::MAX,
                    kanji_only: is_kanji_only(&suffix[..prefix_len]),
                };
                self.add_edge_in_lattice(edge);
                found = true;
            }

            // In the case of normal mode, it doesn't process unknown word greedily.
            if search_mode.is_search()
                || unknown_word_end.map(|index| index <= start).unwrap_or(true)
            {
                if let Some(first_char) = suffix.chars().next() {
                    let categories = char_definitions.lookup_categories(first_char);
                    for (category_ord, &category) in categories.iter().enumerate() {
                        unknown_word_end = self.process_unknown_word(
                            char_definitions,
                            unknown_dictionary,
                            category,
                            category_ord,
                            unknown_word_end,
                            start,
                            suffix,
                            found,
                        );
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_unknown_word(
        &mut self,
        char_definitions: &CharacterDefinition,
        unknown_dictionary: &UnknownDictionary,
        category: CategoryId,
        category_ord: usize,
        unknown_word_index: Option<usize>,
        start: usize,
        suffix: &str,
        found: bool,
    ) -> Option<usize> {
        let mut unknown_word_num_chars: usize = 0;
        let category_data = char_definitions.lookup_definition(category);
        if category_data.invoke || !found {
            unknown_word_num_chars = 1;
            if category_data.group {
                for c in suffix.chars().skip(1) {
                    let categories = char_definitions.lookup_categories(c);
                    if categories.len() > category_ord && categories[category_ord] == category {
                        unknown_word_num_chars += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        if unknown_word_num_chars > 0 {
            // optimize
            let unknown_word = suffix
                .chars()
                .take(unknown_word_num_chars)
                .collect::<String>();
            for &word_id in unknown_dictionary.lookup_word_ids(category) {
                let word_entry = unknown_dictionary.word_entry(word_id);
                let edge = Edge {
                    edge_type: EdgeType::UNKNOWN,
                    word_entry,
                    left_edge: None,
                    start_index: start as u32,
                    stop_index: (start + unknown_word.len()) as u32,
                    path_cost: i32::MAX,
                    kanji_only: is_kanji_only(&unknown_word[..]),
                };
                self.add_edge_in_lattice(edge);
            }
            return Some(start + unknown_word.len());
        }
        unknown_word_index
    }

    fn add_edge_in_lattice(&mut self, edge: Edge) {
        let start_index = edge.start_index as usize;
        let stop_index = edge.stop_index as usize;
        let edge_id = self.add_edge(edge);
        self.starts_at[start_index].push(edge_id);
        self.ends_at[stop_index].push(edge_id);
    }

    fn add_edge(&mut self, edge: Edge) -> EdgeId {
        let edge_id = EdgeId(self.edges.len() as u32);
        self.edges.push(edge);
        edge_id
    }

    pub fn edge(&self, edge_id: EdgeId) -> &Edge {
        &self.edges[edge_id.0 as usize]
    }

    #[inline(never)]
    pub fn calculate_path_costs(&mut self, cost_matrix: &ConnectionCostMatrix, mode: &Mode) {
        let text_len = self.starts_at.len();
        for i in 0..text_len {
            let left_edge_ids = &self.ends_at[i];
            let right_edge_ids = &self.starts_at[i];
            for &right_edge_id in right_edge_ids {
                let right_word_entry = self.edge(right_edge_id).word_entry;
                let best_path = left_edge_ids
                    .iter()
                    .cloned()
                    .map(|left_edge_id| {
                        let left_edge = self.edge(left_edge_id);
                        let mut path_cost = left_edge.path_cost
                            + cost_matrix
                                .cost(left_edge.word_entry.right_id(), right_word_entry.left_id());
                        path_cost += mode.penalty_cost(left_edge);
                        (path_cost, left_edge_id)
                    })
                    .min_by_key(|&(cost, _)| cost);
                if let Some((best_cost, best_left)) = best_path {
                    let edge = &mut self.edges[right_edge_id.0 as usize];
                    edge.left_edge = Some(best_left);
                    edge.path_cost = right_word_entry.word_cost as i32 + best_cost;
                }
            }
        }
    }

    pub fn tokens_offset(&self) -> Vec<(usize, WordId)> {
        let mut offsets = Vec::new();
        let mut edge_id = EOS_NODE;
        let _edge = self.edge(EOS_NODE);
        loop {
            let edge = self.edge(edge_id);
            if let Some(left_edge_id) = edge.left_edge {
                offsets.push((edge.start_index as usize, edge.word_entry.word_id));
                edge_id = left_edge_id;
            } else {
                break;
            }
        }
        offsets.reverse();
        offsets.pop();
        offsets
    }
}

#[cfg(test)]
mod tests {
    use crate::viterbi::{WordEntry, WordId};

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry = WordEntry {
            word_id: WordId {
                id: 1u32,
                is_system: true,
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
