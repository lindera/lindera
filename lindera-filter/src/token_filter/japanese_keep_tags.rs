use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

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

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.get_details() {
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
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_core::word_entry::WordId;
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use lindera_tokenizer::token::Token;

    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    use crate::token_filter::{
        japanese_keep_tags::{JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig},
        TokenFilter,
    };

    #[test]
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_keep_tags_token_filter_config_from_slice_ipadic() {
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
    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    fn test_japanese_keep_tags_token_filter_from_slice_ipadic() {
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
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    fn test_japanese_keep_tags_token_filter_apply_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

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

        let mut tokens: Vec<Token> = vec![
            Token::new("すもも", 0, 9, 0, WordId(36165, true), &dictionary, None),
            Token::new("も", 9, 12, 1, WordId(73246, true), &dictionary, None),
            Token::new("もも", 12, 18, 2, WordId(74990, true), &dictionary, None),
            Token::new("も", 18, 21, 3, WordId(73246, true), &dictionary, None),
            Token::new("もも", 21, 27, 4, WordId(74990, true), &dictionary, None),
            Token::new("の", 27, 30, 5, WordId(55831, true), &dictionary, None),
            Token::new("うち", 30, 36, 6, WordId(8029, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "すもも");
        assert_eq!(&tokens[1].text, "もも");
        assert_eq!(&tokens[2].text, "もも");
        assert_eq!(&tokens[3].text, "うち");
    }
}
