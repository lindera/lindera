//! Common macros for dictionary data loading and decompression

/// Macro for decompressing dictionary data
/// This macro handles both compressed and uncompressed data formats
#[macro_export]
macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            use $crate::decompress::{CompressedData, decompress};

            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&$bytes[..]);
            match rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned) {
                Ok(compressed_data) => {
                    // Successfully decoded as CompressedData, now decompress it
                    match decompress(compressed_data) {
                        Ok(decompressed) => decompressed,
                        Err(_) => {
                            // Decompression failed, fall back to raw data
                            $bytes.to_vec()
                        }
                    }
                }
                Err(_) => {
                    // Not compressed data format, use as raw binary
                    $bytes.to_vec()
                }
            }
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}
