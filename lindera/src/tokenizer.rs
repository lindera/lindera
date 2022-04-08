use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use byteorder::{ByteOrder, LittleEndian};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(feature = "cc-cedict")]
use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
use lindera_core::dictionary::Dictionary;
#[cfg(any(
    feature = "ipadic",
    feature = "unidic",
    feature = "ko-dic",
    feature = "cc-cedict"
))]
use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::file_util::read_file;
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::viterbi::{Lattice, Mode as LinderaCoreMode};
use lindera_core::word_entry::WordId;
#[cfg(feature = "ipadic")]
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
#[cfg(feature = "ko-dic")]
use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;
#[cfg(feature = "unidic")]
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

use crate::error::{LinderaError, LinderaErrorKind};
use crate::mode::Mode;
use crate::LinderaResult;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum DictionaryType {
    #[cfg(feature = "ipadic")]
    #[serde(rename = "ipadic")]
    Ipadic,
    #[cfg(feature = "unidic")]
    #[serde(rename = "unidic")]
    Unidic,
    #[cfg(feature = "ko-dic")]
    #[serde(rename = "ko-dic")]
    KoDic,
    #[cfg(feature = "cc-cedict")]
    #[serde(rename = "cc-cedict")]
    CcCedict,
    LocalDictionary,
}

impl FromStr for DictionaryType {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<DictionaryType, Self::Err> {
        match input {
            #[cfg(feature = "ipadic")]
            "ipadic" => Ok(DictionaryType::Ipadic),
            #[cfg(feature = "unidic")]
            "unidic" => Ok(DictionaryType::Unidic),
            #[cfg(feature = "ko-dic")]
            "ko-dic" => Ok(DictionaryType::KoDic),
            #[cfg(feature = "cc-cedict")]
            "cc-cedict" => Ok(DictionaryType::CcCedict),
            "local" => Ok(DictionaryType::LocalDictionary),
            _ => Err(LinderaErrorKind::DictionaryTypeError
                .with_error(anyhow::anyhow!("Invalid dictionary type: {}", input))),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum UserDictionaryType {
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "binary")]
    Binary,
}

impl FromStr for UserDictionaryType {
    type Err = LinderaError;
    fn from_str(input: &str) -> Result<UserDictionaryType, Self::Err> {
        match input {
            "csv" => Ok(UserDictionaryType::Csv),
            "bin" => Ok(UserDictionaryType::Binary),
            _ => Err(LinderaErrorKind::UserDictionaryTypeError
                .with_error(anyhow::anyhow!("Invalid user dictionary type: {}", input))),
        }
    }
}

pub const SUPPORTED_DICTIONARY_TYPE: &[&str] = &[
    #[cfg(feature = "ipadic")]
    "ipadic",
    #[cfg(feature = "unidic")]
    "unidic",
    #[cfg(feature = "ko-dic")]
    "ko-dic",
    #[cfg(feature = "cc-cedict")]
    "cc-cedict",
    "local",
];

pub const DEFAULT_DICTIONARY_TYPE: &str = SUPPORTED_DICTIONARY_TYPE[0];

#[derive(Serialize, Clone)]
/// Token Object
pub struct Token<'a> {
    pub text: &'a str,
    pub detail: Vec<String>,
}

/// Tokenizer config
pub struct TokenizerConfig {
    /// The type of System Dictionary
    pub dict_type: DictionaryType,
    /// The path of System Dictionary
    pub dict_path: Option<PathBuf>,
    /// The path of User Dictionary
    pub user_dict_path: Option<PathBuf>,
    /// The type of User Dictionary
    pub user_dict_type: UserDictionaryType,
    /// Tokenize mode
    pub mode: Mode,
}

impl Default for TokenizerConfig {
    /// Return default Tokenizer config
    /// default mode is Mode::Normal
    fn default() -> Self {
        Self {
            dict_type: DictionaryType::from_str(DEFAULT_DICTIONARY_TYPE).unwrap(),
            dict_path: None,
            user_dict_path: None,
            user_dict_type: UserDictionaryType::Csv,
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
            DictType,
            DictPath,
            UserDictPath,
            UserDictType,
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
                        formatter.write_str("`dict_type`, `dict_path`, `user_dict_path`, `user_dict_type` or `mode`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "dict_type" => Ok(Field::DictType),
                            "dict_path" => Ok(Field::DictPath),
                            "user_dict_path" => Ok(Field::UserDictPath),
                            "user_dict_type" => Ok(Field::UserDictType),
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
                let dict_type = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let dict_path = seq.next_element()?.unwrap_or_else(|| None);
                let user_dict_path = seq.next_element()?.unwrap_or_else(|| None);
                let user_dict_type = seq
                    .next_element()?
                    .unwrap_or_else(|| UserDictionaryType::Csv);
                let mode = seq.next_element()?.unwrap_or_else(|| Mode::Normal);

                Ok(TokenizerConfig {
                    dict_type,
                    dict_path,
                    user_dict_path,
                    user_dict_type,
                    mode,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<TokenizerConfig, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut dict_type = None;
                let mut dict_path = None;
                let mut user_dict_path = None;
                let mut user_dict_type = None;
                let mut mode = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::DictType => {
                            if dict_type.is_some() {
                                return Err(de::Error::duplicate_field("dict_type"));
                            }
                            dict_type = Some(map.next_value()?);
                        }
                        Field::DictPath => {
                            if dict_path.is_some() {
                                return Err(de::Error::duplicate_field("dict_path"));
                            }
                            dict_path = Some(map.next_value()?);
                        }
                        Field::UserDictPath => {
                            if user_dict_path.is_some() {
                                return Err(de::Error::duplicate_field("user_dict_path"));
                            }
                            user_dict_path = Some(map.next_value()?);
                        }
                        Field::UserDictType => {
                            if user_dict_type.is_some() {
                                return Err(de::Error::duplicate_field("user_dict_type"));
                            }
                            user_dict_type = Some(map.next_value()?);
                        }
                        Field::Mode => {
                            if mode.is_some() {
                                return Err(de::Error::duplicate_field("mode"));
                            }
                            mode = Some(map.next_value()?);
                        }
                    }
                }
                let dict_type = dict_type.ok_or_else(|| de::Error::missing_field("dict_type"))?;
                let dict_path = dict_path.unwrap_or_else(|| None);
                let user_dict_path = user_dict_path.unwrap_or_else(|| None);
                let user_dict_type = user_dict_type.unwrap_or_else(|| UserDictionaryType::Csv);
                let mode = mode.unwrap_or_else(|| Mode::Normal);
                Ok(TokenizerConfig {
                    dict_type,
                    dict_path,
                    user_dict_path,
                    user_dict_type,
                    mode,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "dict_type",
            "dict_path",
            "user_dict_path",
            "user_dict_type",
            "mode",
        ];
        deserializer.deserialize_struct("TokenizerConfig", FIELDS, DurationVisitor)
    }
}

fn build_user_dict(
    dict_type: DictionaryType,
    path: PathBuf,
    user_dict_type: UserDictionaryType,
) -> LinderaResult<UserDictionary> {
    match user_dict_type {
        UserDictionaryType::Csv => match dict_type {
            #[cfg(feature = "ipadic")]
            DictionaryType::Ipadic => {
                let builder = IpadicBuilder::new();
                builder
                    .build_user_dict(&path)
                    .map_err(|e| LinderaErrorKind::DictionaryBuildError.with_error(e))
            }
            #[cfg(feature = "unidic")]
            DictionaryType::Unidic => {
                let builder = UnidicBuilder::new();
                builder
                    .build_user_dict(&path)
                    .map_err(|e| LinderaErrorKind::DictionaryBuildError.with_error(e))
            }
            #[cfg(feature = "ko-dic")]
            DictionaryType::KoDic => {
                let builder = KoDicBuilder::new();
                builder
                    .build_user_dict(&path)
                    .map_err(|e| LinderaErrorKind::DictionaryBuildError.with_error(e))
            }
            #[cfg(feature = "cc-cedict")]
            DictionaryType::CcCedict => {
                let builder = CcCedictBuilder::new();
                builder
                    .build_user_dict(&path)
                    .map_err(|e| LinderaErrorKind::DictionaryBuildError.with_error(e))
            }
            _ => {
                return Err(LinderaErrorKind::DictionaryNotFound
                    .with_error(anyhow::anyhow!("user dictionary path is not set.")));
            }
        },
        UserDictionaryType::Binary => {
            let user_dict_bin_data =
                read_file(&path).map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?;

            UserDictionary::load(&user_dict_bin_data)
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))
        }
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
        let dictionary = match config.dict_type {
            DictionaryType::LocalDictionary => match config.dict_path.clone() {
                Some(path) => lindera_dictionary::load_dictionary(path)
                    .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
                None => {
                    return Err(LinderaErrorKind::DictionaryNotFound
                        .with_error(anyhow::anyhow!("dictionary path is not set.")));
                }
            },
            #[cfg(feature = "ipadic")]
            DictionaryType::Ipadic => lindera_ipadic::load_dictionary()
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
            #[cfg(feature = "unidic")]
            DictionaryType::Unidic => lindera_unidic::load_dictionary()
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
            #[cfg(feature = "ko-dic")]
            DictionaryType::KoDic => lindera_ko_dic::load_dictionary()
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
            #[cfg(feature = "cc-cedict")]
            DictionaryType::CcCedict => lindera_cc_cedict::load_dictionary()
                .map_err(|e| LinderaErrorKind::DictionaryNotFound.with_error(e))?,
        };

        let user_dictionary = match config.user_dict_path {
            Some(path) => Some(build_user_dict(
                config.dict_type,
                path,
                config.user_dict_type,
            )?),
            None => None,
        };

        let tokenizer = Tokenizer {
            dictionary,
            user_dictionary,
            mode: config.mode,
        };

        Ok(tokenizer)
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

        let mode = LinderaCoreMode::from(self.mode.clone());

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
                detail: self.word_detail(word_id)?,
            })
        }

        Ok(())
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
    pub fn tokenize<'a>(&self, mut text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut lattice = Lattice::default();
        let mut tokens = Vec::new();
        while let Some(split_idx) = text.find(|c| c == '。' || c == '、') {
            self.tokenize_without_split(&text[..split_idx + 3], &mut tokens, &mut lattice)?;
            text = &text[split_idx + 3..];
        }
        if !text.is_empty() {
            self.tokenize_without_split(text, &mut tokens, &mut lattice)?;
        }

        Ok(tokens)
    }

    /// Tokenize the text
    ///
    /// # Arguments
    ///
    /// * `text`: Japanese text
    ///
    /// returns: Result<Vec<&str>, LinderaError>
    ///
    /// * Vec<Token> : the list of `words` if succeeded
    /// * LinderaError : Error message with LinderaErrorKind
    ///
    pub fn tokenize_str<'a>(&self, text: &'a str) -> LinderaResult<Vec<&'a str>> {
        let tokens = self
            .tokenize(text)?
            .into_iter()
            .map(|token| token.text)
            .collect();

        Ok(tokens)
    }
}

#[cfg(test)]
#[cfg(feature = "ipadic")]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::PathBuf;

    use lindera_core::word_entry::WordId;

    use crate::mode::{Mode, Penalty};
    use crate::tokenizer::{Token, Tokenizer, TokenizerConfig, UserDictionaryType};

    #[test]
    fn test_from_bytes_normal() {
        let json_str = r#"
        {
            "dict_type": "ipadic",
            "mode": "normal"
        }
        "#;
        let json = json_str.as_bytes();

        let args = serde_json::from_slice::<TokenizerConfig>(json).unwrap();
        assert_eq!(args.dict_type, DictionaryType::Ipadic);
        assert_eq!(args.mode, Mode::Normal);
    }

    #[test]
    fn test_from_bytes_decompose() {
        let json_str = r#"
        {
            "dict_type": "ipadic",
            "dict_path": null,
            "user_dict_type": "csv",
            "user_dict_path": "./resources/user_dict.csv",
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
        assert_eq!(args.dict_type, DictionaryType::Ipadic);
        assert_eq!(args.dict_path, None);
        assert_eq!(args.user_dict_type, UserDictionaryType::Csv);
        assert_eq!(
            args.user_dict_path,
            Some(PathBuf::from("./resources/user_dict.csv"))
        );
        assert_eq!(args.mode, Mode::Decompose(Penalty::default()));
    }

    #[test]
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
        let tokens: Vec<&str> = tokenizer.tokenize_str("すもももももももものうち").unwrap();
        assert_eq!(
            tokens,
            vec!["すもも", "も", "もも", "も", "もも", "の", "うち"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_gyoi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("御意。 御意〜。").unwrap();
        assert_eq!(tokens, vec!["御意", "。", " ", "御意", "〜", "。"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_demoyorokobi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("〜でも喜び").unwrap();
        assert_eq!(tokens, vec!["〜", "でも", "喜び"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mukigen_normal2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("—でも").unwrap();
        assert_eq!(tokens, vec!["—", "でも"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_atodedenwa() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("後で").unwrap();
        assert_eq!(tokens, vec!["後で"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_ikkagetsu() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ーヶ月").unwrap();
        assert_eq!(tokens, vec!["ーヶ", "月"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mukigen_normal() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("無期限に—でもどの種を?").unwrap();
        assert_eq!(
            tokens,
            vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]
        );
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_demo() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("――!!?").unwrap();
        assert_eq!(tokens, vec!["――!!?"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_kaikeishi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ジム・コガン").unwrap();
        assert_eq!(tokens, vec!["ジム・コガン"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_bruce() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ブルース・モラン").unwrap();
        assert_eq!(tokens, vec!["ブルース・モラン"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_real() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer
            .tokenize_str(
                "本項で解説する地方病とは、山梨県における日本住血吸虫症の呼称であり、\
             長い間その原因が明らかにならず住民を苦しめた感染症である。",
            )
            .unwrap();
        assert_eq!(
            tokens,
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
        let tokens: Vec<&str> = tokenizer.tokenize_str("満々!").unwrap();
        assert_eq!(tokens, &["満々", "!"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_short() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("日本住").unwrap();
        assert_eq!(tokens, vec!["日本", "住"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_short2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ここでは").unwrap();
        assert_eq!(tokens, vec!["ここ", "で", "は"]);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_simple_user_dict() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("userdic.csv");

        let config = TokenizerConfig {
            user_dict_path: Some(userdic_file),
            mode: Mode::Normal,
            ..TokenizerConfig::default()
        };
        let tokenizer = Tokenizer::with_config(config).unwrap();
        assert!(tokenizer.user_dictionary.is_some());
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
            tokens[0].detail
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
            .join("detailed_userdic.csv");

        let config = TokenizerConfig {
            user_dict_path: Some(userdic_file),
            user_dict_type: UserDictionaryType::Csv,
            mode: Mode::Normal,
            ..TokenizerConfig::default()
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
            tokens[0].detail
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
            .join("mixed_userdic.csv");

        let config = TokenizerConfig {
            user_dict_path: Some(userdic_file),
            mode: Mode::Normal,
            ..TokenizerConfig::default()
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
            tokens[0].detail
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
            tokens[4].detail
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
    #[should_panic(expected = "failed to parse word_cost")]
    fn test_user_dict_invalid_word_cost() {
        let config = TokenizerConfig {
            user_dict_path: Some(PathBuf::from("test/fixtures/userdic_invalid_word_cost.csv")),
            mode: Mode::Normal,
            ..TokenizerConfig::default()
        };
        Tokenizer::with_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13 fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let config = TokenizerConfig {
            user_dict_path: Some(PathBuf::from(
                "test/fixtures/userdic_insufficient_number_of_fields.csv",
            )),
            mode: Mode::Normal,
            ..TokenizerConfig::default()
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
