use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::CategoryId;
use crate::dictionary::word_entry::WordEntry;
use lindera_ipadic::unknown_dict;

//TODO optimize
#[derive(Serialize, Deserialize)]
pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
}

impl UnknownDictionary {
    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        self.costs[word_id as usize]
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[u32] {
        &self.category_references[category_id.0][..]
    }

    pub fn load() -> UnknownDictionary {
        bincode::deserialize(unknown_dict()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary::unknown_dictionary::UnknownDictionary;

    #[test]
    fn test_parse_unknown_dictionary() {
        let _unknown_dict = UnknownDictionary::load();
    }
}
