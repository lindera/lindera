use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

pub const KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "korean_keep_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KoreanKeepTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl KoreanKeepTagsTokenFilterConfig {
    pub fn new(tags: HashSet<String>) -> Self {
        Self { tags }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<KoreanKeepTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Keep only tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct KoreanKeepTagsTokenFilter {
    config: KoreanKeepTagsTokenFilterConfig,
}

impl KoreanKeepTagsTokenFilter {
    pub fn new(config: KoreanKeepTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(KoreanKeepTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for KoreanKeepTagsTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.get_details() {
                if self.config.tags.contains(details[0]) {
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
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use lindera_core::word_entry::WordId;
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use lindera_tokenizer::token::Token;

    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    use crate::token_filter::{
        korean_keep_tags::{KoreanKeepTagsTokenFilter, KoreanKeepTagsTokenFilterConfig},
        TokenFilter,
    };

    #[test]
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    fn test_korean_keep_tags_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let config = KoreanKeepTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 1);
    }

    #[test]
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    fn test_korean_keep_tags_token_filter_from_slice() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let result = KoreanKeepTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(all(feature = "ko-dic", feature = "ko-dic-filter",))]
    fn test_korean_keep_tags_token_filter_apply() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "tags": [
                    "NNG"
                ]
            }
            "#;
        let filter = KoreanKeepTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("한국어", 0, 9, 0, WordId(770060, true), &dictionary, None),
            Token::new("의", 9, 12, 1, WordId(576336, true), &dictionary, None),
            Token::new("형태소", 12, 21, 2, WordId(787807, true), &dictionary, None),
            Token::new("분석", 21, 27, 3, WordId(383955, true), &dictionary, None),
            Token::new("을", 27, 30, 4, WordId(574939, true), &dictionary, None),
            Token::new("할", 30, 33, 5, WordId(774117, true), &dictionary, None),
            Token::new("수", 33, 36, 6, WordId(444151, true), &dictionary, None),
            Token::new("있", 36, 39, 6, WordId(602850, true), &dictionary, None),
            Token::new("습니다", 39, 48, 6, WordId(458024, true), &dictionary, None),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].text, "한국어");
        assert_eq!(&tokens[1].text, "형태소");
        assert_eq!(&tokens[2].text, "분석");
        assert_eq!(&tokens[3].text, "수");
    }
}
