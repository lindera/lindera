use std::collections::HashSet;

use serde_json::Value;

use crate::LinderaResult;
use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_keep_tags";

pub type JapaneseKeepTagsTokenFilterConfig = Value;

/// Keep only tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct JapaneseKeepTagsTokenFilter {
    tags: HashSet<String>,
}

impl JapaneseKeepTagsTokenFilter {
    pub fn new(tags: HashSet<String>) -> Self {
        let tags: HashSet<String> = tags
            .into_iter()
            .map(|v| {
                let mut tag_parts: Vec<&str> = v.split(',').collect();
                tag_parts.resize(4, "*");
                tag_parts.join(",")
            })
            .collect();

        Self { tags }
    }

    pub fn from_config(config: &JapaneseKeepTagsTokenFilterConfig) -> LinderaResult<Self> {
        let tags: HashSet<String> = config["tags"]
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

impl TokenFilter for JapaneseKeepTagsTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME
    }

    /// Filters tokens based on part-of-speech tags and updates the token list.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The tokens will be filtered in place by keeping only those with part-of-speech tags that match the configuration.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Filtering**:
    ///    - The function iterates over the tokens and extracts the part-of-speech tags from each token's details.
    ///    - If the token has at least 4 details, the first 4 elements are used as the tag. Otherwise, only the first element is used.
    ///
    /// 2. **Tag Matching**:
    ///    - The tags are constructed as a comma-separated string and checked against the set of tags specified in the configuration (`self.config.tags`).
    ///
    /// 3. **Token Retention**:
    ///    - Only the tokens whose tags match the configuration are retained in the resulting `filtered_tokens` vector.
    ///
    /// 4. **Replace Tokens**:
    ///    - After filtering, the original tokens vector is replaced with the filtered list.
    ///
    /// # Errors
    ///
    /// If any issue arises during token processing or filtering, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        // Create a new vector to store the filtered tokens
        let mut filtered_tokens = Vec::with_capacity(tokens.len());

        // Iterate over the tokens and filter them based on the part-of-speech tags in the config.
        for mut token in tokens.drain(..) {
            let details = token.details();

            // Determine the length of the tags to consider (either 4 or 1)
            let tags_len = details.len().min(4);

            // Construct the tag string from the token's details.
            let tag = details[0..tags_len].join(",");

            // Add the token to the filtered_tokens vector if the tag is in the set of tags.
            if self.tags.contains(&tag) {
                filtered_tokens.push(token);
            }
        }

        // Replace the original tokens vector with the filtered tokens.
        *tokens = filtered_tokens;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_japanese_keep_tags_token_filter_config() {
        use crate::token_filter::japanese_keep_tags::JapaneseKeepTagsTokenFilterConfig;

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
        let result: Result<JapaneseKeepTagsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_japanese_keep_tags_token_filter() {
        use crate::token_filter::japanese_keep_tags::{
            JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
        };

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
        let config: JapaneseKeepTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseKeepTagsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embedded-ipadic")]
    fn test_japanese_keep_tags_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_keep_tags::{
            JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
        };

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
        let config: JapaneseKeepTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseKeepTagsTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("すもも"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 36165,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("すもも"),
                    Cow::Borrowed("スモモ"),
                    Cow::Borrowed("スモモ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("も"),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 73246,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("係助詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("も"),
                    Cow::Borrowed("モ"),
                    Cow::Borrowed("モ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("もも"),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 74990,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("もも"),
                    Cow::Borrowed("モモ"),
                    Cow::Borrowed("モモ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("も"),
                byte_start: 18,
                byte_end: 21,
                position: 3,
                position_length: 1,
                word_id: WordId {
                    id: 73246,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("係助詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("も"),
                    Cow::Borrowed("モ"),
                    Cow::Borrowed("モ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("もも"),
                byte_start: 21,
                byte_end: 27,
                position: 4,
                position_length: 1,
                word_id: WordId {
                    id: 74990,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("もも"),
                    Cow::Borrowed("モモ"),
                    Cow::Borrowed("モモ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("の"),
                byte_start: 27,
                byte_end: 30,
                position: 5,
                position_length: 1,
                word_id: WordId {
                    id: 55831,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("助詞"),
                    Cow::Borrowed("連体化"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("の"),
                    Cow::Borrowed("ノ"),
                    Cow::Borrowed("ノ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("うち"),
                byte_start: 30,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId {
                    id: 8029,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("非自立"),
                    Cow::Borrowed("副詞可能"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("うち"),
                    Cow::Borrowed("ウチ"),
                    Cow::Borrowed("ウチ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].surface, "すもも");
        assert_eq!(&tokens[1].surface, "もも");
        assert_eq!(&tokens[2].surface, "もも");
        assert_eq!(&tokens[3].surface, "うち");
    }
}
