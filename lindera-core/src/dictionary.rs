use std::borrow::Cow;

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
}
