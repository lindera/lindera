use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use yada::{builder::DoubleArrayBuilder, DoubleArray};

use lindera_core::character_filter::CharacterFilter;

use crate::{error::LinderaErrorKind, LinderaResult};

pub const MAPPING_CHARACTER_FILTER_NAME: &str = "mapping";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MappingCharacterFilterConfig {
    pub mapping: HashMap<String, String>,
}

impl MappingCharacterFilterConfig {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self { mapping: map }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone)]
pub struct MappingCharacterFilter {
    config: MappingCharacterFilterConfig,
    trie: DoubleArray<Vec<u8>>,
}

impl MappingCharacterFilter {
    pub fn new(config: MappingCharacterFilterConfig) -> LinderaResult<Self> {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = config.mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let data = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        let trie = DoubleArray::new(data);

        Ok(Self { config, trie })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let config = MappingCharacterFilterConfig::from_slice(data)?;

        Self::new(config)
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>)> {
        let mut offsets: Vec<usize> = Vec::new();
        let mut diffs: Vec<i64> = Vec::new();

        let mut result = String::new();
        let mut start = 0_usize;
        let len = text.len();

        while start < len {
            let suffix = &text[start..];
            match self
                .trie
                .common_prefix_search(suffix.as_bytes())
                .last()
                .map(|(_offset_len, prefix_len)| prefix_len)
            {
                Some(prefix_len) => {
                    let surface = &text[start..start + prefix_len];
                    let replacement = &self.config.mapping[surface];

                    let replacement_len = replacement.len();
                    let diff = prefix_len as i64 - replacement_len as i64;
                    let input_offset = start + prefix_len;

                    if diff != 0 {
                        let prev_diff = *diffs.last().unwrap_or(&0);

                        if diff > 0 {
                            // Replacement is shorter than matched surface.
                            offsets.push((input_offset as i64 - diff - prev_diff) as usize);
                            diffs.push(prev_diff + diff);
                        } else {
                            // Replacement is longer than matched surface.
                            let output_start = input_offset + (-prev_diff as usize);
                            for extra_idx in 0..-diff as usize {
                                offsets.push(output_start + extra_idx);
                                diffs.push(prev_diff - extra_idx as i64 - 1);
                            }
                        }
                    }

                    result.push_str(replacement);

                    // move start offset
                    start += prefix_len;
                }
                None => {
                    match suffix.chars().next() {
                        Some(c) => {
                            result.push(c);

                            // move start offset
                            start += c.len_utf8();
                        }
                        None => break,
                    }
                }
            }
        }

        *text = result;

        println!("offsets: {:?}", offsets);
        println!("diffs: {:?}", diffs);

        Ok((offsets, diffs))
    }
}

#[cfg(test)]
mod tests {
    use lindera_core::character_filter::CharacterFilter;

    use crate::character_filter::mapping::{MappingCharacterFilter, MappingCharacterFilterConfig};

    #[test]
    fn test_mapping_character_filter_config_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = MappingCharacterFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!("ア", config.mapping.get("ｱ").unwrap());
    }

    #[test]
    fn test_mapping_character_filter_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result = MappingCharacterFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_mapping_character_filter_apply() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ｱｲｳｴｵ".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("アイウエオ", text);

        let config_str = r#"
        {
            "mapping": {
                "ﾘﾝﾃﾞﾗ": "リンデラ",
                "リンデラ": "Lindera"
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ﾘﾝﾃﾞﾗ".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("リンデラ", text);

        let config_str = r#"
        {
            "mapping": {
                "ﾘﾝﾃﾞﾗ": "リンデラ",
                "リンデラ": "Lindera"
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "Rust製形態素解析器ﾘﾝﾃﾞﾗで日本語を形態素解析する。".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("Rust製形態素解析器リンデラで日本語を形態素解析する。", text);
    }

    #[test]
    fn test_mapping_character_filter_apply_offsets_diffs() {
        let config_str = r#"
        {
            "mapping": {
                "B": "bbb",
                "DE": "ddd",
                "G": "gggg"
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ABCDEFG".to_string();
        let (offsets, diffs) = filter.apply(&mut text).unwrap();
        assert_eq!("AbbbCdddFgggg", text);
        assert_eq!(vec![2, 3, 7, 10, 11, 12], offsets);
        assert_eq!(vec![-1, -2, -3, -4, -5, -6], diffs);

        let config_str = r#"
        {
            "mapping": {
                "BCD": "b",
                "FG": "f",
                "HIJ": "hh",
                "KL": ""
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ABCDEFGHIJKL".to_string();
        let (offsets, diffs) = filter.apply(&mut text).unwrap();
        assert_eq!("AbEfhh", text);
        assert_eq!(vec![2, 4, 6, 6], offsets);
        assert_eq!(vec![2, 3, 4, 6], diffs);
    }
}
