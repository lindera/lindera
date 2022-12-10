use std::borrow::Cow;

use serde_json::Value;

use lindera_core::{
    character_filter::{correct_offset, CharacterFilter},
    token_filter::TokenFilter,
};

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
        japanese_base_form::{JapaneseBaseFormTokenFilter, JAPANESE_BASE_FORM_TOKEN_FILTER_NAME},
        japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
        },
        japanese_keep_tags::{JapaneseKeepTagsTokenFilter, JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME},
        japanese_reading_form::{
            JapaneseReadingFormTokenFilter, JAPANESE_READING_FORM_TOKEN_FILTER_NAME,
        },
        japanese_stop_tags::{JapaneseStopTagsTokenFilter, JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME},
        keep_words::{KeepWordsTokenFilter, KEEP_WORDS_TOKEN_FILTER_NAME},
        korean_keep_tags::{KoreanKeepTagsTokenFilter, KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME},
        korean_reading_form::{
            KoreanReadingFormTokenFilter, KOREAN_READING_FORM_TOKEN_FILTER_NAME,
        },
        korean_stop_tags::{KoreanStopTagsTokenFilter, KOREAN_STOP_TAGS_TOKEN_FILTER_NAME},
        length::{LengthTokenFilter, LENGTH_TOKEN_FILTER_NAME},
        lowercase::{LowercaseTokenFilter, LOWERCASE_TOKEN_FILTER_NAME},
        stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME},
        uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME},
    },
    tokenizer::Tokenizer,
    LinderaResult, Token,
};

pub struct Analyzer {
    character_filters: Vec<Box<dyn CharacterFilter>>,
    tokenizer: Tokenizer,
    token_filters: Vec<Box<dyn TokenFilter>>,
    with_details: bool,
}

impl Analyzer {
    pub fn new(
        character_filters: Vec<Box<dyn CharacterFilter>>,
        tokenizer: Tokenizer,
        token_filters: Vec<Box<dyn TokenFilter>>,
    ) -> Self {
        let with_details = token_filters
            .iter()
            .map(|token_filter| token_filter.name())
            .any(|name| {
                name == JAPANESE_BASE_FORM_TOKEN_FILTER_NAME
                    || name == JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME
                    || name == JAPANESE_READING_FORM_TOKEN_FILTER_NAME
                    || name == JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME
                    || name == KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME
                    || name == KOREAN_READING_FORM_TOKEN_FILTER_NAME
                    || name == KOREAN_STOP_TAGS_TOKEN_FILTER_NAME
            });

        Self {
            character_filters,
            tokenizer,
            token_filters,
            with_details,
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
                        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(JapaneseBaseFormTokenFilter::from_slice(
                                &args_bytes,
                            )?));
                        }
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
                        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(KoreanKeepTagsTokenFilter::from_slice(
                                &args_bytes,
                            )?));
                        }
                        KOREAN_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(KoreanReadingFormTokenFilter::default()));
                        }
                        KOREAN_STOP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(Box::new(KoreanStopTagsTokenFilter::from_slice(
                                &args_bytes,
                            )?));
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

    pub fn analyze(&self, text: &str) -> crate::LinderaResult<Vec<crate::Token>> {
        let mut text_len_vec: Vec<usize> = Vec::new();
        let mut offsets_vec: Vec<Vec<usize>> = Vec::new();
        let mut diffs_vec: Vec<Vec<i64>> = Vec::new();

        let mut tmp_text = text.to_string();

        // Appy character filters.
        for character_filter in &self.character_filters {
            let (new_text, offsets, diffs) = character_filter.apply(&tmp_text)?;

            // Record the length of the text after each character filter is applied.
            text_len_vec.insert(0, new_text.len());
            // Record the offsets of each character filter.
            offsets_vec.insert(0, offsets);
            // Record the diffs of each character filter.
            diffs_vec.insert(0, diffs);

            tmp_text = new_text;
        }

        // Tokenize.
        let mut tmp_tokens = if self.with_details {
            self.tokenizer.tokenize_with_details(&tmp_text)?
        } else {
            self.tokenizer.tokenize(&tmp_text)?
        };

        // Apply token filters.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tmp_tokens)?;
        }

        // Correct token offsets
        let mut tokens = Vec::new();
        for token in tmp_tokens.iter_mut() {
            let mut new_token = Token {
                text: Cow::Owned(token.text.to_string()),
                details: token.details.clone(),
                byte_start: token.byte_start,
                byte_end: token.byte_end,
            };

            for (i, offsets) in offsets_vec.iter().enumerate() {
                new_token.byte_start = correct_offset(
                    new_token.byte_start,
                    offsets,
                    &diffs_vec[i],
                    text_len_vec[i],
                );
                new_token.byte_end =
                    correct_offset(new_token.byte_end, offsets, &diffs_vec[i], text_len_vec[i]);
            }

            tokens.push(new_token);
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
                            "リンデラ": "Lindera"
                        }
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
                    "kind": "japanese_stop_tags",
                    "args": {
                        "stop_tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
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
                            "リンデラ": "Lindera"
                        }
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
                    "kind": "japanese_stop_tags",
                    "args": {
                        "stop_tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
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

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
                vec!["Lindera", "形態素", "解析", "エンジン"]
            );

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "Lindera");
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
                vec!["10", "ガロン", "ガソリン"]
            );

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "10");
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガロン");
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガソリン");
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }
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
                            "リンデラ": "Lindera"
                        }
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
                    "kind": "japanese_stop_tags",
                    "args": {
                        "stop_tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "unexisting_filter",
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
