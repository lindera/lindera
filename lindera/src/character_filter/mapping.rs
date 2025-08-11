use std::collections::HashMap;

use serde_json::Value;
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::LinderaResult;
use crate::character_filter::{CharacterFilter, OffsetMapping, Transformation};
use crate::error::LinderaErrorKind;

pub const MAPPING_CHARACTER_FILTER_NAME: &str = "mapping";

pub type MappingCharacterFilterConfig = Value;

#[derive(Clone)]
pub struct MappingCharacterFilter {
    mapping: HashMap<String, String>,
    trie: DoubleArray<Vec<u8>>,
}

impl MappingCharacterFilter {
    pub fn new(mapping: HashMap<String, String>) -> LinderaResult<Self> {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let data = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Build.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        let trie = DoubleArray::new(data);

        Ok(Self { mapping, trie })
    }

    pub fn from_config(config: &MappingCharacterFilterConfig) -> LinderaResult<Self> {
        let mapping = config
            .get("mapping")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("mapping must be an object."))
            })?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<HashMap<String, String>>();

        Self::new(mapping)
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn name(&self) -> &'static str {
        MAPPING_CHARACTER_FILTER_NAME
    }

    /// Apply the filter using the OffsetMapping API
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
        let mut filtered_text = String::with_capacity(text.len());
        let mut mapping = OffsetMapping::new();
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
                    let replacement_text = &self.mapping[input_text];
                    let replacement_len = replacement_text.len();

                    // Record transformation if text changed
                    if input_len != replacement_len {
                        let transformation = Transformation::new(
                            input_start,
                            input_start + input_len,
                            filtered_text.len(),
                            filtered_text.len() + replacement_len,
                        );
                        mapping.add_transformation(transformation);
                    }

                    filtered_text.push_str(replacement_text);
                    input_start += input_len;
                }
                None => {
                    if let Some(c) = suffix.chars().next() {
                        filtered_text.push(c);
                        input_start += c.len_utf8();
                    } else {
                        break;
                    }
                }
            }
        }

        *text = filtered_text;
        Ok(mapping)
    }
}

#[cfg(test)]
mod tests {
    use crate::character_filter::CharacterFilter;
    use crate::character_filter::mapping::{MappingCharacterFilter, MappingCharacterFilterConfig};

    #[test]
    fn test_mapping_character_filter_config() {
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
        let result: Result<MappingCharacterFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mapping_character_filter_from_config() {
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
        let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

        let result = MappingCharacterFilter::from_config(&config);
        assert!(result.is_ok());
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
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();

            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("アイウエオ", text.as_str());
            assert!(mapping.is_empty());

            // Test text fragments
            let start = 3;
            let end = 6;
            assert_eq!("イ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("ｲ", &original_text[correct_start..correct_end]);
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
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());

            // Verify transformation: "ﾃﾞ"(6-12) → "デ"(6-9)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(6, transform.original_start);
            assert_eq!(12, transform.original_end);
            assert_eq!(6, transform.filtered_start);
            assert_eq!(9, transform.filtered_end);

            // Test text fragments
            let start = 6;
            let end = 9;
            assert_eq!("デ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "ﾘﾝﾃﾞﾗ": "リンデラ"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());

            // Verify transformation: "ﾘﾝﾃﾞﾗ"(0-15) → "リンデラ"(0-12)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(0, transform.original_start);
            assert_eq!(15, transform.original_end);
            assert_eq!(0, transform.filtered_start);
            assert_eq!(12, transform.filtered_end);

            // Test text fragments
            let start = 0;
            let end = 12;
            assert_eq!("リンデラ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(0, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("ﾘﾝﾃﾞﾗ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "リンデラ": "Lindera"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "Rust製形態素解析器リンデラで日本語を形態素解析する。";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!(
                "Rust製形態素解析器Linderaで日本語を形態素解析する。",
                text.as_str()
            );

            // Verify transformation: "リンデラ"(25-37) → "Lindera"(25-32)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(25, transform.original_start);
            assert_eq!(37, transform.original_end);
            assert_eq!(25, transform.filtered_start);
            assert_eq!(32, transform.filtered_end);

            // Test text fragments
            let start = 25;
            let end = 32;
            assert_eq!("Lindera", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(25, correct_start);
            assert_eq!(37, correct_end);
            assert_eq!("リンデラ", &original_text[correct_start..correct_end]);

            let start = 35;
            let end = 44;
            assert_eq!("日本語", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(40, correct_start);
            assert_eq!(49, correct_end);
            assert_eq!("日本語", &original_text[correct_start..correct_end]);
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
            let config = serde_json::from_str(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "１０㍑";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("10リットル", text.as_str());

            // All three replacements are recorded because of byte length differences
            assert_eq!(3, mapping.transformations.len());

            // Verify the last transformation: "㍑"(6-9) → "リットル"(2-14)
            let transform = &mapping.transformations[2];
            assert_eq!(6, transform.original_start);
            assert_eq!(9, transform.original_end);
            assert_eq!(2, transform.filtered_start);
            assert_eq!(14, transform.filtered_end);

            // Test text fragments
            let start = 2;
            let end = 14;
            assert_eq!("リットル", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㍑", &original_text[correct_start..correct_end]);
        }
    }
}
