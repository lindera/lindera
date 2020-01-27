use crate::core::character_definition::CategoryId;
use crate::core::word_entry::WordEntry;
use serde::{Deserialize, Serialize};

const CHAR_DEFINITION_DATA: &'static [u8] = include_bytes!("../../dict/unk.bin");

//TODO optimize
#[derive(Serialize, Deserialize)]
pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
}

#[derive(Debug)]
pub struct DictionaryEntry {
    surface: String,
    left_id: u32,
    right_id: u32,
    word_cost: i32,
}

impl UnknownDictionary {
    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        self.costs[word_id as usize]
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[u32] {
        &self.category_references[category_id.0][..]
    }

    pub fn load() -> UnknownDictionary {
        bincode::deserialize(CHAR_DEFINITION_DATA).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::unknown_dictionary::UnknownDictionary;
    use crate::unknown_dictionary::UnknownDictionary;

    #[test]
    fn test_parse_unknown_dictionary() {
        let _unknown_dict = UnknownDictionary::load();
    }
}
