use std::borrow::Cow;

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
        let idx = LittleEndian::read_u32(&self.words_idx_data[4 * word_id..][..4]);
        let data = &self.words_data[idx as usize..];
        match bincode::deserialize_from(data) {
            Ok(details) => Some(details),
            Err(_err) => None,
        }
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
