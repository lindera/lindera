use serde::{Deserialize, Serialize};

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

fn algorithm_compression_ratio_estimation() -> f64 {
    unimplemented!()
}

pub fn compress(data: &[u8], algorithm: Algorithm) -> anyhow::Result<CompressedData> {
    match algorithm {
        Algorithm::LZMA { preset } => Ok(CompressedData {
            // TODO: バッファのサイズを意識する
            data: lzma::compress(data, preset)?,
            algorithm,
        }),
        Algorithm::Raw => Ok(CompressedData {
            data: data.to_vec(),
            algorithm,
        }),
        _ => {
            unimplemented!()
        }
    }
}

pub fn decompress(data: CompressedData) -> anyhow::Result<Vec<u8>> {
    match data.algorithm {
        Algorithm::LZMA { preset: _ } => Ok(lzma::decompress(&data.data)?), // TODO: バッファのサイズを意識する
        Algorithm::Raw => Ok(data.data),
        _ => {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    #[test]
    fn compress_decompress() {
        let mut rng = rand::thread_rng();
        let mut buf = Vec::new();

        for _i in 0..10000 {
            buf.push(rng.gen())
        }

        let compress_data = compress(&buf, Algorithm::LZMA { preset: 9 }).unwrap();

        let data = decompress(compress_data).unwrap();

        assert_eq!(&buf, &data);
    }
}
