use std::str::FromStr;

use log::warn;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::dictionary::DetailFields;
use crate::dictionary::character_definition::CategoryId;
use crate::error::LinderaErrorKind;
use crate::util::joined_details_at;
use crate::viterbi::WordEntry;

#[derive(Serialize, Deserialize, Clone, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
    /// Byte offset for each entry's details in `words_data`.
    pub words_idx_data: Vec<u32>,
    /// Packed details bytes: for each entry, 4-byte LE length followed by
    /// NUL-separated detail fields (e.g. "名詞\0一般\0*\0...").
    pub words_data: Vec<u8>,
}

impl UnknownDictionary {
    pub fn load(unknown_data: &[u8]) -> LinderaResult<UnknownDictionary> {
        let mut aligned = rkyv::util::AlignedVec::<16>::new();
        aligned.extend_from_slice(unknown_data);
        rkyv::from_bytes::<UnknownDictionary, rkyv::rancor::Error>(&aligned).map_err(|err| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
        })
    }

    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        self.costs[word_id as usize]
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[u32] {
        &self.category_references[category_id.0][..]
    }

    /// Retrieve the detail fields (POS, etc.) for an unknown word entry.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The unknown-word entry id.
    ///
    /// # Returns
    ///
    /// A freshly allocated vector of the entry's fields, or `None` when the
    /// id is out of range or the entry is malformed. Prefer
    /// [`UnknownDictionary::word_details_iter`] on per-token paths, which
    /// yields the same fields without allocating.
    pub fn word_details(&self, word_id: u32) -> Option<Vec<&str>> {
        self.word_details_iter(word_id).map(Iterator::collect)
    }

    /// Yields the detail fields of an unknown-word entry, borrowed from this
    /// dictionary's own bytes.
    ///
    /// Unlike the packed dictionaries, this one distinguishes "no such entry"
    /// from "an entry with no fields", so the absence stays in the return
    /// type rather than collapsing into a sentinel; the caller decides what
    /// to substitute (see
    /// [`Dictionary::unknown_word_details_iter`](crate::dictionary::Dictionary::unknown_word_details_iter)).
    ///
    /// # Arguments
    ///
    /// * `word_id` - The unknown-word entry id.
    ///
    /// # Returns
    ///
    /// The entry's fields, or `None` when the id is out of range or the entry
    /// is malformed.
    #[inline]
    pub fn word_details_iter<'a>(&'a self, word_id: u32) -> Option<DetailFields<'a>> {
        // `words_idx_data` here is a `Vec<u32>`, not a packed byte table, so
        // the index lookup cannot share `util::words_idx_offset`; the blob
        // decode below is shared.
        let offset = *self.words_idx_data.get(word_id as usize)? as usize;
        joined_details_at(&self.words_data, offset).map(DetailFields::from_joined)
    }
}

#[derive(Debug)]
pub struct UnknownDictionaryEntry {
    pub surface: String,
    pub left_id: u32,
    pub right_id: u32,
    pub word_cost: i32,
}

fn parse_dictionary_entry(
    fields: &[&str],
    expected_fields_len: usize,
) -> LinderaResult<UnknownDictionaryEntry> {
    if fields.len() != expected_fields_len {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "Invalid number of fields. Expect {}, got {}",
            expected_fields_len,
            fields.len()
        )));
    }
    let surface = fields[0];
    let left_id = u32::from_str(fields[1])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
    let right_id = u32::from_str(fields[2])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
    let word_cost = i32::from_str(fields[3])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;

    Ok(UnknownDictionaryEntry {
        surface: surface.to_string(),
        left_id,
        right_id,
        word_cost,
    })
}

fn get_entry_id_matching_surface(
    entries: &[UnknownDictionaryEntry],
    target_surface: &str,
) -> Vec<u32> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(entry_id, entry)| {
            if entry.surface == *target_surface {
                Some(entry_id as u32)
            } else {
                None
            }
        })
        .collect()
}

fn make_category_references(
    categories: &[String],
    entries: &[UnknownDictionaryEntry],
) -> Vec<Vec<u32>> {
    categories
        .iter()
        .map(|category| get_entry_id_matching_surface(entries, category))
        .collect()
}

fn make_costs_array(
    entries: &[UnknownDictionaryEntry],
    remap: Option<&crate::dictionary::context_id_map::ContextIdMap>,
) -> Vec<WordEntry> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            // Do not perform strict checks on left context id and right context id in unk.def.
            // Just output a warning.
            if e.left_id != e.right_id {
                warn!("left id and right id are not same: {e:?}");
            }
            let (left_id, right_id) = (e.left_id as u16, e.right_id as u16);
            // Relabel to match the remapped connection matrix. An id out of range
            // of the matrix (and so of the remap) is rejected before parsing when
            // the dictionary builder knows the matrix size; `get().unwrap_or` only
            // keeps a caller without that check from panicking here.
            let (left_id, right_id) = match remap {
                Some(m) => (
                    m.left.get(left_id as usize).copied().unwrap_or(left_id),
                    m.right.get(right_id as usize).copied().unwrap_or(right_id),
                ),
                None => (left_id, right_id),
            };
            WordEntry::new(
                crate::viterbi::WordId::new(crate::viterbi::LexType::Unknown, i as u32),
                e.word_cost as i16,
                left_id,
                right_id,
            )
        })
        .collect()
}

pub fn parse_unk(
    categories: &[String],
    file_content: &str,
    remap: Option<&crate::dictionary::context_id_map::ContextIdMap>,
) -> LinderaResult<UnknownDictionary> {
    let mut unknown_dict_entries = Vec::new();
    let mut words_idx_data = Vec::new();
    let mut words_data: Vec<u8> = Vec::new();

    for line in file_content.lines() {
        let fields: Vec<&str> = line.split(',').collect::<Vec<&str>>();
        let entry = parse_dictionary_entry(&fields[..], fields.len())?;
        unknown_dict_entries.push(entry);

        // Store detail fields (columns after the first 4: category, left_id, right_id, cost)
        let offset = words_data.len() as u32;
        words_idx_data.push(offset);

        let details = if fields.len() > 4 {
            fields[4..].join("\0")
        } else {
            String::new()
        };
        let details_bytes = details.as_bytes();
        let len = details_bytes.len() as u32;
        words_data.extend_from_slice(&len.to_le_bytes());
        words_data.extend_from_slice(details_bytes);
    }

    let category_references = make_category_references(categories, &unknown_dict_entries[..]);
    let costs = make_costs_array(&unknown_dict_entries[..], remap);
    Ok(UnknownDictionary {
        category_references,
        costs,
        words_idx_data,
        words_data,
    })
}

#[cfg(test)]
mod tests {
    use super::UnknownDictionary;

    /// Builds a dictionary whose `words_data` holds `entries`, each encoded
    /// as a 4-byte LE length followed by its NUL-joined fields -- the layout
    /// `unknown_dictionary_from_reader` writes.
    fn with_entries(entries: &[&[&str]]) -> UnknownDictionary {
        let mut words_idx_data = Vec::new();
        let mut words_data = Vec::new();
        for fields in entries {
            words_idx_data.push(words_data.len() as u32);
            let joined = fields.join("\0");
            words_data.extend_from_slice(&(joined.len() as u32).to_le_bytes());
            words_data.extend_from_slice(joined.as_bytes());
        }
        UnknownDictionary {
            category_references: Vec::new(),
            costs: Vec::new(),
            words_idx_data,
            words_data,
        }
    }

    /// Fields come back in order, with the exact count that was written --
    /// the property the presized vector must not disturb.
    #[test]
    fn word_details_returns_fields_in_order() {
        let dict = with_entries(&[
            &["名詞", "一般", "*", "*", "*", "*", "*"],
            &["記号", "一般", "*", "*", "*", "*", "*"],
        ]);

        assert_eq!(
            dict.word_details(0),
            Some(vec!["名詞", "一般", "*", "*", "*", "*", "*"])
        );
        assert_eq!(
            dict.word_details(1),
            Some(vec!["記号", "一般", "*", "*", "*", "*", "*"])
        );
    }

    /// A single-field entry has no separator at all, so the capacity must
    /// still be 1 rather than 0.
    #[test]
    fn word_details_handles_single_field() {
        let dict = with_entries(&[&["ONLY"]]);
        assert_eq!(dict.word_details(0), Some(vec!["ONLY"]));
    }

    /// Empty fields are preserved rather than collapsed, including a leading
    /// and a trailing one.
    #[test]
    fn word_details_preserves_empty_fields() {
        let dict = with_entries(&[&["", "a", "", "b", ""]]);
        assert_eq!(dict.word_details(0), Some(vec!["", "a", "", "b", ""]));
    }

    /// An entry with no bytes at all still yields one empty field, matching
    /// `str::split`.
    #[test]
    fn word_details_empty_entry_yields_one_empty_field() {
        let dict = with_entries(&[&[]]);
        assert_eq!(dict.word_details(0), Some(vec![""]));
    }

    /// Out-of-range ids return `None` (the caller maps this to `UNK`).
    #[test]
    fn word_details_out_of_range_returns_none() {
        let dict = with_entries(&[&["名詞", "一般"]]);
        assert_eq!(dict.word_details(1), None);
        assert_eq!(dict.word_details(u32::MAX), None);
    }

    /// Invalid UTF-8 in the blob returns `None` rather than panicking.
    #[test]
    fn word_details_invalid_utf8_returns_none() {
        let mut dict = with_entries(&[&["ok"]]);
        // Overwrite the payload (after the 4-byte length) with a lone
        // continuation byte, which is never valid UTF-8.
        let payload = dict.words_data.len() - 2;
        dict.words_data[payload] = 0xff;
        assert_eq!(dict.word_details(0), None);
    }

    /// A declared length running past the buffer returns `None`.
    #[test]
    fn word_details_truncated_entry_returns_none() {
        let mut dict = with_entries(&[&["名詞", "一般"]]);
        // Past the end of the blob, but small enough that `offset + 4 + len`
        // cannot overflow a 32-bit `usize` (the crate builds for wasm32).
        let past_end = (dict.words_data.len() + 1) as u32;
        dict.words_data[0..4].copy_from_slice(&past_end.to_le_bytes());
        assert_eq!(dict.word_details(0), None);
    }
}
