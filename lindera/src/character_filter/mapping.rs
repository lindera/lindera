use std::collections::HashMap;

use serde_json::Value;
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::LinderaResult;
use crate::character_filter::{CharacterFilter, add_offset_diff};
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
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
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

    /// Applies the configured mappings to the input text, replacing matching substrings and tracking offsets.
    ///
    /// # Arguments
    ///
    /// * `text` - A mutable reference to the input text (`String`) that will be modified in place.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult` containing:
    /// - A vector of offsets where modifications occurred.
    /// - A vector of differences (in bytes) for each modification.
    /// - The final length of the modified text.
    ///
    /// # Process
    ///
    /// 1. **Prefix Matching**:
    ///    - The function uses a trie structure to search for common prefixes in the input text.
    ///    - If a match is found, the corresponding replacement text is inserted into the output, and the difference in length between the original and replacement is recorded.
    ///
    /// 2. **Offset and Difference Tracking**:
    ///    - If the replacement text is shorter or longer than the matched text, the offsets and differences are adjusted accordingly to maintain the correct byte positions in the modified text.
    ///
    /// 3. **Text Construction**:
    ///    - The function constructs the filtered text in `filtered_text` by pushing characters or replacements into it while updating the input start position.
    ///
    /// # Returns
    ///
    /// - `offsets`: A vector of byte positions in the original text where changes occurred.
    /// - `diffs`: A vector of byte differences at each position where changes were made.
    /// - `text.len()`: The length of the modified text.
    fn apply<'a>(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>, usize)> {
        let mut offsets: Vec<usize> = Vec::new();
        let mut diffs: Vec<i64> = Vec::new();

        let mut filtered_text = String::with_capacity(text.len());
        let mut input_start = 0_usize;
        let len = text.len();
        let mut prev_diff = 0_i64;

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
                    let diff_len = input_len as i64 - replacement_len as i64;
                    let input_offset = input_start + input_len;

                    if diff_len != 0 {
                        if diff_len > 0 {
                            // Replacement is shorter than matched surface.
                            let offset = (input_offset as i64 - diff_len - prev_diff) as usize;
                            let diff = prev_diff + diff_len;
                            add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                        } else {
                            // Replacement is longer than matched surface.
                            let output_offset = (input_offset as i64 - prev_diff) as usize;
                            for extra_idx in 0..diff_len.unsigned_abs() as usize {
                                let offset = output_offset + extra_idx;
                                let diff = prev_diff - extra_idx as i64 - 1;
                                add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                            }
                        }
                        prev_diff += diff_len;
                    }

                    filtered_text.push_str(replacement_text);

                    // move start offset
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

        Ok((offsets, diffs, text.len()))
    }
}

#[cfg(test)]
mod tests {
    use crate::character_filter::mapping::{MappingCharacterFilter, MappingCharacterFilterConfig};
    use crate::character_filter::{CharacterFilter, correct_offset};

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
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("イ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
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
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());
            assert_eq!(vec![9], offsets);
            assert_eq!(vec![3], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("デ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
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
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());
            assert_eq!(vec![12], offsets);
            assert_eq!(vec![3], diffs);
            let start = 0;
            let end = 12;
            assert_eq!("リンデラ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
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
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!(
                "Rust製形態素解析器Linderaで日本語を形態素解析する。",
                text.as_str()
            );
            assert_eq!(vec![32], offsets);
            assert_eq!(vec![5], diffs);
            let start = 25;
            let end = 32;
            assert_eq!("Lindera", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(25, correct_start);
            assert_eq!(37, correct_end);
            assert_eq!("リンデラ", &original_text[correct_start..correct_end]);
            let start = 35;
            let end = 44;
            assert_eq!("日本語", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
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
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "１０㍑";
            let mut text = original_text.to_string();
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("10リットル", text.as_str());
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2, -3, -4, -5], diffs);
            let start = 0;
            let end = 2;
            assert_eq!("10", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(0, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("１０", &original_text[correct_start..correct_end]);
            let start = 2;
            let end = 14;
            assert_eq!("リットル", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㍑", &original_text[correct_start..correct_end]);
        }
    }
}
