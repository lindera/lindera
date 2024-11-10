use std::borrow::Cow;
use std::num::NonZeroUsize;

use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME: &str = "japanese_katakana_stem";
const DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK: char = '\u{30FC}';

pub type JapaneseKatakanaStemTokenFilterConfig = Value;

/// Normalizes common katakana spelling variations ending with a long sound (U+30FC)
/// by removing that character.
/// Only katakana words longer than the minimum length are stemmed.
///
#[derive(Clone, Debug)]
pub struct JapaneseKatakanaStemTokenFilter {
    // config: JapaneseKatakanaStemTokenFilterConfig,
    min: NonZeroUsize,
}

impl JapaneseKatakanaStemTokenFilter {
    pub fn new(min: NonZeroUsize) -> Self {
        Self { min }
    }

    pub fn from_config(config: &JapaneseKatakanaStemTokenFilterConfig) -> LinderaResult<Self> {
        let min = config
            .get("min")
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing min config."))
            })?
            .as_u64()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("min must be a positive integer."))
            })?;

        let min = NonZeroUsize::new(min as usize).ok_or_else(|| {
            LinderaErrorKind::Args.with_error(anyhow::anyhow!("invalid min config."))
        })?;

        Ok(Self::new(min))
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
        let min_len = self.min.get();

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
    fn test_japanese_katakana_stem_token_filter_config() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilterConfig;

        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let result: Result<JapaneseKatakanaStemTokenFilterConfig, _> =
            serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_katakana_stem_token_filter_config_zero() {
        use crate::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilterConfig;

        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let result: Result<JapaneseKatakanaStemTokenFilterConfig, _> =
            serde_json::from_str(config_str);

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_katakana_stem_token_filter() {
        use crate::token_filter::japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
        };

        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let config: JapaneseKatakanaStemTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let result = JapaneseKatakanaStemTokenFilter::from_config(&config);

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_zero() {
        use crate::token_filter::japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
        };

        let config_str = r#"
            {
                "min": 0
            }
            "#;
        let config: JapaneseKatakanaStemTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let result = JapaneseKatakanaStemTokenFilter::from_config(&config);

        assert_eq!(result.is_err(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_katakana_stem_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
        };
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "min": 3
            }
            "#;
        let config: JapaneseKatakanaStemTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKatakanaStemTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("バター"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 94843,
                    is_system: true,
                },
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
                word_id: WordId {
                    id: 100137,
                    is_system: true,
                },
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
