use std::num::NonZeroUsize;

use lindera_core::token_filter::TokenFilter;
use serde::{Deserialize, Serialize};

use crate::{error::LinderaErrorKind, LinderaResult, Token};

const DEFAULT_MIN: usize = 3;
const DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK: char = '\u{30FC}';

fn default_min() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_MIN).unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKatakanaStemmingTokenFilterConfig {
    #[serde(default = "default_min")]
    min: NonZeroUsize,
}

impl JapaneseKatakanaStemmingTokenFilterConfig {
    pub fn new(min: NonZeroUsize) -> Self {
        Self { min }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

pub struct JapaneseKatakanaStemmingTokenFilter {
    config: JapaneseKatakanaStemmingTokenFilterConfig,
}

impl JapaneseKatakanaStemmingTokenFilter {
    pub fn new(config: JapaneseKatakanaStemmingTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self { config })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Self::new(JapaneseKatakanaStemmingTokenFilterConfig::from_slice(data)?)
    }
}

impl TokenFilter for JapaneseKatakanaStemmingTokenFilter {
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let min = self.config.min.get();

        for token in tokens.iter_mut() {
            if !is_katakana(token.text) {
                continue;
            }

            if token
                .text
                .ends_with(DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK)
                && token.text.chars().count() > min
            {
                token.text = &token.text[..token.text.len()
                    - DEFAULT_HIRAGANA_KATAKANA_PROLONGED_SOUND_MARK.len_utf8()];
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
    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::japanese_katakana_stemming::{
            JapaneseKatakanaStemmingTokenFilter, JapaneseKatakanaStemmingTokenFilterConfig,
        },
        Token,
    };

    #[test]
    fn test_japanese_katakana_stemming_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "min": 1
        }
        "#;
        let config =
            JapaneseKatakanaStemmingTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.min.get(), 1);
    }

    #[test]
    fn test_japanese_katakana_stemming_token_filter_config_from_slice_zero() {
        let config_str = r#"
        {
            "min": 0
        }
        "#;
        let result = JapaneseKatakanaStemmingTokenFilterConfig::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_japanese_katakana_stemming_token_filter_from_slice() {
        let config_str = r#"
        {
            "min": 1
        }
        "#;
        let result = JapaneseKatakanaStemmingTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_japanese_katakana_stemming_token_filter_from_slice_zero() {
        let config_str = r#"
        {
            "min": 0
        }
        "#;
        let result = JapaneseKatakanaStemmingTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_japanese_katakana_stemming_token_filter_apply() {
        let config_str = r#"
        {
            "min": 3
        }
        "#;
        let filter =
            JapaneseKatakanaStemmingTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: "カー",
                details: None,
            },
            Token {
                text: "かー",
                details: None,
            },
            Token {
                text: "レバー",
                details: None,
            },
            Token {
                text: "ればー",
                details: None,
            },
            Token {
                text: "サッカー",
                details: None,
            },
            Token {
                text: "さっかー",
                details: None,
            },
            Token {
                text: "レシーバー",
                details: None,
            },
            Token {
                text: "れしーばー",
                details: None,
            },
            Token {
                text: "ア",
                details: None,
            },
            Token {
                text: "あ",
                details: None,
            },
            Token {
                text: "アイ",
                details: None,
            },
            Token {
                text: "あい",
                details: None,
            },
            Token {
                text: "アイウ",
                details: None,
            },
            Token {
                text: "あいう",
                details: None,
            },
            Token {
                text: "アイウエ",
                details: None,
            },
            Token {
                text: "あいうえ",
                details: None,
            },
            Token {
                text: "アイウエオ",
                details: None,
            },
            Token {
                text: "あいうえお",
                details: None,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 18);
        assert_eq!(tokens[0].text, "カー");
        assert_eq!(tokens[1].text, "かー");
        assert_eq!(tokens[2].text, "レバー");
        assert_eq!(tokens[3].text, "ればー");
        assert_eq!(tokens[4].text, "サッカ");
        assert_eq!(tokens[5].text, "さっかー");
        assert_eq!(tokens[6].text, "レシーバ");
        assert_eq!(tokens[7].text, "れしーばー");
        assert_eq!(tokens[8].text, "ア");
        assert_eq!(tokens[9].text, "あ");
        assert_eq!(tokens[10].text, "アイ");
        assert_eq!(tokens[11].text, "あい");
        assert_eq!(tokens[12].text, "アイウ");
        assert_eq!(tokens[13].text, "あいう");
        assert_eq!(tokens[14].text, "アイウエ");
        assert_eq!(tokens[15].text, "あいうえ");
        assert_eq!(tokens[16].text, "アイウエオ");
        assert_eq!(tokens[17].text, "あいうえお");
    }
}
