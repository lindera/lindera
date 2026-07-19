use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

pub const KEEP_WORDS_TOKEN_FILTER_NAME: &str = "keep_words";

pub type KeepWordsTokenFilterConfig = Value;

/// Keep only the tokens of the specified text.
///
#[derive(Clone, Debug)]
pub struct KeepWordsTokenFilter {
    words: HashSet<String>,
}

impl KeepWordsTokenFilter {
    pub fn new(words: HashSet<String>) -> Self {
        Self { words }
    }

    pub fn from_config(config: &KeepWordsTokenFilterConfig) -> LinderaResult<Self> {
        let words: HashSet<String> = config["words"]
            .as_array()
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("words is required"))
            })?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!("words must be string"))
                    })
                    .map(|s| s.to_string())
            })
            .collect::<LinderaResult<HashSet<String>>>()?;

        Ok(Self::new(words))
    }
}

impl TokenFilter for KeepWordsTokenFilter {
    fn name(&self) -> &'static str {
        KEEP_WORDS_TOKEN_FILTER_NAME
    }

    /// Filters tokens by retaining only those that match a predefined list of words.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. Only tokens that match the words defined in the configuration will be retained.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating the success of the operation.
    ///
    /// # Process
    ///
    /// 1. **Token Filtering**:
    ///    - The function iterates over the provided `tokens` vector and filters out any token whose text does not match one of the words in the `config.words` set.
    ///    - The `retain` method is used, which iterates over each token and checks if the token's text exists in the `config.words` set.
    ///
    /// 2. **Text Matching**:
    ///    - For each token, the `text` field is converted to a reference using `as_ref()` and is checked for existence in the `config.words` set.
    ///    - If the word is found in the set, the token is kept; otherwise, it is removed from the `tokens` vector.
    ///
    /// # Example
    ///
    /// This function is useful when you have a list of specific words (such as keywords or stop words) and you want to retain only the tokens that match those words.
    ///
    /// # Errors
    ///
    /// The function will return an error in the form of `LinderaResult<()>` if any issues arise during the filtering process, though normally no errors are expected in this operation.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        tokens.retain(|token| self.words.contains(token.surface.as_ref()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::keep_words::{KeepWordsTokenFilter, KeepWordsTokenFilterConfig};

    #[test]
    fn test_keep_words_token_filter_config() {
        let config_str = r#"
            {
                "words": [
                    "すもも",
                    "もも"
                ]
            }
            "#;
        let result: Result<KeepWordsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_keep_words_token_filter() {
        let config_str = r#"
            {
                "words": [
                    "すもも",
                    "もも"
                ]
            }
            "#;
        let config: KeepWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = KeepWordsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_keep_words_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
            {
                "words": [
                    "すもも",
                    "もも"
                ]
            }
            "#;
        let config: KeepWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = KeepWordsTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("すもも"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 36165),
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
                word_id: WordId::new(LexType::System, 73246),
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
                word_id: WordId::new(LexType::System, 74990),
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
                word_id: WordId::new(LexType::System, 73246),
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
                word_id: WordId::new(LexType::System, 74990),
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
                word_id: WordId::new(LexType::System, 55831),
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
                word_id: WordId::new(LexType::System, 8029),
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

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].surface, "すもも");
        assert_eq!(&tokens[1].surface, "もも");
        assert_eq!(&tokens[2].surface, "もも");
    }
}
