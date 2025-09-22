use std::{io::Read, str::FromStr};

use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::error::{LinderaError, LinderaErrorKind};

#[derive(Debug, Clone, EnumIter, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)] // explicit representation for consistency
#[serde(rename_all = "lowercase")]
pub enum Algorithm {
    Deflate = 0,
    Zlib = 1,
    Gzip = 2,
    Raw = 3,
}

impl Algorithm {
    pub fn variants() -> Vec<Algorithm> {
        Algorithm::iter().collect::<Vec<_>>()
    }

    pub fn as_str(&self) -> &str {
        match self {
            Algorithm::Deflate => "deflate",
            Algorithm::Zlib => "zlib",
            Algorithm::Gzip => "gzip",
            Algorithm::Raw => "raw",
        }
    }
}

impl FromStr for Algorithm {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<Algorithm, Self::Err> {
        match input {
            "deflate" => Ok(Algorithm::Deflate),
            "zlib" => Ok(Algorithm::Zlib),
            "gzip" => Ok(Algorithm::Gzip),
            "raw" => Ok(Algorithm::Raw),
            _ => Err(LinderaErrorKind::Algorithm
                .with_error(anyhow::anyhow!("Invalid algorithm: {input}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedData {
    algorithm: Algorithm,
    data: Vec<u8>,
}

impl CompressedData {
    pub fn new(algorithm: Algorithm, data: Vec<u8>) -> Self {
        CompressedData { algorithm, data }
    }
}

pub fn decompress(data: CompressedData) -> anyhow::Result<Vec<u8>> {
    match data.algorithm {
        Algorithm::Deflate => {
            let mut decoder = DeflateDecoder::new(data.data.as_slice());
            let mut output_data = Vec::new();
            decoder.read_to_end(&mut output_data)?;
            Ok(output_data)
        }
        Algorithm::Zlib => {
            let mut decoder = ZlibDecoder::new(data.data.as_slice());
            let mut output_data = Vec::new();
            decoder.read_to_end(&mut output_data)?;
            Ok(output_data)
        }
        Algorithm::Gzip => {
            let mut decoder = GzDecoder::new(data.data.as_slice());
            let mut output_data = Vec::new();
            decoder.read_to_end(&mut output_data)?;
            Ok(output_data)
        }
        Algorithm::Raw => Ok(data.data),
    }
}
