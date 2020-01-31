use tantivy_fst;

const DICTIONARY_DATA: &'static [u8] = include_bytes!("../../dict/dict.fst");

pub struct SystemDict<Data = &'static [u8]> {
    pub fst: tantivy_fst::raw::Fst<Data>
}

impl Default for SystemDict<&'static [u8]> {
    fn default() -> SystemDict<&'static [u8]> {
        SystemDict::from_static_slice(DICTIONARY_DATA).unwrap()
    }
}

impl SystemDict<&'static [u8]> {
    pub fn from_static_slice(
        fst_data: &'static [u8],
    ) -> tantivy_fst::Result<SystemDict> {
        let fst = tantivy_fst::raw::Fst::new(fst_data)?;
        Ok(SystemDict { fst })
    }
}
