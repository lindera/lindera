use std::{collections::HashSet, mem};

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
}

impl TokenFilter for JapaneseCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        let mut new_tokens = Vec::new();

        let mut formatted_details = match self.config.kind {
            #[cfg(feature = "ipadic")]
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
                        // let mut compound_token = Token::new(
                        //     token.get_text(),
                        //     token.byte_start,
                        //     token.byte_end,
                        //     token.word_id,
                        //     token.dictionary,
                        //     token.user_dictionary,
                        // );
                        // compound_token.set_details(token.get_details());
                        // compound_token_opt = Some(compound_token);
                        compound_token_opt = Some(token.clone());
                    } else {
                        let compound_token = compound_token_opt.take().ok_or_else(|| {
                            LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
                        })?;
                        let mut new_compound_token = Token::new(
                            &format!("{}{}", compound_token.get_text(), token.get_text()),
                            compound_token.byte_start,
                            compound_token.byte_end,
                            compound_token.word_id,
                            compound_token.dictionary,
                            compound_token.user_dictionary,
                        );
                        new_compound_token.set_details(Some(formatted_details.clone()));
                        compound_token_opt = Some(new_compound_token);
                    }
                } else {
                    if compound_token_opt.is_some() {
                        let compound_token = compound_token_opt.take().unwrap();
                        new_tokens.push(compound_token);
                        compound_token_opt = None;
                    }

                    // let mut new_token = Token::new(
                    //     token.get_text(),
                    //     token.byte_start,
                    //     token.byte_end,
                    //     token.word_id,
                    //     token.dictionary,
                    //     token.user_dictionary,
                    // );
                    // let details = match token.get_details() {
                    //     Some(details) => Some(details.iter().map(|v| v.to_string()).collect::<Vec<String>>()),
                    //     None => None,
                    // };
                    // new_token.set_details(details);

                    // new_tokens.push(new_token);

                    new_tokens.push(token.clone());
                }
            }
        }

        if compound_token_opt.is_some() {
            let compound_token = compound_token_opt.take().unwrap();
            new_tokens.push(compound_token);
        }

        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    use crate::token_filter::japanese_compound_word::{
        JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
    };

    #[cfg(feature = "ipadic")]
    use crate::{builder, DictionaryKind, Token};

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
    #[cfg(feature = "ipadic")]
    fn test_japanese_compound_word_token_filter_apply_ipadic() {
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

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![
                Token::new("１", 0, 3, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "１".to_string(),
                        "イチ".to_string(),
                        "イチ".to_string(),
                    ]))
                    .clone(),
                Token::new("０", 3, 6, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "０".to_string(),
                        "ゼロ".to_string(),
                        "ゼロ".to_string(),
                    ]))
                    .clone(),
                Token::new("０", 6, 9, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "０".to_string(),
                        "ゼロ".to_string(),
                        "ゼロ".to_string(),
                    ]))
                    .clone(),
                Token::new("円", 9, 12, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "接尾".to_string(),
                        "助数詞".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "円".to_string(),
                        "エン".to_string(),
                        "エン".to_string(),
                    ]))
                    .clone(),
                Token::new("玉", 12, 15, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "接尾".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "玉".to_string(),
                        "ダマ".to_string(),
                        "ダマ".to_string(),
                    ]))
                    .clone(),
                Token::new("を", 15, 18, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "助詞".to_string(),
                        "格助詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "を".to_string(),
                        "ヲ".to_string(),
                        "ヲ".to_string(),
                    ]))
                    .clone(),
                Token::new("拾う", 18, 24, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "動詞".to_string(),
                        "自立".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "五段・ワ行促音便".to_string(),
                        "基本形".to_string(),
                        "拾う".to_string(),
                        "ヒロウ".to_string(),
                        "ヒロウ".to_string(),
                    ]))
                    .clone(),
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 4);
            assert_eq!(tokens[0].get_text(), "１００円");
            assert_eq!(tokens[1].get_text(), "玉");
            assert_eq!(tokens[2].get_text(), "を");
            assert_eq!(tokens[3].get_text(), "拾う");

            assert_eq!(
                tokens[0].get_details(),
                Some(vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*",])
            );
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token::new("渋谷", 0, 6, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "固有名詞".to_string(),
                        "地域".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "渋谷".to_string(),
                        "シブヤ".to_string(),
                        "シブヤ".to_string(),
                    ]))
                    .clone(),
                Token::new("１", 6, 9, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "１".to_string(),
                        "イチ".to_string(),
                        "イチ".to_string(),
                    ]))
                    .clone(),
                Token::new("０", 9, 12, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "０".to_string(),
                        "ゼロ".to_string(),
                        "ゼロ".to_string(),
                    ]))
                    .clone(),
                Token::new("９", 9, 12, WordId::default(), &dictionary, None)
                    .set_details(Some(vec![
                        "名詞".to_string(),
                        "数".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "９".to_string(),
                        "キュウ".to_string(),
                        "キュー".to_string(),
                    ]))
                    .clone(),
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].get_text(), "渋谷");
            assert_eq!(tokens[1].get_text(), "１０９");

            assert_eq!(
                tokens[1].get_details(),
                Some(vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*",])
            );
        }
    }
}
