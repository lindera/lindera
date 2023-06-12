use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

pub const JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME: &str = "japanese_stop_tags";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseStopTagsTokenFilterConfig {
    tags: HashSet<String>,
}

impl JapaneseStopTagsTokenFilterConfig {
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
        let tmp_config = serde_json::from_slice::<JapaneseStopTagsTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self::new(tmp_config.tags))
    }
}

/// Remove tokens with the specified part-of-speech tag.
///
#[derive(Clone, Debug)]
pub struct JapaneseStopTagsTokenFilter {
    config: JapaneseStopTagsTokenFilterConfig,
}

impl JapaneseStopTagsTokenFilter {
    pub fn new(config: JapaneseStopTagsTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseStopTagsTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseStopTagsTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.get_details() {
                let mut formatted_tags = vec!["*", "*", "*", "*"];
                let tags_len = if details.len() >= 4 { 4 } else { 1 };
                for (i, j) in details[0..tags_len].iter().enumerate() {
                    formatted_tags[i] = j;
                }
                if !self.config.tags.contains(&formatted_tags.join(",")) {
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
        japanese_stop_tags::{JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig},
        TokenFilter,
    };

    #[test]
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    fn test_japanese_stop_tags_token_filter_config_from_slice_ipadic() {
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
        let config = JapaneseStopTagsTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 25);
    }

    #[test]
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    fn test_japanese_stop_tagss_token_filter_from_slice_ipadic() {
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
        let result = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    fn test_japanese_stop_tags_token_filter_apply_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

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
        let filter = JapaneseStopTagsTokenFilter::from_slice(config_str.as_bytes()).unwrap();

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
