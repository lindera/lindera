pub mod character_definition;
pub mod connection_cost_matrix;
pub mod prefix_dictionary;
pub mod unknown_dictionary;

use std::str;

use byteorder::{ByteOrder, LittleEndian};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

pub static UNK: Lazy<Vec<&str>> = Lazy::new(|| vec!["UNK"]);

#[derive(Clone)]
pub struct Dictionary {
    pub prefix_dictionary: PrefixDictionary,
    pub connection_cost_matrix: ConnectionCostMatrix,
    pub character_definition: CharacterDefinition,
    pub unknown_dictionary: UnknownDictionary,
}

impl Dictionary {
    pub fn word_details(&self, word_id: usize) -> Vec<&str> {
        if 4 * word_id >= self.prefix_dictionary.words_idx_data.len() {
            return vec![];
        }

        let idx: usize = match LittleEndian::read_u32(
            &self.prefix_dictionary.words_idx_data[4 * word_id..][..4],
        )
        .try_into()
        {
            Ok(value) => value,
            Err(_) => return UNK.to_vec(), // return empty vector if conversion fails
        };
        let data = &self.prefix_dictionary.words_data[idx..];
        let joined_details_len: usize = match LittleEndian::read_u32(data).try_into() {
            Ok(value) => value,
            Err(_) => return UNK.to_vec(), // return empty vector if conversion fails
        };
        let joined_details_bytes: &[u8] =
            &self.prefix_dictionary.words_data[idx + 4..idx + 4 + joined_details_len];

        let mut details = Vec::new();
        for bytes in joined_details_bytes.split(|&b| b == 0) {
            let detail = match str::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => return UNK.to_vec(), // return empty vector if conversion fails
            };
            details.push(detail);
        }
        details
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserDictionary {
    pub dict: PrefixDictionary,
}

impl UserDictionary {
    pub fn load(user_dict_data: &[u8]) -> LinderaResult<UserDictionary> {
        bincode::deserialize(user_dict_data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))
    }

    pub fn word_details(&self, word_id: usize) -> Vec<&str> {
        if 4 * word_id >= self.dict.words_idx_data.len() {
            return UNK.to_vec(); // return empty vector if conversion fails
        }
        let idx = LittleEndian::read_u32(&self.dict.words_idx_data[4 * word_id..][..4]);
        let data = &self.dict.words_data[idx as usize..];
        match bincode::deserialize(data) {
            Ok(details) => details,
            Err(_) => UNK.to_vec(), // return empty vector if conversion fails
        }
    }
}
