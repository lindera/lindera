use std::borrow::Cow;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use yada::DoubleArray;

use crate::viterbi::WordEntry;

#[derive(Serialize, Deserialize)]
#[serde(remote = "DoubleArray")]
struct DoubleArrayDef<T>(pub T)
where
    T: Deref<Target = [u8]>;

// note: Cow is only used as an enum over Vec<u8> and &'static [u8]
//	copy-on-write capability is not used
type Data = Cow<'static, [u8]>;

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
        da_data: Vec<u8>,
        vals_data: Vec<u8>,
        words_idx_data: Vec<u8>,
        words_data: Vec<u8>,
        is_system: bool,
    ) -> PrefixDictionary {
        let da = DoubleArray::new(Cow::Owned(da_data));

        PrefixDictionary {
            da,
            vals_data: Cow::Owned(vals_data),
            words_idx_data: Cow::Owned(words_idx_data),
            words_data: Cow::Owned(words_data),
            is_system,
        }
    }

    pub fn load_static(
        da_data: &'static [u8],
        vals_data: &'static [u8],
        words_idx_data: &'static [u8],
        words_data: &'static [u8],
    ) -> PrefixDictionary {
        let da = DoubleArray::new(Cow::Borrowed(da_data));

        PrefixDictionary {
            da,
            vals_data: Cow::Borrowed(vals_data),
            words_idx_data: Cow::Borrowed(words_idx_data),
            words_data: Cow::Borrowed(words_data),
            is_system: true,
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
}

#[cfg(test)]
mod tests {}
