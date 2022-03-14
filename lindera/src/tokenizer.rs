use std::path::PathBuf;
use std::str::FromStr;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use serde::Serialize;

#[cfg(feature = "cc-cedict")]
use lindera_cc_cedict_builder::cc_cedict_builder::CedictBuilder;
use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
#[cfg(any(
    feature = "ipadic",
    feature = "unidic",
    feature = "ko-dic",
    feature = "cc-cedict"
))]
use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::file_util::read_file;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::viterbi::{Lattice, Mode as LinderaCoreMode};
use lindera_core::word_entry::WordId;
#[cfg(feature = "ipadic")]
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
#[cfg(feature = "ko-dic")]
use lindera_ko_dic_builder::ko_dic_builder::KodicBuilder;
#[cfg(feature = "unidic")]
use lindera_unidic_builder::unidic_builder::UnidicBuilder;

use crate::error::LinderaErrorKind;
use crate::mode::Mode;
use crate::LinderaResult;

#[derive(Debug, Clone)]
pub enum DictionaryType {
    #[cfg(feature = "ipadic")]
    Ipadic,
    #[cfg(feature = "unidic")]
    Unidic,
    #[cfg(feature = "ko-dic")]
    Kodic,
    #[cfg(feature = "cc-cedict")]
    Cedict,
    LocalDictionary,
}

impl FromStr for DictionaryType {
    type Err = ();
    fn from_str(input: &str) -> Result<DictionaryType, Self::Err> {
        match input {
            #[cfg(feature = "ipadic")]
            "ipadic" => Ok(DictionaryType::Ipadic),
            #[cfg(feature = "unidic")]
            "unidic" => Ok(DictionaryType::Unidic),
            #[cfg(feature = "ko-dic")]
            "ko-dic" => Ok(DictionaryType::Kodic),
            #[cfg(feature = "cc-cedict")]
            "cc-cedict" => Ok(DictionaryType::Cedict),
            "local" => Ok(DictionaryType::LocalDictionary),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UserDictionaryType {
    Csv,
    Binary,
}

impl FromStr for UserDictionaryType {
    type Err = ();
    fn from_str(input: &str) -> Result<UserDictionaryType, Self::Err> {
        match input {
            "csv" => Ok(UserDictionaryType::Csv),
            "bin" => Ok(UserDictionaryType::Binary),
            _ => Err(()),
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
            DictionaryType::Kodic => {
                let builder = KodicBuilder::new();
                builder
                    .build_user_dict(&path)
                    .map_err(|e| LinderaErrorKind::DictionaryBuildError.with_error(e))
            }
            #[cfg(feature = "cc-cedict")]
            DictionaryType::Cedict => {
                let builder = CedictBuilder::new();
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
    dict: PrefixDict<Vec<u8>>,
    cost_matrix: ConnectionCostMatrix,
    char_definitions: CharacterDefinitions,
    unknown_dictionary: UnknownDictionary,
    words_idx_data: Vec<u8>,
    words_data: Vec<u8>,
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
        let dict;
        let cost_matrix;
        let char_definitions;
        let unknown_dictionary;
        let words_idx_data;
        let words_data;
        match config.dict_type {
            DictionaryType::LocalDictionary => match config.dict_path.clone() {
                Some(path) => {
                    dict = lindera_dictionary::prefix_dict(path.clone())
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                    cost_matrix = lindera_dictionary::connection(path.clone())
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                    char_definitions = lindera_dictionary::char_def(path.clone())
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                    unknown_dictionary = lindera_dictionary::unknown_dict(path.clone())
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                    words_idx_data = lindera_dictionary::words_idx_data(path.clone())
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                    words_data = lindera_dictionary::words_data(path)
                        .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                }
                None => {
                    return Err(LinderaErrorKind::DictionaryNotFound
                        .with_error(anyhow::anyhow!("dictionary path is not set.")));
                }
            },
            #[cfg(feature = "ipadic")]
            DictionaryType::Ipadic => {
                dict = lindera_ipadic::prefix_dict();
                cost_matrix = lindera_ipadic::connection();
                char_definitions = lindera_ipadic::char_def()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                unknown_dictionary = lindera_ipadic::unknown_dict()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                words_idx_data = lindera_ipadic::words_idx_data();
                words_data = lindera_ipadic::words_data();
            }
            #[cfg(feature = "unidic")]
            DictionaryType::Unidic => {
                dict = lindera_unidic::prefix_dict();
                cost_matrix = lindera_unidic::connection();
                char_definitions = lindera_unidic::char_def()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                unknown_dictionary = lindera_unidic::unknown_dict()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                words_idx_data = lindera_unidic::words_idx_data();
                words_data = lindera_unidic::words_data();
            }
            #[cfg(feature = "ko-dic")]
            DictionaryType::Kodic => {
                dict = lindera_ko_dic::prefix_dict();
                cost_matrix = lindera_ko_dic::connection();
                char_definitions = lindera_ko_dic::char_def()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                unknown_dictionary = lindera_ko_dic::unknown_dict()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                words_idx_data = lindera_ko_dic::words_idx_data();
                words_data = lindera_ko_dic::words_data();
            }
            #[cfg(feature = "cc-cedict")]
            DictionaryType::Cedict => {
                dict = lindera_cc_cedict::prefix_dict();
                cost_matrix = lindera_cc_cedict::connection();
                char_definitions = lindera_cc_cedict::char_def()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                unknown_dictionary = lindera_cc_cedict::unknown_dict()
                    .map_err(|err| LinderaErrorKind::DictionaryLoadError.with_error(err))?;
                words_idx_data = lindera_cc_cedict::words_idx_data();
                words_data = lindera_cc_cedict::words_data();
            }
        }

        let user_dictionary = match config.user_dict_path {
            Some(path) => Some(build_user_dict(
                config.dict_type,
                path,
                config.user_dict_type,
            )?),
            None => None,
        };

        let tokenizer = Tokenizer {
            dict,
            cost_matrix,
            char_definitions,
            unknown_dictionary,
            words_idx_data,
            words_data,
            mode: config.mode,
            user_dictionary,
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
            &self.dict,
            &self.user_dictionary.as_ref().map(|d| &d.dict),
            &self.char_definitions,
            &self.unknown_dictionary,
            text,
            &mode,
        );
        lattice.calculate_path_costs(&self.cost_matrix, &mode);
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
            (self.words_idx_data.as_slice(), self.words_data.as_slice())
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
    fn test_tokenize_sumomomomo() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("すもももももももものうち").unwrap();
        assert_eq!(
            tokens,
            vec!["すもも", "も", "もも", "も", "もも", "の", "うち"]
        );
    }

    #[test]
    fn test_gyoi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("御意。 御意〜。").unwrap();
        assert_eq!(tokens, vec!["御意", "。", " ", "御意", "〜", "。"]);
    }

    #[test]
    fn test_demoyorokobi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("〜でも喜び").unwrap();
        assert_eq!(tokens, vec!["〜", "でも", "喜び"]);
    }

    #[test]
    fn test_mukigen_normal2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("—でも").unwrap();
        assert_eq!(tokens, vec!["—", "でも"]);
    }

    #[test]
    fn test_atodedenwa() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("後で").unwrap();
        assert_eq!(tokens, vec!["後で"]);
    }

    #[test]
    fn test_ikkagetsu() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ーヶ月").unwrap();
        assert_eq!(tokens, vec!["ーヶ", "月"]);
    }

    #[test]
    fn test_mukigen_normal() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("無期限に—でもどの種を?").unwrap();
        assert_eq!(
            tokens,
            vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]
        );
    }

    #[test]
    fn test_demo() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("――!!?").unwrap();
        assert_eq!(tokens, vec!["――!!?"]);
    }

    #[test]
    fn test_kaikeishi() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ジム・コガン").unwrap();
        assert_eq!(tokens, vec!["ジム・コガン"]);
    }

    #[test]
    fn test_bruce() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ブルース・モラン").unwrap();
        assert_eq!(tokens, vec!["ブルース・モラン"]);
    }

    #[test]
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
    fn test_hitobito() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("満々!").unwrap();
        assert_eq!(tokens, &["満々", "!"]);
    }

    #[test]
    fn test_tokenize_short() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("日本住").unwrap();
        assert_eq!(tokens, vec!["日本", "住"]);
    }

    #[test]
    fn test_tokenize_short2() {
        let tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ここでは").unwrap();
        assert_eq!(tokens, vec!["ここ", "で", "は"]);
    }

    #[test]
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
