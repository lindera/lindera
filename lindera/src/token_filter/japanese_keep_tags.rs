use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, LinderaResult, Token};

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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = token.get_details() {
                if self.config.tags.contains(&details[0..4].join(",")) {
                    new_tokens.push(token.clone());
                }
            }
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::japanese_keep_tags::{
        JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
    };
    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

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
    #[cfg(feature = "ipadic")]
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

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("すもも", 0, 9, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "すもも".to_string(),
                    "スモモ".to_string(),
                    "スモモ".to_string(),
                ]))
                .clone(),
            Token::new("も", 9, 12, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ]))
                .clone(),
            Token::new("もも", 12, 18, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ]))
                .clone(),
            Token::new("も", 18, 21, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助詞".to_string(),
                    "係助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "も".to_string(),
                    "モ".to_string(),
                    "モ".to_string(),
                ]))
                .clone(),
            Token::new("もも", 21, 27, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "もも".to_string(),
                    "モモ".to_string(),
                    "モモ".to_string(),
                ]))
                .clone(),
            Token::new("の", 27, 30, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助詞".to_string(),
                    "連体化".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "の".to_string(),
                    "ノ".to_string(),
                    "ノ".to_string(),
                ]))
                .clone(),
            Token::new("うち", 30, 36, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "非自立".to_string(),
                    "副詞可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "うち".to_string(),
                    "ウチ".to_string(),
                    "ウチ".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "すもも");
        assert_eq!(tokens[1].get_text(), "もも");
        assert_eq!(tokens[2].get_text(), "もも");
        assert_eq!(tokens[3].get_text(), "うち");
    }
}
