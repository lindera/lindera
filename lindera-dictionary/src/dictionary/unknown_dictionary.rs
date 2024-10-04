use std::str::FromStr;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::CategoryId;
use crate::error::LinderaErrorKind;
use crate::viterbi::{WordEntry, WordId};
use crate::LinderaResult;

#[derive(Serialize, Deserialize, Clone)]
pub struct UnknownDictionary {
    pub category_references: Vec<Vec<u32>>,
    pub costs: Vec<WordEntry>,
}

impl UnknownDictionary {
    pub fn load(unknown_data: &[u8]) -> LinderaResult<UnknownDictionary> {
        bincode::deserialize(unknown_data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))
    }

    pub fn word_entry(&self, word_id: u32) -> WordEntry {
        self.costs[word_id as usize]
    }

    pub fn lookup_word_ids(&self, category_id: CategoryId) -> &[u32] {
        &self.category_references[category_id.0][..]
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

fn make_costs_array(entries: &[UnknownDictionaryEntry]) -> Vec<WordEntry> {
    entries
        .iter()
        .map(|e| {
            // Do not perform strict checks on left context id and right context id in unk.def.
            // Just output a warning.
            if e.left_id != e.right_id {
                warn!("left id and right id are not same: {:?}", e);
            }
            WordEntry {
                word_id: WordId {
                    id: u32::MAX,
                    is_system: true,
                },
                left_id: e.left_id as u16,
                right_id: e.right_id as u16,
                word_cost: e.word_cost as i16,
            }
        })
        .collect()
}

pub fn parse_unk(
    categories: &[String],
    file_content: &str,
    expected_fields_len: usize,
) -> LinderaResult<UnknownDictionary> {
    let mut unknown_dict_entries = Vec::new();
    for line in file_content.lines() {
        let fields: Vec<&str> = line.split(',').collect::<Vec<&str>>();
        let entry = parse_dictionary_entry(&fields[..], expected_fields_len)?;
        unknown_dict_entries.push(entry);
    }

    let category_references = make_category_references(categories, &unknown_dict_entries[..]);
    let costs = make_costs_array(&unknown_dict_entries[..]);
    Ok(UnknownDictionary {
        category_references,
        costs,
    })
}

#[cfg(test)]
mod tests {}
