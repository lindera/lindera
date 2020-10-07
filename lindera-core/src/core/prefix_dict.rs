use std::ops::Deref;

use yada::DoubleArray;

use crate::core::word_entry::WordEntry;

#[derive(Clone)]
pub struct PrefixDict<Data = Vec<u8>> {
    pub da: DoubleArray<Vec<u8>>,
    pub vals_data: Data,
    pub is_system: bool,
}

impl PrefixDict<&[u8]> {
    pub fn from_static_slice(da_data: &[u8], vals_data: &[u8]) -> Result<PrefixDict, String> {
        let da = DoubleArray::new(da_data.to_vec());
        Ok(PrefixDict {
            da,
            vals_data: vals_data.to_vec(),
            is_system: true,
        })
    }
}

impl<D: Deref<Target = [u8]>> PrefixDict<D> {
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
}

#[cfg(test)]
mod tests {}
