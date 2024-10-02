use std::borrow::Cow;
use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

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
        serde_json::from_slice::<JapaneseKatakanaStemTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        serde_json::from_value::<JapaneseKatakanaStemTokenFilterConfig>(value.clone())
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
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
    pub fn new(config: JapaneseKatakanaStemTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(
            JapaneseKatakanaStemTokenFilterConfig::from_slice(data)?,
        ))
    }
}

impl TokenFilter for JapaneseKatakanaStemTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME
    }

    /// Removes prolonged sound marks from katakana tokens if they meet the specified conditions.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The `text` field of each token will be modified in place if the token is katakana and ends with a prolonged sound mark.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Processing**:
    ///    - The function iterates over the provided list of tokens.
    ///    - For each token, it checks whether the token's text is katakana. If not, the token is skipped.
    ///
    /// 2. **Prolonged Sound Mark Removal**:
    ///    - If the token ends with a prolonged sound mark (such as `ー`) and its length exceeds the specified minimum (`min`), the prolonged sound mark is removed.
    ///    - The token's text is updated by removing the last character (the prolonged sound mark).
    ///
    /// # Configurations:
    ///
    /// - **Minimum Length (`min`)**: The token must be longer than this value for the prolonged sound mark to be removed.
    ///
    /// # Errors
    ///
    /// If any issue arises during token processing, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
        let min_len = self.config.min.get();

        for token in tokens.iter_mut() {
            // Skip if the token is not katakana
            if !is_katakana(&token.text) {
                continue;
            }

            // Check if the token ends with the prolonged sound mark and is longer than the minimum length
            if token
                .text
                .ends_with(DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK)
                && token.text.chars().count() > min_len
            {
                // Remove the prolonged sound mark
                let new_len =
                    token.text.len() - DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK.len_utf8();
                token.text = Cow::Owned(token.text[..new_len].to_string());
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
    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_config_from_slice_ipadic() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilterConfig;

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
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_config_from_slice_zero_ipadic() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilterConfig;

        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilterConfig::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_from_slice_ipadic() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;

        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_from_slice_zero_ipadic() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;

        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let result = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "min": 3
            }
            "#;
        let filter = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("バター"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(94843, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("バター"),
                    Cow::Borrowed("バター"),
                    Cow::Borrowed("バター"),
                ]),
            },
            Token {
                text: Cow::Borrowed("メーカー"),
                byte_start: 9,
                byte_end: 21,
                position: 1,
                position_length: 1,
                word_id: WordId(100137, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("バター"),
                    Cow::Borrowed("バター"),
                    Cow::Borrowed("バター"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].text, "バター");
        assert_eq!(&tokens[1].text, "メーカ");
    }
}
