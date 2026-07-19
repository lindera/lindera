use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::token_filter::tags::{TagPolicy, apply_tag_filter, normalize_japanese_tags, parse_tags};
use lindera::LinderaResult;
use lindera::token::Token;

pub const JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_stop_tags";

pub type JapaneseStopTagsTokenFilterConfig = Value;

/// Remove tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct JapaneseStopTagsTokenFilter {
    tags: HashSet<String>,
}

impl JapaneseStopTagsTokenFilter {
    pub fn new(tags: HashSet<String>) -> Self {
        Self {
            tags: normalize_japanese_tags(tags),
        }
    }

    pub fn from_config(config: &JapaneseStopTagsTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new(parse_tags(config)?))
    }
}

impl TokenFilter for JapaneseStopTagsTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME
    }

    /// Filters tokens based on part-of-speech tags and removes tokens that match the tags in the configuration.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The function filters the tokens based on their part-of-speech tags and retains only those tokens whose tags are not in the configuration.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Filtering**:
    ///    - The function iterates over the `tokens` vector and extracts the part-of-speech details of each token.
    ///    - If the token has at least 4 details, the first 4 elements are used to create a tag. If it has fewer details, only the available details are used.
    ///
    /// 2. **Tag Matching**:
    ///    - A tag string is created by joining the extracted part-of-speech details with commas (`,`) for comparison.
    ///    - If the tag is **not** found in the configuration (`self.config.tags`), the token is added to the `filtered_tokens` vector.
    ///    - If the tag is present in the configuration, the token is discarded.
    ///
    /// 3. **Token Replacement**:
    ///    - Once the iteration is complete, the original `tokens` vector is replaced with the filtered tokens, i.e., only those tokens whose part-of-speech tags are not in the configuration remain.
    ///
    /// # Example
    ///
    /// Suppose you have a set of part-of-speech tags in the configuration that you want to exclude from the token list. This function will iterate through each token, check its tag, and retain only those tokens that do not match the tags in the configuration.
    ///
    /// # Errors
    ///
    /// Returns a `LinderaResult` error if there is an issue during processing, but typically this function is expected to complete successfully unless there are issues with the token or tag data.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        apply_tag_filter(tokens, &self.tags, TagPolicy::Remove, |token| {
            let details = token.details();
            // If the length of the details is greater than or equal to 4,
            // the tag length is 4, otherwise 1 is assigned to tags_len.
            let tags_len = if details.len() >= 4 { 4 } else { 1 };
            // Make a string of the part-of-speech tags.
            details[0..tags_len].join(",")
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    use crate::token_filter::japanese_stop_tags::{
        JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig,
    };

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_stop_tags_token_filter_config() {
        let config_str = r#"
            {
                "tags": [
                    "接続詞",
                    "助詞",
                    "助詞,格助詞",
                    "助詞,格助詞,一般",
                    "助詞,格助詞,引用",
                    "助詞,格助詞,連語",
                    "助詞,係助詞",
                    "助詞,副助詞",
                    "助詞,間投助詞",
                    "助詞,並立助詞",
                    "助詞,終助詞",
                    "助詞,副助詞／並立助詞／終助詞",
                    "助詞,連体化",
                    "助詞,副詞化",
                    "助詞,特殊",
                    "助動詞",
                    "記号",
                    "記号,一般",
                    "記号,読点",
                    "記号,句点",
                    "記号,空白",
                    "記号,括弧閉",
                    "その他,間投",
                    "フィラー",
                    "非言語音"
                ]
            }
            "#;
        let result: Result<JapaneseStopTagsTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_stop_tagss_token_filter() {
        let config_str = r#"
            {
                "tags": [
                    "接続詞",
                    "助詞",
                    "助詞,格助詞",
                    "助詞,格助詞,一般",
                    "助詞,格助詞,引用",
                    "助詞,格助詞,連語",
                    "助詞,係助詞",
                    "助詞,副助詞",
                    "助詞,間投助詞",
                    "助詞,並立助詞",
                    "助詞,終助詞",
                    "助詞,副助詞／並立助詞／終助詞",
                    "助詞,連体化",
                    "助詞,副詞化",
                    "助詞,特殊",
                    "助動詞",
                    "記号",
                    "記号,一般",
                    "記号,読点",
                    "記号,句点",
                    "記号,空白",
                    "記号,括弧閉",
                    "その他,間投",
                    "フィラー",
                    "非言語音"
                ]
            }
            "#;
        let config: JapaneseStopTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let result = JapaneseStopTagsTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_stop_tags_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
            {
                "tags": [
                    "接続詞",
                    "助詞",
                    "助詞,格助詞",
                    "助詞,格助詞,一般",
                    "助詞,格助詞,引用",
                    "助詞,格助詞,連語",
                    "助詞,係助詞",
                    "助詞,副助詞",
                    "助詞,間投助詞",
                    "助詞,並立助詞",
                    "助詞,終助詞",
                    "助詞,副助詞／並立助詞／終助詞",
                    "助詞,連体化",
                    "助詞,副詞化",
                    "助詞,特殊",
                    "助動詞",
                    "記号",
                    "記号,一般",
                    "記号,読点",
                    "記号,句点",
                    "記号,空白",
                    "記号,括弧閉",
                    "その他,間投",
                    "フィラー",
                    "非言語音"
                ]
            }
            "#;
        let config: JapaneseStopTagsTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseStopTagsTokenFilter::from_config(&config).unwrap();

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

        assert_eq!(tokens.len(), 4);
        assert_eq!(&tokens[0].surface, "すもも");
        assert_eq!(&tokens[1].surface, "もも");
        assert_eq!(&tokens[2].surface, "もも");
        assert_eq!(&tokens[3].surface, "うち");
    }
}
