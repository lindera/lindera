use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;

#[cfg(feature = "mmap")]
use memmap2::Mmap;

use anyhow::anyhow;
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
#[cfg(feature = "compress")]
use crate::compress::compress;
use crate::decompress::Algorithm;
use crate::error::LinderaErrorKind;

#[cfg(feature = "compress")]
pub fn compress_write<W: Write>(
    buffer: &[u8],
    algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    let compressed = compress(buffer, algorithm).map_err(|err| {
        LinderaErrorKind::Compression
            .with_error(err)
            .add_context(format!(
                "Failed to compress data with {:?} algorithm",
                algorithm
            ))
    })?;
    bincode::serde::encode_into_std_write(&compressed, writer, bincode::config::legacy()).map_err(
        |err| {
            LinderaErrorKind::Io
                .with_error(err)
                .add_context("Failed to write compressed data to output")
        },
    )?;

    Ok(())
}

#[cfg(not(feature = "compress"))]
pub fn compress_write<W: Write>(
    buffer: &[u8],
    _algorithm: Algorithm,
    writer: &mut W,
) -> LinderaResult<()> {
    writer.write_all(buffer).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(err)
            .add_context("Failed to write data to output")
    })?;

    Ok(())
}

pub fn read_file(filename: &Path) -> LinderaResult<Vec<u8>> {
    let mut input_read = File::open(filename).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(err)
            .add_context(format!("Failed to open file: {}", filename.display()))
    })?;
    let mut buffer = Vec::new();
    input_read.read_to_end(&mut buffer).map_err(|err| {
        LinderaErrorKind::Io.with_error(err).add_context(format!(
            "Failed to read file contents: {}",
            filename.display()
        ))
    })?;
    Ok(buffer)
}

#[cfg(feature = "mmap")]
pub fn mmap_file(filename: &Path) -> LinderaResult<Mmap> {
    let file = File::open(filename).map_err(|err| {
        LinderaErrorKind::Io.with_error(err).add_context(format!(
            "Failed to open file for memory mapping: {}",
            filename.display()
        ))
    })?;
    let mmap = unsafe { Mmap::map(&file) }.map_err(|err| {
        LinderaErrorKind::Io
            .with_error(err)
            .add_context(format!("Failed to memory map file: {}", filename.display()))
    })?;
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

pub enum Data {
    Static(&'static [u8]),
    Vec(Vec<u8>),
    #[cfg(feature = "mmap")]
    Map(Mmap),
}

impl Deref for Data {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Data::Static(s) => s,
            Data::Vec(v) => v,
            #[cfg(feature = "mmap")]
            Data::Map(m) => m,
        }
    }
}

impl From<&'static [u8]> for Data {
    fn from(s: &'static [u8]) -> Self {
        Self::Static(s)
    }
}

impl<T: Deref<Target = [u8]>> From<&'static T> for Data {
    fn from(t: &'static T) -> Self {
        Self::Static(t)
    }
}

impl From<Vec<u8>> for Data {
    fn from(v: Vec<u8>) -> Self {
        Self::Vec(v)
    }
}

#[cfg(feature = "mmap")]
impl From<Mmap> for Data {
    fn from(m: Mmap) -> Self {
        Self::Map(m)
    }
}

impl Clone for Data {
    fn clone(&self) -> Self {
        match self {
            Data::Static(s) => Data::Static(s),
            Data::Vec(v) => Data::Vec(v.clone()),
            #[cfg(feature = "mmap")]
            Data::Map(m) => Data::Vec(m.to_vec()),
        }
    }
}

impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Static(s) => serializer.serialize_bytes(s),
            Self::Vec(v) => serializer.serialize_bytes(v),
            #[cfg(feature = "mmap")]
            Self::Map(m) => serializer.serialize_bytes(m),
        }
    }
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer).map(Self::Vec)
    }
}
