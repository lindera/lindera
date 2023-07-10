use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::Token, token_filter::TokenFilter};

pub const JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME: &str = "japanese_katakana_stem";
const DEFAULT_MIN: usize = 3;
const DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK: char = '\u{30FC}';

fn default_min() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_MIN).unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKatakanaStemTokenFilterConfig {
    /// Minimum length.
    #[serde(default = "default_min")]
    min: NonZeroUsize,
}

impl JapaneseKatakanaStemTokenFilterConfig {
    pub fn new(min: NonZeroUsize) -> Self {
        Self { min }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Normalizes common katakana spelling variations ending with a long sound (U+30FC)
/// by removing that character.
/// Only katakana words longer than the minimum length are stemmed.
///
#[derive(Clone, Debug)]
pub struct JapaneseKatakanaStemTokenFilter {
    config: JapaneseKatakanaStemTokenFilterConfig,
}

impl JapaneseKatakanaStemTokenFilter {
    pub fn new(config: JapaneseKatakanaStemTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self { config })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Self::new(JapaneseKatakanaStemTokenFilterConfig::from_slice(data)?)
    }
}

impl TokenFilter for JapaneseKatakanaStemTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
        let min = self.config.min.get();

        for token in tokens.iter_mut() {
            if !is_katakana(&token.text) {
                continue;
            }

            if token
                .text
                .ends_with(DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK)
                && token.text.chars().count() > min
            {
                token.text = token.text[..token.text.len()
                    - DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK.len_utf8()]
                    .to_string()
                    .into();
            }
        }

        Ok(())
    }
}

fn is_katakana(text: &str) -> bool {
    for ch in text.chars() {
        let block = unicode_blocks::find_unicode_block(ch).unwrap();
        if block != unicode_blocks::KATAKANA {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_core::word_entry::WordId;

    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use crate::{
        token::Token,
        token_filter::{
            japanese_katakana_stem::{
                JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
            },
            TokenFilter,
        },
    };

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_katakana_stem_token_filter_config_from_slice_ipadic() {
        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let config =
            JapaneseKatakanaStemTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.min.get(), 1);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_katakana_stem_token_filter_config_from_slice_zero_ipadic() {
        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilterConfig::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_katakana_stem_token_filter_from_slice_ipadic() {
        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_katakana_stem_token_filter_from_slice_zero_ipadic() {
        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    fn test_japanese_katakana_stem_token_filter_apply_ipadic() {
        let config_str = r#"
            {
                "min": 3
            }
            "#;
        let filter = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: "バター".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(94843, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "バター".to_string(),
                    "バター".to_string(),
                    "バター".to_string(),
                ],
            },
            Token {
                text: "メーカー".to_string(),
                byte_start: 9,
                byte_end: 21,
                position: 1,
                position_length: 1,
                word_id: WordId(100137, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "バター".to_string(),
                    "バター".to_string(),
                    "バター".to_string(),
                ],
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].text, "バター");
        assert_eq!(&tokens[1].text, "メーカ");
    }
}
