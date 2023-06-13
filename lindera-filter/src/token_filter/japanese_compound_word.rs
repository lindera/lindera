use std::{borrow::Cow, collections::HashSet, mem};

use serde::{Deserialize, Serialize};

use lindera_core::{error::LinderaErrorKind, LinderaResult};
use lindera_dictionary::DictionaryKind;
use lindera_tokenizer::token::Token;

use crate::token_filter::TokenFilter;

pub const JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME: &str = "japanese_compound_word";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseCompoundWordTokenFilterConfig {
    kind: DictionaryKind,
    tags: HashSet<String>,
    new_tag: Option<String>,
}

impl JapaneseCompoundWordTokenFilterConfig {
    pub fn new(kind: DictionaryKind, tags: HashSet<String>, new_tag: Option<String>) -> Self {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in tags.iter() {
            let mut formatted_tag = vec!["*", "*", "*", "*"];

            let tag_array: Vec<&str> = tag.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            formatted_tags.insert(formatted_tag.join(","));
        }

        let formatted_new_tag = if let Some(new_tag_str) = new_tag {
            let mut formatted_tag = vec!["*", "*", "*", "*"];

            let tag_array: Vec<&str> = new_tag_str.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            Some(formatted_tag.join(","))
        } else {
            None
        };

        Self {
            kind,
            tags: formatted_tags,
            new_tag: formatted_new_tag,
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let tmp_config = serde_json::from_slice::<JapaneseCompoundWordTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Ok(Self::new(
            tmp_config.kind,
            tmp_config.tags,
            tmp_config.new_tag,
        ))
    }
}

/// Compond consecutive tokens that have specified part-of-speech tags into a single token.
///
#[derive(Clone, Debug)]
pub struct JapaneseCompoundWordTokenFilter {
    config: JapaneseCompoundWordTokenFilterConfig,
}

impl JapaneseCompoundWordTokenFilter {
    pub fn new(config: JapaneseCompoundWordTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(
            JapaneseCompoundWordTokenFilterConfig::from_slice(data)?,
        ))
    }

    fn concat_token<'a>(&self, token1: &mut Token<'a>, token2: &Token<'a>) {
        token1.text = Cow::Owned(format!("{}{}", token1.text, token2.text));
        token1.byte_end = token2.byte_end;
        token1.position_length += token2.position_length;

        let mut formatted_details = match self.config.kind {
            #[cfg(any(feature = "ipadic", feature = "ipadic-neologd"))]
            DictionaryKind::IPADIC => "複合語,*,*,*,*,*,*,*,*"
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            #[cfg(feature = "unidic")]
            DictionaryKind::UniDic => "複合語,*,*,*,*,*,*,*,*,*,*,*,*,*,*,*,*"
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            _ => "複合語,*,*,*,*,*,*,*,*"
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        };
        if let Some(new_tag_str) = &self.config.new_tag {
            let new_tag_array: Vec<&str> = new_tag_str.split(',').collect();
            for (i, j) in new_tag_array.iter().enumerate() {
                formatted_details[i] = j.to_string();
            }
        }

        token1.set_details(Some(formatted_details));
    }
}

impl TokenFilter for JapaneseCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();
        let mut compound_token_opt = None;
        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.get_details() {
                let mut formatted_tags = vec!["*", "*", "*", "*"];
                let tags_len = if details.len() >= 4 { 4 } else { 1 };
                for (i, j) in details[0..tags_len].iter().enumerate() {
                    formatted_tags[i] = j;
                }

                let pos = formatted_tags.join(",");

                if self.config.tags.contains(&pos) {
                    if compound_token_opt.is_none() {
                        // Create new compound token start.
                        compound_token_opt = Some(token.clone());
                    } else {
                        let mut compound_token = compound_token_opt.take().ok_or_else(|| {
                            LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
                        })?;
                        self.concat_token(&mut compound_token, token);
                        compound_token_opt = Some(compound_token);
                    }
                } else {
                    if compound_token_opt.is_some() {
                        let compound_token = compound_token_opt.take().ok_or_else(|| {
                            LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
                        })?;

                        new_tokens.push(compound_token);

                        // Clear compound token
                        compound_token_opt = None;
                    }
                    let new_token = token.clone();
                    new_tokens.push(new_token);
                }
            }
        }

        if compound_token_opt.is_some() {
            let compound_token = compound_token_opt.take().ok_or_else(|| {
                LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
            })?;
            new_tokens.push(compound_token);
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
        japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
        },
        TokenFilter,
    };

    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    #[test]
    fn test_japanese_compound_word_token_filter_config_from_slice_ipadic() {
        let config_str = r#"
            {
                "kind": "ipadic",
                "tags": [
                    "名詞,数",
                    "名詞,接尾,助数詞"
                ],
                "new_tag": "複合語"
            }
            "#;
        let config =
            JapaneseCompoundWordTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.tags.len(), 2);
    }

    #[cfg(all(feature = "ipadic", feature = "ipadic-filter"))]
    #[test]
    fn test_japanese_compound_word_token_filter_from_slice_ipadic() {
        let config_str = r#"
            {
                "kind": "ipadic",
                "tags": [
                    "名詞,数",
                    "名詞,接尾,助数詞"
                ],
                "new_tag": "複合語"
            }
            "#;
        let result = JapaneseCompoundWordTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(any(all(feature = "ipadic", feature = "ipadic-filter",),))]
    fn test_japanese_compound_word_token_filter_apply_ipadic() {
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };
        let dictionary = load_dictionary_from_config(dictionary_config).unwrap();

        let config_str = r#"
            {
                "kind": "ipadic",
                "tags": [
                    "名詞,数",
                    "名詞,接尾,助数詞"
                ],
                "new_tag": "複合語"
            }
            "#;
        let filter = JapaneseCompoundWordTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        {
            let mut tokens: Vec<Token> = vec![
                Token::new("１", 0, 3, 0, WordId(391174, true), &dictionary, None),
                Token::new("０", 3, 6, 1, WordId(391171, true), &dictionary, None),
                Token::new("０", 6, 9, 2, WordId(391171, true), &dictionary, None),
                Token::new("円", 9, 12, 3, WordId(137904, true), &dictionary, None),
                Token::new("玉", 12, 15, 4, WordId(287427, true), &dictionary, None),
                Token::new("を", 15, 18, 5, WordId(80582, true), &dictionary, None),
                Token::new("拾う", 18, 24, 6, WordId(228047, true), &dictionary, None),
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 4);
            assert_eq!(tokens[0].text, "１００円".to_string());
            assert_eq!(tokens[0].byte_start, 0);
            assert_eq!(tokens[0].byte_end, 12);
            assert_eq!(tokens[0].position, 0);
            assert_eq!(tokens[0].position_length, 4);
            assert_eq!(tokens[1].text, "玉".to_string());
            assert_eq!(tokens[1].byte_start, 12);
            assert_eq!(tokens[1].byte_end, 15);
            assert_eq!(tokens[1].position, 4);
            assert_eq!(tokens[1].position_length, 1);
            assert_eq!(tokens[2].text, "を".to_string());
            assert_eq!(tokens[2].byte_start, 15);
            assert_eq!(tokens[2].byte_end, 18);
            assert_eq!(tokens[2].position, 5);
            assert_eq!(tokens[2].position_length, 1);
            assert_eq!(tokens[3].text, "拾う".to_string());
            assert_eq!(tokens[3].byte_start, 18);
            assert_eq!(tokens[3].byte_end, 24);
            assert_eq!(tokens[3].position, 6);
            assert_eq!(tokens[3].position_length, 1);

            assert_eq!(
                tokens[0].get_details().unwrap(),
                vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*",]
            );
        }
    }
}
