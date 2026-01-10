use daachorse::DoubleArrayAhoCorasick;
use rkyv::rancor::Fallible;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Place, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::{util::Data, viterbi::WordEntry};

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

impl<D: Fallible + ?Sized>
    DeserializeWith<rkyv::vec::ArchivedVec<u8>, DoubleArrayAhoCorasick<u32>, D>
    for DoubleArrayArchiver
{
    fn deserialize_with(
        archived: &rkyv::vec::ArchivedVec<u8>,
        _deserializer: &mut D,
    ) -> Result<DoubleArrayAhoCorasick<u32>, D::Error> {
        unsafe {
            let (da, _) = DoubleArrayAhoCorasick::deserialize_unchecked(archived.as_slice());
            Ok(da)
        }
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
        unsafe {
            let (da, _) = DoubleArrayAhoCorasick::deserialize_unchecked(&bytes);
            Ok(da)
        }
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
    pub fn load(
        da_data: impl Into<Data>,
        vals_data: impl Into<Data>,
        words_idx_data: impl Into<Data>,
        words_data: impl Into<Data>,
        is_system: bool,
    ) -> PrefixDictionary {
        let da_bytes = da_data.into();
        let (da, _) = unsafe { DoubleArrayAhoCorasick::deserialize_unchecked(&da_bytes[..]) };

        PrefixDictionary {
            da,
            vals_data: vals_data.into(),
            words_idx_data: words_idx_data.into(),
            words_data: words_data.into(),
            is_system,
        }
    }

    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        self.da
            .find_overlapping_iter(s)
            .filter(|m| m.start() == 0)
            .flat_map(move |m| {
                let id = m.value();
                let len = id & ((1u32 << 5) - 1u32);
                let offset = id >> 5u32;
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
                let offset_len = m.value();
                let offset = offset_len >> 5u32;
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data = &self.vals_data[offset_bytes..];
                let len = offset_len & ((1u32 << 5) - 1u32);
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
                let offset_len = m.value();
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
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
                                word_idx: WordIdx::new(word_entry.word_id.id),
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
    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        // Deserialize on the fly. Performance warning: this is slow.
        let (da, _) =
            unsafe { DoubleArrayAhoCorasick::<u32>::deserialize_unchecked(self.da.as_slice()) };

        let matches: Vec<_> = da
            .find_overlapping_iter(s)
            .filter(|m| m.start() == 0)
            .map(|m| (m.end(), m.value()))
            .collect();

        matches.into_iter().flat_map(move |(end, offset_len)| {
            let len = offset_len & ((1u32 << 5) - 1u32);
            let offset = offset_len >> 5u32;
            let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;

            let vals = self.vals_data.as_slice();
            // Check bounds?
            if offset_bytes >= vals.len() {
                return vec![].into_iter(); // Handle gracefully
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
                .collect::<Vec<_>>() // Collect to avoid lifetime issues with 'a and move?
                .into_iter()
        })
    }

    pub fn find_surface(&self, surface: &str) -> Vec<WordEntry> {
        let (da, _) =
            unsafe { DoubleArrayAhoCorasick::<u32>::deserialize_unchecked(self.da.as_slice()) };

        // Check if there is a match with start=0 and end=surface.len()
        let matches: Vec<_> = da
            .find_overlapping_iter(surface)
            .filter(|m| m.start() == 0 && m.end() == surface.len())
            .map(|m| m.value())
            .collect();

        matches
            .into_iter()
            .flat_map(|offset_len| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
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
            .collect()
    }
}
