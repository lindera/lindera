use std::ops::Deref;

use crate::core::word_entry::WordEntry;
use yada::DoubleArray;

pub struct PrefixDict<Data = Vec<u8>> {
    pub da: DoubleArray<Vec<u8>>,
    pub vals_data: Data,
    pub is_system: bool,
}

impl PrefixDict<&[u8]> {
    pub fn from_static_slice(da_data: &[u8], vals_data: &[u8]) -> Result<PrefixDict, String> {
        //let fst = lindera_fst::raw::Fst::new(fst_data.to_vec())?;
        //TODO
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
mod tests {
    //    use crate::core::prefix_dict::PrefixDict;
    //
    //    #[test]
    //    fn test_fst_prefix_2() {
    //        let prefix_dict = PrefixDict::default();
    //        let count_prefix = prefix_dict.prefix("—でも").count();
    //        assert_eq!(count_prefix, 1);
    //    }
    //
    //    #[test]
    //    fn test_fst_prefix_tilde() {
    //        let prefix_dict = PrefixDict::default();
    //        let count_prefix = prefix_dict.prefix("〜").count();
    //        assert_eq!(count_prefix, 2);
    //    }
    //
    //    #[test]
    //    fn test_fst_ikkagetsu() {
    //        let prefix_dict = PrefixDict::default();
    //        let count_prefix = prefix_dict.prefix("ー").count();
    //        assert_eq!(count_prefix, 0);
    //
    //        let count_prefix = prefix_dict.prefix("ヶ月").count();
    //        assert_eq!(count_prefix, 1);
    //    }
    //
    //    #[test]
    //    fn test_fst_prefix_asterisk_symbol() {
    //        let prefix_dict = PrefixDict::default();
    //        let count_prefix = prefix_dict.prefix("※").count();
    //        assert_eq!(count_prefix, 1);
    //    }
}
