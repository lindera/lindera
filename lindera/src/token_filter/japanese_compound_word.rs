use std::{borrow::Cow, collections::HashSet, mem};

use lindera_core::token_filter::TokenFilter;
use serde::{Deserialize, Serialize};

use crate::{error::LinderaErrorKind, DictionaryKind, LinderaResult, Token};

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
}

impl TokenFilter for JapaneseCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        let mut formatted_details = match self.config.kind {
            #[cfg(feature = "ipadic")]
            DictionaryKind::IPADIC => "複合語,*,*,*,*,*,*,*,*".split(',').collect::<Vec<&str>>(),
            #[cfg(feature = "unidic")]
            DictionaryKind::UniDic => "複合語,*,*,*,*,*,*,*,*,*,*,*,*,*,*,*,*"
                .split(',')
                .collect::<Vec<&str>>(),
            _ => "UNK".split(',').collect::<Vec<&str>>(),
        };

        if let Some(new_tag_str) = &self.config.new_tag {
            let new_tag_array: Vec<&str> = new_tag_str.split(',').collect();
            for (i, j) in new_tag_array.iter().enumerate() {
                formatted_details[i] = j;
            }
        }

        let mut compound_token_opt = None;
        for token in tokens.iter_mut() {
            if let Some(details) = &mut token.details {
                let mut formatted_tags = vec!["*", "*", "*", "*"];
                let tags_len = if details.len() >= 4 { 4 } else { 1 };
                for (i, j) in details[0..tags_len].iter().enumerate() {
                    formatted_tags[i] = j;
                }

                if self.config.tags.contains(&formatted_tags.join(",")) {
                    if compound_token_opt.is_none() {
                        compound_token_opt = Some(Token {
                            text: Cow::Owned(token.text.to_string()),
                            details: token.details.clone(),
                            byte_start: token.byte_start,
                            byte_end: token.byte_end,
                        });
                    } else {
                        let compound_token = compound_token_opt.take().unwrap();
                        compound_token_opt = Some(Token {
                            text: Cow::Owned(format!("{}{}", compound_token.text, token.text)),
                            details: Some(
                                formatted_details.iter().map(|s| s.to_string()).collect(),
                            ),
                            byte_start: compound_token.byte_start,
                            byte_end: token.byte_end,
                        });
                    }
                } else {
                    if compound_token_opt.is_some() {
                        let compound_token = compound_token_opt.take().unwrap();
                        new_tokens.push(compound_token);
                    }
                    new_tokens.push(Token {
                        text: Cow::Owned(token.text.to_string()),
                        details: token.details.clone(),
                        byte_start: token.byte_start,
                        byte_end: token.byte_end,
                    });
                }
            }
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
        },
        Token,
    };

    #[test]
    fn test_japanese_compound_word_token_filter_config_from_slice() {
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

    #[test]
    fn test_japanese_compound_word_token_filter_from_slice() {
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
    fn test_japanese_compound_word_token_filter_apply() {
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

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("１"),
                details: Some(vec![
                    "名詞".to_string(),
                    "数".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "１".to_string(),
                    "イチ".to_string(),
                    "イチ".to_string(),
                ]),
                byte_start: 0,
                byte_end: 3,
            },
            Token {
                text: Cow::Borrowed("０"),
                details: Some(vec![
                    "名詞".to_string(),
                    "数".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "０".to_string(),
                    "ゼロ".to_string(),
                    "ゼロ".to_string(),
                ]),
                byte_start: 3,
                byte_end: 6,
            },
            Token {
                text: Cow::Borrowed("０"),
                details: Some(vec![
                    "名詞".to_string(),
                    "数".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "０".to_string(),
                    "ゼロ".to_string(),
                    "ゼロ".to_string(),
                ]),
                byte_start: 6,
                byte_end: 9,
            },
            Token {
                text: Cow::Borrowed("円"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "助数詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "円".to_string(),
                    "エン".to_string(),
                    "エン".to_string(),
                ]),
                byte_start: 9,
                byte_end: 12,
            },
            Token {
                text: Cow::Borrowed("玉"),
                details: Some(vec![
                    "名詞".to_string(),
                    "接尾".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "玉".to_string(),
                    "ダマ".to_string(),
                    "ダマ".to_string(),
                ]),
                byte_start: 12,
                byte_end: 15,
            },
            Token {
                text: Cow::Borrowed("を"),
                details: Some(vec![
                    "助詞".to_string(),
                    "格助詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "を".to_string(),
                    "ヲ".to_string(),
                    "ヲ".to_string(),
                ]),
                byte_start: 27,
                byte_end: 30,
            },
            Token {
                text: Cow::Borrowed("拾う"),
                details: Some(vec![
                    "動詞".to_string(),
                    "自立".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "五段・ワ行促音便".to_string(),
                    "基本形".to_string(),
                    "拾う".to_string(),
                    "ヒロウ".to_string(),
                    "ヒロウ".to_string(),
                ]),
                byte_start: 30,
                byte_end: 36,
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "１００円");
        assert_eq!(tokens[1].text, "玉");
        assert_eq!(tokens[2].text, "を");
        assert_eq!(tokens[3].text, "拾う");

        assert_eq!(
            tokens[0].details,
            Some(vec![
                "複合語".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "*".to_string(),
            ])
        );
    }
}
