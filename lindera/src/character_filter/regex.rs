use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::character_filter::{add_offset_diff, CharacterFilter};

pub const REGEX_CHARACTER_FILTER_NAME: &str = "regex";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegexCharacterFilterConfig {
    pub pattern: String,
    pub replacement: String,
}

impl RegexCharacterFilterConfig {
    pub fn new(pattern: String, replacement: String) -> Self {
        Self {
            pattern,
            replacement,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<RegexCharacterFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<RegexCharacterFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Character filter that uses a regular expression for the target of replace string.
///
#[derive(Clone, Debug)]
pub struct RegexCharacterFilter {
    config: RegexCharacterFilterConfig,
    regex: Regex,
}

impl RegexCharacterFilter {
    pub fn new(config: RegexCharacterFilterConfig) -> LinderaResult<Self> {
        let regex =
            Regex::new(&config.pattern).map_err(|err| LinderaErrorKind::Args.with_error(err))?;

        Ok(Self { config, regex })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Self::new(RegexCharacterFilterConfig::from_slice(data)?)
    }
}

impl CharacterFilter for RegexCharacterFilter {
    fn name(&self) -> &'static str {
        REGEX_CHARACTER_FILTER_NAME
    }

    fn apply<'a>(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>, usize)> {
        let mut offsets: Vec<usize> = Vec::new();
        let mut diffs: Vec<i64> = Vec::new();

        self.regex.find_iter(text).for_each(|mat| {
            let input_start = mat.start();
            let input_text = mat.as_str();
            let input_len = input_text.len();
            let replacement_text = self.config.replacement.as_str();
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
                    let output_start = (input_offset as i64 + -prev_diff) as usize;
                    for extra_idx in 0..diff_len.unsigned_abs() as usize {
                        let offset = output_start + extra_idx;
                        let diff = prev_diff - extra_idx as i64 - 1;
                        add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                    }
                }
            }
        });

        let filtered_text = self
            .regex
            .replace_all(text, &self.config.replacement)
            .to_string();

        *text = filtered_text;

        Ok((offsets, diffs, text.len()))
    }
}

#[cfg(test)]
mod tests {
    use crate::character_filter::regex::{RegexCharacterFilter, RegexCharacterFilterConfig};
    use crate::character_filter::{correct_offset, CharacterFilter};

    #[test]
    fn test_regex_character_filter_config_from_slice() {
        let config_str = r#"
        {
            "pattern": "リンデラ",
            "replacement": "Lindera"
        }
        "#;
        let config = RegexCharacterFilterConfig::from_slice(config_str.as_bytes()).unwrap();
        assert_eq!("リンデラ", config.pattern);
        assert_eq!("Lindera", config.replacement);
    }

    #[test]
    fn test_regex_character_filter_from_slice() {
        let config_str = r#"
        {
            "pattern": "リンデラ",
            "replacement": "Lindera"
        }
        "#;
        let result = RegexCharacterFilterConfig::from_slice(config_str.as_bytes());
        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_regex_character_filter_apply() {
        {
            let config_str = r#"
            {
                "pattern": "リンデラ",
                "replacement": "Lindera"
            }
            "#;
            let filter = RegexCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let original_text = "リンデラは形態素解析器です。";
            let mut text = original_text.to_string();
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("Linderaは形態素解析器です。", text.as_str());
            assert_eq!(vec![7], offsets);
            assert_eq!(vec![5], diffs);
            let start = 0;
            let end = 7;
            assert_eq!("Lindera", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(0, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("リンデラ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "pattern": "\\s{2,}",
                "replacement": " "
            }
            "#;
            let filter = RegexCharacterFilter::from_slice(config_str.as_bytes()).unwrap();
            let original_text = "a     b     c";
            let mut text = original_text.to_string();
            let (offsets, diffs, text_len) = filter.apply(&mut text).unwrap();
            assert_eq!("a b c", text.as_str());
            assert_eq!(vec![2, 4], offsets);
            assert_eq!(vec![4, 8], diffs);
            let start = 2;
            let end = 3;
            assert_eq!("b", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(7, correct_end);
            assert_eq!("b", &original_text[correct_start..correct_end]);
        }
    }
}
