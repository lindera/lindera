use std::ops::Deref;

use lindera_fst;
use lindera_fst::raw::Output;

use crate::core::word_entry::WordEntry;

#[derive(Clone)]
pub struct PrefixDict<Data = Vec<u8>> {
    pub fst: lindera_fst::raw::Fst<Data>,
    pub vals_data: Data,
    pub is_system: bool,
}

impl PrefixDict<&[u8]> {
    pub fn from_static_slice(fst_data: &[u8], vals_data: &[u8]) -> lindera_fst::Result<PrefixDict> {
        let fst = lindera_fst::raw::Fst::new(fst_data.to_vec())?;
        Ok(PrefixDict {
            fst,
            vals_data: vals_data.to_vec(),
            is_system: true,
        })
    }
}

impl<D: Deref<Target = [u8]>> PrefixDict<D> {
    pub fn prefix<'a>(&'a self, s: &'a str) -> impl Iterator<Item = (usize, WordEntry)> + 'a {
        s.as_bytes()
            .iter()
            .scan(
                (0, self.fst.root(), Output::zero()),
                move |(prefix_len, node, output), &byte| {
                    if let Some(b_index) = node.find_input(byte) {
                        let transition = node.transition(b_index);
                        *prefix_len += 1;
                        *output = output.cat(transition.out);
                        *node = self.fst.node(transition.addr);
                        return Some((node.is_final(), *prefix_len, output.value()));
                    }
                    None
                },
            )
            .filter_map(|(is_final, prefix_len, offset_len)| {
                if is_final {
                    Some((prefix_len, offset_len))
                } else {
                    None
                }
            })
            .flat_map(move |(prefix_len, offset_len)| {
                let len = offset_len & ((1u64 << 5) - 1u64);
                let offset = offset_len >> 5u64;
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
