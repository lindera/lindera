use lindera_core::{character_filter::CharacterFilter, token_filter::TokenFilter};
use serde_json::Value;

use crate::{
    character_filter::{
        mapping::{MappingCharacterFilter, MAPPING_CHARACTER_FILTER_NAME},
        regex::{RegexCharacterFilter, REGEX_CHARACTER_FILTER_NAME},
        unicode_normalize::{
            UnicodeNormalizeCharacterFilter, UNICODE_NORMALIZE_CHARACTER_FILTER_NAME,
        },
    },
    error::LinderaErrorKind,
    token_filter::{
        length::{LengthTokenFilter, LENGTH_TOKEN_FILTER_NAME},
        stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME},
    },
    tokenizer::Tokenizer,
    LinderaResult,
};

pub struct Analyzer {
    character_filters: Vec<Box<dyn CharacterFilter>>,
    tokenizer: Tokenizer,
    token_filters: Vec<Box<dyn TokenFilter>>,
}

impl Analyzer {
    pub fn new(
        character_filters: Vec<Box<dyn CharacterFilter>>,
        tokenizer: Tokenizer,
        token_filters: Vec<Box<dyn TokenFilter>>,
    ) -> Self {
        Self {
            character_filters,
            tokenizer,
            token_filters,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let args = serde_json::from_slice::<Value>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        let mut character_filters: Vec<Box<dyn CharacterFilter>> = Vec::new();
        let character_filter_settings = args["character_filters"].as_array();
        if let Some(character_filter_settings) = character_filter_settings {
            for character_filter_setting in character_filter_settings {
                let character_filter_name = character_filter_setting["kind"].as_str();
                if let Some(character_filter_name) = character_filter_name {
                    let args_value =
                        character_filter_setting["args"]
                            .as_object()
                            .ok_or_else(|| {
                                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                                    "character filter's arguments for {}.",
                                    character_filter_name
                                ))
                            })?;
                    let arg_bytes = serde_json::to_vec(args_value)
                        .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

                    match character_filter_name {
                        UNICODE_NORMALIZE_CHARACTER_FILTER_NAME => {
                            character_filters.push(Box::new(
                                UnicodeNormalizeCharacterFilter::from_slice(&arg_bytes)?,
                            ));
                        }
                        MAPPING_CHARACTER_FILTER_NAME => {
                            character_filters
                                .push(Box::new(MappingCharacterFilter::from_slice(&arg_bytes)?));
                        }
                        REGEX_CHARACTER_FILTER_NAME => {
                            character_filters
                                .push(Box::new(RegexCharacterFilter::from_slice(&arg_bytes)?));
                        }
                        _ => {
                            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                                "unknown character filter {}.",
                                character_filter_name
                            )))
                        }
                    }
                }
            }
        }

        let args_value = args["tokenizer"].as_object().ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing tokenizer config."))
        })?;
        let arg_bytes = serde_json::to_vec(args_value)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        let tokenizer_config = serde_json::from_slice(&arg_bytes)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        let tokenizer = Tokenizer::with_config(tokenizer_config)?;

        let mut token_filters: Vec<Box<dyn TokenFilter>> = Vec::new();
        let token_filter_settings = args["token_filters"].as_array();
        if let Some(token_filter_settings) = token_filter_settings {
            for token_filter_setting in token_filter_settings {
                let token_filter_name = token_filter_setting["kind"].as_str();
                if let Some(token_filter_name) = token_filter_name {
                    let arg_value = token_filter_setting["args"].as_object().ok_or_else(|| {
                        LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                            "token filter's arguments for {}.",
                            token_filter_name
                        ))
                    })?;
                    let args_bytes = serde_json::to_vec(arg_value)
                        .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

                    match token_filter_name {
                        STOP_WORDS_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(Box::new(StopWordsTokenFilter::from_slice(&args_bytes)?));
                        }
                        LENGTH_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(Box::new(LengthTokenFilter::from_slice(&args_bytes)?));
                        }
                        _ => {
                            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                                "unknown token filter {}.",
                                token_filter_name
                            )))
                        }
                    }
                }
            }
        }

        Ok(Self::new(character_filters, tokenizer, token_filters))
    }

    pub fn analyze<'a>(&self, text: &'a mut String) -> crate::LinderaResult<Vec<crate::Token<'a>>> {
        for character_filter in &self.character_filters {
            character_filter.apply(text)?;
        }

        let mut tokens = self.tokenizer.tokenize(text.as_str())?;

        for token_filter in &self.token_filters {
            token_filter.apply(&mut tokens)?;
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzer::Analyzer;

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_ipadic_analyzer_from_slice() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "ｱ": "ア",
                            "ｲ": "イ",
                            "ｳ": "ウ",
                            "ｴ": "エ",
                            "ｵ": "オ"
                        }            
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "リンデラ",
                        "replacement": "lindera"
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": {
                    "decompose": {
                        "kanji_penalty_length_threshold": 2,
                        "kanji_penalty_length_penalty": 3000,
                        "other_penalty_length_threshold": 7,
                        "other_penalty_length_penalty": 1700
                    }
                }
            },
            "token_filters": [
                {
                    "kind": "stop_words",
                    "args": {
                        "stop_words": [
                            "a",
                            "an",
                            "and",
                            "are",
                            "as",
                            "at",
                            "be",
                            "but",
                            "by",
                            "for",
                            "if",
                            "in",
                            "into",
                            "is",
                            "it",
                            "no",
                            "not",
                            "of",
                            "on",
                            "or",
                            "such",
                            "that",
                            "the",
                            "their",
                            "then",
                            "there",
                            "these",
                            "they",
                            "this",
                            "to",
                            "was",
                            "will",
                            "with"
                        ]
                    }
                },
                {
                    "kind": "length",
                    "args": {
                        "min": 1,
                        "max": 3
                    }
                }
            ]
        }
        "#;
        let result = Analyzer::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_ipadic_analyzer_analyze() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "リンデラ",
                        "replacement": "lindera"
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": {
                    "decompose": {
                        "kanji_penalty_length_threshold": 2,
                        "kanji_penalty_length_penalty": 3000,
                        "other_penalty_length_threshold": 7,
                        "other_penalty_length_penalty": 1700
                    }
                }
            },
            "token_filters": [
                {
                    "kind": "length",
                    "args": {
                        "min": 2
                    }
                }
            ]
        }
        "#;
        let analyzer = Analyzer::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ﾘﾝﾃﾞﾗは、日本語の形態素解析エンジンです。".to_string();
        let tokens = analyzer.analyze(&mut text).unwrap();

        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["lindera", "日本語", "形態素", "解析", "エンジン", "です"]
        );
    }

    #[test]
    // #[cfg(feature = "ipadic")]
    fn test_analyzer_from_slice_wrong_config() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "（株）": "株式会社",
                            "〒": "郵便"
                        }            
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "リンデラ",
                        "replacement": "lindera"
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": {
                    "decompose": {
                        "kanji_penalty_length_threshold": 2,
                        "kanji_penalty_length_penalty": 3000,
                        "other_penalty_length_threshold": 7,
                        "other_penalty_length_penalty": 1700
                    }
                }
            },
            "token_filters": [
                {
                    "kind": "stop_words",
                    "args": {
                        "stop_words": [
                            "a",
                            "an",
                            "and",
                            "are",
                            "as",
                            "at",
                            "with"
                        ]
                    }
                },
                {
                    "kind": "length",
                    "args": {
                        "min": 1,
                        "max": 3,  // wrong
                    }
                }
            ]
        }
        "#;
        let result = Analyzer::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_err());
    }
}
