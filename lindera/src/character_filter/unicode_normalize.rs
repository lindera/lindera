use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use lindera_core::character_filter::CharacterFilter;

use crate::{error::LinderaErrorKind, LinderaResult};

pub const UNICODE_NORMALIZE_CHARACTER_FILTER_NAME: &str = "unicode_normalize";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum UnicodeNormalizeKind {
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
    pub kind: UnicodeNormalizeKind,
}

impl UnidoceNormalizeCharacterFilterConfig {
    pub fn new(kind: UnicodeNormalizeKind) -> Self {
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
            UnicodeNormalizeKind::NFC => text.nfc().collect::<String>(),
            UnicodeNormalizeKind::NFD => text.nfd().collect::<String>(),
            UnicodeNormalizeKind::NFKC => text.nfkc().collect::<String>(),
            UnicodeNormalizeKind::NFKD => text.nfkd().collect::<String>(),
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lindera_core::character_filter::CharacterFilter;

    use crate::character_filter::unicode_normalize::{
        UnicodeNormalizeCharacterFilter, UnidoceNormalizeCharacterFilterConfig,
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

        assert_eq!(config.kind, super::UnicodeNormalizeKind::NFKC);
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
}
