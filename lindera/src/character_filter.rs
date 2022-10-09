use std::collections::HashMap;

use lindera_core::character_filter::CharacterFilter;
use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::{error::LinderaErrorKind, LinderaResult};

pub const UNICODE_NORMALIZE_CHARACTER_FILTER_NAME: &str = "unicode_normalize";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum UnidoceNormalizeKind {
    #[serde(rename = "nfc")]
    NFC,
    #[serde(rename = "nfd")]
    NFD,
    #[serde(rename = "nfkc")]
    NFKC,
    #[serde(rename = "nfkd")]
    NFKD,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UnidoceNormalizeCharacterFilterConfig {
    pub kind: UnidoceNormalizeKind,
}

impl UnidoceNormalizeCharacterFilterConfig {
    pub fn new(kind: UnidoceNormalizeKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct UnicodeNormalizeCharacterFilter {
    config: UnidoceNormalizeCharacterFilterConfig,
}

impl UnicodeNormalizeCharacterFilter {
    pub fn new(config: UnidoceNormalizeCharacterFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(
            UnidoceNormalizeCharacterFilterConfig::from_slice(data)?,
        ))
    }
}

impl CharacterFilter for UnicodeNormalizeCharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<()> {
        *text = match self.config.kind {
            UnidoceNormalizeKind::NFC => text.nfc().collect::<String>(),
            UnidoceNormalizeKind::NFD => text.nfd().collect::<String>(),
            UnidoceNormalizeKind::NFKC => text.nfkc().collect::<String>(),
            UnidoceNormalizeKind::NFKD => text.nfkd().collect::<String>(),
        };

        Ok(())
    }
}

pub const MAPPING_CHARACTER_FILTER_NAME: &str = "mapping";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MappingCharacterFilterConfig {
    pub mapping: HashMap<char, char>,
}

impl MappingCharacterFilterConfig {
    pub fn new(map: HashMap<char, char>) -> Self {
        Self { mapping: map }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct MappingCharacterFilter {
    config: MappingCharacterFilterConfig,
}

impl MappingCharacterFilter {
    pub fn new(config: MappingCharacterFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(MappingCharacterFilterConfig::from_slice(data)?))
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<()> {
        *text = text
            .chars()
            .map(|c| {
                if let Some(ch) = self.config.mapping.get(&c) {
                    *ch
                } else {
                    c
                }
            })
            .collect::<String>();

        Ok(())
    }
}

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
    use crate::character_filter::{
        CharacterFilter, MappingCharacterFilter, MappingCharacterFilterConfig,
        RegexCharacterFilter, RegexCharacterFilterConfig, UnicodeNormalizeCharacterFilter,
        UnidoceNormalizeCharacterFilterConfig,
    };

    #[test]
    fn test_unicode_normalize_character_filter_config_from_slice() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let config =
            UnidoceNormalizeCharacterFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, super::UnidoceNormalizeKind::NFKC);
    }

    #[test]
    fn test_unicode_normalize_character_filter_from_slice() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let result = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let filter = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ＡＢＣＤＥ".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("ABCDE", text);

        let mut text = "ｱｲｳｴｵ".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("アイウエオ", text);

        let mut text = "０１２３４５６７８９".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("0123456789", text);
    }

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

        assert_eq!(&'ア', config.mapping.get(&'ｱ').unwrap());
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
    }

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
