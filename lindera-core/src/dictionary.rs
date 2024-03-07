use std::borrow::Cow;
use std::str;

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use crate::{
    character_definition::CharacterDefinitions, connection::ConnectionCostMatrix,
    error::LinderaErrorKind, prefix_dict::PrefixDict, unknown_dictionary::UnknownDictionary,
    LinderaResult,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Dictionary {
    pub dict: PrefixDict<Vec<u8>>,
    pub cost_matrix: ConnectionCostMatrix,
    pub char_definitions: CharacterDefinitions,
    pub unknown_dictionary: UnknownDictionary,
    pub words_idx_data: Cow<'static, [u8]>,
    pub words_data: Cow<'static, [u8]>,
}

impl Dictionary {
    pub fn word_details(&self, word_id: usize) -> Option<Vec<String>> {
        if 4 * word_id >= self.words_idx_data.len() {
            return None;
        }
        let idx: usize = LittleEndian::read_u32(&self.words_idx_data[4 * word_id..][..4])
            .try_into()
            .ok()?;
        let data = &self.words_data[idx..];
        let joined_details_len: usize = LittleEndian::read_u32(data).try_into().ok()?;
        let joined_details_bytes: &[u8] = &self.words_data[idx + 4..idx + 4 + joined_details_len];

        let mut details = Vec::new();
        for bytes in joined_details_bytes.split(|&b| b == 0) {
            let detail = str::from_utf8(bytes).ok()?.to_string();
            details.push(detail);
        }
        Some(details)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserDictionary {
    pub dict: PrefixDict<Vec<u8>>,
    pub words_idx_data: Vec<u8>,
    pub words_data: Vec<u8>,
}

impl UserDictionary {
    pub fn load(user_dict_data: &[u8]) -> LinderaResult<UserDictionary> {
        bincode::deserialize(user_dict_data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))
    }

    pub fn word_details(&self, word_id: usize) -> Option<Vec<String>> {
        if 4 * word_id >= self.words_idx_data.len() {
            return None;
        }
        let idx = LittleEndian::read_u32(&self.words_idx_data[4 * word_id..][..4]);
        let data = &self.words_data[idx as usize..];
        match bincode::deserialize_from(data) {
            Ok(details) => Some(details),
            Err(_err) => None,
        }
    }
}
