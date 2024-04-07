use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use yada::{builder::DoubleArrayBuilder, DoubleArray};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::character_filter::{add_offset_diff, CharacterFilter};

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

/// Replace characters with the specified character mappings.
///
#[derive(Clone)]
pub struct MappingCharacterFilter {
    config: MappingCharacterFilterConfig,
    trie: DoubleArray<Vec<u8>>,
}

impl MappingCharacterFilter {
    pub fn new(config: MappingCharacterFilterConfig) -> Self {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = config.mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let data = DoubleArrayBuilder::build(&keyset)
            .ok_or_else(|| {
                LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
            })
            .unwrap();

        let trie = DoubleArray::new(data);

        Self { config, trie }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let config = MappingCharacterFilterConfig::from_slice(data)?;

        Ok(Self::new(config))
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn name(&self) -> &'static str {
        MAPPING_CHARACTER_FILTER_NAME
    }

    fn apply(&self, text: &str) -> LinderaResult<(String, Vec<usize>, Vec<i64>)> {
        let mut offsets: Vec<usize> = Vec::new();
        let mut diffs: Vec<i64> = Vec::new();

        let mut result = String::new();
        let mut input_start = 0_usize;
        let len = text.len();

        while input_start < len {
            let suffix = &text[input_start..];
            match self
                .trie
                .common_prefix_search(suffix.as_bytes())
                .last()
                .map(|(_offset_len, prefix_len)| prefix_len)
            {
                Some(input_len) => {
                    let input_text = &text[input_start..input_start + input_len];
                    let replacement_text = &self.config.mapping[input_text];
                    let replacement_len = replacement_text.len();
                    let diff_len = input_len as i64 - replacement_len as i64;
                    let input_offset = input_start + input_len;

                    if diff_len != 0 {
                        let prev_diff = *diffs.last().unwrap_or(&0);

                        if diff_len > 0 {
                            // Replacement is shorter than matched surface.
                            let offset = (input_offset as i64 - diff_len - prev_diff) as usize;
                            let diff = prev_diff + diff_len;
                            add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                        } else {
                            // Replacement is longer than matched surface.
                            let output_offset = (input_offset as i64 + -prev_diff) as usize;
                            for extra_idx in 0..diff_len.unsigned_abs() as usize {
                                let offset = output_offset + extra_idx;
                                let diff = prev_diff - extra_idx as i64 - 1;
                                add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                            }
                        }
                    }

                    result.push_str(replacement_text);

                    // move start offset
                    input_start += input_len;
                }
                None => {
                    match suffix.chars().next() {
                        Some(c) => {
                            result.push(c);

                            // move start offset
                            input_start += c.len_utf8();
                        }
                        None => break,
                    }
                }
            }
        }

        Ok((result, offsets, diffs))
    }
}

#[cfg(test)]
mod tests {
    use crate::character_filter::{
        correct_offset,
        mapping::{MappingCharacterFilter, MappingCharacterFilterConfig},
        CharacterFilter,
    };

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
        {
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
            let text = "ｱｲｳｴｵ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("イ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("ｲ", &text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "ﾘ": "リ",
                    "ﾝ": "ン",
                    "ﾃﾞ": "デ",
                    "ﾗ": "ラ"
                }
            }
            "#;
            let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(&text).unwrap();
            assert_eq!("リンデラ", filterd_text);
            assert_eq!(vec![9], offsets);
            assert_eq!(vec![3], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("デ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "ﾘﾝﾃﾞﾗ": "リンデラ"
                }
            }
            "#;
            let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("リンデラ", filterd_text);
            assert_eq!(vec![12], offsets);
            assert_eq!(vec![3], diffs);
            let start = 0;
            let end = 12;
            assert_eq!("リンデラ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(0, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("ﾘﾝﾃﾞﾗ", &text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "リンデラ": "Lindera"
                }
            }
            "#;
            let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let text = "Rust製形態素解析器リンデラで日本語を形態素解析する。";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!(
                "Rust製形態素解析器Linderaで日本語を形態素解析する。",
                filterd_text
            );
            assert_eq!(vec![32], offsets);
            assert_eq!(vec![5], diffs);
            let start = 25;
            let end = 32;
            assert_eq!("Lindera", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(25, correct_start);
            assert_eq!(37, correct_end);
            assert_eq!("リンデラ", &text[correct_start..correct_end]);
            let start = 35;
            let end = 44;
            assert_eq!("日本語", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(40, correct_start);
            assert_eq!(49, correct_end);
            assert_eq!("日本語", &text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "１": "1",
                    "０": "0",
                    "㍑": "リットル"
                }
            }
            "#;
            let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let text = "１０㍑";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("10リットル", filterd_text);
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2, -3, -4, -5], diffs);
            let start = 0;
            let end = 2;
            assert_eq!("10", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(0, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("１０", &text[correct_start..correct_end]);
            let start = 2;
            let end = 14;
            assert_eq!("リットル", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㍑", &text[correct_start..correct_end]);
        }
    }
}
