use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera_segmenter::LinderaResult;
use lindera_segmenter::error::LinderaErrorKind;
use lindera_segmenter::token::Token;

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
        // An empty keep-word set never matches, so every token would be
        // removed anyway -- skip straight to clearing instead of running
        // retain over the whole list.
        if self.words.is_empty() {
            tokens.clear();
            return Ok(());
        }

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
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

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
            Token::new(
                Cow::Borrowed("すもも"),
                0,
                9,
                0,
                WordId::new(LexType::System, 36165),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("も"),
                9,
                12,
                1,
                WordId::new(LexType::System, 73246),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("もも"),
                12,
                18,
                2,
                WordId::new(LexType::System, 74990),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("も"),
                18,
                21,
                3,
                WordId::new(LexType::System, 73246),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("もも"),
                21,
                27,
                4,
                WordId::new(LexType::System, 74990),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("の"),
                27,
                30,
                5,
                WordId::new(LexType::System, 55831),
                &dictionary,
                None,
            )
            .with_details(vec![
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
            Token::new(
                Cow::Borrowed("うち"),
                30,
                36,
                6,
                WordId::new(LexType::System, 8029),
                &dictionary,
                None,
            )
            .with_details(vec![
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
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].surface, "すもも");
        assert_eq!(&tokens[1].surface, "もも");
        assert_eq!(&tokens[2].surface, "もも");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_keep_words_token_filter_apply_empty_set_removes_all() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let filter = KeepWordsTokenFilter::new(std::collections::HashSet::new());

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new(
                Cow::Borrowed("もも"),
                0,
                6,
                0,
                WordId::new(LexType::System, 4294967295),
                &dictionary,
                None,
            )
            .with_details(vec![Cow::Borrowed("UNK")]),
        ];

        filter.apply(&mut tokens).unwrap();

        // Empty keep-word set: the early-return path must remove everything.
        assert_eq!(tokens.len(), 0);
    }
}
