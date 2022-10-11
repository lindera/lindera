use regex::Regex;
use serde::{Deserialize, Serialize};

use lindera_core::character_filter::CharacterFilter;

use crate::{error::LinderaErrorKind, LinderaResult};

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
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct RegexCharacterFilter {
    config: RegexCharacterFilterConfig,
    regex: Regex,
}

impl RegexCharacterFilter {
    pub fn new(config: RegexCharacterFilterConfig) -> Self {
        let regex = Regex::new(&config.pattern).unwrap();

        Self { config, regex }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(RegexCharacterFilterConfig::from_slice(data)?))
    }
}

impl CharacterFilter for RegexCharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<()> {
        *text = self
            .regex
            .replace_all(text, &self.config.replacement)
            .to_mut()
            .to_string();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lindera_core::character_filter::CharacterFilter;

    use crate::character_filter::regex::{RegexCharacterFilter, RegexCharacterFilterConfig};

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
        let config_str = r#"
        {
            "pattern": "リンデラ",
            "replacement": "Lindera"
        }
        "#;
        let filter = RegexCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "リンデラは形態素解析器です。".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("Linderaは形態素解析器です。", text);
    }
}
