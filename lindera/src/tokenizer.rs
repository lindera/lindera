use std::borrow::Cow;

use lindera_core::dictionary::{Dictionary, UserDictionary};
use lindera_core::mode::Mode;
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::character_filter::{correct_offset, BoxCharacterFilter, CharacterFilterLoader};
use crate::segmenter::{Segmenter, SegmenterConfig};
use crate::token::Token;
use crate::token_filter::{BoxTokenFilter, TokenFilterLoader};

pub type TokenizerConfig = Value;

pub struct Tokenizer {
    /// Segmenter
    pub segmenter: Segmenter,

    /// Character filters
    pub character_filters: Vec<BoxCharacterFilter>,

    /// Token filters
    pub token_filters: Vec<BoxTokenFilter>,
}

impl Tokenizer {
    pub fn from_config(config: &TokenizerConfig) -> LinderaResult<Self> {
        // Load a JSON object for segmenter config from the tokenizer config.
        let args_value = config["segmenter"].as_object().ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing segmenter config."))
        })?;
        let arg_bytes = serde_json::to_vec(args_value)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        // Load a segmenter config from the segmenter config JSON object.
        let segmenter_config = serde_json::from_slice::<SegmenterConfig>(&arg_bytes)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        // Create a segmenter from the segmenter config.
        let segmenter = Segmenter::from_config(segmenter_config)?;

        // Create a tokenizer from the segmenter.
        let mut tokenizer = Tokenizer::from_segmenter(segmenter);

        // Load character filter settings from the tokenizer config if it is not empty.
        if let Some(character_filter_settings) = config["character_filters"].as_array() {
            for character_filter_setting in character_filter_settings {
                let character_filter_name = character_filter_setting["kind"].as_str();
                if let Some(character_filter_name) = character_filter_name {
                    // Append a character filter to the tokenizer.
                    tokenizer.append_character_filter(CharacterFilterLoader::load_from_value(
                        character_filter_name,
                        &character_filter_setting["args"],
                    )?);
                }
            }
        }

        // Load token filter settings from the tokenizer config if it is not empty.
        if let Some(token_filter_settings) = config["token_filters"].as_array() {
            for token_filter_setting in token_filter_settings {
                let token_filter_name = token_filter_setting["kind"].as_str();
                if let Some(token_filter_name) = token_filter_name {
                    // Append a token filter to the tokenizer.
                    tokenizer.append_token_filter(TokenFilterLoader::load_from_value(
                        token_filter_name,
                        &token_filter_setting["args"],
                    )?);
                }
            }
        }

        Ok(tokenizer)
    }

    pub fn new(
        mode: Mode,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        Tokenizer::from_segmenter(Segmenter::new(mode, dictionary, user_dictionary))
    }

    pub fn from_segmenter(segmenter: Segmenter) -> Self {
        Self {
            segmenter,
            character_filters: Vec::new(),
            token_filters: Vec::new(),
        }
    }

    pub fn append_character_filter(&mut self, character_filter: BoxCharacterFilter) -> &mut Self {
        self.character_filters.push(character_filter);

        self
    }

    pub fn append_token_filter(&mut self, token_filter: BoxTokenFilter) -> &mut Self {
        self.token_filters.push(token_filter);

        self
    }

    pub fn tokenize<'a>(&'a self, text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut normalized_text: Cow<'a, str> = Cow::Borrowed(text);

        let mut text_len_vec: Vec<usize> = Vec::new();
        let mut offsets_vec: Vec<Vec<usize>> = Vec::new();
        let mut diffs_vec: Vec<Vec<i64>> = Vec::new();

        // Appy character filters to the text if it is not empty.
        for character_filter in &self.character_filters {
            let (offsets, diffs, text_len) =
                character_filter.apply(&mut normalized_text.to_mut())?;

            if !offsets.is_empty() {
                // Record the offsets of each character filter.
                offsets_vec.insert(0, offsets);

                // Record the diffs of each character filter.
                diffs_vec.insert(0, diffs);

                // Record the length of the text after each character filter is applied.
                text_len_vec.insert(0, text_len);
            }
        }

        // Setment a text.
        let mut tokens = self.segmenter.segment(normalized_text)?;

        // Apply token filters to the tokens if they are not empty.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tokens)?;
        }

        // Correct token offsets if character filters are applied.
        if !offsets_vec.is_empty() {
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
        }

        Ok(tokens)
    }
}

impl Clone for Tokenizer {
    fn clone(&self) -> Self {
        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();
        for character_filter in self.character_filters.iter() {
            character_filters.push(character_filter.box_clone());
        }

        let mut token_filters: Vec<BoxTokenFilter> = Vec::new();
        for token_filter in self.token_filters.iter() {
            token_filters.push(token_filter.box_clone());
        }

        Tokenizer {
            character_filters,
            segmenter: self.segmenter.clone(),
            token_filters,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use crate::tokenizer::{Tokenizer, TokenizerConfig};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenizer_config_from_slice() {
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
            "segmenter": {
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

        let result: Result<TokenizerConfig, _> = serde_json::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenizer_config_clone() {
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
            "segmenter": {
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

        let tokenizer_config: TokenizerConfig =
            serde_json::from_slice(config_str.as_bytes()).unwrap();

        let cloned_tokenizer_config = tokenizer_config.clone();

        assert_eq!(tokenizer_config, cloned_tokenizer_config);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_ipadic() {
        use std::borrow::Cow;

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
            "segmenter": {
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
        let tokenizer_config: TokenizerConfig =
            serde_json::from_slice(config_str.as_bytes()).unwrap();

        let analyzer = Tokenizer::from_config(&tokenizer_config).unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("Lindera"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 15);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, Some(vec![Cow::Borrowed("UNK")]));
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("形態素"));
                assert_eq!(token.byte_start, 18);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("形態素"),
                        Cow::Borrowed("ケイタイソ"),
                        Cow::Borrowed("ケイタイソ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("解析"));
                assert_eq!(token.byte_start, 27);
                assert_eq!(token.byte_end, 33);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("サ変接続"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("解析"),
                        Cow::Borrowed("カイセキ"),
                        Cow::Borrowed("カイセキ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("エンジン"));
                assert_eq!(token.byte_start, 33);
                assert_eq!(token.byte_end, 48);
                assert_eq!(token.position, 4);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("エンジン"),
                        Cow::Borrowed("エンジン"),
                        Cow::Borrowed("エンジン"),
                    ])
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("Lindera"));
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("10"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, Some(vec![Cow::Borrowed("UNK")]));
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ガロン"));
                assert_eq!(token.byte_start, 6);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 1);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("接尾"),
                        Cow::Borrowed("助数詞"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ガロン"),
                        Cow::Borrowed("ガロン"),
                        Cow::Borrowed("ガロン"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ガソリン"));
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ガソリン"),
                        Cow::Borrowed("ガソリン"),
                        Cow::Borrowed("ガソリン"),
                    ])
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("10"));
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("ガロン"));
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("ガソリン"));
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }

        {
            let text = "お釣りは百三十四円です。";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("お釣り"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("お釣り"),
                        Cow::Borrowed("オツリ"),
                        Cow::Borrowed("オツリ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("百三十四円"));
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 5);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("複合語"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                    ])
                );
            }
        }

        {
            let text = "ここは騒々しい";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ここ"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("代名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ここ"),
                        Cow::Borrowed("ココ"),
                        Cow::Borrowed("ココ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("騒騒しい"));
                assert_eq!(token.byte_start, 9);
                assert_eq!(token.byte_end, 21);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("形容詞"),
                        Cow::Borrowed("自立"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("形容詞・イ段"),
                        Cow::Borrowed("基本形"),
                        Cow::Borrowed("騒騒しい"),
                        Cow::Borrowed("ソウゾウシイ"),
                        Cow::Borrowed("ソーゾーシイ"),
                    ])
                );
            }
        }
    }
}
