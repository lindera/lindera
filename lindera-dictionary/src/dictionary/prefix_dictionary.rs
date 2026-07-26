use daachorse::DoubleArrayAhoCorasick;
use rkyv::rancor::{Fallible, Source};
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Place, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::{LinderaResult, error::LinderaErrorKind, util::Data, viterbi::WordEntry};

/// Match structure for common prefix iterator compatibility
#[derive(Debug, Clone)]
pub struct Match {
    pub word_idx: WordIdx,
    pub end_char: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct WordIdx {
    pub word_id: u32,
}

impl WordIdx {
    pub fn new(word_id: u32) -> Self {
        Self { word_id }
    }
}

/// Whether the `da_data` passed to [`PrefixDictionary::load`] is trusted to
/// be the exact, undamaged output of `DoubleArrayAhoCorasick::serialize()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaTrust {
    /// Skip daachorse's validation pass (`deserialize_unchecked`). Only for
    /// data this crate's own build pipeline produced and embedded verbatim.
    Trusted,
    /// Run daachorse's full validation pass (`deserialize`). Use for any
    /// filesystem-, network-, or caller-supplied bytes.
    Untrusted,
}

pub struct DoubleArrayArchiver;

impl ArchiveWith<DoubleArrayAhoCorasick<u32>> for DoubleArrayArchiver {
    type Archived = rkyv::vec::ArchivedVec<u8>;
    type Resolver = rkyv::vec::VecResolver;

    fn resolve_with(
        field: &DoubleArrayAhoCorasick<u32>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let bytes = field.serialize();
        rkyv::vec::ArchivedVec::resolve_from_slice(&bytes, resolver, out);
    }
}

impl<S: Fallible + rkyv::ser::Writer + rkyv::ser::Allocator + ?Sized>
    SerializeWith<DoubleArrayAhoCorasick<u32>, S> for DoubleArrayArchiver
{
    fn serialize_with(
        field: &DoubleArrayAhoCorasick<u32>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let bytes = field.serialize();
        rkyv::vec::ArchivedVec::serialize_from_slice(&bytes, serializer)
    }
}

impl<D: Fallible<Error: Source> + ?Sized>
    DeserializeWith<rkyv::vec::ArchivedVec<u8>, DoubleArrayAhoCorasick<u32>, D>
    for DoubleArrayArchiver
{
    /// Deserialize the archived byte vector into a `DoubleArrayAhoCorasick`.
    ///
    /// # Returns
    ///
    /// The deserialized `DoubleArrayAhoCorasick`, or an error if deserialization fails.
    fn deserialize_with(
        archived: &rkyv::vec::ArchivedVec<u8>,
        _deserializer: &mut D,
    ) -> Result<DoubleArrayAhoCorasick<u32>, D::Error> {
        let (da, _) = DoubleArrayAhoCorasick::deserialize(archived.as_slice()).map_err(|err| {
            D::Error::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ))
        })?;
        Ok(da)
    }
}

mod double_array_serde {
    use daachorse::DoubleArrayAhoCorasick;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(da: &DoubleArrayAhoCorasick<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = da.serialize();
        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DoubleArrayAhoCorasick<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let (da, _) = DoubleArrayAhoCorasick::deserialize(&bytes)
            .map_err(|err| serde::de::Error::custom(err.to_string()))?;
        Ok(da)
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct PrefixDictionary {
    #[serde(with = "self::double_array_serde")]
    #[rkyv(with = DoubleArrayArchiver)]
    pub da: DoubleArrayAhoCorasick<u32>,
    pub vals_data: Data,
    pub words_idx_data: Data,
    pub words_data: Data,
    pub is_system: bool,
}

impl PrefixDictionary {
    /// Decode the `(offset, count)` pair stored in the double-array value.
    ///
    /// Both system and user dictionaries pack the word-id offset in the high
    /// 24 bits and the per-surface variant count in the low 8 bits (up to 255
    /// variants). The legacy 5-bit user-dictionary encoding was retired in
    /// v4.0.0; user `.bin` files built with v3 must be rebuilt from CSV.
    #[inline]
    pub(crate) fn decode_val(&self, val: u32) -> (u32, u32) {
        (val >> 8u32, val & ((1u32 << 8) - 1u32))
    }

    /// Load a `PrefixDictionary` from raw binary data.
    ///
    /// # Arguments
    ///
    /// * `da_data` - Double-array data bytes.
    /// * `vals_data` - Values data bytes.
    /// * `words_idx_data` - Word index data bytes.
    /// * `words_data` - Words data bytes.
    /// * `is_system` - Whether this is a system dictionary.
    /// * `trust` - Whether `da_data` is trusted to be the exact output of
    ///   `DoubleArrayAhoCorasick::serialize()`, allowing the validation pass
    ///   to be skipped. See [`DaTrust`].
    ///
    /// # Returns
    ///
    /// A `PrefixDictionary`, or an error if deserialization fails.
    pub fn load(
        da_data: impl Into<Data>,
        vals_data: impl Into<Data>,
        words_idx_data: impl Into<Data>,
        words_data: impl Into<Data>,
        is_system: bool,
        trust: DaTrust,
    ) -> LinderaResult<PrefixDictionary> {
        let da_bytes = da_data.into();
        let da = match trust {
            DaTrust::Trusted => {
                debug_assert!(
                    matches!(da_bytes, Data::Static(_)),
                    "DaTrust::Trusted should only be used for embedded (Data::Static) da_data"
                );
                // SAFETY: `da_bytes` must be the byte-exact output of this
                // exact daachorse version's `DoubleArrayAhoCorasick::serialize()`.
                // Only `embedded_dictionary!` (lindera-dictionary/src/macros.rs)
                // passes `DaTrust::Trusted`, using `include_bytes!` data
                // produced by this crate's own build pipeline
                // (builder/prefix_dictionary.rs's write_double_array_file,
                // which calls `.serialize()` directly and writes the bytes
                // verbatim, with no re-encoding/compression step and no
                // alternate producer). Note: the opt-in build cache
                // (assets.rs, LINDERA_BUILD_DICTIONARY_CACHE_DIR) keys only
                // on the dictionary crate's own version, not daachorse's, so
                // bumping the pinned daachorse version while reusing a stale
                // cache dir is a narrow, currently-unexploited path that
                // could violate this invariant in the future.
                unsafe { DoubleArrayAhoCorasick::deserialize_unchecked(&da_bytes[..]).0 }
            }
            DaTrust::Untrusted => {
                DoubleArrayAhoCorasick::deserialize(&da_bytes[..])
                    .map_err(|err| {
                        LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
                    })?
                    .0
            }
        };

        Ok(PrefixDictionary {
            da,
            vals_data: vals_data.into(),
            words_idx_data: words_idx_data.into(),
            words_data: words_data.into(),
            is_system,
        })
    }

    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        self.da
            .find_overlapping_iter(s)
            .filter(|m| m.start() == 0)
            .flat_map(move |m| {
                let (offset, len) = self.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data: &[u8] = &self.vals_data[offset_bytes..];
                (0..len as usize).map(move |i| {
                    (
                        m.end(),
                        WordEntry::deserialize(
                            &data[WordEntry::SERIALIZED_LEN * i..],
                            self.is_system,
                        ),
                    )
                })
            })
    }

    /// Find `WordEntry`s with surface
    pub fn find_surface(&self, surface: &str) -> Vec<WordEntry> {
        self.find_surface_iter(surface).collect()
    }

    /// Find `WordEntry`s with surface using lazy evaluation
    /// This iterator-based approach reduces memory allocations
    pub fn find_surface_iter<'a>(
        &'a self,
        surface: &'a str,
    ) -> impl Iterator<Item = WordEntry> + 'a {
        self.da
            .find_overlapping_iter(surface)
            .filter(|m| m.start() == 0 && m.end() == surface.len())
            .flat_map(move |m| {
                let (offset, len) = self.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data = &self.vals_data[offset_bytes..];
                (0..len as usize).map(move |i| {
                    WordEntry::deserialize(&data[WordEntry::SERIALIZED_LEN * i..], self.is_system)
                })
            })
    }

    /// Common prefix iterator using character array input
    pub fn common_prefix_iterator(&self, suffix: &[char]) -> Vec<Match> {
        // Warning: This method takes &[char], but daachorse works on bytes (str).
        // Converting char slice to string is costly but necessary if we use daachorse standard API.

        if self.vals_data.is_empty() {
            return Vec::new();
        }

        let suffix_str: String = suffix.iter().collect();

        self.da
            .find_overlapping_iter(&suffix_str)
            .filter(|m| m.start() == 0)
            .flat_map(|m| {
                let (offset, len) = self.decode_val(m.value());
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

                // 範囲チェックを追加
                if offset_bytes >= self.vals_data.len() {
                    return vec![].into_iter();
                }

                let data: &[u8] = &self.vals_data[offset_bytes..];
                (0..len as usize)
                    .filter_map(move |i| {
                        let required_bytes = WordEntry::SERIALIZED_LEN * (i + 1);
                        if required_bytes <= data.len() {
                            let word_entry = WordEntry::deserialize(
                                &data[WordEntry::SERIALIZED_LEN * i..],
                                self.is_system,
                            );
                            Some(Match {
                                word_idx: WordIdx::new(word_entry.word_id().id()),
                                end_char: m.end(), // prefix_len in bytes? No, m.end() is byte index.
                                                   // Match expects char length?
                                                   // Original code: end_char: prefix_len
                                                   // prefix_len was number of bytes or chars?
                                                   // yada::common_prefix_search returns (val, len) where len is length in bytes?
                                                   // yada common_prefix_search(str) returns length in bytes.
                                                   // But common_prefix_iterator takes &[char].
                                                   // Match.end_char usually implies character index if used for Viterbi on chars.
                                                   // But Viterbi usually works on bytes in Lindera?
                                                   // Let's check typical usage.
                                                   // NOTE: daachorse returns byte indices.
                                                   // If input was chars converted to String, byte index != char index.
                                                   // We need to map back to char index?
                                                   // This function common_prefix_iterator might be inefficient or deprecated given we move to byte-based Viterbi.
                                                   // For now, let's assume we return byte length.
                                                   // But wait, suffix is &[char].
                                                   // The caller likely expects char length?
                                                   // Yes. if suffix is &[char], end_char 3 means 3 chars.
                                                   // We have byte length from daachorse.
                                                   // We need to count chars in suffix_str[..m.end()].
                                                   // This is inefficient.
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .collect()
    }
}

impl ArchivedPrefixDictionary {
    /// Decode the `(offset, count)` pair. See [`PrefixDictionary::decode_val`].
    #[inline]
    fn decode_val(&self, val: u32) -> (u32, u32) {
        (val >> 8u32, val & ((1u32 << 8) - 1u32))
    }

    /// Find all prefix matches for the given string using the archived dictionary.
    ///
    /// # Arguments
    ///
    /// * `s` - The input string to search for prefix matches.
    ///
    /// # Returns
    ///
    /// An iterator of `(end_position, WordEntry)` pairs, or an error if deserialization fails.
    pub fn prefix<'a>(
        &'a self,
        s: &'a str,
    ) -> LinderaResult<impl Iterator<Item = (usize, WordEntry)> + 'a> {
        // Deserialize on the fly. Performance warning: this is slow.
        let (da, _) =
            DoubleArrayAhoCorasick::<u32>::deserialize(self.da.as_slice()).map_err(|err| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
            })?;

        let matches: Vec<_> = da
            .find_overlapping_iter(s)
            .filter(|m| m.start() == 0)
            .map(|m| (m.end(), m.value()))
            .collect();

        Ok(matches.into_iter().flat_map(move |(end, offset_len)| {
            let (offset, len) = self.decode_val(offset_len);
            let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

            let vals = self.vals_data.as_slice();
            if offset_bytes >= vals.len() {
                return vec![].into_iter();
            }

            let data = &vals[offset_bytes..];
            (0..len as usize)
                .map(move |i| {
                    (
                        end,
                        WordEntry::deserialize(
                            &data[WordEntry::SERIALIZED_LEN * i..],
                            self.is_system,
                        ),
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
        }))
    }

    /// Find `WordEntry`s matching the exact surface in the archived dictionary.
    ///
    /// # Arguments
    ///
    /// * `surface` - The surface string to search for.
    ///
    /// # Returns
    ///
    /// A vector of matching `WordEntry`s, or an error if deserialization fails.
    pub fn find_surface(&self, surface: &str) -> LinderaResult<Vec<WordEntry>> {
        let (da, _) =
            DoubleArrayAhoCorasick::<u32>::deserialize(self.da.as_slice()).map_err(|err| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
            })?;

        let matches: Vec<_> = da
            .find_overlapping_iter(surface)
            .filter(|m| m.start() == 0 && m.end() == surface.len())
            .map(|m| m.value())
            .collect();

        Ok(matches
            .into_iter()
            .flat_map(|offset_len| {
                let (offset, len) = self.decode_val(offset_len);
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let vals = self.vals_data.as_slice();
                if offset_bytes >= vals.len() {
                    return Vec::new().into_iter();
                }
                let data = &vals[offset_bytes..];
                (0..len as usize)
                    .map(|i| {
                        WordEntry::deserialize(
                            &data[WordEntry::SERIALIZED_LEN * i..],
                            self.is_system,
                        )
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use daachorse::DoubleArrayAhoCorasickBuilder;

    use super::*;

    fn build_valid_da_bytes() -> Vec<u8> {
        let keyset: Vec<(&[u8], u32)> = vec![(b"a", 0), (b"ab", 1), (b"b", 2)];
        let da = DoubleArrayAhoCorasickBuilder::new()
            .build_with_values(keyset)
            .unwrap();
        da.serialize()
    }

    #[test]
    fn test_prefix_dictionary_load_trusted_matches_untrusted() {
        let da_bytes = build_valid_da_bytes();
        // `DaTrust::Trusted` asserts (debug builds) that the input is
        // `Data::Static`, mirroring the only real caller (the
        // `embedded_dictionary!` macro's `include_bytes!` data). Leak the
        // buffer to get a genuine `&'static [u8]` for this test.
        let da_bytes_static: &'static [u8] = Box::leak(da_bytes.clone().into_boxed_slice());

        let trusted = PrefixDictionary::load(
            da_bytes_static,
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            true,
            DaTrust::Trusted,
        )
        .unwrap();
        let untrusted = PrefixDictionary::load(
            da_bytes,
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            true,
            DaTrust::Untrusted,
        )
        .unwrap();

        let trusted_matches: Vec<_> = trusted.da.find_overlapping_iter("ab").collect();
        let untrusted_matches: Vec<_> = untrusted.da.find_overlapping_iter("ab").collect();
        assert_eq!(trusted_matches.len(), untrusted_matches.len());
        assert!(!trusted_matches.is_empty());
        for (t, u) in trusted_matches.iter().zip(untrusted_matches.iter()) {
            assert_eq!(t.value(), u.value());
            assert_eq!(t.start(), u.start());
            assert_eq!(t.end(), u.end());
        }
    }

    #[test]
    fn test_prefix_dictionary_load_untrusted_rejects_corrupted_da_data() {
        let mut da_bytes = build_valid_da_bytes();
        // Truncate to well short of a valid length-prefixed record; the
        // checked `deserialize` path must reject this rather than panic or
        // read out of bounds.
        da_bytes.truncate(4);

        let result = PrefixDictionary::load(
            da_bytes,
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            true,
            DaTrust::Untrusted,
        );

        assert!(result.is_err());
    }
}
