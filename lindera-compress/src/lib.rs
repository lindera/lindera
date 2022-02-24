pub use lindera_decompress::{Algorithm, CompressedData};
#[cfg(windows)]
use lzma_rs::xz_compress;
#[cfg(windows)]
use std::io::BufReader;

#[allow(dead_code)]
fn algorithm_compression_ratio_estimation() -> f64 {
    unimplemented!()
}

#[cfg(not(windows))]
pub fn compress(data: &[u8], algorithm: Algorithm) -> anyhow::Result<CompressedData> {
    match algorithm {
        Algorithm::LZMA { preset } => {
            let output_data = lzma::compress(data, preset)?;

            Ok(CompressedData::new(algorithm, output_data))
        }
        Algorithm::Raw => Ok(CompressedData::new(algorithm, data.to_vec())),
        _ => {
            unimplemented!()
        }
    }
}

#[cfg(windows)]
pub fn compress(data: &[u8], algorithm: Algorithm) -> anyhow::Result<CompressedData> {
    match algorithm {
        Algorithm::LZMA { preset: _ } => {
            let mut buf_reader = BufReader::new(data);
            let mut output_data = Vec::new();
            xz_compress(&mut buf_reader, &mut output_data)?;
            Ok(CompressedData::new(algorithm, output_data))
        }
        Algorithm::Raw => Ok(CompressedData::new(algorithm, data.to_vec())),
        _ => {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lindera_decompress::decompress;
    use rand::prelude::*;

    #[test]
    fn compress_decompress() {
        let mut rng = rand::thread_rng();
        let mut buf = Vec::new();

        for _i in 0..10000 {
            buf.push(rng.gen())
        }
        for _i in 0..10000 {
            buf.push(0)
        }

        let compress_data = compress(&buf, Algorithm::LZMA { preset: 9 }).unwrap();

        let data = decompress(compress_data).unwrap();

        assert_eq!(&buf, &data);
    }
}
