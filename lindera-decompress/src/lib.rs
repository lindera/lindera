use lzma_rs::xz_decompress;
use serde::{Deserialize, Serialize};
use std::io::BufReader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Algorithm {
    Bzip,
    LZ77,
    LZMA { preset: u32 },
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
        Algorithm::LZMA { preset: _ } => {
            let mut buf_reader = BufReader::new(data.data.as_slice());
            let mut output_data = Vec::new();
            xz_decompress(&mut buf_reader, &mut output_data)?;
            Ok(output_data)
        } // TODO: バッファのサイズを意識する
        Algorithm::Raw => Ok(data.data),
        _ => {
            unimplemented!()
        }
    }
}
