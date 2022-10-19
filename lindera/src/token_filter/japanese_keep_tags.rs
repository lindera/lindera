use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_keep_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKeepTagsTokenFilterConfig {
    keep_tags: HashSet<String>,
}

impl JapaneseKeepTagsTokenFilterConfig {
    pub fn new(keep_tags: HashSet<String>) -> Self {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in keep_tags.iter() {
            let mut formatted_tag = vec!["*", "*", "*", "*"];

            let tag_array: Vec<&str> = tag.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            formatted_tags.insert(formatted_tag.join(","));
        }

        Self {
            keep_tags: formatted_tags,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let tmp_config = serde_json::from_slice::<JapaneseKeepTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self::new(tmp_config.keep_tags))
    }
}

#[derive(Clone, Debug)]
pub struct JapaneseKeepTagsTokenFilter {
    config: JapaneseKeepTagsTokenFilterConfig,
}

impl JapaneseKeepTagsTokenFilter {
    pub fn new(config: JapaneseKeepTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseKeepTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseKeepTagsTokenFilter {
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        tokens.retain(|token| {
            if let Some(details) = &token.details {
                self.config.keep_tags.contains(&details[0..4].join(","))
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
        token_filter::japanese_keep_tags::{
            JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
        },
        Token,
    };

    #[test]
    fn test_japanese_keep_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "keep_tags": [
                "名詞",
                "名詞,一般",
                "名詞,固有名詞",
                "名詞,固有名詞,一般",
                "名詞,固有名詞,人名",
                "名詞,固有名詞,人名,一般",
                "名詞,固有名詞,人名,姓",
                "名詞,固有名詞,人名,名",
                "名詞,固有名詞,組織",
                "名詞,固有名詞,地域",
                "名詞,固有名詞,地域,一般",
                "名詞,固有名詞,地域,国",
                "名詞,代名詞",
                "名詞,代名詞,一般",
                "名詞,代名詞,縮約",
                "名詞,副詞可能",
                "名詞,サ変接続",
                "名詞,形容動詞語幹",
                "名詞,数",
                "名詞,非自立",
                "名詞,非自立,一般",
                "名詞,非自立,副詞可能",
                "名詞,非自立,助動詞語幹",
                "名詞,非自立,形容動詞語幹",
                "名詞,特殊",
                "名詞,特殊,助動詞語幹",
                "名詞,接尾",
                "名詞,接尾,一般",
                "名詞,接尾,人名",
                "名詞,接尾,地域",
                "名詞,接尾,サ変接続",
                "名詞,接尾,助動詞語幹",
                "名詞,接尾,形容動詞語幹",
                "名詞,接尾,副詞可能",
                "名詞,接尾,助数詞",
                "名詞,接続詞的",
                "名詞,動詞非自立的",
                "名詞,引用文字列",
                "名詞,ナイ形容詞語幹"
            ]
        }
        "#;
        let config = JapaneseKeepTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.keep_tags.len(), 39);
    }

    #[test]
    fn test_japanese_keep_tagss_token_filter_from_slice() {
        let config_str = r#"
        {
            "keep_tags": [
                "名詞",
                "名詞,一般",
                "名詞,固有名詞",
                "名詞,固有名詞,一般",
                "名詞,固有名詞,人名",
                "名詞,固有名詞,人名,一般",
                "名詞,固有名詞,人名,姓",
                "名詞,固有名詞,人名,名",
                "名詞,固有名詞,組織",
                "名詞,固有名詞,地域",
                "名詞,固有名詞,地域,一般",
                "名詞,固有名詞,地域,国",
                "名詞,代名詞",
                "名詞,代名詞,一般",
                "名詞,代名詞,縮約",
                "名詞,副詞可能",
                "名詞,サ変接続",
                "名詞,形容動詞語幹",
                "名詞,数",
                "名詞,非自立",
                "名詞,非自立,一般",
                "名詞,非自立,副詞可能",
                "名詞,非自立,助動詞語幹",
                "名詞,非自立,形容動詞語幹",
                "名詞,特殊",
                "名詞,特殊,助動詞語幹",
                "名詞,接尾",
                "名詞,接尾,一般",
                "名詞,接尾,人名",
                "名詞,接尾,地域",
                "名詞,接尾,サ変接続",
                "名詞,接尾,助動詞語幹",
                "名詞,接尾,形容動詞語幹",
                "名詞,接尾,副詞可能",
                "名詞,接尾,助数詞",
                "名詞,接続詞的",
                "名詞,動詞非自立的",
                "名詞,引用文字列",
                "名詞,ナイ形容詞語幹"
            ]
        }
        "#;
        let result = JapaneseKeepTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_japanese_keep_tags_token_filter_apply() {
        let config_str = r#"
        {
            "keep_tags": [
                "名詞",
                "名詞,一般",
                "名詞,固有名詞",
                "名詞,固有名詞,一般",
                "名詞,固有名詞,人名",
                "名詞,固有名詞,人名,一般",
                "名詞,固有名詞,人名,姓",
                "名詞,固有名詞,人名,名",
                "名詞,固有名詞,組織",
                "名詞,固有名詞,地域",
                "名詞,固有名詞,地域,一般",
                "名詞,固有名詞,地域,国",
                "名詞,代名詞",
                "名詞,代名詞,一般",
                "名詞,代名詞,縮約",
                "名詞,副詞可能",
                "名詞,サ変接続",
                "名詞,形容動詞語幹",
                "名詞,数",
                "名詞,非自立",
                "名詞,非自立,一般",
                "名詞,非自立,副詞可能",
                "名詞,非自立,助動詞語幹",
                "名詞,非自立,形容動詞語幹",
                "名詞,特殊",
                "名詞,特殊,助動詞語幹",
                "名詞,接尾",
                "名詞,接尾,一般",
                "名詞,接尾,人名",
                "名詞,接尾,地域",
                "名詞,接尾,サ変接続",
                "名詞,接尾,助動詞語幹",
                "名詞,接尾,形容動詞語幹",
                "名詞,接尾,副詞可能",
                "名詞,接尾,助数詞",
                "名詞,接続詞的",
                "名詞,動詞非自立的",
                "名詞,引用文字列",
                "名詞,ナイ形容詞語幹"
            ]
        }
        "#;
        let filter = JapaneseKeepTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
