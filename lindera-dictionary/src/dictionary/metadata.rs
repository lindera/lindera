use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{LinderaResult, decompress::Algorithm, error::LinderaErrorKind};

#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub encoding: String,
    pub compress_algorithm: Algorithm,
    pub simple_userdic_fields_num: usize,
    pub simple_word_cost: i16,
    pub simple_context_id: u16,
    pub detailed_userdic_fields_num: usize,
    pub unk_fields_num: usize,
}

impl Default for Metadata {
    fn default() -> Self {
        // Default metadata values can be adjusted as needed
        Metadata::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
        )
    }
}

impl Metadata {
    pub fn new(
        encoding: String,
        compress_algorithm: Algorithm,
        simple_userdic_fields_num: usize,
        simple_word_cost: i16,
        simple_context_id: u16,
        detailed_userdic_fields_num: usize,
        unk_fields_num: usize,
    ) -> Self {
        Self {
            encoding,
            compress_algorithm,
            simple_userdic_fields_num,
            simple_word_cost,
            simple_context_id,
            detailed_userdic_fields_num,
            unk_fields_num,
        }
    }

    pub fn ipadic() -> Self {
        Self::new(
            "EUC-JP".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
        )
    }

    pub fn ipadic_neologd() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            13,
            11,
        )
    }

    pub fn unidic() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            21,
            10,
        )
    }

    pub fn ko_dic() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            12,
        )
    }

    pub fn cc_cedict() -> Self {
        Self::new(
            "UTF-8".to_string(),
            Algorithm::Deflate,
            3,
            -10000,
            0,
            12,
            10,
        )
    }
}
