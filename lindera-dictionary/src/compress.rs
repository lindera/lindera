use std::io::Write;

use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;

use crate::decompress::{Algorithm, CompressedData};

#[allow(dead_code)]
fn algorithm_compression_ratio_estimation() -> f64 {
    unimplemented!()
}

pub fn compress(data: &[u8], algorithm: Algorithm) -> anyhow::Result<CompressedData> {
    match algorithm {
        Algorithm::Deflate => {
            let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
            e.write_all(data)?;

            Ok(CompressedData::new(algorithm, e.finish()?))
        }
        Algorithm::Zlib => {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
            e.write_all(data)?;

            Ok(CompressedData::new(algorithm, e.finish()?))
        }
        Algorithm::Gzip => {
            let mut e = GzEncoder::new(Vec::new(), Compression::default());
            e.write_all(data)?;
            Ok(CompressedData::new(algorithm, e.finish()?))
        }
        Algorithm::Raw => Ok(CompressedData::new(algorithm, data.to_vec())),
    }
}

#[cfg(test)]
mod tests {
    use crate::decompress::decompress;

    use super::*;
    use rand::prelude::*;

    #[test]
    fn compress_decompress() {
        let mut rng = rand::rng();
        let mut buf = Vec::new();

        for _i in 0..10000 {
            buf.push(rng.random())
        }
        for _i in 0..10000 {
            buf.push(0)
        }

        let compress_data = compress(&buf, Algorithm::Deflate).unwrap();

        let data = decompress(compress_data).unwrap();

        assert_eq!(&buf, &data);
    }
}
