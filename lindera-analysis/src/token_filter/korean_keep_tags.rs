use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::token_filter::tags::{TagPolicy, apply_tag_filter, parse_tags};
use lindera::LinderaResult;
use lindera::token::Token;

pub const KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME: &str = "korean_keep_tags";

pub type KoreanKeepTagsTokenFilterConfig = Value;

/// Keep only tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct KoreanKeepTagsTokenFilter {
    tags: HashSet<String>,
}

impl KoreanKeepTagsTokenFilter {
    pub fn new(tags: HashSet<String>) -> Self {
        Self { tags }
    }

    pub fn from_config(config: &KoreanKeepTagsTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new(parse_tags(config)?))
    }
}

impl TokenFilter for KoreanKeepTagsTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME
    }

    /// Filters tokens based on part-of-speech tags, retaining only tokens that match the configured tags.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The tokens are filtered based on their part-of-speech tags.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating the success of the operation.
    ///
    /// # Process
    ///
    /// 1. **Token Filtering**:
    ///    - The function iterates over each token and retrieves the first part-of-speech tag using `get_detail(0)`.
    ///    - If the first tag is `None`, a default empty string is used instead.
    ///    - The function then checks if the tag is present in the configured set of tags (`self.config.tags`).
    ///
    /// 2. **Token Retention**:
    ///    - Tokens whose part-of-speech tags are in the configuration are retained, while others are removed from the `tokens` vector.
    ///
    /// # Example
    ///
    /// This function is useful when filtering tokens based on specific part-of-speech tags (e.g., filtering out all verbs or nouns).
    ///
    /// # Errors
    ///
    /// The function returns a `LinderaResult<()>` if any issue occurs during token filtering, but typically no errors are expected during this operation.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        apply_tag_filter(tokens, &self.tags, TagPolicy::Keep, |token| {
            // Use the first part-of-speech tag as the comparison key.
            token.get_detail(0).unwrap_or_default().to_string()
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_keep_tags::{
        KoreanKeepTagsTokenFilter, KoreanKeepTagsTokenFilterConfig,
    };

    #[test]
    fn test_korean_keep_tags_token_filter_config() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let result: Result<KoreanKeepTagsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_korean_keep_tags_token_filter() {
        let config_str = r#"
        {
            "tags": [
                "NNG"
            ]
        }
        "#;
        let config: KoreanKeepTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = KoreanKeepTagsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_keep_tags_token_filter_apply() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
            {
                "tags": [
                    "NNG"
                ]
            }
            "#;
        let config: KoreanKeepTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = KoreanKeepTagsTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("한국어"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 770060),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("한국어"),
                    Cow::Borrowed("Compound"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("한국/NNG/*+어/NNG/*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("의"),
                byte_start: 9,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId::new(LexType::System, 576336),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("JKG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("의"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("형태소"),
                byte_start: 12,
                byte_end: 21,
                position: 2,
                position_length: 1,
                word_id: WordId::new(LexType::System, 787807),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("형태소"),
                    Cow::Borrowed("Compound"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("형태/NNG/*+소/NNG/*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("분석"),
                byte_start: 21,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId::new(LexType::System, 383955),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("행위"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("분석"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("을"),
                byte_start: 27,
                byte_end: 30,
                position: 4,
                position_length: 1,
                word_id: WordId::new(LexType::System, 574939),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("JKO"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("을"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("할"),
                byte_start: 30,
                byte_end: 33,
                position: 5,
                position_length: 1,
                word_id: WordId::new(LexType::System, 774117),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("VV+ETM"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("할"),
                    Cow::Borrowed("Inflect"),
                    Cow::Borrowed("VV"),
                    Cow::Borrowed("ETM"),
                    Cow::Borrowed("하/VV/*+ᆯ/ETM/*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("수"),
                byte_start: 33,
                byte_end: 36,
                position: 6,
                position_length: 1,
                word_id: WordId::new(LexType::System, 444151),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("NNG"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("수"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("있"),
                byte_start: 36,
                byte_end: 39,
                position: 7,
                position_length: 1,
                word_id: WordId::new(LexType::System, 602850),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("VX"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("T"),
                    Cow::Borrowed("있"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("습니다"),
                byte_start: 39,
                byte_end: 48,
                position: 8,
                position_length: 1,
                word_id: WordId::new(LexType::System, 458024),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("EF"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed("습니다"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].surface, "한국어");
        assert_eq!(&tokens[1].surface, "형태소");
        assert_eq!(&tokens[2].surface, "분석");
        assert_eq!(&tokens[3].surface, "수");
    }
}
