use std::borrow::Cow;
use std::{collections::HashSet, mem};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::dictionary::DictionaryKind;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME: &str = "japanese_compound_word";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseCompoundWordTokenFilterConfig {
    kind: DictionaryKind,
    tags: HashSet<String>,
    new_tag: Option<String>,
}

impl JapaneseCompoundWordTokenFilterConfig {
    pub fn new(
        kind: DictionaryKind,
        tags: HashSet<String>,
        new_tag: Option<String>,
    ) -> LinderaResult<Self> {
        let mut formatted_tags: HashSet<String> = HashSet::new();
        for tag in tags.iter() {
            let mut formatted_tag = ["*", "*", "*", "*"];

            let tag_array: Vec<&str> = tag.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            formatted_tags.insert(formatted_tag.join(","));
        }

        let formatted_new_tag = if let Some(new_tag_str) = new_tag {
            let mut formatted_tag = ["*", "*", "*", "*"];

            let tag_array: Vec<&str> = new_tag_str.split(',').collect();
            for (i, j) in tag_array.iter().enumerate() {
                formatted_tag[i] = j;
            }

            Some(formatted_tag.join(","))
        } else {
            None
        };

        Ok(Self {
            kind,
            tags: formatted_tags,
            new_tag: formatted_new_tag,
        })
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let tmp_config = serde_json::from_slice::<JapaneseCompoundWordTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Self::new(tmp_config.kind, tmp_config.tags, tmp_config.new_tag)
    }

    pub fn from_value(value: &Value) -> LinderaResult<Self> {
        let tmp_config =
            serde_json::from_value::<JapaneseCompoundWordTokenFilterConfig>(value.clone())
                .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;

        Self::new(tmp_config.kind, tmp_config.tags, tmp_config.new_tag)
    }
}

/// Compound consecutive tokens that have specified part-of-speech tags into a single token.
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

    // Concatenate two tokens into one.
    fn concat_token(&self, token1: &mut Token, token2: &Token) {
        token1.text = Cow::Owned(format!("{}{}", token1.text, token2.text));
        token1.byte_end = token2.byte_end;
        token1.position_length += token2.position_length;

        let details = match self.config.kind {
            #[cfg(feature = "ipadic")]
            DictionaryKind::IPADIC => vec![
                Cow::Borrowed("複合語"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ],
            #[cfg(feature = "ipadic-neologd")]
            DictionaryKind::IPADICNEologd => vec![
                Cow::Borrowed("複合語"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ],
            #[cfg(feature = "unidic")]
            DictionaryKind::UniDic => vec![
                Cow::Borrowed("複合語"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ],
            _ => vec![],
        };

        token1.details = Some(details);
    }
}

impl TokenFilter for JapaneseCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    /// Merges tokens based on matching part-of-speech tags and updates the token list.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The tokens will be modified in place by merging consecutive tokens that share matching tags.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Processing**:
    ///    - The function iterates over the list of tokens, and for each token, it checks the part-of-speech tags up to 4 elements (`tags_len`).
    ///    - If the token's tag matches one of the tags specified in the configuration (`self.config.tags`), it attempts to merge the token with the subsequent tokens.
    ///
    /// 2. **Token Merging**:
    ///    - When two consecutive tokens have matching tags, they are merged by concatenating their details into a single token.
    ///    - If no matching tag is found for the next token, the current token is finalized and added to the new token list.
    ///
    /// 3. **Replacing Tokens**:
    ///    - After processing all tokens, the original token list is replaced by the new list (`new_tokens`) that contains merged tokens where applicable.
    ///
    /// # Special Cases:
    ///
    /// - If no tags match, the original tokens are retained without modification.
    /// - If multiple tokens match, they are merged into a single token.
    ///
    /// # Errors
    ///
    /// If any issue arises during token processing, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        // New tokens
        let mut new_tokens = Vec::new();

        // Index of the current token
        let mut i = 0;

        while i < tokens.len() {
            // Get the current token by reference and clone it later only if needed.
            let current = &mut tokens[i];
            i += 1;

            let current_details = current.details();
            let current_tags_len = current_details.len().min(4);
            let current_tag = current_details[0..current_tags_len].join(",");

            // If the tag matches, merge the tokens
            if self.config.tags.contains(&current_tag) {
                // Clone the current token as it will be modified
                let mut merged_token = current.clone();

                while i < tokens.len() {
                    let next = &mut tokens[i];
                    let next_details = next.details();
                    let next_tags_len = next_details.len().min(4);
                    let next_tag = next_details[0..next_tags_len].join(",");

                    // If the next tag matches, merge the tokens; otherwise, break the loop
                    if self.config.tags.contains(&next_tag) {
                        // Concatenate the current token and the next token.
                        self.concat_token(&mut merged_token, next);
                        i += 1; // Move to the next token
                    } else {
                        break; // No match, stop merging
                    }
                }
                new_tokens.push(merged_token);
            } else {
                // No need to merge, just clone the current token
                new_tokens.push(current.clone());
            }
        }

        // Replace the original tokens with the new tokens after processing.
        mem::swap(tokens, &mut new_tokens);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_compound_word_token_filter_config_from_slice_ipadic() {
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilterConfig;

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

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_compound_word_token_filter_from_slice_ipadic() {
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;

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
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use crate::token::Token;
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
        use crate::token_filter::TokenFilter;

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

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("１"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId(391174, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("１"),
                    Cow::Borrowed("イチ"),
                    Cow::Borrowed("イチ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("０"),
                byte_start: 3,
                byte_end: 6,
                position: 1,
                position_length: 1,
                word_id: WordId(391171, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("０"),
                    Cow::Borrowed("ゼロ"),
                    Cow::Borrowed("ゼロ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("０"),
                byte_start: 6,
                byte_end: 9,
                position: 2,
                position_length: 1,
                word_id: WordId(391171, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("０"),
                    Cow::Borrowed("ゼロ"),
                    Cow::Borrowed("ゼロ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("円"),
                byte_start: 9,
                byte_end: 12,
                position: 3,
                position_length: 1,
                word_id: WordId(137904, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("助数詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("円"),
                    Cow::Borrowed("エン"),
                    Cow::Borrowed("エン"),
                ]),
            },
            Token {
                text: Cow::Borrowed("玉"),
                byte_start: 12,
                byte_end: 15,
                position: 4,
                position_length: 1,
                word_id: WordId(287427, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("玉"),
                    Cow::Borrowed("ダマ"),
                    Cow::Borrowed("ダマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("を"),
                byte_start: 15,
                byte_end: 18,
                position: 5,
                position_length: 1,
                word_id: WordId(80582, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("玉"),
                    Cow::Borrowed("ダマ"),
                    Cow::Borrowed("ダマ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("拾う"),
                byte_start: 18,
                byte_end: 24,
                position: 6,
                position_length: 1,
                word_id: WordId(228047, true),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("動詞"),
                    Cow::Borrowed("自立"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("五段・ワ行促音便"),
                    Cow::Borrowed("基本形"),
                    Cow::Borrowed("拾う"),
                    Cow::Borrowed("ヒロウ"),
                    Cow::Borrowed("ヒロウ"),
                ]),
            },
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
            tokens[0].details(),
            vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*",]
        );
    }
}
