use serde_json::Value;

use lindera_core::{character_filter::CharacterFilter, token_filter::TokenFilter};

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
        japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
        },
        japanese_keep_tags::{JapaneseKeepTagsTokenFilter, JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME},
        japanese_reading_form::{
            JapaneseReadingFormTokenFilter, JAPANESE_READING_FORM_TOKEN_FILTER_NAME,
        },
        japanese_stop_tags::{JapaneseStopTagsTokenFilter, JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME},
        keep_words::{KeepWordsTokenFilter, KEEP_WORDS_TOKEN_FILTER_NAME},
        length::{LengthTokenFilter, LENGTH_TOKEN_FILTER_NAME},
        lowercase::{LowercaseTokenFilter, LOWERCASE_TOKEN_FILTER_NAME},
        stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME},
        uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME},
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
                        MAPPING_CHARACTER_FILTER_NAME => {
                            character_filters
                                .push(Box::new(MappingCharacterFilter::from_slice(&arg_bytes)?));
                        }
                        REGEX_CHARACTER_FILTER_NAME => {
                            character_filters
                                .push(Box::new(RegexCharacterFilter::from_slice(&arg_bytes)?));
                        }
                        UNICODE_NORMALIZE_CHARACTER_FILTER_NAME => {
                            character_filters.push(Box::new(
                                UnicodeNormalizeCharacterFilter::from_slice(&arg_bytes)?,
                            ));
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
                        JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(
                                JapaneseKatakanaStemTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(JapaneseKeepTagsTokenFilter::from_slice(
                                &args_bytes,
                            )?));
                        }
                        JAPANESE_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(
                                JapaneseReadingFormTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(JapaneseStopTagsTokenFilter::from_slice(
                                &args_bytes,
                            )?));
                        }
                        KEEP_WORDS_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(Box::new(KeepWordsTokenFilter::from_slice(&args_bytes)?));
                        }
                        LENGTH_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(Box::new(LengthTokenFilter::from_slice(&args_bytes)?));
                        }
                        LOWERCASE_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(LowercaseTokenFilter::default()));
                        }
                        STOP_WORDS_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(Box::new(StopWordsTokenFilter::from_slice(&args_bytes)?));
                        }
                        UPPERCASE_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(UppercaseTokenFilter::default()));
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

        let mut tokens = self.tokenizer.tokenize_with_details(text.as_str())?;

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
                            "(株)": "株式会社"
                        }            
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "\\s{2,}",
                        "replacement": " "
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "stop_words",
                    "args": {
                        "stop_words": [
                            "be",
                            "is",
                            "not",
                            "or",
                            "the",
                            "this",
                            "to"
                        ]
                    }
                },
                {
                    "kind": "length",
                    "args": {
                        "min": 1
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
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
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "(株)": "株式会社"
                        }            
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "\\s{2,}",
                        "replacement": " "
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "stop_words",
                    "args": {
                        "stop_words": [
                            "be",
                            "is",
                            "not",
                            "or",
                            "the",
                            "this",
                            "to"
                        ]
                    }
                },
                {
                    "kind": "length",
                    "args": {
                        "min": 2
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let analyzer = Analyzer::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "Ｌｉｎｄｅｒａは、日本語の形態素解析ｴﾝｼﾞﾝです。".to_string();
        let tokens = analyzer.analyze(&mut text).unwrap();

        assert_eq!(
            tokens.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
            vec!["Lindera", "日本語", "形態素", "解析", "エンジン", "です"]
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
                            "(株)": "株式会社"
                        }            
                    }
                },
                {
                    "kind": "regex",
                    "args": {
                        "pattern": "\\s{2,}",
                        "replacement": " "
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "stop_words",
                    "args": {
                        "stop_words": [
                            "be",
                            "is",
                            "not",
                            "or",
                            "the",
                            "this",
                            "to"
                        ]
                    }
                },
                {
                    "kind": "length",
                    "args": {
                        "min": 2
                    }
                },
                {
                    "kind": "japanese_katakana_stem_wrong",  // wrong token filter name
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let result = Analyzer::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_err());
    }
}
