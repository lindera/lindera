use std::path::Path;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use serde::Serialize;

use lindera_core::character_definition::CharacterDefinitions;
use lindera_core::connection::ConnectionCostMatrix;
use lindera_core::error::LinderaErrorKind;
use lindera_core::prefix_dict::PrefixDict;
use lindera_core::unknown_dictionary::UnknownDictionary;
use lindera_core::viterbi::{Lattice, Mode};
use lindera_core::word_entry::WordId;
use lindera_core::LinderaResult;

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    pub text: &'a str,
    pub detail: Vec<String>,
}

pub struct TokenizerConfig<'a> {
    pub dict_path: Option<&'a Path>,
    pub user_dict_path: Option<&'a Path>,
    pub mode: Mode,
}

impl Default for TokenizerConfig<'_> {
    fn default() -> Self {
        Self {
            dict_path: None,
            user_dict_path: None,
            mode: Mode::Normal,
        }
    }
}

#[derive(Clone)]
pub struct Tokenizer {
    dict: PrefixDict<Vec<u8>>,
    cost_matrix: ConnectionCostMatrix,
    lattice: Lattice,
    char_definitions: CharacterDefinitions,
    unknown_dictionary: UnknownDictionary,
    words_idx_data: Vec<u8>,
    words_data: Vec<u8>,
    user_dict: Option<PrefixDict<Vec<u8>>>,
    user_dict_words_idx_data: Option<Vec<u8>>,
    user_dict_words_data: Option<Vec<u8>>,
    mode: Mode,
}

impl Tokenizer {
    pub fn new() -> LinderaResult<Tokenizer> {
        let config = TokenizerConfig::default();
        Tokenizer::with_config(config)
    }

    pub fn with_config(config: TokenizerConfig) -> LinderaResult<Tokenizer> {
        let dict = if let Some(ref path) = config.dict_path {
            lindera_dictionary::prefix_dict(path)?
        } else {
            lindera_ipadic::prefix_dict()
        };

        let cost_matrix = if let Some(path) = config.dict_path {
            lindera_dictionary::connection(path)?
        } else {
            lindera_ipadic::connection()
        };

        let char_definitions = if let Some(path) = config.dict_path {
            lindera_dictionary::char_def(&path)?
        } else {
            lindera_ipadic::char_def()?
        };

        let unknown_dictionary = if let Some(path) = config.dict_path {
            lindera_dictionary::unknown_dict(path)?
        } else {
            lindera_ipadic::unknown_dict()?
        };

        let words_idx_data = if let Some(path) = config.dict_path {
            lindera_dictionary::words_idx_data(path)?
        } else {
            lindera_ipadic::words_idx_data()
        };

        let words_data = if let Some(path) = config.dict_path {
            lindera_dictionary::words_data(path)?
        } else {
            lindera_ipadic::words_data()
        };

        let (user_dict, user_dict_words_idx_data, user_dict_words_data) =
            if let Some(path) = config.user_dict_path {
                let user_dict = lindera_ipadic_builder::builder::build_user_dict(path)?;
                (
                    Some(user_dict.dict),
                    Some(user_dict.words_idx_data),
                    Some(user_dict.words_data),
                )
            } else {
                (None, None, None)
            };

        let tokenizer = Tokenizer {
            dict,
            cost_matrix,
            lattice: Lattice::default(),
            char_definitions,
            unknown_dictionary,
            words_idx_data,
            words_data,
            user_dict,
            user_dict_words_idx_data,
            user_dict_words_data,
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
    pub(crate) fn tokenize_offsets(&mut self, text: &str) -> Vec<(usize, WordId)> {
        if text.is_empty() {
            return Vec::new();
        }
        self.lattice.set_text(
            &self.dict,
            &self.user_dict,
            &self.char_definitions,
            &self.unknown_dictionary,
            text,
            &self.mode,
        );
        self.lattice
            .calculate_path_costs(&self.cost_matrix, &self.mode);
        self.lattice.tokens_offset()
    }

    fn tokenize_without_split<'a>(
        &mut self,
        text: &'a str,
        tokens: &mut Vec<Token<'a>>,
    ) -> LinderaResult<()> {
        let offsets = self.tokenize_offsets(text);

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
                self.user_dict_words_idx_data
                    .as_ref()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("internal error."))
                    })?
                    .as_slice(),
                self.user_dict_words_data
                    .as_ref()
                    .ok_or_else(|| {
                        LinderaErrorKind::Content.with_error(anyhow::anyhow!("internal error."))
                    })?
                    .as_slice(),
            )
        };
        let idx = LittleEndian::read_u32(&words_idx_data[4 * word_id.0 as usize..][..4]);
        let data = &words_data[idx as usize..];
        let word_detail = bincode::deserialize_from(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))?;

        Ok(word_detail)
    }

    pub fn tokenize<'a>(&mut self, mut text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens = Vec::new();
        while let Some(split_idx) = text.find(|c| c == '。' || c == '、') {
            self.tokenize_without_split(&text[..split_idx + 3], &mut tokens)?;
            text = &text[split_idx + 3..];
        }
        if !text.is_empty() {
            self.tokenize_without_split(&text, &mut tokens)?;
        }

        Ok(tokens)
    }

    pub fn tokenize_str<'a>(&mut self, text: &'a str) -> LinderaResult<Vec<&'a str>> {
        let tokens = self
            .tokenize(text)?
            .into_iter()
            .map(|token| token.text)
            .collect();

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::Path;

    use lindera_core::viterbi::{Mode, Penalty};
    use lindera_core::word_entry::WordId;

    use crate::tokenizer::{Token, Tokenizer, TokenizerConfig};

    #[test]
    fn test_empty() {
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: None,
            mode: Mode::Decompose(Penalty::default()),
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets("");
        assert_eq!(tokens, &[]);
    }

    #[test]
    fn test_space() {
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: None,
            mode: Mode::Decompose(Penalty::default()),
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets(" ");
        assert_eq!(tokens, &[(0, WordId(4294967295, true))]);
    }

    #[test]
    fn test_boku_ha() {
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: None,
            mode: Mode::Decompose(Penalty::default()),
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        let tokens = tokenizer.tokenize_offsets("僕は");
        assert_eq!(
            tokens,
            &[(0, WordId(132630, true)), (3, WordId(57063, true))]
        );
    }

    #[test]
    fn test_tokenize_sumomomomo() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("すもももももももものうち").unwrap();
        assert_eq!(
            tokens,
            vec!["すもも", "も", "もも", "も", "もも", "の", "うち"]
        );
    }

    #[test]
    fn test_gyoi() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("御意。 御意〜。").unwrap();
        assert_eq!(tokens, vec!["御意", "。", " ", "御意", "〜", "。"]);
    }

    #[test]
    fn test_demoyorokobi() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("〜でも喜び").unwrap();
        assert_eq!(tokens, vec!["〜", "でも", "喜び"]);
    }

    #[test]
    fn test_mukigen_normal2() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("—でも").unwrap();
        assert_eq!(tokens, vec!["—", "でも"]);
    }

    #[test]
    fn test_atodedenwa() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("後で").unwrap();
        assert_eq!(tokens, vec!["後で"]);
    }

    #[test]
    fn test_ikkagetsu() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ーヶ月").unwrap();
        assert_eq!(tokens, vec!["ーヶ", "月"]);
    }

    #[test]
    fn test_mukigen_normal() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("無期限に—でもどの種を?").unwrap();
        assert_eq!(
            tokens,
            vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]
        );
    }

    #[test]
    fn test_demo() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("――!!?").unwrap();
        assert_eq!(tokens, vec!["――!!?"]);
    }

    #[test]
    fn test_kaikeishi() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ジム・コガン").unwrap();
        assert_eq!(tokens, vec!["ジム・コガン"]);
    }

    #[test]
    fn test_bruce() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ブルース・モラン").unwrap();
        assert_eq!(tokens, vec!["ブルース・モラン"]);
    }

    #[test]
    fn test_tokenize_real() {
        let mut tokenizer = Tokenizer::new().unwrap();
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
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("満々!").unwrap();
        assert_eq!(tokens, &["満々", "!"]);
    }

    #[test]
    fn test_tokenize_short() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("日本住").unwrap();
        assert_eq!(tokens, vec!["日本", "住"]);
    }

    #[test]
    fn test_tokenize_short2() {
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ここでは").unwrap();
        assert_eq!(tokens, vec!["ここ", "で", "は"]);
    }

    #[test]
    fn test_simple_user_dict() {
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: Some(&Path::new("resources/userdic.csv")),
            mode: Mode::Normal,
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        assert!(tokenizer.user_dict.is_some());
        assert!(tokenizer.user_dict_words_idx_data.is_some());
        assert!(tokenizer.user_dict_words_data.is_some());
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
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: Some(&Path::new("resources/detailed_userdic.csv")),
            mode: Mode::Normal,
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        assert!(tokenizer.user_dict.is_some());
        assert!(tokenizer.user_dict_words_idx_data.is_some());
        assert!(tokenizer.user_dict_words_data.is_some());
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
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: Some(&Path::new("resources/mixed_userdic.csv")),
            mode: Mode::Normal,
        };
        let mut tokenizer = Tokenizer::with_config(config).unwrap();
        assert!(tokenizer.user_dict.is_some());
        assert!(tokenizer.user_dict_words_idx_data.is_some());
        assert!(tokenizer.user_dict_words_data.is_some());

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
            dict_path: None,
            user_dict_path: Some(&Path::new("test/fixtures/userdic_invalid_word_cost.csv")),
            mode: Mode::Normal,
        };
        Tokenizer::with_config(config).unwrap();
    }

    #[test]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13 fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let config = TokenizerConfig {
            dict_path: None,
            user_dict_path: Some(&Path::new(
                "test/fixtures/userdic_insufficient_number_of_fields.csv",
            )),
            mode: Mode::Normal,
        };
        Tokenizer::with_config(config).unwrap();
    }

    #[test]
    fn test_long_text() {
        let mut large_file = BufReader::new(File::open("resources/bocchan.txt").unwrap());
        let mut large_text = String::new();
        let _size = large_file.read_to_string(&mut large_text).unwrap();
        let mut tokenizer = Tokenizer::new().unwrap();
        let tokens = tokenizer.tokenize(large_text.as_str()).unwrap();
        assert!(!tokens.is_empty());
    }
}
