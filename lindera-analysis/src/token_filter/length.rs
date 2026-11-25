use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera_segmenter::LinderaResult;
use lindera_segmenter::token::Token;

pub const LENGTH_TOKEN_FILTER_NAME: &str = "length";

pub type LengthTokenFilterConfig = Value;

/// Keep only tokens with the specified number of characters of text.
///
#[derive(Clone, Debug)]
pub struct LengthTokenFilter {
    min: Option<usize>,
    max: Option<usize>,
}

impl LengthTokenFilter {
    pub fn new(min: Option<usize>, max: Option<usize>) -> Self {
        Self { min, max }
    }

    pub fn from_config(config: &LengthTokenFilterConfig) -> LinderaResult<Self> {
        let min: Option<usize> = config
            .get("min")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let max: Option<usize> = config
            .get("max")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        Ok(Self::new(min, max))
    }
}

impl TokenFilter for LengthTokenFilter {
    fn name(&self) -> &'static str {
        LENGTH_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        tokens.retain(|token| {
            let len = token.surface.chars().count();
            if let Some(min) = self.min
                && len < min
            {
                return false;
            }
            if let Some(max) = self.max
                && len > max
            {
                return false;
            }
            true
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::length::{LengthTokenFilter, LengthTokenFilterConfig};

    #[test]
    fn test_length_token_filter_confige() {
        let config_str = r#"
            {
                "min": 1,
                "max": 3
            }
            "#;
        let result: Result<LengthTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());

        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let result: Result<LengthTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());

        let config_str = r#"
            {
                "max": 2
            }
            "#;
        let result: Result<LengthTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_length_token_filter() {
        let config_str = r#"
            {
                "min": 1,
                "max": 3
            }
            "#;
        let config: LengthTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = LengthTokenFilter::from_config(&config);
        assert!(result.is_ok());

        let config_str = r#"
            {
                "min": 1
            }
            "#;
        let config: LengthTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = LengthTokenFilter::from_config(&config);
        assert!(result.is_ok());

        let config_str = r#"
            {
                "max": 2
            }
            "#;
        let config: LengthTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = LengthTokenFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_length_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let config_str = r#"
            {
                "min": 2,
                "max": 3
            }
            "#;
        let config: LengthTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = LengthTokenFilter::from_config(&config).unwrap();

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
}
