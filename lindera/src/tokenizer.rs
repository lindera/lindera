use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use byteorder::{ByteOrder, LittleEndian};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use lindera_core::dictionary::Dictionary;
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::viterbi::Lattice;
use lindera_core::word_entry::WordId;

use crate::builder::{load_dictionary, load_user_dictionary};
use crate::error::LinderaErrorKind;
use crate::mode::Mode;
use crate::{DictionaryKind, LinderaResult};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictionaryConfig {
    pub kind: DictionaryKind,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserDictionaryConfig {
    pub kind: DictionaryKind,
    pub path: PathBuf,
}

pub const SUPPORTED_DICTIONARY_KIND: &[&str] = &[
    #[cfg(feature = "ipadic")]
    "ipadic",
    #[cfg(feature = "unidic")]
    "unidic",
    #[cfg(feature = "ko-dic")]
    "ko-dic",
    #[cfg(feature = "cc-cedict")]
    "cc-cedict",
];

pub const DEFAULT_DICTIONARY_KIND: &str = SUPPORTED_DICTIONARY_KIND[0];

#[derive(Serialize, Clone)]
/// Token Object
pub struct Token<'a> {
    pub text: &'a str,
    pub word_id: WordId,
}

/// Tokenizer config
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenizerConfig {
    /// The dictionary metadata
    pub dictionary: DictionaryConfig,

    /// The user dictionary metadata.
    pub user_dictionary: Option<UserDictionaryConfig>,

    /// Tokenize mode
    pub mode: Mode,
}

impl Default for TokenizerConfig {
    /// Return default Tokenizer config
    /// default mode is Mode::Normal
    fn default() -> Self {
        Self {
            dictionary: DictionaryConfig {
                kind: DictionaryKind::from_str(self::DEFAULT_DICTIONARY_KIND).unwrap(),
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
                        formatter.write_str("`dictionary`, `user_dictionary` or `mode`")
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

        const FIELDS: &[&str] = &["dictionary", "user_dictionary", "mode"];
        deserializer.deserialize_struct("TokenizerConfig", FIELDS, DurationVisitor)
    }
}

#[derive(Clone)]
/// Tokenizer
pub struct Tokenizer {
    dictionary: Dictionary,
    user_dictionary: Option<UserDictionary>,
    mode: Mode,
}

impl Tokenizer {
    /// Creates a new instance with default TokenizerConfig
    pub fn new() -> LinderaResult<Tokenizer> {
        let config = TokenizerConfig::default();
        Tokenizer::with_config(config)
    }

    /// Creates a new instance with the config
    ///
    /// # Arguments
    ///
    /// * `config`: settings of Tokenizer
    ///
    /// returns: Result<Tokenizer, LinderaError>
    ///
    pub fn with_config(config: TokenizerConfig) -> LinderaResult<Tokenizer> {
        let dictionary = load_dictionary(config.dictionary)?;

        let user_dictionary = match config.user_dictionary {
            Some(user_dict_conf) => Some(load_user_dictionary(user_dict_conf)?),
            None => None,
        };

        let tokenizer = Tokenizer {
            dictionary,
            user_dictionary,
            mode: config.mode,
        };

        Ok(tokenizer)
    }

    pub fn word_detail(&self, word_id: WordId) -> LinderaResult<Vec<String>> {
        if word_id.is_unknown() {
            return Ok(vec!["UNK".to_string()]);
        }

        let (words_idx_data, words_data) = if word_id.is_system() {
            (
                self.dictionary.words_idx_data.as_slice(),
                self.dictionary.words_data.as_slice(),
            )
        } else {
            (
                self.user_dictionary
                    .as_ref()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("internal error."))
                    })?
                    .words_idx_data
                    .as_slice(),
                self.user_dictionary
                    .as_ref()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("internal error."))
                    })?
                    .words_data
                    .as_slice(),
            )
        };
        let idx = LittleEndian::read_u32(&words_idx_data[4 * word_id.0 as usize..][..4]);
        let data = &words_data[idx as usize..];
        let word_detail = bincode::deserialize_from(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

        Ok(word_detail)
    }

    /// Returns an array of offsets that mark the beginning of each tokens,
    /// in bytes.
    ///
    /// For instance
    /// e.g. "僕は'
    ///
    /// returns the array `[0, 3]`
    ///
    /// The array, always starts with 0, except if you tokenize the empty string,
    /// in which case an empty array is returned.
    ///
    /// Whitespaces also count as tokens.
    pub(crate) fn tokenize_offsets(
        &self,
        text: &str,
        lattice: &mut Lattice,
    ) -> Vec<(usize, WordId)> {
        if text.is_empty() {
            return Vec::new();
        }

        let mode = self.mode.clone();

        lattice.set_text(
            &self.dictionary.dict,
            &self.user_dictionary.as_ref().map(|d| &d.dict),
            &self.dictionary.char_definitions,
            &self.dictionary.unknown_dictionary,
            text,
            &mode,
        );
        lattice.calculate_path_costs(&self.dictionary.cost_matrix, &mode);
        lattice.tokens_offset()
    }

    fn tokenize_without_split<'a>(
        &self,
        text: &'a str,
        tokens: &mut Vec<Token<'a>>,
        lattice: &mut Lattice,
    ) -> LinderaResult<()> {
        let offsets = self.tokenize_offsets(text, lattice);

        for i in 0..offsets.len() {
            let (token_start, word_id) = offsets[i];
            let token_stop = if i == offsets.len() - 1 {
                text.len()
            } else {
                let (next_start, _) = offsets[i + 1];
                next_start
            };
            tokens.push(Token {
                text: &text[token_start..token_stop],
                word_id,
            })
        }

        Ok(())
    }

    /// Tokenize the text
    ///
    /// # Arguments
    ///
    /// * `text`: Japanese text
    ///
    /// returns: Result<Vec<Token>, LinderaError>
    ///
    /// * Vec<Token> : the list of `Token` if succeeded
    /// * LinderaError : Error message with LinderaErrorKind
    ///
    pub fn tokenize<'a>(&self, text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut lattice = Lattice::default();
        let mut tokens = Vec::new();
        for sub_str in text.split_inclusive(&['。', '、']) {
            self.tokenize_without_split(sub_str, &mut tokens, &mut lattice)?;
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::PathBuf;

    use lindera_core::word_entry::WordId;

    use crate::mode::{Mode, Penalty};
    use crate::tokenizer::{Token, Tokenizer, TokenizerConfig};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_from_bytes_ipdic_default() {
        let json_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            }
        }
        "#;
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        assert_eq!(args.dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(args.user_dictionary, None);
        assert_eq!(args.mode, Mode::Normal);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_from_bytes_ipadic_normal() {
        let json_str = r#"
        {
            "dictionary": {
                "kind": "ipadic",
                "mode": "normal"
            }
        }
        "#;
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        assert_eq!(args.dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(args.user_dictionary, None);
        assert_eq!(args.mode, Mode::Normal);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_from_bytes_ipadic_decompose() {
        let json_str = r#"
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
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        assert_eq!(args.dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(args.user_dictionary, None);
        assert_eq!(args.mode, Mode::Decompose(Penalty::default()));
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_from_bytes_local_dictionary() {
        let json_str = r#"
        {
            "dictionary": {
                "kind": "ipadic",
                "path": "./resources/ipadic"
            },
            "mode": "normal"
        }
        "#;
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        assert_eq!(args.dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(
            args.dictionary.path,
            Some(PathBuf::from("./resources/ipadic"))
        );
        assert_eq!(args.user_dictionary, None);
        assert_eq!(args.mode, Mode::Normal);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_from_bytes_user_dictionary() {
        let json_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": "./resources/ipadic_simple_userdic.csv"
            },
            "mode": "normal"
        }
        "#;
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        let user_dictionary = args.user_dictionary.unwrap();
        assert_eq!(args.dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(user_dictionary.kind, DictionaryKind::IPADIC);
        assert_eq!(
            user_dictionary.path,
            PathBuf::from("./resources/ipadic_simple_userdic.csv")
        );
        assert_eq!(args.mode, Mode::Normal);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_empty() {
        let config = TokenizerConfig {
            mode: Mode::Decompose(Penalty::default()),
            ..TokenizerConfig::default()
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets("", &mut Lattice::default());
        assert_eq!(tokens, &[]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_space() {
        let config = TokenizerConfig {
            mode: Mode::Decompose(Penalty::default()),
            ..TokenizerConfig::default()
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets(" ", &mut Lattice::default());
        assert_eq!(tokens, &[(0, WordId(4294967295, true))]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_boku_ha() {
        let config = TokenizerConfig {
            mode: Mode::Decompose(Penalty::default()),
            ..TokenizerConfig::default()
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets("僕は", &mut Lattice::default());
        assert_eq!(
            tokens,
            &[(0, WordId(132630, true)), (3, WordId(57063, true))]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_sumomomomo() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("すもももももももものうち").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["すもも", "も", "もも", "も", "もも", "の", "うち"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_gyoi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("御意。 御意〜。").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["御意", "。", " ", "御意", "〜", "。"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_demoyorokobi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("〜でも喜び").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["〜", "でも", "喜び"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mukigen_normal2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("—でも").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["—", "でも"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_atodedenwa() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("後で").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["後で"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_ikkagetsu() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("ーヶ月").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["ーヶ", "月"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mukigen_normal() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("無期限に—でもどの種を?").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_demo() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("――!!?").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["――!!?"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_kaikeishi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("ジム・コガン").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["ジム・コガン"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_bruce() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("ブルース・モラン").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["ブルース・モラン"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_real() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer
            .tokenize(
                "本項で解説する地方病とは、山梨県における日本住血吸虫症の呼称であり、\
             長い間その原因が明らかにならず住民を苦しめた感染症である。",
            )
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "本",
                "項",
                "で",
                "解説",
                "する",
                "地方",
                "病",
                "と",
                "は",
                "、",
                "山梨",
                "県",
                "における",
                "日本",
                "住",
                "血",
                "吸",
                "虫",
                "症",
                "の",
                "呼称",
                "で",
                "あり",
                "、",
                "長い",
                "間",
                "その",
                "原因",
                "が",
                "明らか",
                "に",
                "なら",
                "ず",
                "住民",
                "を",
                "苦しめ",
                "た",
                "感染",
                "症",
                "で",
                "ある",
                "。"
            ]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_hitobito() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("満々!").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            &["満々", "!"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_short() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("日本住").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["日本", "住"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_short2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("ここでは").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["ここ", "で", "は"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_simple_user_dict() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens: Vec<Token> = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            .unwrap();
        assert_eq!("東京スカイツリー", tokens[0].text);
        assert_eq!(
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
            ],
            tokenizer.word_detail(tokens[0].word_id).unwrap()
        );
        let token_texts: Vec<&str> = tokens.iter().map(|token| token.text).collect();
        assert_eq!(
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です"
            ],
            token_texts
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_simple_user_dict_bin() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.bin");
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens: Vec<Token> = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            .unwrap();
        assert_eq!("東京スカイツリー", tokens[0].text);
        assert_eq!(
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
            ],
            tokenizer.word_detail(tokens[0].word_id).unwrap()
        );
        let token_texts: Vec<&str> = tokens.iter().map(|token| token.text).collect();
        assert_eq!(
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です"
            ],
            token_texts
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_detailed_user_dict() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_detailed_userdic.csv");
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens: Vec<Token> = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            .unwrap();
        assert_eq!("東京スカイツリー", tokens[0].text);
        assert_eq!(
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
            ],
            tokenizer.word_detail(tokens[0].word_id).unwrap()
        );
        let token_texts: Vec<&str> = tokens.iter().map(|token| token.text).collect();
        assert_eq!(
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です"
            ],
            token_texts
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mixed_user_dict() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_mixed_userdic.csv");
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        assert!(tokenizer.user_dictionary.is_some());

        let tokens: Vec<Token> = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            .unwrap();
        assert_eq!("東京スカイツリー", tokens[0].text);
        assert_eq!(
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
            ],
            tokenizer.word_detail(tokens[0].word_id).unwrap()
        );
        assert_eq!("とうきょうスカイツリー駅", tokens[4].text);
        assert_eq!(
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
            ],
            tokenizer.word_detail(tokens[4].word_id).unwrap()
        );

        let token_texts: Vec<&str> = tokens.iter().map(|token| token.text).collect();
        assert_eq!(
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です"
            ],
            token_texts
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary: user_dictionary,
            mode: Mode::Normal,
        };
        Tokenizer::with_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };
        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: userdic_file,
        });
        let config = TokenizerConfig {
            dictionary,
            user_dictionary: user_dictionary,
            mode: Mode::Normal,
        };
        Tokenizer::with_config(config).unwrap();
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
        let tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize(large_text.as_str()).unwrap();
        assert!(!tokens.is_empty());
    }
}
