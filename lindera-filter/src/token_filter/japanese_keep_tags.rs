use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};

use crate::{token::FilteredToken, token_filter::TokenFilter};

pub const JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_keep_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseKeepTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl JapaneseKeepTagsTokenFilterConfig {
    pub fn new(tags: HashSet<String>) -> Self {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in tags.iter() {
            let mut formatted_tag = vec!["*", "*", "*", "*"];

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
        let tmp_config = serde_json::from_slice::<JapaneseKeepTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self::new(tmp_config.tags))
    }
}

/// Keep only tokens with the specified part-of-speech tag.
///
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
    fn name(&self) -> &'static str {
        JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<FilteredToken>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if self.config.tags.contains(&token.details[0..4].join(",")) {
                new_tokens.push(token.clone());
            }
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::japanese_keep_tags::{
        JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
    };
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "ipadic-neologd", feature = "ipadic-neologd-filter",),
    ))]
    use crate::{token::FilteredToken, token_filter::TokenFilter};

    #[test]
    fn test_japanese_keep_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "tags": [
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

        assert_eq!(config.tags.len(), 39);
    }

    #[test]
    fn test_japanese_keep_tagss_token_filter_from_slice() {
        let config_str = r#"
        {
            "tags": [
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
    #[cfg(any(
        all(feature = "ipadic", feature = "ipadic-filter",),
        all(feature = "ipadic-neologd", feature = "ipadic-neologd-filter",),
    ))]
    fn test_japanese_keep_tags_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "tags": [
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

        let mut tokens: Vec<FilteredToken> = vec![
            FilteredToken {
                text: "すもも".to_string(),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
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
            FilteredToken {
                text: "も".to_string(),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
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
            FilteredToken {
                text: "もも".to_string(),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
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
            FilteredToken {
                text: "も".to_string(),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
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
            FilteredToken {
                text: "もも".to_string(),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
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
            FilteredToken {
                text: "の".to_string(),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
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
            FilteredToken {
                text: "うち".to_string(),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
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
