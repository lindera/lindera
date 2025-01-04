use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;

use flate2::Status;
#[cfg(feature = "memmap")]
use memmap2::Mmap;

use anyhow::anyhow;
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

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

#[cfg(feature = "memmap")]
pub fn memmap_file(filename: &Path) -> LinderaResult<Mmap> {
    let file = File::open(filename)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    Ok(mmap)
}

pub fn read_file_with_encoding(filepath: &Path, encoding_name: &str) -> LinderaResult<String> {
    let encoding = Encoding::for_label_no_replacement(encoding_name.as_bytes());
    let encoding = encoding.ok_or_else(|| {
        LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", encoding_name))
    })?;

    let buffer = read_file(filepath)?;
    Ok(encoding.decode(&buffer).0.into_owned())
}

#[derive(Serialize, Deserialize)]
pub enum Data {
    // serde is only used for user dict, which only uses the Vec variant, so this should be ok
    // we should probably customize this and call unreachable! to be safe
    #[serde(skip)]
    Static(&'static [u8]),
    Vec(Vec<u8>),
    #[cfg(feature = "memmap")]
    #[serde(skip)]
    Map(Mmap),
}

impl Deref for Data {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Data::Static(s) => s.deref(),
            Data::Vec(v) => v.deref(),
            #[cfg(feature = "memmap")]
            Data::Map(m) => m.deref(),
        }
    }
}

impl Into<Data> for &'static [u8] {
    fn into(self) -> Data {
        Data::Static(self)
    }
}

impl Into<Data> for Vec<u8> {
    fn into(self) -> Data {
        Data::Vec(self)
    }
}

#[cfg(feature = "memmap")]
impl Into<Data> for Mmap {
    fn into(self) -> Data {
        Data::Map(self)
    }
}
