use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use encoding_rs::Encoding;
#[cfg(feature = "compress")]
use lindera_compress::compress;
use lindera_core::error::LinderaErrorKind;
use lindera_core::file_util::read_file;
use lindera_core::LinderaResult;
use lindera_decompress::Algorithm;

#[cfg(feature = "compress")]
pub fn compress_write<W: Write>(
    buffer: &[u8],
    algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    let compressed = compress(buffer, algorithm)
        .map_err(|err| LinderaErrorKind::Compress.with_error(anyhow::anyhow!(err)))?;
    bincode::serialize_into(writer, &compressed)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    Ok(())
}

#[cfg(not(feature = "compress"))]
pub fn compress_write<W: Write>(
    buffer: &[u8],
    _algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    writer
        .write_all(buffer)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

    Ok(())
}

pub fn read_file_with_encoding(filepath: &Path, encoding_name: &str) -> LinderaResult<String> {
    let encoding = Encoding::for_label_no_replacement(encoding_name.as_bytes());
    let encoding = encoding.ok_or_else(|| {
        LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", encoding_name))
    })?;

    let buffer = read_file(&filepath)?;
    Ok(encoding.decode(&buffer).0.into_owned())
}
