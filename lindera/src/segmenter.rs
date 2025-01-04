use std::borrow::Cow;

use lindera_dictionary::mode::Mode;

use lindera_dictionary::dictionary::{Dictionary, UserDictionary};
use lindera_dictionary::viterbi::Lattice;
use serde_json::Value;

use crate::dictionary::{load_dictionary_from_config, load_user_dictionary_from_config};
use crate::error::LinderaErrorKind;
use crate::token::Token;
use crate::LinderaResult;

pub type SegmenterConfig = Value;

/// Segmenter
#[derive(Clone)]
pub struct Segmenter {
    /// The segmentation mode to be used by the segmenter.
    /// This determines how the text will be split into segments.
    pub mode: Mode,

    /// The dictionary used for segmenting text. This dictionary contains the necessary
    /// data structures and algorithms to perform morphological analysis and tokenization.
    pub dictionary: Dictionary,

    /// An optional user-defined dictionary that can be used to customize the segmentation process.
    /// If provided, this dictionary will be used in addition to the default dictionary to improve
    /// the accuracy of segmentation for specific words or phrases.
    pub user_dictionary: Option<UserDictionary>,
}

impl Segmenter {
    /// Creates a new instance with the specified mode, dictionary, and optional user dictionary.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `Mode` in which the instance will operate. This typically defines how aggressively the text is segmented or processed.
    /// * `dictionary` - A `Dictionary` object that provides the core data and rules for processing text.
    /// * `user_dictionary` - An optional `UserDictionary` that allows for additional, user-defined tokens or rules to be used in conjunction with the main dictionary.
    ///
    /// # Returns
    ///
    /// Returns a new instance of the struct with the provided mode, dictionary, and user dictionary (if any).
    ///
    /// # Details
    ///
    /// - `mode`: This defines the behavior of the instance, such as whether to process text in normal or aggressive mode.
    /// - `dictionary`: The main dictionary containing tokenization or processing rules.
    /// - `user_dictionary`: This is optional. If provided, it allows the user to extend or override the rules of the main dictionary with custom tokens.
    pub fn new(
        mode: Mode,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        Self {
            mode,
            dictionary,
            user_dictionary,
        }
    }

    /// A struct representing a segmenter for tokenizing text.
    ///
    /// The `Segmenter` struct provides methods for creating a segmenter from a configuration,
    /// creating a new segmenter, and segmenting text into tokens.
    ///
    /// # Methods
    ///
    /// - `from_config`: Creates a `Segmenter` from a given configuration.
    /// - `new`: Creates a new `Segmenter` with the specified mode, dictionary, and optional user dictionary.
    /// - `segment`: Segments the given text into tokens.
    ///
    /// # Errors
    ///
    /// Methods that return `LinderaResult` may produce errors related to dictionary loading,
    /// user dictionary loading, or tokenization process.
    pub fn from_config(config: &SegmenterConfig) -> LinderaResult<Self> {
        // Load the dictionary from the config
        let dictionary =
            load_dictionary_from_config(config.get("dictionary").ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("dictionary field is missing"))
            })?)?;

        // Load the user dictionary from the config
        let user_dictionary = config
            .get("user_dictionary")
            .map(load_user_dictionary_from_config)
            .transpose()?;

        // Load the mode from the config
        let mode: Mode = config.get("mode").map_or_else(
            || Ok(Mode::Normal),
            |v| {
                serde_json::from_value(v.clone()).map_err(|e| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("mode field is invalid: {}", e))
                })
            },
        )?;

        Ok(Self::new(mode, dictionary, user_dictionary))
    }

    /// Segments the input text into tokens based on the dictionary and user-defined rules.
    ///
    /// # Arguments
    ///
    /// * `text` - A `Cow<'a, str>` representing the input text. This can either be borrowed or owned, allowing for efficient text handling depending on the use case.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<Vec<Token<'a>>>` which contains a vector of tokens segmented from the input text. Each token represents a portion of the original text, along with metadata such as byte offsets and dictionary information.
    ///
    /// # Process
    ///
    /// 1. **Sentence Splitting**:
    ///    - The input text is split into sentences using Japanese punctuation (`。`, `、`, `\n`, `\t`). Each sentence is processed individually.
    ///
    /// 2. **Lattice Processing**:
    ///    - For each sentence, a lattice structure is set up using the main dictionary and, if available, the user dictionary. The lattice helps identify possible token boundaries within the sentence.
    ///    - The cost matrix is used to calculate the best path (i.e., the optimal sequence of tokens) through the lattice based on the mode.
    ///
    /// 3. **Token Generation**:
    ///    - For each segment (determined by the lattice), a token is generated using the byte offsets. The tokens contain the original text (in `Cow::Owned` form to ensure safe return), byte start/end positions, token positions, and dictionary references.
    ///
    /// # Notes
    ///
    /// - The function ensures that each token is safely returned by converting substrings into `Cow::Owned` strings.
    /// - Byte offsets are carefully calculated to ensure that token boundaries are correct even across multiple sentences.
    ///
    /// # Example Flow
    ///
    /// - Text is split into sentences based on punctuation.
    /// - A lattice is created and processed for each sentence.
    /// - Tokens are extracted from the lattice and returned in a vector.
    ///
    /// # Errors
    ///
    /// - If the lattice fails to be processed or if there is an issue with the segmentation process, the function returns an error.
    pub fn segment<'a>(&'a self, text: Cow<'a, str>) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut lattice = Lattice::default();

        let mut position = 0_usize;
        let mut byte_position = 0_usize;

        // Split text into sentences using Japanese punctuation.
        for sentence in text.split_inclusive(&['。', '、', '\n', '\t']) {
            if sentence.is_empty() {
                continue;
            }

            lattice.set_text(
                &self.dictionary.prefix_dictionary,
                &self.user_dictionary.as_ref().map(|d| &d.dict),
                &self.dictionary.character_definition,
                &self.dictionary.unknown_dictionary,
                sentence,
                &self.mode,
            );
            lattice.calculate_path_costs(&self.dictionary.connection_cost_matrix, &self.mode);

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

                // Use Cow::Owned to ensure the token data can be returned safely
                tokens.push(Token::new(
                    Cow::Owned(surface.to_string()), // Clone the string here
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
    #[allow(unused_imports)]
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    #[allow(unused_imports)]
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use crate::mode::{Mode, Penalty};

    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use crate::segmenter::{Segmenter, SegmenterConfig};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segmenter_config_ipadic_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": "normal"
        }
        "#;

        let result: Result<SegmenterConfig, _> = serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segmenter_config_ipadic_decompose() {
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

        let result: Result<SegmenterConfig, _> = serde_json::from_str(config_str);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_ipadic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "日本語の形態素解析を行うことができます。テスト。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "日本語");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "形態素");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "を", "ヲ", "ヲ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "が", "ガ", "ガ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "テスト");
            assert_eq!(token.byte_start, 60);
            assert_eq!(token.byte_end, 69);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 69);
            assert_eq!(token.byte_end, 72);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_segment_unidic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": {
                "kind": "unidic"
            },
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("日本語の形態素解析を行うことができます。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "日本");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "語");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "形態");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "素");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
    fn test_segment_ko_dic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": {
                "kind": "ko-dic"
            },
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("한국어의형태해석을실시할수있습니다."))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한국어");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "의");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["JKG", "*", "F", "의", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "형태");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNG", "*", "F", "형태", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "해석");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNG", "행위", "T", "해석", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "을");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["JKO", "*", "T", "을", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "실시");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNG", "행위", "F", "실시", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "할");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "XSV+ETM",
                    "*",
                    "T",
                    "할",
                    "Inflect",
                    "XSV",
                    "ETM",
                    "하/XSV/*+ᆯ/ETM/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "수");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNB", "*", "F", "수", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "있");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["VV", "*", "T", "있", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "습니다");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["EF", "*", "F", "습니다", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 52);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_segment_cc_cedict() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": {
                "kind": "cc-cedict"
            },
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("可以进行中文形态学分析。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "可以");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "进行");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "中文");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "形态学");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "分析");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_with_simple_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_segment_with_simple_userdic_unidic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "unidic"
            },
            "user_dictionary": {
                "kind": "unidic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
    fn test_segment_with_simple_userdic_ko_dic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ko-dic"
            },
            "user_dictionary": {
                "kind": "ko-dic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("하네다공항한정토트백."))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_segment_with_simple_userdic_cc_cedict() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "cc-cedict"
            },
            "user_dictionary": {
                "kind": "cc-cedict",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田机场限定托特包。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_with_simple_userdic_bin_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_segment_with_simple_userdic_bin_unidic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "unidic"
            },
            "user_dictionary": {
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
    fn test_segment_with_simple_userdic_bin_ko_dic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ko-dic"
            },
            "user_dictionary": {
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("하네다공항한정토트백."))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_segment_with_simple_userdic_bin_cc_cedict() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "cc-cedict"
            },
            "user_dictionary": {
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田机场限定托特包。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_with_detailed_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_detailed_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mixed_user_dict() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_mixed_userdic.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },
            "user_dictionary": {
                "kind": "ipadic",
                "path": userdic_file.to_str().unwrap()
            },
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_with_nomal_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },

            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田空港");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_segment_with_decompose_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
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
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "空港");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.text, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_long_text() {
        use std::borrow::Cow;

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

        let config = serde_json::json!({
            "dictionary": {
                "kind": "ipadic"
            },

            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();

        let tokens = segmenter
            .segment(Cow::Borrowed(large_text.as_str()))
            .unwrap();
        assert!(!tokens.is_empty());
    }
}
