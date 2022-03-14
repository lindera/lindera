use serde::{Deserialize, Serialize};

use crate::error::LinderaErrorKind;
use crate::prefix_dict::PrefixDict;
use crate::LinderaResult;

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
