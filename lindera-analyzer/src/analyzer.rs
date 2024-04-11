use std::{fs, path::Path};

use serde::Serialize;
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;
use lindera_filter::character_filter::{correct_offset, BoxCharacterFilter, CharacterFilterLoader};
use lindera_filter::token::Token;
use lindera_filter::token_filter::{BoxTokenFilter, TokenFilterLoader};
use lindera_tokenizer::tokenizer::Tokenizer;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AnalyzerConfig {
    inner: Value,
}

impl AnalyzerConfig {
    pub fn from_file(path: &Path) -> LinderaResult<Self> {
        let bytes = fs::read(path).map_err(|err| LinderaErrorKind::Io.with_error(err))?;

        Self::from_slice(&bytes)
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let args = serde_json::from_slice::<Value>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self { inner: args })
    }
}

pub struct Analyzer {
    /// Character filters
    pub character_filters: Vec<BoxCharacterFilter>,

    /// Tokenizer
    pub tokenizer: Tokenizer,

    /// Token filters
    pub token_filters: Vec<BoxTokenFilter>,
}

impl Analyzer {
    pub fn from_config(config: &AnalyzerConfig) -> LinderaResult<Self> {
        let value = &config.inner;

        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();
        let character_filter_settings = value["character_filters"].as_array();
        if let Some(character_filter_settings) = character_filter_settings {
            for character_filter_setting in character_filter_settings {
                let character_filter_name = character_filter_setting["kind"].as_str();
                if let Some(character_filter_name) = character_filter_name {
                    let character_filter = CharacterFilterLoader::load_from_value(
                        character_filter_name,
                        &character_filter_setting["args"],
                    )?;
                    character_filters.push(character_filter);
                }
            }
        }

        let args_value = value["tokenizer"].as_object().ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing tokenizer config."))
        })?;
        let arg_bytes = serde_json::to_vec(args_value)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        let tokenizer_config = serde_json::from_slice(&arg_bytes)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        let tokenizer = Tokenizer::from_config(tokenizer_config)?;

        let mut token_filters: Vec<BoxTokenFilter> = Vec::new();
        let token_filter_settings = value["token_filters"].as_array();
        if let Some(token_filter_settings) = token_filter_settings {
            for token_filter_setting in token_filter_settings {
                let token_filter_name = token_filter_setting["kind"].as_str();
                if let Some(token_filter_name) = token_filter_name {
                    token_filters.push(TokenFilterLoader::load_from_value(
                        token_filter_name,
                        &token_filter_setting["args"],
                    )?);
                }
            }
        }

        Ok(Self::new(character_filters, tokenizer, token_filters))
    }

    pub fn new(
        character_filters: Vec<BoxCharacterFilter>,
        tokenizer: Tokenizer,
        token_filters: Vec<BoxTokenFilter>,
    ) -> Self {
        Self {
            character_filters,
            tokenizer,
            token_filters,
        }
    }

    pub fn analyze(&self, text: &str) -> LinderaResult<Vec<Token>> {
        let mut normalized_text = text.to_string();

        let mut text_len_vec: Vec<usize> = Vec::new();
        let mut offsets_vec: Vec<Vec<usize>> = Vec::new();
        let mut diffs_vec: Vec<Vec<i64>> = Vec::new();

        // Appy character filters.
        for character_filter in &self.character_filters {
            let (new_text, offsets, diffs) = character_filter.apply(normalized_text.as_str())?;

            if !offsets.is_empty() {
                // Record the offsets of each character filter.
                offsets_vec.insert(0, offsets);

                // Record the diffs of each character filter.
                diffs_vec.insert(0, diffs);

                // Record the length of the text after each character filter is applied.
                text_len_vec.insert(0, new_text.len());
            }

            normalized_text = new_text;
        }

        // Tokenize.
        let mut tmp_tokens = self.tokenizer.tokenize(&normalized_text)?;

        // Make analyzed tokens.
        let mut tokens = Vec::new();
        for token in tmp_tokens.iter_mut() {
            tokens.push(Token {
                text: token.text.to_string(),
                byte_start: token.byte_start,
                byte_end: token.byte_end,
                position: token.position,
                position_length: token.position_length,
                word_id: token.word_id,
                details: token
                    .get_details()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
                    })?
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            });
        }

        // Apply token filters.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tokens)?;
        }

        // Correct token offsets
        for token in tokens.iter_mut() {
            // Override details.
            for (i, offsets) in offsets_vec.iter().enumerate() {
                // Override start.
                token.byte_start =
                    correct_offset(token.byte_start, offsets, &diffs_vec[i], text_len_vec[i]);

                // Override end.
                token.byte_end =
                    correct_offset(token.byte_end, offsets, &diffs_vec[i], text_len_vec[i]);
            }
        }

        Ok(tokens)
    }
}

impl Clone for Analyzer {
    fn clone(&self) -> Self {
        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();
        for character_filter in self.character_filters.iter() {
            character_filters.push(character_filter.box_clone());
        }

        let mut token_filters: Vec<BoxTokenFilter> = Vec::new();
        for token_filter in self.token_filters.iter() {
            token_filters.push(token_filter.box_clone());
        }

        Analyzer {
            character_filters,
            tokenizer: self.tokenizer.clone(),
            token_filters,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(
        any(feature = "ipadic", feature = "ipadic-neologd"),
        feature = "filter"
    ))]
    use crate::analyzer::{Analyzer, AnalyzerConfig};

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
    fn test_analyzer_config_from_slice() {
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
                        "tags": [
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
        let result = AnalyzerConfig::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
    fn test_analyzer_config_clone() {
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
                        "tags": [
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
        let analyzer_config = AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let cloned_analyzer_config = analyzer_config.clone();

        assert_eq!(analyzer_config.inner, cloned_analyzer_config.inner);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
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
                    "kind": "japanese_iteration_mark",
                    "args": {
                        "normalize_kanji": true,
                        "normalize_kana": true
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
                    "kind": "japanese_compound_word",
                    "args": {
                        "kind": "ipadic",
                        "tags": [
                            "名詞,数",
                            "名詞,接尾,助数詞"
                        ]
                    }
                },
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
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
        let analyzer_config = AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let analyzer = Analyzer::from_config(&analyzer_config).unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "Lindera".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 15);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, vec!["UNK".to_string()]);
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "形態素".to_string());
                assert_eq!(token.byte_start, 18);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "形態素".to_string(),
                        "ケイタイソ".to_string(),
                        "ケイタイソ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "解析".to_string());
                assert_eq!(token.byte_start, 27);
                assert_eq!(token.byte_end, 33);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "サ変接続".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "解析".to_string(),
                        "カイセキ".to_string(),
                        "カイセキ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "エンジン".to_string());
                assert_eq!(token.byte_start, 33);
                assert_eq!(token.byte_end, 48);
                assert_eq!(token.position, 4);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "エンジン".to_string(),
                        "エンジン".to_string(),
                        "エンジン".to_string()
                    ]
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "Lindera".to_string());
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "10".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, vec!["UNK".to_string()]);
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ガロン".to_string());
                assert_eq!(token.byte_start, 6);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 1);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "接尾".to_string(),
                        "助数詞".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ガロン".to_string(),
                        "ガロン".to_string(),
                        "ガロン".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ガソリン".to_string());
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ガソリン".to_string(),
                        "ガソリン".to_string(),
                        "ガソリン".to_string()
                    ]
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "10".to_string());
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガロン".to_string());
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガソリン".to_string());
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }

        {
            let text = "お釣りは百三十四円です。".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "お釣り".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "お釣り".to_string(),
                        "オツリ".to_string(),
                        "オツリ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "百三十四円".to_string());
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 5);
                assert_eq!(
                    token.details,
                    vec![
                        "複合語".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string()
                    ]
                );
            }
        }

        {
            let text = "ここは騒々しい".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ここ".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "代名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ここ".to_string(),
                        "ココ".to_string(),
                        "ココ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "騒騒しい".to_string());
                assert_eq!(token.byte_start, 9);
                assert_eq!(token.byte_end, 21);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "形容詞".to_string(),
                        "自立".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "形容詞・イ段".to_string(),
                        "基本形".to_string(),
                        "騒騒しい".to_string(),
                        "ソウゾウシイ".to_string(),
                        "ソーゾーシイ".to_string()
                    ]
                );
            }
        }
    }
}
