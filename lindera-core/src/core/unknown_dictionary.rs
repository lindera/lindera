use serde::{Deserialize, Serialize};

use crate::core::character_definition::CategoryId;
use crate::core::word_entry::WordEntry;

//TODO optimize
#[derive(Serialize, Deserialize, Clone)]
pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
}

impl UnknownDictionary {
    pub fn load(unknown_data: &[u8]) -> UnknownDictionary {
        bincode::deserialize(unknown_data).unwrap()
    }

    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        self.costs[word_id as usize]
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[u32] {
        &self.category_references[category_id.0][..]
    }
}

#[cfg(test)]
mod tests {
    //    use crate::core::unknown_dictionary::UnknownDictionary;

    //    #[test]
    //    fn test_parse_unknown_dictionary() {
    //        let _unknown_dict = UnknownDictionary::load();
    //    }
}
