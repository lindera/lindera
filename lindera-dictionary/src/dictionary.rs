pub mod character_definition;
pub mod connection_cost_matrix;
pub mod prefix_dictionary;
pub mod unknown_dictionary;

use std::borrow::Cow;
use std::str;

use bincode::config::standard;
use bincode::serde::decode_from_slice;
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
    pub fn word_details<'a>(&'a self, word_id: usize) -> Vec<Cow<'a, str>> {
        if 4 * word_id >= self.prefix_dictionary.words_idx_data.len() {
            return vec![];
        }

        let idx: usize = match LittleEndian::read_u32(
            &self.prefix_dictionary.words_idx_data[4 * word_id..][..4],
        )
        .try_into()
        {
            Ok(value) => value,
            Err(_) => return UNK.iter().map(|s| Cow::Borrowed(*s)).collect(), // return empty vector if conversion fails
        };

        let data = &self.prefix_dictionary.words_data[idx..];
        let joined_details_len: usize = match LittleEndian::read_u32(data).try_into() {
            Ok(value) => value,
            Err(_) => return UNK.iter().map(|s| Cow::Borrowed(*s)).collect(),
        };

        let joined_details_bytes: &[u8] =
            &self.prefix_dictionary.words_data[idx + 4..idx + 4 + joined_details_len];

        let mut details = Vec::new();
        for bytes in joined_details_bytes.split(|&b| b == 0) {
            match str::from_utf8(bytes) {
                Ok(s) => details.push(Cow::Borrowed(s)),
                Err(_) => return UNK.iter().map(|s| Cow::Borrowed(*s)).collect(),
            };
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
        let (dict, _): (UserDictionary, _) = decode_from_slice(user_dict_data, standard())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;
        Ok(dict)
    }

    pub fn word_details<'a>(&'a self, word_id: usize) -> Vec<Cow<'a, str>> {
        if 4 * word_id >= self.dict.words_idx_data.len() {
            return UNK.iter().map(|s| Cow::Borrowed(*s)).collect();
        }

        let idx = LittleEndian::read_u32(&self.dict.words_idx_data[4 * word_id..][..4]);
        let data = &self.dict.words_data[idx as usize..];

        match decode_from_slice::<Vec<String>, _>(data, standard()) {
            Ok((details, _)) => details.into_iter().map(Cow::Owned).collect(), // `Vec<Cow<'a, str>>`
            Err(_) => UNK.iter().map(|s| Cow::Borrowed(*s)).collect(),
        }
    }
}
