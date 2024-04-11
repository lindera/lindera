use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_stop_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseStopTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl JapaneseStopTagsTokenFilterConfig {
    pub fn new(tags: HashSet<String>) -> Self {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in tags.iter() {
            let mut formatted_tag = ["*", "*", "*", "*"];

            let tag_array: Vec<&str> = tag.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            formatted_tags.insert(formatted_tag.join(","));
        }

        Self {
            tags: formatted_tags,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let args = serde_json::from_slice::<Value>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        Self::from_value(&args)
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        let tags = value["tags"]
            .as_array()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("tags is required"))
            })?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!("tag must be string"))
                    })
                    .map(|s| s.to_string())
            })
            .collect::<LinderaResult<HashSet<String>>>()?;
        Ok(Self::new(tags))
    }
}

/// Remove tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct JapaneseStopTagsTokenFilter {
    config: JapaneseStopTagsTokenFilterConfig,
}

impl JapaneseStopTagsTokenFilter {
    pub fn new(config: JapaneseStopTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseStopTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseStopTagsTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()> {
        tokens.retain(|token| {
            let mut formatted_tags = ["*", "*", "*", "*"];
            let tags_len = if token.details.len() >= 4 { 4 } else { 1 };
            for (i, j) in token.details[0..tags_len].iter().enumerate() {
                formatted_tags[i] = j;
            }
            !self.config.tags.contains(&formatted_tags.join(","))
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "ipadic", feature = "filter"))]
    use lindera_core::word_entry::WordId;

    #[cfg(all(feature = "ipadic", feature = "filter"))]
    use crate::{
        token::Token,
        token_filter::{
            japanese_stop_tags::{JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig},
            TokenFilter,
        },
    };

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter"))]
    fn test_japanese_stop_tags_token_filter_config_from_slice_ipadic() {
        let config_str = r#"
            {
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
            "#;
        let config = JapaneseStopTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 25);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter"))]
    fn test_japanese_stop_tagss_token_filter_from_slice_ipadic() {
        let config_str = r#"
            {
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
            "#;
        let result = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter"))]
    fn test_japanese_stop_tags_token_filter_apply_ipadic() {
        let config_str = r#"
            {
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
            "#;
        let filter = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: "すもも".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(36165, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "すもも".to_string(),
                    "スモモ".to_string(),
                    "スモモ".to_string(),
                ],
            },
            Token {
                text: "も".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId(73246, true),
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            Token {
                text: "もも".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId(74990, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            Token {
                text: "も".to_string(),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
                word_id: WordId(73246, true),
                details: vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ],
            },
            Token {
                text: "もも".to_string(),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                word_id: WordId(74990, true),
                details: vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ],
            },
            Token {
                text: "の".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
                word_id: WordId(55831, true),
                details: vec![
                    "助詞".to_string(),
                    "連体化".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "の".to_string(),
                    "ノ".to_string(),
                    "ノ".to_string(),
                ],
            },
            Token {
                text: "うち".to_string(),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId(8029, true),
                details: vec![
                    "名詞".to_string(),
                    "非自立".to_string(),
                    "副詞可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "うち".to_string(),
                    "ウチ".to_string(),
                    "ウチ".to_string(),
                ],
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
