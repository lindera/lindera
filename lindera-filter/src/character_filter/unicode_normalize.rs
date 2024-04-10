use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::character_filter::{add_offset_diff, CharacterFilter};

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
pub struct UnicodeNormalizeCharacterFilterConfig {
    pub kind: UnicodeNormalizeKind,
}

impl UnicodeNormalizeCharacterFilterConfig {
    pub fn new(kind: UnicodeNormalizeKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<UnicodeNormalizeCharacterFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<UnicodeNormalizeCharacterFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Normalize Unicode string.
///
#[derive(Clone, Debug)]
pub struct UnicodeNormalizeCharacterFilter {
    config: UnicodeNormalizeCharacterFilterConfig,
}

impl UnicodeNormalizeCharacterFilter {
    pub fn new(config: UnicodeNormalizeCharacterFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(
            UnicodeNormalizeCharacterFilterConfig::from_slice(data)?,
        ))
    }
}

impl CharacterFilter for UnicodeNormalizeCharacterFilter {
    fn name(&self) -> &'static str {
        UNICODE_NORMALIZE_CHARACTER_FILTER_NAME
    }

    fn apply(&self, text: &str) -> LinderaResult<(String, Vec<usize>, Vec<i64>)> {
        let mut offsets: Vec<usize> = Vec::new();
        let mut diffs: Vec<i64> = Vec::new();

        let mut result = String::new();
        let mut input_start = 0;

        for c in text.graphemes(true) {
            let input_len = c.len();
            let replacement_text = match self.config.kind {
                UnicodeNormalizeKind::NFC => c.nfc().collect::<String>(),
                UnicodeNormalizeKind::NFD => c.nfd().collect::<String>(),
                UnicodeNormalizeKind::NFKC => c.nfkc().collect::<String>(),
                UnicodeNormalizeKind::NFKD => c.nfkd().collect::<String>(),
            };
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

            result.push_str(&replacement_text);

            // move start offset
            input_start += input_len;
        }

        Ok((result, offsets, diffs))
    }
}

#[cfg(test)]
mod tests {

    use crate::character_filter::unicode_normalize::{
        UnicodeNormalizeCharacterFilter, UnicodeNormalizeCharacterFilterConfig,
    };
    use crate::character_filter::{correct_offset, CharacterFilter};

    #[test]
    fn test_unicode_normalize_character_filter_config_from_slice() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let config =
            UnicodeNormalizeCharacterFilterConfig::from_slice(config_str.as_bytes()).unwrap();

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
    fn test_unicode_normalize_character_filter_apply_nfc() {
        let config_str = r#"
        {
            "kind": "nfc"
        }
        "#;
        let filter = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        {
            let text = "ＡＢＣＤＥ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ＡＢＣＤＥ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("Ｂ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("Ｂ", &text[correct_start..correct_end]);
        }

        {
            let text = "ABCDE";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 5;
            assert_eq!("DE", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(5, correct_end);
            assert_eq!("DE", &text[correct_start..correct_end]);
        }

        {
            let text = "ｱｲｳｴｵ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ｱｲｳｴｵ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 9;
            assert_eq!("ｲｳ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("ｲｳ", &text[correct_start..correct_end]);
        }

        {
            let text = "アイウエオ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("オ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("オ", &text[correct_start..correct_end]);
        }

        {
            let text = "０１２３４５６７８９";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("０１２３４５６７８９", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("４", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("４", &text[correct_start..correct_end]);
        }

        {
            let text = "0123456789";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 5;
            let end = 6;
            assert_eq!("5", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(5, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("5", &text[correct_start..correct_end]);
        }

        {
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ﾘﾝﾃﾞﾗ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ﾃﾞ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &text[correct_start..correct_end]);
        }

        {
            let text = "１０㌎";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("１０㌎", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("㌎", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &text[correct_start..correct_end]);
        }
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfd() {
        let config_str = r#"
        {
            "kind": "nfd"
        }
        "#;
        let filter = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        {
            let text = "ＡＢＣＤＥ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ＡＢＣＤＥ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 6;
            assert_eq!("Ｂ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("Ｂ", &text[correct_start..correct_end]);
        }

        {
            let text = "ABCDE";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 5;
            assert_eq!("DE", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(5, correct_end);
            assert_eq!("DE", &text[correct_start..correct_end]);
        }

        {
            let text = "ｱｲｳｴｵ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ｱｲｳｴｵ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 3;
            let end = 9;
            assert_eq!("ｲｳ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(3, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("ｲｳ", &text[correct_start..correct_end]);
        }

        {
            let text = "アイウエオ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("オ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("オ", &text[correct_start..correct_end]);
        }

        {
            let text = "０１２３４５６７８９";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("０１２３４５６７８９", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 12;
            let end = 15;
            assert_eq!("４", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(12, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("４", &text[correct_start..correct_end]);
        }

        {
            let text = "0123456789";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 5;
            let end = 6;
            assert_eq!("5", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(5, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("5", &text[correct_start..correct_end]);
        }

        {
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ﾘﾝﾃﾞﾗ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ﾃﾞ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &text[correct_start..correct_end]);
        }

        {
            let text = "１０㌎";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("１０㌎", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("㌎", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &text[correct_start..correct_end]);
        }
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfkc() {
        let config_str = r#"
        {
            "kind": "nfkc"
        }
        "#;
        let filter = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        {
            let text = "ＡＢＣＤＥ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(vec![1, 2, 3, 4, 5], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10], diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ＣＤ", &text[correct_start..correct_end]);
        }

        {
            let text = "ABCDE";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(2, correct_start);
            assert_eq!(4, correct_end);
            assert_eq!("CD", &text[correct_start..correct_end]);
        }

        {
            let text = "ｱｲｳｴｵ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ｳｴ", &text[correct_start..correct_end]);
        }

        {
            let text = "アイウエオ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ウエ", &text[correct_start..correct_end]);
        }

        {
            let text = "０１２３４５６７８９";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(18, correct_start);
            assert_eq!(27, correct_end);
            assert_eq!("６７８", &text[correct_start..correct_end]);
        }

        {
            let text = "0123456789";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("678", &text[correct_start..correct_end]);
        }

        {
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("リンデラ", filterd_text);
            assert_eq!(vec![9], offsets);
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
            let text = "１０㌎";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("10ガロン", filterd_text);
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2], diffs);
            let start = 2;
            let end = 11;
            assert_eq!("ガロン", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &text[correct_start..correct_end]);
        }
    }

    #[test]
    fn test_unicode_normalize_character_filter_apply_nfkd() {
        let config_str = r#"
        {
            "kind": "nfkd"
        }
        "#;
        let filter = UnicodeNormalizeCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        {
            let text = "ＡＢＣＤＥ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(vec![1, 2, 3, 4, 5], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10], diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ＣＤ", &text[correct_start..correct_end]);
        }

        {
            let text = "ABCDE";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("ABCDE", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 2;
            let end = 4;
            assert_eq!("CD", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(2, correct_start);
            assert_eq!(4, correct_end);
            assert_eq!("CD", &text[correct_start..correct_end]);
        }

        {
            let text = "ｱｲｳｴｵ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ｳｴ", &text[correct_start..correct_end]);
        }

        {
            let text = "アイウエオ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("アイウエオ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 12;
            assert_eq!("ウエ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ウエ", &text[correct_start..correct_end]);
        }

        {
            let text = "０１２３４５６７８９";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], offsets);
            assert_eq!(vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20], diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(18, correct_start);
            assert_eq!(27, correct_end);
            assert_eq!("６７８", &text[correct_start..correct_end]);
        }

        {
            let text = "0123456789";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("0123456789", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 9;
            assert_eq!("678", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("678", &text[correct_start..correct_end]);
        }

        {
            let text = "ﾘﾝﾃﾞﾗ";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("リンテ\u{3099}ラ", filterd_text);
            assert_eq!(Vec::<usize>::new(), offsets);
            assert_eq!(Vec::<i64>::new(), diffs);
            let start = 6;
            let end = 15;
            assert_eq!("テ\u{3099}ラ", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("ﾃﾞﾗ", &text[correct_start..correct_end]);
        }

        {
            let text = "１０㌎";
            let (filterd_text, offsets, diffs) = filter.apply(text).unwrap();
            assert_eq!("10カ\u{3099}ロン", filterd_text);
            assert_eq!(vec![1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13], offsets);
            assert_eq!(vec![2, 4, 3, 2, 1, 0, -1, -2, -3, -4, -5], diffs);
            let start = 2;
            let end = 14;
            assert_eq!("カ\u{3099}ロン", &filterd_text[start..end]);
            let correct_start = correct_offset(start, &offsets, &diffs, filterd_text.len());
            let correct_end = correct_offset(end, &offsets, &diffs, filterd_text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㌎", &text[correct_start..correct_end]);
        }
    }
}
