use std::{fs, path::Path};

use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;
use lindera_filter::{
    character_filter::{
        correct_offset,
        japanese_iteration_mark::{
            JapaneseIterationMarkCharacterFilter, JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME,
        },
        mapping::{MappingCharacterFilter, MAPPING_CHARACTER_FILTER_NAME},
        regex::{RegexCharacterFilter, REGEX_CHARACTER_FILTER_NAME},
        unicode_normalize::{
            UnicodeNormalizeCharacterFilter, UNICODE_NORMALIZE_CHARACTER_FILTER_NAME,
        },
        BoxCharacterFilter,
    },
    token::FilteredToken,
    token_filter::{
        japanese_kana::{JapaneseKanaTokenFilter, JAPANESE_KANA_TOKEN_FILTER_NAME},
        japanese_katakana_stem::{
            JapaneseKatakanaStemTokenFilter, JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
        },
        keep_words::{KeepWordsTokenFilter, KEEP_WORDS_TOKEN_FILTER_NAME},
        length::{LengthTokenFilter, LENGTH_TOKEN_FILTER_NAME},
        lowercase::{LowercaseTokenFilter, LOWERCASE_TOKEN_FILTER_NAME},
        mapping::{MappingTokenFilter, MAPPING_TOKEN_FILTER_NAME},
        stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME},
        uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME},
        BoxTokenFilter,
    },
};
use lindera_tokenizer::tokenizer::Tokenizer;

#[cfg(any(
    all(feature = "ipadic", feature = "ipadic-filter",),
    all(feature = "unidic", feature = "unidic-filter",)
))]
use lindera_filter::token_filter::{
    japanese_base_form::JAPANESE_BASE_FORM_TOKEN_FILTER_NAME,
    japanese_compound_word::JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME,
    japanese_keep_tags::JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME,
    japanese_number::JAPANESE_NUMBER_TOKEN_FILTER_NAME,
    japanese_reading_form::JAPANESE_READING_FORM_TOKEN_FILTER_NAME,
    japanese_stop_tags::JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME,
};

#[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
use lindera_filter::token_filter::{
    korean_keep_tags::KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME,
    korean_reading_form::KOREAN_READING_FORM_TOKEN_FILTER_NAME,
    korean_stop_tags::KOREAN_STOP_TAGS_TOKEN_FILTER_NAME,
};

#[cfg(any(
    all(feature = "ipadic", feature = "ipadic-filter",),
    all(feature = "unidic", feature = "unidic-filter",)
))]
use lindera_filter::token_filter::{
    japanese_base_form::JapaneseBaseFormTokenFilter,
    japanese_compound_word::JapaneseCompoundWordTokenFilter,
    japanese_keep_tags::JapaneseKeepTagsTokenFilter, japanese_number::JapaneseNumberTokenFilter,
    japanese_reading_form::JapaneseReadingFormTokenFilter,
    japanese_stop_tags::JapaneseStopTagsTokenFilter,
};

#[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
use lindera_filter::token_filter::{
    korean_keep_tags::KoreanKeepTagsTokenFilter, korean_reading_form::KoreanReadingFormTokenFilter,
    korean_stop_tags::KoreanStopTagsTokenFilter,
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
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
                        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseBaseFormTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
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
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
                        JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseKeepTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
                        JAPANESE_NUMBER_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseNumberTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
                        JAPANESE_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                JapaneseReadingFormTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        #[cfg(any(
                            all(feature = "ipadic", feature = "ipadic-filter",),
                            all(feature = "unidic", feature = "unidic-filter",)
                        ))]
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
                        #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
                        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KoreanKeepTagsTokenFilter::from_slice(&args_bytes)?,
                            ));
                        }
                        #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
                        KOREAN_READING_FORM_TOKEN_FILTER_NAME => {
                            token_filters.push(BoxTokenFilter::from(
                                KoreanReadingFormTokenFilter::default(),
                            ));
                        }
                        #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
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

    pub fn analyze(&self, text: &str) -> LinderaResult<Vec<FilteredToken>> {
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

        // Make filtered tokens.
        let mut filtered_tokens = Vec::new();
        for tmp_token in tmp_tokens.iter_mut() {
            filtered_tokens.push(FilteredToken {
                text: tmp_token.text.to_string(),
                byte_start: tmp_token.byte_start,
                byte_end: tmp_token.byte_end,
                position: tmp_token.position,
                position_length: tmp_token.position_length,
                details: tmp_token
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
            token_filter.apply(&mut filtered_tokens)?;
        }

        // Correct token offsets
        for token in filtered_tokens.iter_mut() {
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

        Ok(filtered_tokens)
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
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
    use crate::analyzer::Analyzer;

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
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
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
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
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
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

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter",))]
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
