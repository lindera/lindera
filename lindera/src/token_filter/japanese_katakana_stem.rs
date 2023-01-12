use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME: &str = "japanese_katakana_stem";
const DEFAULT_MIN: usize = 3;
const DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK: char = '\u{30FC}';

fn default_min() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_MIN).unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKatakanaStemTokenFilterConfig {
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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let min = self.config.min.get();

        for token in tokens.iter_mut() {
            if !is_katakana(token.get_text()) {
                continue;
            }

            if token
                .get_text()
                .ends_with(DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK)
                && token.get_text().chars().count() > min
            {
                token.set_text(
                    token.get_text()[..token.get_text().len()
                        - DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK.len_utf8()]
                        .to_string(),
                );
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
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::japanese_katakana_stem::{
        JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
    };
    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

    #[test]
    fn test_japanese_katakana_stem_token_filter_config_from_slice() {
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
    fn test_japanese_katakana_stem_token_filter_config_from_slice_zero() {
        let config_str = r#"
        {
            "min": 0
        }
        "#;
        let result = JapaneseKatakanaStemTokenFilterConfig::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_japanese_katakana_stem_token_filter_from_slice() {
        let config_str = r#"
        {
            "min": 1
        }
        "#;
        let result = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_katakana_stem_token_filter_from_slice_zero() {
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
        let config_str = r#"
        {
            "min": 3
        }
        "#;
        let filter = JapaneseKatakanaStemTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("カー", 0, 6, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "カー".to_string(),
                    "カー".to_string(),
                    "カー".to_string(),
                ]))
                .clone(),
            Token::new("レバー", 7, 16, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "レバー".to_string(),
                    "レバー".to_string(),
                    "レバー".to_string(),
                ]))
                .clone(),
            Token::new("サッカー", 17, 29, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "サッカー".to_string(),
                    "サッカー".to_string(),
                    "サッカー".to_string(),
                ]))
                .clone(),
            Token::new("レシーバー", 30, 45, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "レシーバー".to_string(),
                    "レシーバー".to_string(),
                    "レシーバー".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "カー");
        assert_eq!(tokens[1].get_text(), "レバー");
        assert_eq!(tokens[2].get_text(), "サッカ");
        assert_eq!(tokens[3].get_text(), "レシーバ");
    }
}
