use std::ops::Deref;

use serde::{Deserialize, Serialize};
use yada::DoubleArray;

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

#[derive(Clone, Serialize, Deserialize)]
pub struct PrefixDictionary {
    #[serde(with = "DoubleArrayDef")]
    pub da: DoubleArray<Data>,
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
        let da = DoubleArray::new(da_data.into());

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
            .common_prefix_search(s)
            .flat_map(move |(offset_len, prefix_len)| {
                let len = offset_len & ((1u32 << 5) - 1u32);
                let offset = offset_len >> 5u32;
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data: &[u8] = &self.vals_data[offset_bytes..];
                (0..len as usize).map(move |i| {
                    (
                        prefix_len,
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
        match self.da.exact_match_search(surface) {
            Some(offset_len) => {
                let offset = offset_len >> 5u32;
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data: &[u8] = &self.vals_data[offset_bytes..];
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize)
                    .map(|i| {
                        WordEntry::deserialize(
                            &data[WordEntry::SERIALIZED_LEN * i..],
                            self.is_system,
                        )
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
                let offset_bytes = (offset as usize) * WordEntry::SERIALIZED_LEN;
                let data = &self.vals_data[offset_bytes..];
                let len = offset_len & ((1u32 << 5) - 1u32);
                (0..len as usize).map(move |i| {
                    WordEntry::deserialize(&data[WordEntry::SERIALIZED_LEN * i..], self.is_system)
                })
            })
            .into_iter()
            .flatten()
    }

    /// Common prefix iterator using character array input
    pub fn common_prefix_iterator(&self, suffix: &[char]) -> Vec<Match> {
        // 空の辞書の場合は空のマッチを返す
        if self.vals_data.is_empty() {
            return Vec::new();
        }

        let suffix_str: String = suffix.iter().collect();
        self.da
            .common_prefix_search(&suffix_str)
            .flat_map(|(offset_len, prefix_len)| {
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
                                end_char: prefix_len,
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

#[cfg(test)]
mod tests {}
