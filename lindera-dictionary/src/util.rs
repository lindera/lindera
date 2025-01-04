use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::anyhow;
use encoding_rs::Encoding;

#[cfg(feature = "compress")]
use crate::compress::compress;
use crate::decompress::Algorithm;
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

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

pub fn read_file(filename: &Path) -> LinderaResult<Vec<u8>> {
    let mut input_read = File::open(filename)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let mut buffer = Vec::new();
    input_read
        .read_to_end(&mut buffer)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    Ok(buffer)
}

pub fn read_file_with_encoding(filepath: &Path, encoding_name: &str) -> LinderaResult<String> {
    let encoding = Encoding::for_label_no_replacement(encoding_name.as_bytes());
    let encoding = encoding.ok_or_else(|| {
        LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", encoding_name))
    })?;

    let buffer = read_file(filepath)?;
    Ok(encoding.decode(&buffer).0.into_owned())
}

// note: Cow is only used as an enum over Vec<u8> and &'static [u8]
//	copy-on-write capability is not used
pub type Data = Cow<'static, [u8]>;
