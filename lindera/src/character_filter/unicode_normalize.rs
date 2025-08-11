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
    use crate::character_filter::{CharacterFilter, correct_offset};

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

        {
            let original_text = "ＡＢＣＤＥ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ＡＢＣＤＥ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("Ｂ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("Ｂ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ABCDE";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 5;
            assert_eq!("DE", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(5, correct_end);
            assert_eq!("DE", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ｱｲｳｴｵ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 9;
            assert_eq!("ｲｳ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("ｲｳ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "アイウエオ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("オ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("オ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "０１２３４５６７８９";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("０１２３４５６７８９", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("４", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("４", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "0123456789";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 5;
            let end = 6;
            assert_eq!("5", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(5, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("5", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ﾘﾝﾃﾞﾗ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ﾃﾞ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "１０㌎";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("１０㌎", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("㌎", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &original_text[correct_start..correct_end]);
        }
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

        {
            let original_text = "ＡＢＣＤＥ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ＡＢＣＤＥ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("Ｂ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("Ｂ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ABCDE";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 5;
            assert_eq!("DE", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(5, correct_end);
            assert_eq!("DE", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ｱｲｳｴｵ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 9;
            assert_eq!("ｲｳ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(3, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("ｲｳ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "アイウエオ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("オ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("オ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "０１２３４５６７８９";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("０１２３４５６７８９", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("４", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("４", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "0123456789";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 5;
            let end = 6;
            assert_eq!("5", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(5, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("5", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ﾘﾝﾃﾞﾗ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ﾃﾞ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "１０㌎";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("１０㌎", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("㌎", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &original_text[correct_start..correct_end]);
        }
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
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(vec![1, 2, 3, 4, 5], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10], diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ＣＤ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ABCDE";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(2, correct_start);
            assert_eq!(4, correct_end);
            assert_eq!("CD", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ｳｴ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "アイウエオ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ウエ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "０１２３４５６７８９";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(18, correct_start);
            assert_eq!(27, correct_end);
            assert_eq!("６７８", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "0123456789";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("678", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("リンデラ", text.as_str());
            assert_eq!(vec![9], offsets);
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
            let original_text = "１０㌎";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("10ガロン", text.as_str());
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2], diffs);
            let start = 2;
            let end = 11;
            assert_eq!("ガロン", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
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

        {
            let original_text = "ＡＢＣＤＥ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(vec![1, 2, 3, 4, 5], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10], diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ＣＤ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ABCDE";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ABCDE", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(2, correct_start);
            assert_eq!(4, correct_end);
            assert_eq!("CD", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ｳｴ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "アイウエオ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("アイウエオ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ウエ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "０１２３４５６７８９";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(18, correct_start);
            assert_eq!(27, correct_end);
            assert_eq!("６７８", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "0123456789";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("0123456789", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("678", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("リンテ\u{3099}ラ", text.as_str());
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 15;
            assert_eq!("テ\u{3099}ラ", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("ﾃﾞﾗ", &original_text[correct_start..correct_end]);
        }

        {
            let original_text = "１０㌎";
            let mut text: String = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("10カ\u{3099}ロン", text.as_str());
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2, -3, -4, -5], diffs);
            let start = 2;
            let end = 14;
            assert_eq!("カ\u{3099}ロン", &text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, text_len);
            let correct_end = correct_offset(end, &offsets, &diffs, text_len);
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &original_text[correct_start..correct_end]);
        }
    }
}
