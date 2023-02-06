use std::fmt;
use std::path::PathBuf;

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use lindera_core::{
    dictionary::Dictionary, token::Token, user_dictionary::UserDictionary, viterbi::Lattice,
};

use crate::{
    builder::{load_dictionary, load_user_dictionary},
    mode::Mode,
    DictionaryKind, LinderaResult,
};

/// Dictionary config
///
/// Use this if you want to use a dictionary when tokenizing.
///
/// Either `kind` or `path` must be specified.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictionaryConfig {
    /// Specify the kind of dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT) if a self-contained dictionary is used for tokenization.
    pub kind: Option<DictionaryKind>,
    /// Specifies the path to a pre-built external dictionary if one is used.
    pub path: Option<PathBuf>,
}

/// User dictionary config
///
/// Use this if you want to use a user dictionary when tokenizing.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserDictionaryConfig {
    /// Path to the user dictionary file.
    pub path: PathBuf,
    /// If the user dictionary was in CSV format, specify the dictionary type (IPADIC, UniDic, ko-dic or CC-CEDICT).
    pub kind: Option<DictionaryKind>,
}

// Only the value specified by the feature flag is stored.
pub const CONTAINED_DICTIONARIES: &[&str] = &[
    #[cfg(feature = "ipadic")]
    "ipadic",
    #[cfg(feature = "unidic")]
    "unidic",
    #[cfg(feature = "ko-dic")]
    "ko-dic",
    #[cfg(feature = "cc-cedict")]
    "cc-cedict",
];

/// Tokenizer config
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenizerConfig {
    /// The dictionary config to be used for tokenization.
    pub dictionary: DictionaryConfig,

    /// The user dictionary config to be used for tokenization. (Optional)
    pub user_dictionary: Option<UserDictionaryConfig>,

    /// The tokenization mode.
    pub mode: Mode,
}

impl Default for TokenizerConfig {
    /// Return default Tokenizer config
    /// default mode is Mode::Normal
    fn default() -> Self {
        Self {
            dictionary: DictionaryConfig {
                kind: None,
                path: None,
            },
            user_dictionary: None,
            mode: Mode::Normal,
        }
    }
}

impl<'de> Deserialize<'de> for TokenizerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Dictionary,
            UserDictionary,
            Mode,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`dictionary`, `user_dictionary`, or `mode`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "dictionary" => Ok(Field::Dictionary),
                            "user_dictionary" => Ok(Field::UserDictionary),
                            "mode" => Ok(Field::Mode),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct DurationVisitor;

        impl<'de> Visitor<'de> for DurationVisitor {
            type Value = TokenizerConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct TokenizerConfig")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<TokenizerConfig, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let dictionary = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let user_dictionary = seq.next_element()?.unwrap_or(None);
                let mode = seq.next_element()?.unwrap_or(Mode::Normal);

                Ok(TokenizerConfig {
                    dictionary,
                    user_dictionary,
                    mode,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<TokenizerConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut dictionary = None;
                let mut user_dictionary = None;
                let mut mode = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Dictionary => {
                            if dictionary.is_some() {
                                return Err(de::Error::duplicate_field("dictionary"));
                            }
                            dictionary = Some(map.next_value()?);
                        }
                        Field::UserDictionary => {
                            if user_dictionary.is_some() {
                                return Err(de::Error::duplicate_field("user_dictionary"));
                            }
                            user_dictionary = Some(map.next_value()?);
                        }
                        Field::Mode => {
                            if mode.is_some() {
                                return Err(de::Error::duplicate_field("mode"));
                            }
                            mode = Some(map.next_value()?);
                        }
                    }
                }
                let dictionary =
                    dictionary.ok_or_else(|| de::Error::missing_field("dictionary"))?;
                let mode = mode.unwrap_or(Mode::Normal);
                Ok(TokenizerConfig {
                    dictionary,
                    user_dictionary,
                    mode,
                })
            }
        }

        const FIELDS: &[&str] = &["dictionary", "user_dictionary", "mode", "with_details"];
        deserializer.deserialize_struct("TokenizerConfig", FIELDS, DurationVisitor)
    }
}

#[derive(Clone)]
/// Tokenizer
pub struct Tokenizer {
    /// The dictionary to be used for tokenization.
    dictionary: Dictionary,

    /// The user dictionary to be used for tokenization. (Optional)
    user_dictionary: Option<UserDictionary>,

    /// The tokenization mode.
    mode: Mode,
}

impl Tokenizer {
    /// Create a new tokenizer from the tokenizer config.
    ///
    /// # Arguments
    ///
    /// * `config`: The tokenizer config.
    ///
    /// returns: LinderaResult<Tokenizer>
    ///
    pub fn from_config(config: TokenizerConfig) -> LinderaResult<Self> {
        let dictionary = load_dictionary(config.dictionary)?;

        let user_dictionary = match config.user_dictionary {
            Some(user_dict_conf) => Some(load_user_dictionary(user_dict_conf)?),
            None => None,
        };

        Ok(Self::new(dictionary, user_dictionary, config.mode))
    }

    /// Create a new tokenizer.
    ///
    /// # Arguments
    ///
    /// * `dictionary`: The dictionary to be used for tokenization.
    /// * `user_dictionary`: The user dictionary to be used for tokenization. (Optional)
    /// * `mode`: The tokenization mode.
    ///
    /// returns: LinderaResult<Tokenizer>
    ///
    pub fn new(
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
        mode: Mode,
    ) -> Self {
        Self {
            dictionary,
            user_dictionary,
            mode,
        }
    }

    /// Tokenize the text
    ///
    /// # Arguments
    ///
    /// * `text`: The text to be tokenized.
    ///
    /// returns: LinderaResult<Vec<Token>>
    ///
    /// * Vec<Token> : The list of `Token` if succeeded
    /// * LinderaError : Error message with LinderaErrorKind
    ///
    pub fn tokenize<'a>(&'a self, text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut lattice = Lattice::default();

        let mut position = 0_usize;
        let mut byte_position = 0_usize;

        // Split text into sentences using Japanese punctuation.
        for sentence in text.split_inclusive(&['。', '、']) {
            if text.is_empty() {
                continue;
            }

            lattice.set_text(
                &self.dictionary.dict,
                &self.user_dictionary.as_ref().map(|d| &d.dict),
                &self.dictionary.char_definitions,
                &self.dictionary.unknown_dictionary,
                sentence,
                &self.mode,
            );
            lattice.calculate_path_costs(&self.dictionary.cost_matrix, &self.mode);

            let offsets = lattice.tokens_offset();

            for i in 0..offsets.len() {
                let (byte_start, word_id) = offsets[i];
                let byte_end = if i == offsets.len() - 1 {
                    sentence.len()
                } else {
                    let (next_start, _word_id) = offsets[i + 1];
                    next_start
                };

                // retrieve token from its sentence byte positions
                let surface = &sentence[byte_start..byte_end];

                // compute the token's absolute byte positions
                let token_start = byte_position;
                byte_position += surface.len();
                let token_end = byte_position;

                tokens.push(Token::new(
                    surface,
                    token_start,
                    token_end,
                    position,
                    word_id,
                    &self.dictionary,
                    self.user_dictionary.as_ref(),
                ));

                position += 1;
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "ipadic",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    #[cfg(any(
        feature = "ipadic",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use crate::{
        mode::{Mode, Penalty},
        tokenizer::{DictionaryConfig, Tokenizer, TokenizerConfig, UserDictionaryConfig},
        DictionaryKind,
    };

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_config_ipadic_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": "normal"
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::IPADIC));
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_config_ipadic_decompose() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::IPADIC));
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。テスト。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "日本語");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "日本語",
                    "ニホンゴ",
                    "ニホンゴ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "形態素");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "形態素",
                    "ケイタイソ",
                    "ケイタイソ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "解析",
                    "カイセキ",
                    "カイセキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "を", "ヲ", "ヲ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "自立",
                    "*",
                    "*",
                    "五段・ワ行促音便",
                    "基本形",
                    "行う",
                    "オコナウ",
                    "オコナウ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "非自立",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "こと",
                    "コト",
                    "コト"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "が", "ガ", "ガ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "自立",
                    "*",
                    "*",
                    "一段",
                    "連用形",
                    "できる",
                    "デキ",
                    "デキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・マス",
                    "基本形",
                    "ます",
                    "マス",
                    "マス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "テスト");
            assert_eq!(token.byte_start, 60);
            assert_eq!(token.byte_end, 69);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "テスト",
                    "テスト",
                    "テスト"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 69);
            assert_eq!(token.byte_end, 72);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "日本");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "地名",
                    "国",
                    "*",
                    "*",
                    "ニッポン",
                    "日本",
                    "日本",
                    "ニッポン",
                    "日本",
                    "ニッポン",
                    "固",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "語");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ゴ",
                    "語",
                    "語",
                    "ゴ",
                    "語",
                    "ゴ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "形態");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ケイタイ",
                    "形態",
                    "形態",
                    "ケータイ",
                    "形態",
                    "ケータイ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "素");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "接尾辞",
                    "名詞的",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ソ",
                    "素",
                    "素",
                    "ソ",
                    "素",
                    "ソ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "サ変可能",
                    "*",
                    "*",
                    "*",
                    "カイセキ",
                    "解析",
                    "解析",
                    "カイセキ",
                    "解析",
                    "カイセキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ヲ",
                    "を",
                    "を",
                    "オ",
                    "を",
                    "オ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "一般",
                    "*",
                    "*",
                    "五段-ワア行",
                    "連体形-一般",
                    "オコナウ",
                    "行う",
                    "行う",
                    "オコナウ",
                    "行う",
                    "オコナウ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "コト",
                    "事",
                    "こと",
                    "コト",
                    "こと",
                    "コト",
                    "和",
                    "コ濁",
                    "基本形",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ガ",
                    "が",
                    "が",
                    "ガ",
                    "が",
                    "ガ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "非自立可能",
                    "*",
                    "*",
                    "上一段-カ行",
                    "連用形-一般",
                    "デキル",
                    "出来る",
                    "でき",
                    "デキ",
                    "できる",
                    "デキル",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-マス",
                    "終止形-一般",
                    "マス",
                    "ます",
                    "ます",
                    "マス",
                    "ます",
                    "マス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("한국어의형태해석을실시할수있습니다.")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "한국어");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "F",
                    "한국어",
                    "Compound",
                    "*",
                    "*",
                    "한국/NNG/*+어/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "의");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["JKG", "*", "F", "의", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "형태");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "F", "형태", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "해석");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "행위", "T", "해석", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "을");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["JKO", "*", "T", "을", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "실시");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "행위", "F", "실시", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "할");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "VV+ETM",
                    "*",
                    "T",
                    "할",
                    "Inflect",
                    "VV",
                    "ETM",
                    "하/VV/*+ᆯ/ETM/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "수");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "F", "수", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "있");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["VX", "*", "T", "있", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "습니다");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["EF", "*", "F", "습니다", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), ".");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 52);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_cc_cedict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("可以进行中文形态学分析。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "可以");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "ke3 yi3",
                    "可以",
                    "可以",
                    "can/may/possible/able to/not bad/pretty good/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "进行");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*", 
                    "*", 
                    "*", 
                    "*", 
                    "jin4 xing2", 
                    "進行", 
                    "进行", 
                    "to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "中文");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "Zhong1 wen2",
                    "中文",
                    "中文",
                    "Chinese language/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "形态学");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xing2 tai4 xue2",
                    "形態學",
                    "形态学",
                    "morphology (in biology or linguistics)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "分析");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "fen1 xi1",
                    "分析",
                    "分析",
                    "to analyze/analysis/CL:個|个[ge4]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_simple_userdic_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_with_simple_userdic_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "モヨリ",
                    "最寄り",
                    "最寄り",
                    "モヨリ",
                    "最寄り",
                    "モヨリ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "エキ",
                    "駅",
                    "駅",
                    "エキ",
                    "駅",
                    "エキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "係助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ハ",
                    "は",
                    "は",
                    "ワ",
                    "は",
                    "ワ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-デス",
                    "終止形-一般",
                    "デス",
                    "です",
                    "です",
                    "デス",
                    "です",
                    "デス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_with_simple_userdic_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "T",
                    "토트백",
                    "Compound",
                    "*",
                    "*",
                    "토트/NNP/인명+백/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_with_simple_userdic_cc_cedict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xian4 ding4",
                    "限定",
                    "限定",
                    "to restrict to/to limit/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "tuo1 te4",
                    "托特",
                    "托特",
                    "(loanword) tote (bag)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*", 
                    "*", 
                    "*", 
                    "*", 
                    "bao1", 
                    "包", 
                    "包", 
                    "to cover/to wrap/to hold/to include/to take charge of/to contract (to or for)/package/wrapper/container/bag/to hold or embrace/bundle/packet/CL:個|个[ge4]", 
                    "隻|只[zhi1]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_simple_userdic_bin_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_with_simple_userdic_bin_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "モヨリ",
                    "最寄り",
                    "最寄り",
                    "モヨリ",
                    "最寄り",
                    "モヨリ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "エキ",
                    "駅",
                    "駅",
                    "エキ",
                    "駅",
                    "エキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "係助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ハ",
                    "は",
                    "は",
                    "ワ",
                    "は",
                    "ワ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-デス",
                    "終止形-一般",
                    "デス",
                    "です",
                    "です",
                    "デス",
                    "です",
                    "デス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_with_simple_userdic_bin_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "T",
                    "토트백",
                    "Compound",
                    "*",
                    "*",
                    "토트/NNP/인명+백/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_with_simple_userdic_bin_cc_cedict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xian4 ding4",
                    "限定",
                    "限定",
                    "to restrict to/to limit/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "tuo1 te4",
                    "托特",
                    "托特",
                    "(loanword) tote (bag)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*", 
                    "*", 
                    "*", 
                    "*", 
                    "bao1", 
                    "包", 
                    "包", 
                    "to cover/to wrap/to hold/to include/to take charge of/to contract (to or for)/package/wrapper/container/bag/to hold or embrace/bundle/packet/CL:個|个[ge4]", 
                    "隻|只[zhi1]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_detailed_userdic_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_detailed_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "カスタム名詞",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "トウキョウスカイツリー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "カスタム名詞",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "トウキョウスカイツリーエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mixed_user_dict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_mixed_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "カスタム名詞",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "トウキョウスカイツリー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        Tokenizer::from_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        Tokenizer::from_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_nomal_mode() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田空港限定トートバッグ").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "羽田空港");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "羽田空港",
                    "ハネダクウコウ",
                    "ハネダクーコー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "限定",
                    "ゲンテイ",
                    "ゲンテイ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_decompose_mode() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Decompose(Penalty::default()),
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田空港限定トートバッグ").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "羽田");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "人名",
                    "姓",
                    "*",
                    "*",
                    "羽田",
                    "ハタ",
                    "ハタ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "空港");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "空港",
                    "クウコウ",
                    "クーコー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "限定",
                    "ゲンテイ",
                    "ゲンテイ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.get_text(), "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_long_text() {
        let mut large_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut large_text = String::new();
        let _size = large_file.read_to_string(&mut large_text).unwrap();

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();

        let tokens = tokenizer.tokenize(large_text.as_str()).unwrap();
        assert!(!tokens.is_empty());
    }
}
