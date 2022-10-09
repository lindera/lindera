use std::fmt;
use std::path::PathBuf;

use byteorder::{ByteOrder, LittleEndian};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use lindera_core::dictionary::Dictionary;
use lindera_core::token::Token;
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::viterbi::Lattice;
use lindera_core::word_entry::WordId;

use crate::builder::{load_dictionary, load_user_dictionary};
use crate::error::LinderaErrorKind;
use crate::mode::Mode;
use crate::{DictionaryKind, LinderaResult};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictionaryConfig {
    pub kind: Option<DictionaryKind>,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserDictionaryConfig {
    pub kind: Option<DictionaryKind>,
    pub path: PathBuf,
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

    fn word_detail(&self, word_id: WordId) -> LinderaResult<Vec<String>> {
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

    fn tokenize_process<'a>(
        &self,
        text: &'a str,
        with_details: bool,
    ) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut lattice = Lattice::default();

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
                let (token_start, word_id) = offsets[i];
                let token_stop = if i == offsets.len() - 1 {
                    sentence.len()
                } else {
                    let (next_start, _word_id) = offsets[i + 1];
                    next_start
                };
                tokens.push(Token {
                    text: &sentence[token_start..token_stop],
                    details: if with_details {
                        Some(self.word_detail(word_id)?)
                    } else {
                        None
                    },
                })
            }
        }

        Ok(tokens)
    }

    /// Tokenize the text (without word details)
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
        self.tokenize_process(text, false)
    }

    /// Tokenize the text (with word details)
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
    pub fn tokenize_with_details<'a>(&self, text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        self.tokenize_process(text, true)
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "日本語",
                "の",
                "形態素",
                "解析",
                "を",
                "行う",
                "こと",
                "が",
                "でき",
                "ます",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "日本", "語", "の", "形態", "素", "解析", "を", "行う", "こと", "が", "でき",
                "ます", "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("한국어의형태해석을실시할수있습니다.")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "한국어",
                "의",
                "형태",
                "해석",
                "을",
                "실시",
                "할",
                "수",
                "있",
                "습니다",
                "."
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("可以进行中文形态学分析。").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["可以", "进行", "中文", "形态学", "分析", "。"]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り",
                "駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["하네다공항", "한정", "토트백", "."]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["羽田机场", "限定", "托特", "包", "。"]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り",
                "駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["하네다공항", "한정", "토트백", "."]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["羽田机场", "限定", "托特", "包", "。"]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec![
                "東京スカイツリー",
                "の",
                "最寄り駅",
                "は",
                "とうきょうスカイツリー駅",
                "です",
                "。"
            ]
        );
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

        Tokenizer::with_config(config).unwrap();
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

        Tokenizer::with_config(config).unwrap();
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("羽田空港限定トートバッグ").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["羽田空港", "限定", "トートバッグ"]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize("羽田空港限定トートバッグ").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.text).collect::<Vec<_>>(),
            vec!["羽田", "空港", "限定", "トートバッグ"]
        );
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

        let tokenizer = Tokenizer::with_config(config).unwrap();

        let tokens = tokenizer.tokenize(large_text.as_str()).unwrap();
        assert!(!tokens.is_empty());
    }
}
