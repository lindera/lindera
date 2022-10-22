use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_stop_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseStopTagsTokenFilterConfig {
    stop_tags: HashSet<String>,
}

impl JapaneseStopTagsTokenFilterConfig {
    pub fn new(stop_tags: HashSet<String>) -> Self {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in stop_tags.iter() {
            let mut formatted_tag = vec!["*", "*", "*", "*"];

            let tag_array: Vec<&str> = tag.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            formatted_tags.insert(formatted_tag.join(","));
        }

        Self {
            stop_tags: formatted_tags,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let tmp_config = serde_json::from_slice::<JapaneseStopTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self::new(tmp_config.stop_tags))
    }
}

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
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| {
            if let Some(details) = &token.details {
                !self.config.stop_tags.contains(&details[0..4].join(","))
            } else {
                false
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::japanese_stop_tags::{
            JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig,
        },
        Token,
    };

    #[test]
    fn test_japanese_stop_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
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
        "#;
        let config = JapaneseStopTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.stop_tags.len(), 25);
    }

    #[test]
    fn test_japanese_stop_tagss_token_filter_from_slice() {
        let config_str = r#"
        {
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
        "#;
        let result = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_japanese_stop_tags_token_filter_apply() {
        let config_str = r#"
        {
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
        "#;
        let filter = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("すもも"),
                details: Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "すもも".to_string(),
                    "スモモ".to_string(),
                    "スモモ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("も"),
                details: Some(vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("もも"),
                details: Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("も"),
                details: Some(vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("もも"),
                details: Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("の"),
                details: Some(vec![
                    "助詞".to_string(),
                    "連体化".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "の".to_string(),
                    "ノ".to_string(),
                    "ノ".to_string(),
                ]),
            },
            Token {
                text: Cow::Borrowed("うち"),
                details: Some(vec![
                    "名詞".to_string(),
                    "非自立".to_string(),
                    "副詞可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "うち".to_string(),
                    "ウチ".to_string(),
                    "ウチ".to_string(),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "すもも");
        assert_eq!(tokens[1].text, "もも");
        assert_eq!(tokens[2].text, "もも");
        assert_eq!(tokens[3].text, "うち");
    }
}
