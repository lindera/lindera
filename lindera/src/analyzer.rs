use std::{fs, mem, path::Path};

use serde_json::Value;

use lindera_core::{
    character_filter::{correct_offset, BoxCharacterFilter},
    token_filter::BoxTokenFilter,
};

use crate::{
    character_filter::{
        japanese_iteration_mark::{
            JapaneseIterationMarkCharacterFilter, JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME,
        },
        mapping::{MappingCharacterFilter, MAPPING_CHARACTER_FILTER_NAME},
        regex::{RegexCharacterFilter, REGEX_CHARACTER_FILTER_NAME},
        unicode_normalize::{
            UnicodeNormalizeCharacterFilter, UNICODE_NORMALIZE_CHARACTER_FILTER_NAME,
        },
    },
    error::LinderaErrorKind,
    token_filter::{
        japanese_base_form::{JapaneseBaseFormTokenFilter, JAPANESE_BASE_FORM_TOKEN_FILTER_NAME},
        japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME,
        },
        japanese_kana::{JapaneseKanaTokenFilter, JAPANESE_KANA_TOKEN_FILTER_NAME},
        japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
        },
        japanese_keep_tags::{JapaneseKeepTagsTokenFilter, JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME},
        japanese_number::{JapaneseNumberTokenFilter, JAPANESE_NUMBER_TOKEN_FILTER_NAME},
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
        mapping::{MappingTokenFilter, MAPPING_TOKEN_FILTER_NAME},
        stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME},
        uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME},
    },
    tokenizer::Tokenizer,
    LinderaResult, Token,
};

pub struct Analyzer {
    character_filters: Vec<BoxCharacterFilter>,
    tokenizer: Tokenizer,
    token_filters: Vec<BoxTokenFilter>,
}

impl Analyzer {
    pub fn from_file(path: &Path) -> LinderaResult<Self> {
        let bytes = fs::read(path).map_err(|err| LinderaErrorKind::Io.with_error(err))?;

        Self::from_slice(&bytes)
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let args = serde_json::from_slice::<Value>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Self::from_value(&args)
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();
        let character_filter_settings = value["character_filters"].as_array();
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
                        JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME => {
                            character_filters.push(BoxCharacterFilter::from(
                                JapaneseIterationMarkCharacterFilter::from_slice(&arg_bytes)?,
                            ));
                        }
                        MAPPING_CHARACTER_FILTER_NAME => {
                            character_filters.push(BoxCharacterFilter::from(
                                MappingCharacterFilter::from_slice(&arg_bytes)?,
                            ));
                        }
                        REGEX_CHARACTER_FILTER_NAME => {
                            character_filters.push(BoxCharacterFilter::from(
                                RegexCharacterFilter::from_slice(&arg_bytes)?,
                            ));
                        }
                        UNICODE_NORMALIZE_CHARACTER_FILTER_NAME => {
                            character_filters.push(BoxCharacterFilter::from(
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
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseBaseFormTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseCompoundWordTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_KANA_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseKanaTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseKatakanaStemTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseKeepTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_NUMBER_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseNumberTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseReadingFormTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseStopTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        KEEP_WORDS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KeepWordsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KoreanKeepTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        KOREAN_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KoreanReadingFormTokenFilter::default(),
                            ));
                        }
                        KOREAN_STOP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KoreanStopTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        LENGTH_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                LengthTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        LOWERCASE_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(BoxTokenFilter::from(LowercaseTokenFilter::default()));
                        }
                        MAPPING_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                MappingTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        STOP_WORDS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                StopWordsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        UPPERCASE_TOKEN_FILTER_NAME => {
                            token_filters
                                .push(BoxTokenFilter::from(UppercaseTokenFilter::default()));
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

    pub fn analyze<'a>(&'a self, text: &'a mut String) -> crate::LinderaResult<Vec<Token<'a>>> {
        let mut text_len_vec: Vec<usize> = Vec::new();
        let mut offsets_vec: Vec<Vec<usize>> = Vec::new();
        let mut diffs_vec: Vec<Vec<i64>> = Vec::new();

        // Appy character filters.
        for character_filter in &self.character_filters {
            let (new_text, offsets, diffs) = character_filter.apply(text)?;

            if !offsets.is_empty() {
                // Record the offsets of each character filter.
                offsets_vec.insert(0, offsets);

                // Record the diffs of each character filter.
                diffs_vec.insert(0, diffs);

                // Record the length of the text after each character filter is applied.
                text_len_vec.insert(0, new_text.len());
            }

            mem::swap(text, &mut new_text.clone());
        }

        // Tokenize.
        let mut tmp_tokens = self.tokenizer.tokenize(text)?;

        // Apply token filters.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tmp_tokens)?;
        }

        // Correct token offsets
        let mut tokens = Vec::new();
        for token in tmp_tokens.iter() {
            let mut new_token = Token::new(
                token.get_text(),
                token.byte_start,
                token.byte_end,
                token.position,
                token.word_id,
                token.dictionary,
                token.user_dictionary,
            );

            // Override details.
            for (i, offsets) in offsets_vec.iter().enumerate() {
                // Override start.
                new_token.byte_start = correct_offset(
                    new_token.byte_start,
                    offsets,
                    &diffs_vec[i],
                    text_len_vec[i],
                );

                // Override end.
                new_token.byte_end =
                    correct_offset(new_token.byte_end, offsets, &diffs_vec[i], text_len_vec[i]);
            }

            tokens.push(new_token);
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
    #[cfg(feature = "ipadic")]
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
        let result = Analyzer::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_ipadic_analyzer_clone() {
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
        let analyzer = Analyzer::from_slice(config_str.as_bytes()).unwrap();

        let cloned_analyzer = analyzer.clone();

        assert_eq!(
            analyzer.character_filters.len(),
            cloned_analyzer.character_filters.len()
        );
        assert_eq!(
            analyzer.token_filters.len(),
            cloned_analyzer.token_filters.len()
        );
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
        let analyzer = Analyzer::from_slice(config_str.as_bytes()).unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.get_text()).collect::<Vec<_>>(),
                vec!["Lindera", "形態素", "解析", "エンジン"]
            );

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.get_text(), "Lindera");
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.get_text()).collect::<Vec<_>>(),
                vec!["10", "ガロン", "ガソリン"]
            );

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.get_text(), "10");
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.get_text(), "ガロン");
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.get_text(), "ガソリン");
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }

        {
            let text = "お釣りは百三十四円です。".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.get_text()).collect::<Vec<_>>(),
                vec!["お釣り", "百三十四円"]
            );
        }

        {
            let text = "ここは騒々しい".to_string();
            let mut analyze_text = text.clone();
            let tokens = analyzer.analyze(&mut analyze_text).unwrap();
            assert_eq!(
                tokens.iter().map(|t| t.get_text()).collect::<Vec<_>>(),
                vec!["ここ", "騒騒しい"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
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
                "mode": "normal",
                "with_details": false
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
