use regex::Regex;
use serde_json::Value;

use crate::LinderaResult;
use crate::character_filter::{CharacterFilter, OffsetMapping, Transformation};
use crate::error::LinderaErrorKind;

pub const REGEX_CHARACTER_FILTER_NAME: &str = "regex";

pub type RegexCharacterFilterConfig = Value;

/// Character filter that uses a regular expression for the target of replace string.
///
#[derive(Clone, Debug)]
pub struct RegexCharacterFilter {
    replacement: String,
    regex: Regex,
}

impl RegexCharacterFilter {
    pub fn new(pattern: &str, replacement: &str) -> LinderaResult<Self> {
        let regex = Regex::new(pattern).map_err(|err| LinderaErrorKind::Args.with_error(err))?;

        Ok(Self {
            replacement: replacement.to_string(),
            regex,
        })
    }

    pub fn from_config(config: &RegexCharacterFilterConfig) -> LinderaResult<Self> {
        let pattern = config
            .get("pattern")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing pattern config."))
            })?
            .as_str()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("invalid pattern config."))
            })?;

        let replacement = config
            .get("replacement")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("missing replacement config."))
            })?
            .as_str()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("invalid replacement config."))
            })?;

        Self::new(pattern, replacement)
    }
}

impl CharacterFilter for RegexCharacterFilter {
    fn name(&self) -> &'static str {
        REGEX_CHARACTER_FILTER_NAME
    }

    /// Apply the filter using the OffsetMapping API
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
        let mut filtered_text = String::with_capacity(text.len());
        let mut mapping = OffsetMapping::new();
        let mut last_match_end = 0;

        for mat in self.regex.find_iter(text) {
            let input_start = mat.start();
            let input_len = mat.len();
            let replacement_text = self.replacement.as_str();
            let replacement_len = replacement_text.len();

            // Append the text before the match
            filtered_text.push_str(&text[last_match_end..input_start]);

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

            // Apply the replacement
            filtered_text.push_str(replacement_text);

            last_match_end = input_start + input_len;
        }

        // Append the remaining text after the last match
        filtered_text.push_str(&text[last_match_end..]);

        *text = filtered_text;
        Ok(mapping)
    }
}

#[cfg(test)]
mod tests {
    use crate::character_filter::CharacterFilter;
    use crate::character_filter::regex::{RegexCharacterFilter, RegexCharacterFilterConfig};

    #[test]
    fn test_regex_character_filter_config() {
        let config_str = r#"
        {
            "pattern": "リンデラ",
            "replacement": "Lindera"
        }
        "#;
        let result: Result<RegexCharacterFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_character_filter_from_config() {
        let config_str = r#"
        {
            "pattern": "リンデラ",
            "replacement": "Lindera"
        }
        "#;
        let config: RegexCharacterFilterConfig = serde_json::from_str(config_str).unwrap();

        let result = RegexCharacterFilter::from_config(&config);
        assert!(result.is_ok());
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
            let config: RegexCharacterFilterConfig = serde_json::from_str(config_str).unwrap();

            let filter = RegexCharacterFilter::from_config(&config).unwrap();
            let original_text = "リンデラは形態素解析器です。";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("Linderaは形態素解析器です。", text.as_str());

            // Verify transformation: "リンデラ"(0-12) → "Lindera"(0-7)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(0, transform.original_start);
            assert_eq!(12, transform.original_end);
            assert_eq!(0, transform.filtered_start);
            assert_eq!(7, transform.filtered_end);

            // Test text fragments
            let start = 0;
            let end = 7;
            assert_eq!("Lindera", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
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
            let config: RegexCharacterFilterConfig = serde_json::from_str(config_str).unwrap();

            let filter = RegexCharacterFilter::from_config(&config).unwrap();
            let original_text = "a     b     c";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("a b c", text.as_str());

            // Verify transformations: two groups of spaces compressed
            assert_eq!(2, mapping.transformations.len());
            let transform1 = &mapping.transformations[0];
            assert_eq!(1, transform1.original_start);
            assert_eq!(6, transform1.original_end);
            assert_eq!(1, transform1.filtered_start);
            assert_eq!(2, transform1.filtered_end);

            let transform2 = &mapping.transformations[1];
            assert_eq!(7, transform2.original_start);
            assert_eq!(12, transform2.original_end);
            assert_eq!(3, transform2.filtered_start);
            assert_eq!(4, transform2.filtered_end);

            // Test text fragments
            let start = 2;
            let end = 3;
            assert_eq!("b", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(7, correct_end);
            assert_eq!("b", &original_text[correct_start..correct_end]);
        }
    }
}
