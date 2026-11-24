//! A table from strings to values of one width, written to bytes and read
//! on them.
//!
//! The bytes of a table, little endian:
//!
//! ```text
//! the number of strings      u32
//! the width of a value       u32, in bytes
//! the number of slots        u32, a power of two above the number of strings
//! the offsets of the strings (number + 1) × u32, into the strings below
//! the strings                UTF-8, one after the other, sorted by their bytes
//! the values                 number × width bytes, in the order of the strings
//! the slots                  slots × u32
//! ```
//!
//! The slots are a hash table with open addressing: a slot is 0 when it is
//! empty, and otherwise one more than the index of a string. A string
//! starts at the slot of its hash, the 64-bit FNV-1a of its bytes folded to
//! 32 bits and cut to the number of slots, and goes to the next free slot
//! after it, the first slot following the last. The strings are put in by
//! their order, and a table is written with at least two slots for a
//! string: most strings that are looked up are not in a table, and the
//! search for one ends at the first empty slot.
//!
//! A string is found in the time of its hash, and by its order as well, so
//! the same strings always give the same bytes. Nothing is copied when a
//! table is read, and no number is read through a pointer, so the bytes can
//! sit at any address.

use std::ops::Range;

use crate::bytes::{Reader, push_u32};
use crate::error::Result;

/// The hash of a string: the 64-bit FNV-1a of its bytes, folded to 32 bits.
///
/// It is part of the format: a table is read with the hash it was written
/// with.
///
/// # Arguments
///
/// * `key` - The bytes of the string.
///
/// # Returns
///
/// The hash.
pub(crate) fn hash(key: &[u8]) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in key {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash ^ (hash >> 32)) as u32 as usize
}

/// Appends a table to bytes.
///
/// # Arguments
///
/// * `bytes` - The bytes to append to.
/// * `width` - The width of a value, in bytes.
/// * `entries` - The strings with their values, sorted by the bytes of the
///   strings, each string once, each value of `width` bytes.
pub(crate) fn write(bytes: &mut Vec<u8>, width: usize, entries: &[(&str, &[u8])]) {
    debug_assert!(
        entries
            .windows(2)
            .all(|pair| pair[0].0.as_bytes() < pair[1].0.as_bytes())
    );
    debug_assert!(entries.iter().all(|(_, value)| value.len() == width));
    // At most half of the slots are taken.
    let slots = (entries.len() * 2).next_power_of_two();
    push_u32(bytes, entries.len() as u32);
    push_u32(bytes, width as u32);
    push_u32(bytes, slots as u32);
    let mut offset = 0;
    push_u32(bytes, offset);
    for (string, _) in entries {
        offset += string.len() as u32;
        push_u32(bytes, offset);
    }
    for (string, _) in entries {
        bytes.extend_from_slice(string.as_bytes());
    }
    for (_, value) in entries {
        bytes.extend_from_slice(value);
    }
    let mut table = vec![0u32; slots];
    for (index, (string, _)) in entries.iter().enumerate() {
        let mut slot = hash(string.as_bytes()) & (slots - 1);
        while table[slot] != 0 {
            slot = (slot + 1) & (slots - 1);
        }
        table[slot] = index as u32 + 1;
    }
    for slot in table {
        push_u32(bytes, slot);
    }
}

/// Where the pieces of a table are in the bytes of a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The number of strings.
    count: usize,
    /// The width of a value, in bytes.
    width: usize,
    /// The offsets of the strings.
    offsets: Range<usize>,
    /// The strings.
    strings: Range<usize>,
    /// The values.
    values: Range<usize>,
    /// The slots.
    slots: Range<usize>,
}

impl Layout {
    /// Reads a table and checks it.
    ///
    /// Every string is looked at once: after this, a string of the table is
    /// found and a string that is not in it is not, whatever the bytes
    /// were.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the table.
    /// * `width` - The width a value has to have, in bytes.
    ///
    /// # Returns
    ///
    /// The layout, counted from the first byte of the reader, or an error:
    /// bytes that end early, another width, offsets that go back or past
    /// the strings, a string that is not UTF-8, strings that are not sorted
    /// with each string once, a number of slots that is not a power of two
    /// above the number of strings, or a string its slots do not lead to.
    pub(crate) fn read(reader: &mut Reader, width: usize) -> Result<Self> {
        let count = reader.u32()? as usize;
        let stored = reader.u32()? as usize;
        if stored != width {
            return Err(reader.error(format_args!(
                "has a table with values of {stored} bytes, and {width} are expected"
            )));
        }
        let slot_count = reader.u32()? as usize;
        if !slot_count.is_power_of_two() || slot_count <= count {
            return Err(reader.error(format_args!(
                "has a table of {count} strings with {slot_count} slots"
            )));
        }

        let start = reader.position();
        // `count + 1` offsets of 4 bytes: a count near the largest number
        // ends the bytes early instead of overflowing.
        let offsets = reader.take((count + 1).saturating_mul(4))?;
        let offsets_range = start..reader.position();
        let length = read_u32(offsets, count);

        let start = reader.position();
        let strings = reader.take(length)?;
        let strings_range = start..reader.position();

        let start = reader.position();
        let values = reader.take(count.saturating_mul(width))?;
        let values_range = start..reader.position();

        let start = reader.position();
        let slots = reader.take(slot_count.saturating_mul(4))?;
        let slots_range = start..reader.position();

        let table = Table {
            count,
            width,
            offsets,
            strings,
            values,
            slots,
        };
        // The offsets go from 0 to the length of the strings, which is the
        // last of them, and never back.
        if read_u32(offsets, 0) != 0 {
            return Err(reader.error("has a table whose offsets are not in order"));
        }
        let mut before: Option<&[u8]> = None;
        for index in 0..count {
            if read_u32(offsets, index + 1) < read_u32(offsets, index) {
                return Err(reader.error("has a table whose offsets are not in order"));
            }
            let key = table.key(index);
            if std::str::from_utf8(key).is_err() {
                return Err(reader.error("has a string that is not UTF-8"));
            }
            if before.is_some_and(|before| before >= key) {
                return Err(reader.error("has a table whose strings are not sorted"));
            }
            before = Some(key);
        }
        if (0..count).any(|index| table.find_bytes(table.key(index)) != Some(index)) {
            return Err(reader.error("has a table whose slots do not lead to its strings"));
        }

        Ok(Self {
            count,
            width,
            offsets: offsets_range,
            strings: strings_range,
            values: values_range,
            slots: slots_range,
        })
    }

    /// The layout counted from another byte.
    ///
    /// # Arguments
    ///
    /// * `base` - The position, in the bytes the layout is to be used on,
    ///   of the byte it is counted from now.
    ///
    /// # Returns
    ///
    /// The layout on those bytes.
    pub(crate) fn shifted(self, base: usize) -> Self {
        let shift = |range: Range<usize>| base + range.start..base + range.end;
        Self {
            count: self.count,
            width: self.width,
            offsets: shift(self.offsets),
            strings: shift(self.strings),
            values: shift(self.values),
            slots: shift(self.slots),
        }
    }

    /// The table of the layout.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes of the model the layout was read from.
    ///
    /// # Returns
    ///
    /// The table. On other bytes than those the layout was read from, it is
    /// a table that finds nothing or finds the wrong values; it does not
    /// panic.
    pub(crate) fn table<'a>(&self, bytes: &'a [u8]) -> Table<'a> {
        let piece = |range: &Range<usize>| bytes.get(range.clone()).unwrap_or_default();
        Table {
            count: self.count,
            width: self.width,
            offsets: piece(&self.offsets),
            strings: piece(&self.strings),
            values: piece(&self.values),
            slots: piece(&self.slots),
        }
    }
}

/// A number of 4 bytes in an array of them.
///
/// # Arguments
///
/// * `bytes` - The array.
/// * `index` - The index of the number.
///
/// # Returns
///
/// The number, or 0 past the end of the array.
fn read_u32(bytes: &[u8], index: usize) -> usize {
    let start = index.saturating_mul(4);
    bytes
        .get(start..start.saturating_add(4))
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(0, |bytes| u32::from_le_bytes(bytes) as usize)
}

/// A table on the bytes of a model.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Table<'a> {
    /// The number of strings.
    count: usize,
    /// The width of a value, in bytes.
    width: usize,
    /// The offsets of the strings.
    offsets: &'a [u8],
    /// The strings.
    strings: &'a [u8],
    /// The values.
    values: &'a [u8],
    /// The slots.
    slots: &'a [u8],
}

impl<'a> Table<'a> {
    /// The number of strings of the table.
    ///
    /// # Returns
    ///
    /// The number of strings.
    pub(crate) fn len(&self) -> usize {
        self.count
    }

    /// A string of the table.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the string, in the order of the table.
    ///
    /// # Returns
    ///
    /// The bytes of the string; none past the last string.
    pub(crate) fn key(&self, index: usize) -> &'a [u8] {
        let from = read_u32(self.offsets, index);
        let to = read_u32(self.offsets, index + 1);
        self.strings.get(from..to).unwrap_or_default()
    }

    /// The value of a string of the table.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the string, in the order of the table.
    ///
    /// # Returns
    ///
    /// The bytes of the value; none past the last string.
    pub(crate) fn value(&self, index: usize) -> &'a [u8] {
        let start = index.saturating_mul(self.width);
        self.values
            .get(start..start.saturating_add(self.width))
            .unwrap_or_default()
    }

    /// Finds a string.
    ///
    /// # Arguments
    ///
    /// * `key` - The string.
    ///
    /// # Returns
    ///
    /// The index of the string in the order of the table, or `None` for a
    /// string the table does not have.
    pub(crate) fn find(&self, key: &str) -> Option<usize> {
        self.find_bytes(key.as_bytes())
    }

    /// Finds a string by its bytes.
    ///
    /// # Arguments
    ///
    /// * `key` - The bytes of the string.
    ///
    /// # Returns
    ///
    /// The index of the string, or `None`.
    fn find_bytes(&self, key: &[u8]) -> Option<usize> {
        let slot_count = self.slots.len() / 4;
        if slot_count == 0 {
            return None;
        }
        // The number of slots is a power of two, which the reading checked.
        let mask = slot_count - 1;
        let mut slot = hash(key) & mask;
        // No more steps than there are slots: slots that are all taken, on
        // bytes that were not checked, end the search as well.
        for _ in 0..slot_count {
            let index = read_u32(self.slots, slot).checked_sub(1)?;
            if index < self.count && self.key(index) == key {
                return Some(index);
            }
            slot = (slot + 1) & mask;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    /// The bytes of a table of values of 2 bytes.
    fn bytes(entries: &[(&str, [u8; 2])]) -> Vec<u8> {
        let entries: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(string, value)| (*string, value.as_slice()))
            .collect();
        let mut bytes = Vec::new();
        write(&mut bytes, 2, &entries);
        bytes
    }

    /// Reads a table of values of 2 bytes, which has to end the bytes.
    fn read(bytes: &[u8]) -> Result<Layout> {
        let mut reader = Reader::new(bytes, "model", Error::Model);
        let layout = Layout::read(&mut reader, 2)?;
        assert_eq!(reader.remaining(), 0);
        Ok(layout)
    }

    /// The message of the error of bytes that are not a table.
    fn error(bytes: &[u8]) -> String {
        read(bytes).unwrap_err().to_string()
    }

    /// Strings that sort by their bytes: the empty one first, and a string
    /// before the one it starts.
    const ENTRIES: [(&str, [u8; 2]); 5] = [
        ("", [1, 2]),
        ("U00:は", [3, 4]),
        ("U00:は/が", [5, 6]),
        ("U01:彼", [7, 8]),
        ("a", [9, 10]),
    ];

    #[test]
    fn every_string_is_found_with_its_value() {
        let bytes = bytes(&ENTRIES);
        let table = read(&bytes).unwrap().table(&bytes);
        assert_eq!(table.len(), 5);
        for (index, (string, value)) in ENTRIES.iter().enumerate() {
            assert_eq!(table.find(string), Some(index), "{string:?}");
            assert_eq!(table.key(index), string.as_bytes());
            assert_eq!(table.value(index), value);
        }
        // Before the first, between two, a prefix, a longer one, after the
        // last.
        for missing in ["U00:", "U00:が", "U00:は/", "U00:は/が/", "U01:彼は", "b"] {
            assert_eq!(table.find(missing), None, "{missing:?}");
        }
        assert_eq!(table.key(5), b"");
        assert_eq!(table.value(5), b"");
    }

    #[test]
    fn a_string_starts_at_the_slot_of_its_hash() {
        // The 64-bit FNV-1a of `a`, `b` and `c` is af63dc4c8601ec8c,
        // af63df4c8601f1a5 and af63de4c8601eff2: folded to 32 bits and cut
        // to the eight slots of three strings, 0, 1 and 6.
        let bytes = bytes(&[("a", [0, 0]), ("b", [0, 0]), ("c", [0, 0])]);
        assert_eq!(bytes[8..12], [8, 0, 0, 0]);
        let slots: Vec<u8> = bytes[bytes.len() - 32..]
            .chunks(4)
            .map(|slot| slot[0])
            .collect();
        assert_eq!(slots, [1, 2, 0, 0, 0, 0, 3, 0]);
        assert_eq!(
            bytes[bytes.len() - 32..]
                .iter()
                .filter(|byte| **byte != 0)
                .count(),
            3
        );
    }

    #[test]
    fn strings_that_share_a_slot_are_all_found() {
        // 300 strings in 1,024 slots: some start at the same slot.
        let keys: Vec<String> = (0..300).map(|index| format!("U{index:03}")).collect();
        let entries: Vec<(&str, [u8; 2])> = keys
            .iter()
            .enumerate()
            .map(|(index, key)| (key.as_str(), [index as u8, (index >> 8) as u8]))
            .collect();
        let bytes = bytes(&entries);
        assert_eq!(bytes[8..12], 1024u32.to_le_bytes());
        let mut homes: Vec<usize> = keys.iter().map(|key| hash(key.as_bytes()) & 1023).collect();
        homes.sort_unstable();
        homes.dedup();
        assert!(homes.len() < keys.len(), "no two strings share a slot");

        let table = read(&bytes).unwrap().table(&bytes);
        for (index, (key, value)) in entries.iter().enumerate() {
            assert_eq!(table.find(key), Some(index), "{key}");
            assert_eq!(table.value(index), value);
        }
        for missing in ["U300", "U", "V000", "U00"] {
            assert_eq!(table.find(missing), None, "{missing}");
        }
    }

    #[test]
    fn an_empty_table_finds_nothing() {
        let bytes = bytes(&[]);
        // No string, values of 2 bytes, one slot, one offset, an empty slot.
        assert_eq!(
            bytes,
            [0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let table = read(&bytes).unwrap().table(&bytes);
        assert_eq!(table.len(), 0);
        assert_eq!(table.find(""), None);
        assert_eq!(table.find("a"), None);
    }

    #[test]
    fn the_layout_is_counted_from_the_base() {
        let table_bytes = bytes(&ENTRIES);
        let mut bytes = vec![0xff; 7];
        bytes.extend_from_slice(&table_bytes);
        let mut reader = Reader::new(&bytes[7..], "model", Error::Model);
        let layout = Layout::read(&mut reader, 2).unwrap().shifted(7);
        let table = layout.table(&bytes);
        assert_eq!(table.find("U01:彼"), Some(3));
        assert_eq!(table.value(3), [7, 8]);
        // The layout of other bytes finds nothing, and does not panic.
        let table = layout.table(&bytes[..9]);
        assert_eq!(table.find("U01:彼"), None);
        assert_eq!(table.value(3), b"");
    }

    #[test]
    fn refuses_bytes_that_are_not_a_table() {
        let bytes = bytes(&ENTRIES);
        // Cut anywhere, the bytes end early.
        for length in 0..bytes.len() {
            let message = error(&bytes[..length]);
            assert!(
                message.starts_with("the model ends early"),
                "{length}: {message}"
            );
        }

        // Another width.
        let mut other = bytes.clone();
        other[4] = 3;
        assert_eq!(
            error(&other),
            "the model has a table with values of 3 bytes, and 2 are expected"
        );

        // A count near the largest number, with the slots of the table and
        // with more slots than strings.
        let mut other = bytes.clone();
        other[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            error(&other),
            "the model has a table of 4294967295 strings with 16 slots"
        );
        other[..4].copy_from_slice(&(1u32 << 30).to_le_bytes());
        other[8..12].copy_from_slice(&(1u32 << 31).to_le_bytes());
        assert!(error(&other).starts_with("the model ends early"));

        // A number of slots that is not a power of two, or not above the
        // number of strings.
        for slots in [0u32, 4, 12] {
            let mut other = bytes.clone();
            other[8..12].copy_from_slice(&slots.to_le_bytes());
            assert_eq!(
                error(&other),
                format!("the model has a table of 5 strings with {slots} slots")
            );
        }

        // The offsets are at 12: the first is not 0, with the second, the
        // end of the empty string, after it.
        let mut other = bytes.clone();
        other[12] = 1;
        other[16] = 1;
        assert_eq!(
            error(&other),
            "the model has a table whose offsets are not in order"
        );
        // The second is past the third.
        let mut other = bytes.clone();
        other[16] = 200;
        assert_eq!(
            error(&other),
            "the model has a table whose offsets are not in order"
        );
    }

    #[test]
    fn refuses_strings_that_are_not_sorted_or_not_utf8() {
        // `ab` and `aa` in this order, then the same string twice.
        for strings in [["ab", "aa"], ["aa", "aa"]] {
            let mut bytes = Vec::new();
            push_u32(&mut bytes, 2);
            push_u32(&mut bytes, 2);
            push_u32(&mut bytes, 4);
            for offset in [0, 2, 4] {
                push_u32(&mut bytes, offset);
            }
            bytes.extend_from_slice(strings[0].as_bytes());
            bytes.extend_from_slice(strings[1].as_bytes());
            // The values, then the slots.
            bytes.extend_from_slice(&[0; 4]);
            bytes.extend_from_slice(&[0; 16]);
            assert_eq!(
                error(&bytes),
                "the model has a table whose strings are not sorted"
            );
        }

        // Slots that are all empty, and slots that lead to the other
        // string: the last 16 bytes are the four slots of two strings.
        let good = bytes(&[("aa", [0, 0]), ("ab", [0, 0])]);
        let slots = good.len() - 16;
        let mut other = good.clone();
        other[slots..].fill(0);
        assert_eq!(
            error(&other),
            "the model has a table whose slots do not lead to its strings"
        );
        let mut other = good.clone();
        for slot in other[slots..].chunks_mut(4) {
            // 1 becomes 2 and 2 becomes 1; 0 stays.
            slot[0] = [0, 2, 1][slot[0] as usize];
        }
        assert_ne!(other, good);
        assert_eq!(
            error(&other),
            "the model has a table whose slots do not lead to its strings"
        );
        // Slots that are all taken: a string that is not there is not
        // looked for without end.
        let mut other = good.clone();
        for (index, slot) in other[slots..].chunks_mut(4).enumerate() {
            slot[0] = if slot[0] == 0 {
                index as u8 % 2 + 1
            } else {
                slot[0]
            };
        }
        let mut reader = Reader::new(&other, "model", Error::Model);
        let table = Layout::read(&mut reader, 2).unwrap().table(&other);
        assert_eq!(table.find("aa"), Some(0));
        assert_eq!(table.find("ab"), Some(1));
        assert_eq!(table.find("ac"), None);
        // A slot that leads past the strings is not the empty string.
        let mut other = good.clone();
        for slot in other[slots..].chunks_mut(4) {
            slot[0] = if slot[0] == 0 { 9 } else { slot[0] };
        }
        let mut reader = Reader::new(&other, "model", Error::Model);
        let table = Layout::read(&mut reader, 2).unwrap().table(&other);
        assert_eq!(table.find("ab"), Some(1));
        assert_eq!(table.find(""), None);

        let mut other = bytes(&[("aa", [0, 0])]);
        // The string is after the count, the width, the number of slots
        // and two offsets.
        other[20] = 0xff;
        assert_eq!(error(&other), "the model has a string that is not UTF-8");
    }
}
