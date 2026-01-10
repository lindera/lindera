use std::ops::Deref;

use byteorder::{ByteOrder, LittleEndian};
use rkyv::rancor::Fallible;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Place, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use yada::DoubleArray;

use crate::viterbi::{LexType, WordId};
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

#[derive(Serialize, Deserialize)]
#[serde(remote = "DoubleArray")]
struct DoubleArrayDef<T>(pub T)
where
    T: Deref<Target = [u8]>;

pub struct DoubleArrayArchiver;

impl ArchiveWith<DoubleArray<Data>> for DoubleArrayArchiver {
    type Archived = rkyv::vec::ArchivedVec<u8>;
    type Resolver = rkyv::vec::VecResolver;

    fn resolve_with(
        field: &DoubleArray<Data>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // DoubleArray<Data> derefs to [u8] via Data
        rkyv::vec::ArchivedVec::resolve_from_slice(&field.0[..], resolver, out);
    }
}

impl<S: Fallible + rkyv::ser::Writer + rkyv::ser::Allocator + ?Sized>
    SerializeWith<DoubleArray<Data>, S> for DoubleArrayArchiver
{
    fn serialize_with(
        field: &DoubleArray<Data>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        rkyv::vec::ArchivedVec::serialize_from_slice(&field.0[..], serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<rkyv::vec::ArchivedVec<u8>, DoubleArray<Data>, D>
    for DoubleArrayArchiver
{
    fn deserialize_with(
        archived: &rkyv::vec::ArchivedVec<u8>,
        _deserializer: &mut D,
    ) -> Result<DoubleArray<Data>, D::Error> {
        let mut vec = Vec::with_capacity(archived.len());
        vec.extend_from_slice(archived.as_slice());
        Ok(DoubleArray::new(Data::Vec(vec)))
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct PrefixDictionary {
    #[serde(with = "DoubleArrayDef")]
    #[rkyv(with = DoubleArrayArchiver)]
    pub da: DoubleArray<Data>,
    pub vals_costs_data: Data,
    pub vals_left_ids_data: Data,
    pub vals_right_ids_data: Data,
    pub vals_word_ids_data: Data,
    pub words_idx_data: Data,
    pub words_data: Data,
    pub is_system: bool,
}

impl PrefixDictionary {
    pub fn load(
        da_data: impl Into<Data>,
        vals_costs_data: impl Into<Data>,
        vals_left_ids_data: impl Into<Data>,
        vals_right_ids_data: impl Into<Data>,
        vals_word_ids_data: impl Into<Data>,
        words_idx_data: impl Into<Data>,
        words_data: impl Into<Data>,
        is_system: bool,
    ) -> PrefixDictionary {
        let da = DoubleArray::new(da_data.into());

        PrefixDictionary {
            da,
            vals_costs_data: vals_costs_data.into(),
            vals_left_ids_data: vals_left_ids_data.into(),
            vals_right_ids_data: vals_right_ids_data.into(),
            vals_word_ids_data: vals_word_ids_data.into(),
            words_idx_data: words_idx_data.into(),
            words_data: words_data.into(),
            is_system,
        }
    }

    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        self.da
            .common_prefix_search(s)
            .flat_map(move |(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
                (0..len as usize).map(move |i| {
                    let idx = (offset as usize) + i;
                    let word_cost =
                        LittleEndian::read_i16(&self.vals_costs_data[idx * 2..idx * 2 + 2]);
                    let left_id =
                        LittleEndian::read_u16(&self.vals_left_ids_data[idx * 2..idx * 2 + 2]);
                    let right_id =
                        LittleEndian::read_u16(&self.vals_right_ids_data[idx * 2..idx * 2 + 2]);
                    let word_id_val =
                        LittleEndian::read_u32(&self.vals_word_ids_data[idx * 4..idx * 4 + 4]);

                    let word_id = WordId::new(
                        if self.is_system {
                            LexType::System
                        } else {
                            LexType::User
                        },
                        word_id_val,
                    );

                    (
                        prefix_len,
                        WordEntry {
                            word_id,
                            word_cost,
                            left_id,
                            right_id,
                        },
                    )
                })
            })
    }

    pub fn prefix_indices<'a>(
        &'a self,
        s: &'a str,
    ) -> impl Iterator<Item = (usize, usize, usize)> + 'a {
        self.da
            .common_prefix_search(s)
            .map(move |(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
                (prefix_len, offset as usize, len as usize)
            })
    }

    /// Find `WordEntry`s with surface
    pub fn find_surface(&self, surface: &str) -> Vec<WordEntry> {
        match self.da.exact_match_search(surface) {
            Some(offset_len) => {
                let offset = offset_len >> 5u32;
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize)
                    .map(|i| {
                        let idx = (offset as usize) + i;
                        let word_cost =
                            LittleEndian::read_i16(&self.vals_costs_data[idx * 2..idx * 2 + 2]);
                        let left_id =
                            LittleEndian::read_u16(&self.vals_left_ids_data[idx * 2..idx * 2 + 2]);
                        let right_id =
                            LittleEndian::read_u16(&self.vals_right_ids_data[idx * 2..idx * 2 + 2]);
                        let word_id_val =
                            LittleEndian::read_u32(&self.vals_word_ids_data[idx * 4..idx * 4 + 4]);

                        let word_id = WordId::new(
                            if self.is_system {
                                LexType::System
                            } else {
                                LexType::User
                            },
                            word_id_val,
                        );

                        WordEntry {
                            word_id,
                            word_cost,
                            left_id,
                            right_id,
                        }
                    })
                    .collect::<Vec<WordEntry>>()
            }
            None => vec![],
        }
    }

    /// Find `WordEntry`s with surface using lazy evaluation
    /// This iterator-based approach reduces memory allocations
    pub fn find_surface_iter(&self, surface: &str) -> impl Iterator<Item = WordEntry> + '_ {
        self.da
            .exact_match_search(surface)
            .map(|offset_len| {
                let offset = offset_len >> 5u32;
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize).map(move |i| {
                    let idx = (offset as usize) + i;
                    let word_cost =
                        LittleEndian::read_i16(&self.vals_costs_data[idx * 2..idx * 2 + 2]);
                    let left_id =
                        LittleEndian::read_u16(&self.vals_left_ids_data[idx * 2..idx * 2 + 2]);
                    let right_id =
                        LittleEndian::read_u16(&self.vals_right_ids_data[idx * 2..idx * 2 + 2]);
                    let word_id_val =
                        LittleEndian::read_u32(&self.vals_word_ids_data[idx * 4..idx * 4 + 4]);

                    let word_id = WordId::new(
                        if self.is_system {
                            LexType::System
                        } else {
                            LexType::User
                        },
                        word_id_val,
                    );

                    WordEntry {
                        word_id,
                        word_cost,
                        left_id,
                        right_id,
                    }
                })
            })
            .into_iter()
            .flatten()
    }

    /// Common prefix iterator using character array input
    pub fn common_prefix_iterator(&self, suffix: &[char]) -> Vec<Match> {
        // 空の辞書の場合は空のマッチを返す
        if self.vals_costs_data.is_empty() {
            return Vec::new();
        }

        let suffix_str: String = suffix.iter().collect();
        self.da
            .common_prefix_search(&suffix_str)
            .flat_map(|(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;

                // 範囲チェックを追加
                let max_idx = (offset as usize) + len as usize;
                if max_idx * 2 > self.vals_costs_data.len() {
                    return vec![].into_iter();
                }

                (0..len as usize)
                    .map(move |i| {
                        let idx = (offset as usize) + i;
                        let word_id_val =
                            LittleEndian::read_u32(&self.vals_word_ids_data[idx * 4..idx * 4 + 4]);

                        let word_entry = WordId::new(
                            if self.is_system {
                                LexType::System
                            } else {
                                LexType::User
                            },
                            word_id_val,
                        );
                        Match {
                            word_idx: WordIdx::new(word_entry.id),
                            end_char: prefix_len,
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
        let da = DoubleArray::new(self.da.as_slice());
        let matches: Vec<_> = da.common_prefix_search(s).collect();

        matches
            .into_iter()
            .flat_map(move |(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
                (0..len as usize).map(move |i| {
                    let idx = (offset as usize) + i;
                    let word_cost = self.vals_costs_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(i16::from_le_bytes)
                        .unwrap_or(0);
                    let left_id = self.vals_left_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(u16::from_le_bytes)
                        .unwrap_or(0);
                    let right_id = self.vals_right_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(u16::from_le_bytes)
                        .unwrap_or(0);
                    let word_id_val = self.vals_word_ids_data.as_slice()[idx * 4..idx * 4 + 4]
                        .try_into()
                        .map(u32::from_le_bytes)
                        .unwrap_or(0);

                    let word_id = WordId::new(
                        if self.is_system {
                            LexType::System
                        } else {
                            LexType::User
                        },
                        word_id_val,
                    );

                    (
                        prefix_len,
                        WordEntry {
                            word_id,
                            word_cost,
                            left_id,
                            right_id,
                        },
                    )
                })
            })
    }

    pub fn find_surface(&self, surface: &str) -> Vec<WordEntry> {
        let da = DoubleArray::new(self.da.as_slice());
        match da.exact_match_search(surface) {
            Some(offset_len) => {
                let offset = offset_len >> 5u32;
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize)
                    .map(|i| {
                        let idx = (offset as usize) + i;
                        let word_cost = self.vals_costs_data.as_slice()[idx * 2..idx * 2 + 2]
                            .try_into()
                            .map(i16::from_le_bytes)
                            .unwrap_or(0);
                        let left_id = self.vals_left_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                            .try_into()
                            .map(u16::from_le_bytes)
                            .unwrap_or(0);
                        let right_id = self.vals_right_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                            .try_into()
                            .map(u16::from_le_bytes)
                            .unwrap_or(0);
                        let word_id_val = self.vals_word_ids_data.as_slice()[idx * 4..idx * 4 + 4]
                            .try_into()
                            .map(u32::from_le_bytes)
                            .unwrap_or(0);

                        let word_id = WordId::new(
                            if self.is_system {
                                LexType::System
                            } else {
                                LexType::User
                            },
                            word_id_val,
                        );

                        WordEntry {
                            word_id,
                            word_cost,
                            left_id,
                            right_id,
                        }
                    })
                    .collect::<Vec<WordEntry>>()
            }
            None => vec![],
        }
    }

    pub fn find_surface_iter(&self, surface: &str) -> impl Iterator<Item = WordEntry> + '_ {
        let da = DoubleArray::new(self.da.as_slice());
        da.exact_match_search(surface)
            .map(|offset_len| {
                let offset = offset_len >> 5u32;
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize).map(move |i| {
                    let idx = (offset as usize) + i;
                    let word_cost = self.vals_costs_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(i16::from_le_bytes)
                        .unwrap_or(0);
                    let left_id = self.vals_left_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(u16::from_le_bytes)
                        .unwrap_or(0);
                    let right_id = self.vals_right_ids_data.as_slice()[idx * 2..idx * 2 + 2]
                        .try_into()
                        .map(u16::from_le_bytes)
                        .unwrap_or(0);
                    let word_id_val = self.vals_word_ids_data.as_slice()[idx * 4..idx * 4 + 4]
                        .try_into()
                        .map(u32::from_le_bytes)
                        .unwrap_or(0);

                    let word_id = WordId::new(
                        if self.is_system {
                            LexType::System
                        } else {
                            LexType::User
                        },
                        word_id_val,
                    );

                    WordEntry {
                        word_id,
                        word_cost,
                        left_id,
                        right_id,
                    }
                })
            })
            .into_iter()
            .flatten()
    }

    pub fn common_prefix_iterator(&self, suffix: &[char]) -> Vec<Match> {
        // 空の辞書の場合は空のマッチを返す
        if self.vals_costs_data.as_slice().is_empty() {
            return Vec::new();
        }

        let suffix_str: String = suffix.iter().collect();
        let da = DoubleArray::new(self.da.as_slice());

        da.common_prefix_search(&suffix_str)
            .flat_map(|(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;

                // 範囲チェックを追加
                let max_idx = (offset as usize) + len as usize;
                if max_idx * 2 > self.vals_costs_data.as_slice().len() {
                    return vec![].into_iter();
                }

                (0..len as usize)
                    .map(move |i| {
                        let idx = (offset as usize) + i;
                        let word_id_val = self.vals_word_ids_data.as_slice()[idx * 4..idx * 4 + 4]
                            .try_into()
                            .map(u32::from_le_bytes)
                            .unwrap_or(0);

                        let word_entry = WordId::new(
                            if self.is_system {
                                LexType::System
                            } else {
                                LexType::User
                            },
                            word_id_val,
                        );
                        Match {
                            word_idx: WordIdx::new(word_entry.id),
                            end_char: prefix_len,
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .collect()
    }
}
