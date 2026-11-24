use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera_segmenter::LinderaResult;
use lindera_segmenter::error::LinderaErrorKind;
use lindera_segmenter::token::Token;

pub const STOP_WORDS_TOKEN_FILTER_NAME: &str = "stop_words";

pub type StopWordsTokenFilterConfig = Value;

/// Remove the tokens of the specified text.
///
#[derive(Clone, Debug)]
pub struct StopWordsTokenFilter {
    words: HashSet<String>,
}

impl StopWordsTokenFilter {
    pub fn new(words: HashSet<String>) -> Self {
        Self { words }
    }

    pub fn from_config(config: &StopWordsTokenFilterConfig) -> LinderaResult<Self> {
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

impl TokenFilter for StopWordsTokenFilter {
    fn name(&self) -> &'static str {
        STOP_WORDS_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        // An empty stop-word set never matches, so every token would be
        // kept anyway -- skip the retain pass entirely.
        if self.words.is_empty() {
            return Ok(());
        }

        tokens.retain(|token| !self.words.contains(token.surface.as_ref()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::stop_words::{StopWordsTokenFilter, StopWordsTokenFilterConfig};

    #[test]
    fn test_stop_words_token_filter_config() {
        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let result: Result<StopWordsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_words_token_filter_from() {
        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let config: StopWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = StopWordsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_stop_words_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let config_str = r#"
            {
                "words": [
                    "も",
                    "の"
                ]
            }
            "#;
        let config: StopWordsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = StopWordsTokenFilter::from_config(&config).unwrap();

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

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].surface, "すもも");
        assert_eq!(&tokens[1].surface, "もも");
        assert_eq!(&tokens[2].surface, "もも");
        assert_eq!(&tokens[3].surface, "うち");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_stop_words_token_filter_apply_empty_set_keeps_all() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let filter = StopWordsTokenFilter::new(std::collections::HashSet::new());

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

        // Empty stop-word set: the early-return path must keep everything.
        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].surface, "もも");
    }
}
