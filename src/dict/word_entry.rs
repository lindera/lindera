use std::io;

use byteorder::WriteBytesExt;
use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

const WORDS_DATA: &'static [u8] = include_bytes!("../../dict/dict.words");
const WORDS_IDX_DATA: &'static [u8] = include_bytes!("../../dict/dict.wordsidx");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WordId(pub u32);

impl WordId {
    pub fn is_unknown(&self) -> bool {
        self.0 == std::u32::MAX
    }
}

impl Default for WordId {
    fn default() -> Self {
        WordId(std::u32::MAX)
    }
}

pub struct WordDictionary;

impl WordDictionary {
    pub fn load_word_id(word_id: WordId) -> WordDetail {
        if word_id.is_unknown() {
            return WordDetail {
                left_id: 0,
                right_id: 0,
                word_cost: 0,
                pos_level1: "UNK".to_string(),
                pos_level2: "*".to_string(),
                pos_level3: "*".to_string(),
                pos_level4: "*".to_string(),
                conjugation_type: "*".to_string(),
                conjugate_form: "*".to_string(),
                base_form: "*".to_string(),
                reading: "*".to_string(),
                pronunciation: "*".to_string(),
            };
        }
        let idx = LittleEndian::read_u32(&WORDS_IDX_DATA[4 * word_id.0 as usize..][..4]);
        let data = &WORDS_DATA[idx as usize..];
        let word_detail = bincode::deserialize_from(data).unwrap();
        word_detail
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WordDetail {
    pub left_id: u32,
    pub right_id: u32,
    pub word_cost: i32,
    pub pos_level1: String,
    pub pos_level2: String,
    pub pos_level3: String,
    pub pos_level4: String,
    pub conjugation_type: String,
    pub conjugate_form: String,
    pub base_form: String,
    pub reading: String,
    pub pronunciation: String,
}

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WordEntry {
    pub word_id: WordId,
    pub word_cost: i16,
    pub cost_id: u16,
}

impl WordEntry {
    pub const SERIALIZED_LEN: usize = 8;

    pub fn left_id(&self) -> u32 {
        self.cost_id as u32
    }

    pub fn right_id(&self) -> u32 {
        self.cost_id as u32
    }

    pub fn serialize<W: io::Write>(&self, wtr: &mut W) -> io::Result<()> {
        wtr.write_u32::<LittleEndian>(self.word_id.0)?;
        wtr.write_i16::<LittleEndian>(self.word_cost)?;
        wtr.write_u16::<LittleEndian>(self.cost_id)?;
        Ok(())
    }

    pub fn deserialize(data: &[u8]) -> WordEntry {
        let word_id = WordId(LittleEndian::read_u32(&data[0..4]));
        let word_cost = LittleEndian::read_i16(&data[4..6]);
        let cost_id = LittleEndian::read_u16(&data[6..8]);
        WordEntry {
            word_id,
            word_cost,
            cost_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dict::word_entry::{WordDictionary, WordEntry, WordId};

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry = WordEntry {
            word_id: WordId(1u32),
            word_cost: -17i16,
            cost_id: 1411u16,
        };
        word_entry.serialize(&mut buffer).unwrap();
        assert_eq!(WordEntry::SERIALIZED_LEN, buffer.len());
        let word_entry2 = WordEntry::deserialize(&buffer[..]);
        assert_eq!(word_entry, word_entry2);
    }

    #[test]
    fn test_dictionary() {
        let word_detail = WordDictionary::load_word_id(WordId(0u32));
        assert_eq!(&word_detail.reading, "ティーシャツ");
        let word_detail = WordDictionary::load_word_id(WordId(1u32));
        assert_eq!(word_detail.reading, "¨".to_string());
    }
}
