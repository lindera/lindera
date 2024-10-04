use std::io::Read;

use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Algorithm {
    Deflate,
    Zlib,
    Gzip,
    Raw,
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
