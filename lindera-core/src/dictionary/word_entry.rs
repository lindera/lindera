use std::io;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WordId(pub u32, pub bool);

impl WordId {
    pub fn is_unknown(&self) -> bool {
        self.0 == std::u32::MAX
    }
    pub fn is_system(&self) -> bool {
        self.1
    }
}

impl Default for WordId {
    fn default() -> Self {
        WordId(std::u32::MAX, true)
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
        wtr.write_u32::<LittleEndian>(self.word_id.0)?;
        wtr.write_i16::<LittleEndian>(self.word_cost)?;
        wtr.write_u16::<LittleEndian>(self.left_id)?;
        wtr.write_u16::<LittleEndian>(self.right_id)?;
        Ok(())
    }

    pub fn deserialize(data: &[u8], is_system_entry: bool) -> WordEntry {
        let word_id = WordId(LittleEndian::read_u32(&data[0..4]), is_system_entry);
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

#[cfg(test)]
mod tests {
    use crate::dictionary::word_entry::{WordEntry, WordId};

    #[test]
    fn test_word_entry() {
        let mut buffer = Vec::new();
        let word_entry = WordEntry {
            word_id: WordId(1u32, true),
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
