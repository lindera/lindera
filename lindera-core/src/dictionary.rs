use serde::{Deserialize, Serialize};

use crate::{
    character_definition::CharacterDefinitions, connection::ConnectionCostMatrix,
    prefix_dict::PrefixDict, unknown_dictionary::UnknownDictionary,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Dictionary {
    pub dict: PrefixDict<Vec<u8>>,
    pub cost_matrix: ConnectionCostMatrix,
    pub char_definitions: CharacterDefinitions,
    pub unknown_dictionary: UnknownDictionary,
    pub words_idx_data: Vec<u8>,
    pub words_data: Vec<u8>,
}
