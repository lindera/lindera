use std::str::FromStr;

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

/// Parses the leading columns of one `unk.def` line: the category, the left
/// and right context ids and the cost.
///
/// # Arguments
///
/// * `fields` - The comma-separated columns of the line.
/// * `source` - The name of the content, for the messages.
/// * `line_number` - The 1-based number of the line, for the messages.
///
/// # Returns
///
/// The entry, or an error naming the source and the line when the line has
/// fewer than four columns or a context id or the cost does not parse.
fn parse_dictionary_entry(
    fields: &[&str],
    source: &str,
    line_number: usize,
) -> LinderaResult<UnknownDictionaryEntry> {
    let [surface, left_id, right_id, word_cost, ..] = fields else {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "{source}: line {line_number}: expected at least 4 fields (category, left context id, \
             right context id, cost), got {}",
            fields.len()
        )));
    };
    let invalid = |column: &str, value: &str, err: std::num::ParseIntError| {
        LinderaErrorKind::Parse.with_error(anyhow::anyhow!(
            "{source}: line {line_number}: invalid {column} `{value}`: {err}"
        ))
    };
    let left_id = u32::from_str(left_id).map_err(|err| invalid("left context id", left_id, err))?;
    let right_id =
        u32::from_str(right_id).map_err(|err| invalid("right context id", right_id, err))?;
    let word_cost = i32::from_str(word_cost).map_err(|err| invalid("cost", word_cost, err))?;

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

/// Makes the word entries of the unknown-word dictionary, one per `unk.def`
/// line, in the order of the lines.
///
/// The left and right context ids of a line are kept apart, so they may
/// differ, as `mecab-dict-index` allows: every line of mecab-ko-dic's
/// `unk.def` has different ones.
///
/// # Arguments
///
/// * `entries` - The parsed `unk.def` lines.
/// * `remap` - The relabeling of the connection matrix's context ids, if the
///   dictionary has one.
///
/// # Returns
///
/// The word entries.
fn make_costs_array(
    entries: &[UnknownDictionaryEntry],
    remap: Option<&crate::dictionary::context_id_map::ContextIdMap>,
) -> Vec<WordEntry> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
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

/// Parses the content of an `unk.def` file into an unknown-word dictionary.
///
/// Every line is an entry: the category, the left and right context ids and
/// the cost, then the details. Comment and empty lines are not skipped; as
/// in MeCab, they are errors.
///
/// # Arguments
///
/// * `categories` - The `char.def` category names, in category id order.
/// * `file_content` - The decoded `unk.def` content.
/// * `remap` - The context-id remap to relabel the entries with, if any.
///
/// # Returns
///
/// The dictionary, whose entry `i` is the line `i + 1`, or an error naming
/// the line (`unk.def: line N: ...`) when a line has fewer than four fields
/// or a context id or the cost is not a number.
pub fn parse_unk(
    categories: &[String],
    file_content: &str,
    remap: Option<&crate::dictionary::context_id_map::ContextIdMap>,
) -> LinderaResult<UnknownDictionary> {
    parse_unk_named("unk.def", categories, file_content, remap)
}

/// [`parse_unk`] with the name that its error messages give the content, so
/// that the dictionary builder can name the file it read.
///
/// # Arguments
///
/// * `source` - The name of the content, such as the path of the file.
/// * `categories` - The `char.def` category names, in category id order.
/// * `file_content` - The decoded `unk.def` content.
/// * `remap` - The context-id remap to relabel the entries with, if any.
///
/// # Returns
///
/// The dictionary, or an error naming `source` and the line.
pub(crate) fn parse_unk_named(
    source: &str,
    categories: &[String],
    file_content: &str,
    remap: Option<&crate::dictionary::context_id_map::ContextIdMap>,
) -> LinderaResult<UnknownDictionary> {
    let mut unknown_dict_entries = Vec::new();
    let mut words_idx_data = Vec::new();
    let mut words_data: Vec<u8> = Vec::new();

    for (index, line) in file_content.lines().enumerate() {
        let fields: Vec<&str> = line.split(',').collect::<Vec<&str>>();
        let entry = parse_dictionary_entry(&fields[..], source, index + 1)?;
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

/// Checks the categories of `unk.def` against those of `char.def`, as
/// `mecab-dict-index` does: every line must name a `char.def` category, and
/// every `char.def` category must have at least one line.
///
/// A line whose category `char.def` does not define is never used. A
/// category without a line creates no unknown word, so a character of it
/// that no dictionary word covers leaves the lattice without a path, and the
/// segmenter outputs only the part of the text before it. [`parse_unk`]
/// accepts both; the dictionary builder and the trainer call this function
/// after it.
///
/// # Arguments
///
/// * `source` - The name of the `unk.def` content, such as the path of the
///   file, for the messages.
/// * `categories` - The `char.def` category names, in category id order.
/// * `lines` - The `unk.def` entry lines with their 1-based line numbers.
///   The category is the text before the first comma.
///
/// # Returns
///
/// `Ok(())`, or an error for the first line, in line order, whose category
/// `char.def` does not define (`{source}: line N: category X is not defined
/// in char.def`), else for the first category, in category id order, without
/// a line (`{source}: no line for category X, ...`), which can name no line.
pub fn check_unknown_word_categories<'a>(
    source: &str,
    categories: &[String],
    lines: impl IntoIterator<Item = (usize, &'a str)>,
) -> LinderaResult<()> {
    let mut has_line = vec![false; categories.len()];
    for (line_number, line) in lines {
        let category = line.split(',').next().unwrap_or_default();
        let Some(category_id) = categories.iter().position(|name| name == category) else {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: line {line_number}: category {category} is not defined in char.def"
            )));
        };
        has_line[category_id] = true;
    }
    if let Some(category_id) = has_line.iter().position(|&found| !found) {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "{source}: no line for category {}, which char.def defines; every char.def \
             category needs at least one unk.def line",
            categories[category_id]
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{UnknownDictionary, check_unknown_word_categories, parse_unk};

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

    /// The categories of the `parse_unk` tests' `char.def`.
    fn categories() -> Vec<String> {
        vec!["DEFAULT".to_string(), "KANJI".to_string()]
    }

    /// Parses `content`, which must be rejected, and returns the message.
    fn parse_error(content: &str) -> String {
        match parse_unk(&categories(), content, None) {
            Ok(_) => panic!("unk.def content was accepted: {content:?}"),
            Err(err) => err.to_string(),
        }
    }

    /// #1175: a line too short to hold the category, the context ids and the
    /// cost is an error naming the line, not an index out of bounds. A
    /// comment and an empty line are such lines, as in MeCab.
    #[test]
    fn parse_unk_rejects_a_line_with_fewer_than_four_fields() {
        for (line, fields) in [("# a comment", 1), ("", 1), ("KANJI,1,1", 3)] {
            let message = parse_error(&format!("DEFAULT,0,0,0,名詞,一般\n{line}\n"));
            assert!(
                message.contains(&format!(
                    "unk.def: line 2: expected at least 4 fields (category, left context id, \
                     right context id, cost), got {fields}"
                )),
                "{line:?}: {message}"
            );
        }
    }

    /// A context id or a cost that is not a number is an error naming the
    /// line and the column.
    #[test]
    fn parse_unk_names_the_line_of_a_field_that_is_not_a_number() {
        for (line, column) in [
            ("KANJI,x,1,0,名詞", "left context id `x`"),
            ("KANJI,1,-1,0,名詞", "right context id `-1`"),
            ("KANJI,1,1,1.5,名詞", "cost `1.5`"),
        ] {
            let message = parse_error(&format!("DEFAULT,0,0,0,名詞,一般\n{line}\n"));
            assert!(
                message.contains(&format!("unk.def: line 2: invalid {column}")),
                "{line:?}: {message}"
            );
        }
    }

    /// Four fields are enough: the entry has no details.
    #[test]
    fn parse_unk_accepts_a_line_with_four_fields() {
        let dict = parse_unk(
            &categories(),
            "DEFAULT,0,0,0,名詞,一般\nKANJI,1,1,10\n",
            None,
        )
        .expect("four fields are enough");

        assert_eq!(dict.category_references, vec![vec![0], vec![1]]);
        assert_eq!(dict.word_details(0), Some(vec!["名詞", "一般"]));
        assert_eq!(dict.word_details(1), Some(vec![""]));
    }

    /// Checks `content`'s lines, numbered from 1, against [`categories`].
    fn check(content: &str) -> Result<(), String> {
        check_unknown_word_categories(
            "unk.def",
            &categories(),
            content
                .lines()
                .enumerate()
                .map(|(index, line)| (index + 1, line)),
        )
        .map_err(|err| err.to_string())
    }

    /// Every line names a category, and every category has a line; a
    /// category can have several.
    #[test]
    fn check_unknown_word_categories_accepts_matching_categories() {
        assert_eq!(
            check("DEFAULT,0,0,0,名詞,一般\nKANJI,1,1,10,名詞,一般\nKANJI,2,2,20,名詞,固有名詞\n"),
            Ok(())
        );
    }

    /// #1195: a line whose category `char.def` does not define is an error
    /// naming the line and the category, as in `mecab-dict-index`.
    #[test]
    fn check_unknown_word_categories_rejects_an_undefined_category() {
        let message = check("DEFAULT,0,0,0,名詞,一般\nKANJI,1,1,10,名詞\nFOO,1,1,10,名詞\n")
            .expect_err("FOO is not a char.def category");

        assert!(
            message.contains("unk.def: line 3: category FOO is not defined in char.def"),
            "{message}"
        );
    }

    /// #1195: a `char.def` category without a line is an error naming the
    /// category, as in `mecab-dict-index`. No line can be named.
    #[test]
    fn check_unknown_word_categories_rejects_a_category_without_a_line() {
        let message = check("DEFAULT,0,0,0,名詞,一般\n").expect_err("KANJI has no line");

        assert!(
            message.contains(
                "unk.def: no line for category KANJI, which char.def defines; every char.def \
                 category needs at least one unk.def line"
            ),
            "{message}"
        );
    }

    /// An undefined category is reported first, with its line, when a
    /// category also lacks a line.
    #[test]
    fn check_unknown_word_categories_reports_the_undefined_category_first() {
        let message = check("FOO,0,0,0,名詞,一般\n").expect_err("both are wrong");

        assert!(
            message.contains("unk.def: line 1: category FOO is not defined in char.def"),
            "{message}"
        );
    }
}
