use std::collections::HashSet;

use serde_json::Value;

use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;
use crate::LinderaResult;

pub const STOP_WORDS_TOKEN_FILTER_NAME: &str = "stop_words";

pub type StopWordsTokenFilterConfig = Value;

/// Remove the tokens of the specified text.
///
#[derive(Clone, Debug)]
pub struct StopWordsTokenFilter {
    words: HashSet<String>,
}

impl StopWordsTokenFilter {
    pub fn new(words: HashSet<String>) -> Self {
        Self { words }
    }

    pub fn from_config(config: &StopWordsTokenFilterConfig) -> LinderaResult<Self> {
        let words: HashSet<String> = config["words"]
            .as_array()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("words is required"))
            })?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!("words must be string"))
                    })
                    .map(|s| s.to_string())
            })
            .collect::<LinderaResult<HashSet<String>>>()?;

        Ok(Self::new(words))
    }
}

impl TokenFilter for StopWordsTokenFilter {
    fn name(&self) -> &'static str {
        STOP_WORDS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        tokens.retain(|token| !self.words.contains(token.text.as_ref()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::stop_words::{StopWordsTokenFilter, StopWordsTokenFilterConfig};

    #[test]
    fn test_stop_words_token_filter_config() {
        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let result: Result<StopWordsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_stop_words_token_filter_from() {
        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let config: StopWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = StopWordsTokenFilter::from_config(&config);

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_stop_words_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind, WordId};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;

        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let config: StopWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = StopWordsTokenFilter::from_config(&config).unwrap();

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("すもも"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 36165,
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
                    Cow::Borrowed("すもも"),
                    Cow::Borrowed("スモモ"),
                    Cow::Borrowed("スモモ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("も"),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 73246,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("係助詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("も"),
                    Cow::Borrowed("モ"),
                    Cow::Borrowed("モ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("もも"),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 74990,
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
                    Cow::Borrowed("もも"),
                    Cow::Borrowed("モモ"),
                    Cow::Borrowed("モモ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("も"),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
                word_id: WordId {
                    id: 73246,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("係助詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("も"),
                    Cow::Borrowed("モ"),
                    Cow::Borrowed("モ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("もも"),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                word_id: WordId {
                    id: 74990,
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
                    Cow::Borrowed("もも"),
                    Cow::Borrowed("モモ"),
                    Cow::Borrowed("モモ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("の"),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
                word_id: WordId {
                    id: 55831,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("連体化"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("の"),
                    Cow::Borrowed("ノ"),
                    Cow::Borrowed("ノ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("うち"),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId {
                    id: 8029,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("非自立"),
                    Cow::Borrowed("副詞可能"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("うち"),
                    Cow::Borrowed("ウチ"),
                    Cow::Borrowed("ウチ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
        assert_eq!(&tokens[3].text, "うち");
    }
}
