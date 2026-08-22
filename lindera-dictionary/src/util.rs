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
use crate::error::LinderaErrorKind;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// Counts the fields a NUL-joined detail blob splits into.
///
/// The dictionary builder joins a row's detail fields with `"\0"` and writes
/// no trailing separator, so the field count is exactly one more than the
/// number of NUL bytes. Callers use this to size their detail vectors up
/// front instead of letting them grow from capacity zero, which cost two
/// reallocations per token for a 9-field dictionary (#966).
///
/// # 引数
///
/// * `joined_details` - The NUL-joined detail blob.
///
/// # 戻り値
///
/// The number of fields, always at least 1 (an empty blob splits into a
/// single empty field, matching `slice::split`).
#[inline]
pub(crate) fn detail_field_count(joined_details: &[u8]) -> usize {
    memchr::memchr_iter(0, joined_details).count() + 1
}

/// Reads the `words_data` byte offset a word id maps to.
///
/// Both packed dictionaries store `words_idx_data` as one little-endian `u32`
/// per word id. Every step is bounds-checked, so a corrupt archive or an
/// absurd id ends in `None` instead of a panic -- the accessors used to slice
/// this unchecked (#966).
///
/// # 引数
///
/// * `words_idx_data` - The word-id index table.
/// * `word_id` - The word id to look up.
///
/// # 戻り値
///
/// The entry's byte offset into `words_data`, or `None` when the id has no
/// slot in the table. Callers map `None` onto their own missing-entry
/// fallback, which differs per dictionary.
#[inline]
pub(crate) fn words_idx_offset(words_idx_data: &[u8], word_id: usize) -> Option<usize> {
    let start = word_id.checked_mul(4)?;
    let bytes = words_idx_data.get(start..start.checked_add(4)?)?;
    let offset: [u8; 4] = bytes.try_into().ok()?;
    Some(u32::from_le_bytes(offset) as usize)
}

/// Locates and validates one entry's NUL-joined detail blob.
///
/// Each entry is a 4-byte little-endian length followed by that many bytes of
/// NUL-joined fields. The whole blob is validated once rather than field by
/// field: NUL is ASCII and therefore never occurs inside a multi-byte UTF-8
/// sequence, so "the blob is valid UTF-8" and "every field is valid UTF-8"
/// are the same statement. One `from_utf8` call replaces one per field, and
/// the resulting `&str` can be split without re-validating.
///
/// # 引数
///
/// * `words_data` - The packed detail records.
/// * `offset` - The entry's byte offset inside `words_data`.
///
/// # 戻り値
///
/// The entry's joined fields, or `None` when the offset, the declared length
/// or the payload's encoding is invalid.
#[inline]
pub(crate) fn joined_details_at(words_data: &[u8], offset: usize) -> Option<&str> {
    let header: [u8; 4] = words_data
        .get(offset..offset.checked_add(4)?)?
        .try_into()
        .ok()?;
    let start = offset + 4;
    let len = u32::from_le_bytes(header) as usize;
    let bytes = words_data.get(start..start.checked_add(len)?)?;
    str::from_utf8(bytes).ok()
}

/// Write data directly to the writer.
pub fn write_data<W: Write>(buffer: &[u8], writer: &mut W) -> LinderaResult<()> {
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

/// Reads a file into a 16-byte aligned buffer, as required when loading rkyv
/// archives (e.g. `char_def.bin`, `unk.bin`).
pub fn read_aligned_file(filename: &Path) -> LinderaResult<rkyv::util::AlignedVec<16>> {
    let raw_data = read_file(filename)?;

    let mut aligned_data = rkyv::util::AlignedVec::<16>::new();
    aligned_data.extend_from_slice(&raw_data);

    Ok(aligned_data)
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
        LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {encoding_name}"))
    })?;

    let buffer = read_file(filepath)?;
    Ok(encoding.decode(&buffer).0.into_owned())
}

use std::sync::Arc;

#[derive(Clone)]
pub enum Data {
    Static(&'static [u8]),
    Vec(Vec<u8>),
    #[cfg(feature = "mmap")]
    Map(Arc<Mmap>),
}

impl Archive for Data {
    type Archived = rkyv::vec::ArchivedVec<u8>;
    type Resolver = rkyv::vec::VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        rkyv::vec::ArchivedVec::resolve_from_slice(self.deref(), resolver, out);
    }
}

impl<S> RkyvSerialize<S> for Data
where
    S: rkyv::rancor::Fallible + rkyv::ser::Writer + rkyv::ser::Allocator + ?Sized,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        rkyv::vec::ArchivedVec::serialize_from_slice(self.deref(), serializer)
    }
}

impl<D: rkyv::rancor::Fallible + ?Sized> RkyvDeserialize<Data, D> for rkyv::vec::ArchivedVec<u8> {
    fn deserialize(&self, _deserializer: &mut D) -> Result<Data, D::Error> {
        let mut vec = Vec::with_capacity(self.len());
        vec.extend_from_slice(self.as_slice());
        Ok(Data::Vec(vec))
    }
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
        Self::Map(Arc::new(m))
    }
}

impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.deref())
    }
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Data::Vec(v))
    }
}

#[cfg(test)]
mod tests {
    use super::{detail_field_count, joined_details_at, words_idx_offset};

    /// The count must match what `slice::split` actually yields, which is the
    /// invariant the presized detail vectors rely on.
    #[test]
    fn detail_field_count_matches_split() {
        for blob in [
            &b""[..],
            &b"a"[..],
            &b"a\0b"[..],
            &b"\0"[..],
            &b"\0\0"[..],
            &b"a\0\0b"[..],
            &b"NOUN\0general\0*\0*\0*\0*\0base\0reading\0pron"[..],
        ] {
            assert_eq!(
                detail_field_count(blob),
                blob.split(|&b| b == 0).count(),
                "blob {blob:?}"
            );
        }
    }

    /// An empty blob still splits into one (empty) field, so the count is
    /// never zero -- a zero capacity would reintroduce the growth this
    /// helper exists to avoid.
    #[test]
    fn detail_field_count_is_never_zero() {
        assert_eq!(detail_field_count(b""), 1);
    }

    /// The IPADIC shape: 9 fields joined by 8 separators.
    #[test]
    fn detail_field_count_ipadic_shape() {
        let blob = b"\xe5\x90\x8d\xe8\xa9\x9e\0*\0*\0*\0*\0*\0a\0b\0c";
        assert_eq!(detail_field_count(blob), 9);
    }

    /// Builds a `words_data` blob holding `entries`, each encoded as a 4-byte
    /// LE length followed by its NUL-joined fields, and returns it with each
    /// entry's offset.
    fn packed(entries: &[&[&str]]) -> (Vec<u8>, Vec<usize>) {
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for fields in entries {
            offsets.push(data.len());
            let joined = fields.join("\0");
            data.extend_from_slice(&(joined.len() as u32).to_le_bytes());
            data.extend_from_slice(joined.as_bytes());
        }
        (data, offsets)
    }

    /// Each entry decodes to its own blob, not to the tail of the buffer.
    #[test]
    fn joined_details_at_reads_the_declared_length() {
        let (data, offsets) = packed(&[&["名詞", "一般"], &["動詞", "自立", "*"]]);

        assert_eq!(joined_details_at(&data, offsets[0]), Some("名詞\0一般"));
        assert_eq!(joined_details_at(&data, offsets[1]), Some("動詞\0自立\0*"));
    }

    /// A truncated header, an offset past the end, and a declared length that
    /// runs past the buffer all yield `None` rather than panicking. The
    /// accessors used to slice this unchecked.
    #[test]
    fn joined_details_at_rejects_out_of_range_offsets_and_lengths() {
        let (mut data, offsets) = packed(&[&["名詞", "一般"]]);

        // Offset past the end, and an offset whose 4-byte header straddles it.
        assert_eq!(joined_details_at(&data, data.len()), None);
        assert_eq!(joined_details_at(&data, data.len() - 2), None);
        // An offset so large that `offset + 4` would overflow.
        assert_eq!(joined_details_at(&data, usize::MAX), None);

        // Declared length running past the buffer.
        let past_end = (data.len() + 1) as u32;
        data[offsets[0]..offsets[0] + 4].copy_from_slice(&past_end.to_le_bytes());
        assert_eq!(joined_details_at(&data, offsets[0]), None);

        // A length so large that `start + len` would overflow.
        data[offsets[0]..offsets[0] + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(joined_details_at(&data, offsets[0]), None);
    }

    /// Invalid UTF-8 anywhere in the blob rejects the whole entry, which is
    /// what per-field validation did too.
    #[test]
    fn joined_details_at_rejects_invalid_utf8() {
        let (mut data, offsets) = packed(&[&["ok", "fields"]]);
        let last = data.len() - 1;
        data[last] = 0xff;

        assert_eq!(joined_details_at(&data, offsets[0]), None);
    }

    /// Word ids index a table of little-endian `u32`s; out-of-range and
    /// overflowing ids yield `None`.
    #[test]
    fn words_idx_offset_reads_and_bounds_checks() {
        let table: Vec<u8> = [7u32, 42u32].iter().flat_map(|v| v.to_le_bytes()).collect();

        assert_eq!(words_idx_offset(&table, 0), Some(7));
        assert_eq!(words_idx_offset(&table, 1), Some(42));
        assert_eq!(words_idx_offset(&table, 2), None);
        // `word_id * 4` overflows.
        assert_eq!(words_idx_offset(&table, usize::MAX), None);
        assert_eq!(words_idx_offset(&table, usize::MAX / 4), None);
    }
}
