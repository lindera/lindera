use std::str::FromStr;

use log::warn;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::dictionary::character_definition::CategoryId;
use crate::error::LinderaErrorKind;
use crate::viterbi::WordEntry;

#[derive(Serialize, Deserialize, Clone, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
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

    /// Unknown word generation with callback system
    pub fn gen_unk_words<F>(
        &self,
        sentence: &str,
        start_pos: usize,
        has_matched: bool,
        max_grouping_len: Option<usize>,
        mut callback: F,
    ) where
        F: FnMut(UnkWord),
    {
        let chars: Vec<char> = sentence.chars().collect();
        let max_len = max_grouping_len.unwrap_or(10);

        // Limit based on dictionary matches for efficiency
        let actual_max_len = if has_matched { 1 } else { max_len.min(3) };

        for length in 1..=actual_max_len {
            if start_pos + length > chars.len() {
                break;
            }

            let end_pos = start_pos + length;

            // Classify character type for unknown word
            let first_char = chars[start_pos];
            let char_type = classify_char_type(first_char);

            // Create unknown word entry
            let unk_word = UnkWord {
                word_idx: WordIdx::new(char_type as u32),
                end_char: end_pos,
            };

            callback(unk_word);
        }
    }

    /// Check compatibility with unknown word based on feature matching
    pub fn compatible_unk_index(
        &self,
        sentence: &str,
        start: usize,
        _end: usize,
        feature: &str,
    ) -> Option<WordIdx> {
        let chars: Vec<char> = sentence.chars().collect();
        if start >= chars.len() {
            return None;
        }

        let first_char = chars[start];
        let char_type = classify_char_type(first_char);

        // Simple compatibility check based on feature string
        if feature.starts_with(&format!("名詞,{}", get_type_name(char_type))) {
            Some(WordIdx::new(char_type as u32))
        } else {
            None
        }
    }
}

/// Unknown word structure for callback system
#[derive(Debug, Clone)]
pub struct UnkWord {
    pub word_idx: WordIdx,
    pub end_char: usize,
}

impl UnkWord {
    pub fn word_idx(&self) -> WordIdx {
        self.word_idx
    }

    pub fn end_char(&self) -> usize {
        self.end_char
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WordIdx {
    pub word_id: u32,
}

impl WordIdx {
    pub fn new(word_id: u32) -> Self {
        Self { word_id }
    }
}

/// Classify character type (compatible with existing system)
fn classify_char_type(ch: char) -> usize {
    if ch.is_ascii_digit() {
        5 // NUMERIC
    } else if ch.is_ascii_alphabetic() {
        4 // ALPHA
    } else if is_kanji(ch) {
        3 // KANJI
    } else if is_katakana(ch) {
        2 // KATAKANA
    } else if is_hiragana(ch) {
        1 // HIRAGANA
    } else {
        0 // DEFAULT
    }
}

fn get_type_name(char_type: usize) -> &'static str {
    match char_type {
        1 => "一般",
        2 => "一般",
        3 => "一般",
        4 => "固有名詞",
        5 => "数",
        _ => "一般",
    }
}

/// Character classification helpers
fn is_hiragana(ch: char) -> bool {
    matches!(ch, '\u{3041}'..='\u{3096}')
}

fn is_katakana(ch: char) -> bool {
    matches!(ch, '\u{30A1}'..='\u{30F6}' | '\u{30F7}'..='\u{30FA}' | '\u{31F0}'..='\u{31FF}')
}

fn is_kanji(ch: char) -> bool {
    matches!(ch, '\u{4E00}'..='\u{9FAF}' | '\u{3400}'..='\u{4DBF}')
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

fn make_costs_array(entries: &[UnknownDictionaryEntry]) -> Vec<WordEntry> {
    entries
        .iter()
        .map(|e| {
            // Do not perform strict checks on left context id and right context id in unk.def.
            // Just output a warning.
            if e.left_id != e.right_id {
                warn!("left id and right id are not same: {e:?}");
            }
            WordEntry {
                word_id: crate::viterbi::WordId::new(crate::viterbi::LexType::Unknown, u32::MAX),
                left_id: e.left_id as u16,
                right_id: e.right_id as u16,
                word_cost: e.word_cost as i16,
            }
        })
        .collect()
}

pub fn parse_unk(categories: &[String], file_content: &str) -> LinderaResult<UnknownDictionary> {
    let mut unknown_dict_entries = Vec::new();
    for line in file_content.lines() {
        let fields: Vec<&str> = line.split(',').collect::<Vec<&str>>();
        let entry = parse_dictionary_entry(&fields[..], fields.len())?;
        unknown_dict_entries.push(entry);
    }

    let category_references = make_category_references(categories, &unknown_dict_entries[..]);
    let costs = make_costs_array(&unknown_dict_entries[..]);
    Ok(UnknownDictionary {
        category_references,
        costs,
    })
}

impl ArchivedUnknownDictionary {
    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        // We have to deserialize the single entry or extract fields.
        // Simple Archive usually preserves layout for primitives.
        // Using deserialize ensures we get the native struct.
        // Since WordEntry is small and Copy, this is efficient enough.
        let archived_entry = &self.costs[word_id as usize];
        rkyv::deserialize::<WordEntry, rkyv::rancor::Error>(archived_entry).unwrap()
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[rkyv::rend::u32_le] {
        self.category_references[category_id.0].as_slice()
    }
}
