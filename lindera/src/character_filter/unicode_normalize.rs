use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::LinderaResult;
use crate::error::{LinderaError, LinderaErrorKind};

use crate::character_filter::{CharacterFilter, OffsetMapping, Transformation};

pub const UNICODE_NORMALIZE_CHARACTER_FILTER_NAME: &str = "unicode_normalize";

pub type UnicodeNormalizeCharacterFilterConfig = Value;

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

impl UnicodeNormalizeKind {
    pub fn as_str(&self) -> &str {
        match self {
            UnicodeNormalizeKind::NFC => "nfc",
            UnicodeNormalizeKind::NFD => "nfd",
            UnicodeNormalizeKind::NFKC => "nfkc",
            UnicodeNormalizeKind::NFKD => "nfkd",
        }
    }
}

impl FromStr for UnicodeNormalizeKind {
    type Err = LinderaError;
    fn from_str(kind: &str) -> Result<Self, Self::Err> {
        match kind {
            "nfc" => Ok(UnicodeNormalizeKind::NFC),
            "nfd" => Ok(UnicodeNormalizeKind::NFD),
            "nfkc" => Ok(UnicodeNormalizeKind::NFKC),
            "nfkd" => Ok(UnicodeNormalizeKind::NFKD),
            _ => {
                Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Invalid normalization kind")))
            }
        }
    }
}

/// Unicode normalization to normalize the input text, that using the specified normalization form, one of NFC, NFD, NFKC, or NFKD.
///
#[derive(Clone, Debug)]
pub struct UnicodeNormalizeCharacterFilter {
    kind: UnicodeNormalizeKind,
}

impl UnicodeNormalizeCharacterFilter {
    pub fn new(kind: UnicodeNormalizeKind) -> Self {
        Self { kind }
    }

    pub fn from_config(config: &UnicodeNormalizeCharacterFilterConfig) -> LinderaResult<Self> {
        let kind = config
            .get("kind")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing kind config."))
            })?
            .as_str()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("invalid kind config."))
            })?;
        let kind = UnicodeNormalizeKind::from_str(kind)?;

        Ok(Self::new(kind))
    }
}

impl CharacterFilter for UnicodeNormalizeCharacterFilter {
    fn name(&self) -> &'static str {
        UNICODE_NORMALIZE_CHARACTER_FILTER_NAME
    }

    /// Apply the filter using the OffsetMapping API
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
        let mut filtered_text = String::with_capacity(text.len());
        let mut mapping = OffsetMapping::new();
        let mut input_start = 0;

        for c in text.graphemes(true) {
            let input_len = c.len();
            let replacement_text = match self.kind {
                UnicodeNormalizeKind::NFC => c.nfc().collect::<String>(),
                UnicodeNormalizeKind::NFD => c.nfd().collect::<String>(),
                UnicodeNormalizeKind::NFKC => c.nfkc().collect::<String>(),
                UnicodeNormalizeKind::NFKD => c.nfkd().collect::<String>(),
            };
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

            filtered_text.push_str(&replacement_text);
            input_start += input_len;
        }

        *text = filtered_text;
        Ok(mapping)
    }
}

#[cfg(test)]
mod tests {

    use crate::character_filter::unicode_normalize::{
        UnicodeNormalizeCharacterFilter, UnicodeNormalizeCharacterFilterConfig,
    };
    use crate::character_filter::CharacterFilter;

    #[test]
    fn test_unicode_normalize_character_filter_config() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let result: Result<UnicodeNormalizeCharacterFilterConfig, _> =
            serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_normalize_character_filter() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let config =
            serde_json::from_str::<UnicodeNormalizeCharacterFilterConfig>(config_str).unwrap();

        let result = UnicodeNormalizeCharacterFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfc() {
        let config_str = r#"
        {
            "kind": "nfc"
        }
        "#;
        let config =
            serde_json::from_str::<UnicodeNormalizeCharacterFilterConfig>(config_str).unwrap();

        let filter = UnicodeNormalizeCharacterFilter::from_config(&config).unwrap();

        // NFC generally doesn't change these examples, so mappings are empty
        let original_text = "ＡＢＣＤＥ";
        let mut text = original_text.to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("ＡＢＣＤＥ", text.as_str());
        assert!(mapping.is_empty());
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfd() {
        let config_str = r#"
        {
            "kind": "nfd"
        }
        "#;
        let config =
            serde_json::from_str::<UnicodeNormalizeCharacterFilterConfig>(config_str).unwrap();

        let filter = UnicodeNormalizeCharacterFilter::from_config(&config).unwrap();

        // NFD generally doesn't change these examples, so mappings are empty
        let original_text = "ＡＢＣＤＥ";
        let mut text = original_text.to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("ＡＢＣＤＥ", text.as_str());
        assert!(mapping.is_empty());
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfkc() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let config =
            serde_json::from_str::<UnicodeNormalizeCharacterFilterConfig>(config_str).unwrap();

        let filter = UnicodeNormalizeCharacterFilter::from_config(&config).unwrap();

        {
            let original_text = "ＡＢＣＤＥ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("ABCDE", text.as_str());
            
            // Verify transformations: 5 full-width chars converted to half-width
            assert_eq!(5, mapping.transformations.len());
            let transform = &mapping.transformations[2]; // Check "Ｃ" (6-9) → "C" (2-3)
            assert_eq!(6, transform.original_start);
            assert_eq!(9, transform.original_end);
            assert_eq!(2, transform.filtered_start);
            assert_eq!(3, transform.filtered_end);
            
            // Test text fragments
            let start = 2;
            let end = 4;
            assert_eq!("CD", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ＣＤ", &original_text[correct_start..correct_end]);
        }

        // Additional test: complex case with ㌎ → ガロン expansion
        {
            let original_text = "１０㌎";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("10ガロン", text.as_str());
            
            // Multiple transformations: 2 character normalizations + 1 expansion
            assert_eq!(3, mapping.transformations.len());
            
            // Test expansion: "㌎"(6-9) → "ガロン"(2-11)
            let expand_transform = &mapping.transformations[2];
            assert_eq!(6, expand_transform.original_start);
            assert_eq!(9, expand_transform.original_end);
            assert_eq!(2, expand_transform.filtered_start);
            assert_eq!(11, expand_transform.filtered_end);
            
            let start = 2;
            let end = 11;
            assert_eq!("ガロン", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &original_text[correct_start..correct_end]);
        }
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfkd() {
        let config_str = r#"
        {
            "kind": "nfkd"
        }
        "#;
        let config =
            serde_json::from_str::<UnicodeNormalizeCharacterFilterConfig>(config_str).unwrap();

        let filter = UnicodeNormalizeCharacterFilter::from_config(&config).unwrap();

        // NFKD: Full-width → half-width conversion
        let original_text = "ＡＢＣＤＥ";
        let mut text = original_text.to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("ABCDE", text.as_str());
        
        // Same as NFKC for this example
        assert_eq!(5, mapping.transformations.len());
        let transform = &mapping.transformations[2];
        assert_eq!(6, transform.original_start);
        assert_eq!(9, transform.original_end);
        assert_eq!(2, transform.filtered_start);
        assert_eq!(3, transform.filtered_end);
    }
}
