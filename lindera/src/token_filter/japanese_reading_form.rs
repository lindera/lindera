use std::borrow::Cow;

use serde_json::Value;

use crate::LinderaResult;
use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_READING_FORM_TOKEN_FILTER_NAME: &str = "japanese_reading_form";

pub type JapaneseReadingFormTokenFilterConfig = Value;

/// Replace the text of a token with the reading of the text as registered in the morphological dictionary.
/// The reading is in katakana.
///
#[derive(Clone, Debug)]
pub struct JapaneseReadingFormTokenFilter {}

impl JapaneseReadingFormTokenFilter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_config(_config: &JapaneseReadingFormTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new())
    }
}

impl Default for JapaneseReadingFormTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for JapaneseReadingFormTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_READING_FORM_TOKEN_FILTER_NAME
    }

    /// Updates token text to the reading form based on the dictionary kind and details configuration.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The text of each token may be updated based on the dictionary kind and token details.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating the success of the operation.
    ///
    /// # Process
    ///
    /// 1. **Token Detail Check**:
    ///    - For each token in the vector, the function checks the first detail (`get_detail(0)`).
    ///    - If the first detail is `"UNK"`, the token is skipped and no further processing is done for that token.
    ///
    /// 2. **Dictionary Type Handling**:
    ///    - Depending on the `config.kind` (which determines the type of dictionary used), the function selects the appropriate index in the token's details:
    ///        - **IPADIC** and **IPADICNEologd**: Use the detail at index 7 to get the reading form.
    ///        - **UniDic**: Use the detail at index 6 to get the reading form.
    ///    - If the dictionary kind does not match any of the supported types, the token is skipped.
    ///
    /// 3. **Text Update**:
    ///    - If a valid detail is found at the selected `detail_index`, the function updates the token's text with the reading form found in that detail.
    ///    - The new text is assigned using `Cow::Owned` to ensure the token owns the new text value.
    ///
    /// # Example
    ///
    /// This function is useful when you need to replace the token's surface form with its reading form based on the specific dictionary configuration (e.g., converting kanji to its phonetic representation).
    ///
    /// # Errors
    ///
    /// Returns a `LinderaResult<()>` if there is an issue during token processing or text conversion. However, under normal circumstances, it should process without errors.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            // Skip unknown tokens
            if let Some(pos) = token.get_detail(0) {
                if pos == "UNK" {
                    continue;
                }
            }

            if let Some(reading) = token.get("reading") {
                token.text = Cow::Owned(reading.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "ipadic", feature = "embedded-ipadic"))]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;

        let filter = JapaneseReadingFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田空港"),
                byte_start: 0,
                byte_end: 12,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 321702,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("羽田空港"),
                    Cow::Borrowed("ハネダクウコウ"),
                    Cow::Borrowed("ハネダクーコー"),
                ]),
            },
            Token {
                text: Cow::Borrowed("限定"),
                byte_start: 12,
                byte_end: 18,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 374175,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("サ変接続"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテイ"),
                    Cow::Borrowed("ゲンテイ"),
                ]),
            },
            Token {
                text: Cow::Borrowed("トートバッグ"),
                byte_start: 18,
                byte_end: 36,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 4294967295,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![Cow::Borrowed("UNK")]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(&tokens[0].text, "ハネダクウコウ");
        assert_eq!(&tokens[1].text, "ゲンテイ");
        assert_eq!(&tokens[2].text, "トートバッグ");
    }

    #[cfg(all(feature = "unidic", feature = "embedded-unidic"))]
    #[test]
    fn test_japanese_reading_form_token_filter_apply_unidic() {
        use std::borrow::Cow;

        use crate::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use crate::token::Token;
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;

        let filter = JapaneseReadingFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                text: Cow::Borrowed("羽田"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId {
                    id: 618177,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("人名"),
                    Cow::Borrowed("姓"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("羽田"),
                    Cow::Borrowed("ハタ"),
                    Cow::Borrowed("固"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("空港"),
                byte_start: 6,
                byte_end: 12,
                position: 1,
                position_length: 1,
                word_id: WordId {
                    id: 587348,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("クウコウ"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("空港"),
                    Cow::Borrowed("クーコー"),
                    Cow::Borrowed("漢"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("限定"),
                byte_start: 12,
                byte_end: 18,
                position: 2,
                position_length: 1,
                word_id: WordId {
                    id: 720499,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("サ変可能"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("ゲンテイ"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテー"),
                    Cow::Borrowed("限定"),
                    Cow::Borrowed("ゲンテー"),
                    Cow::Borrowed("漢"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("トート"),
                byte_start: 18,
                byte_end: 27,
                position: 3,
                position_length: 1,
                word_id: WordId {
                    id: 216230,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("トート"),
                    Cow::Borrowed("外"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
            Token {
                text: Cow::Borrowed("バッグ"),
                byte_start: 27,
                byte_end: 36,
                position: 4,
                position_length: 1,
                word_id: WordId {
                    id: 223781,
                    is_system: true,
                },
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("普通名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ-bag"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("バッグ"),
                    Cow::Borrowed("外"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(&tokens[0].text, "ハタ");
        assert_eq!(&tokens[1].text, "クウコウ");
        assert_eq!(&tokens[2].text, "ゲンテイ");
        assert_eq!(&tokens[3].text, "トート");
        assert_eq!(&tokens[4].text, "バッグ");
    }
}
