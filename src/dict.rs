use std::io::{self, Read, BufRead, BufReader};
use std::fs::File;
use std::u32;
use std::str::FromStr;
use fst;

const DICTIONARY_DATA: &'static [u8] = include_bytes!("../dict/dict.fst");

pub struct Dict {
    pub fst: fst::Map,
}

impl Dict {

    pub fn load_default() -> Dict {
        Dict {
            fst: fst::raw::from_static_slice(DICTIONARY_DATA),
        }
    }
}

