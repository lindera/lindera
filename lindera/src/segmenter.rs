use std::borrow::Cow;
use std::str::FromStr;

use lindera_dictionary::mode::Mode;

use lindera_dictionary::dictionary::character_definition::CategoryId;
use lindera_dictionary::dictionary::{Dictionary, UserDictionary};
use lindera_dictionary::viterbi::Lattice;
use serde_json::Value;

use crate::LinderaResult;
use crate::dictionary::{load_dictionary, load_user_dictionary};
use crate::error::LinderaErrorKind;
use crate::token::Token;

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

    /// Keep whitespace tokens in output.
    ///
    /// When false (default), whitespace is ignored for MeCab compatibility.
    /// When true, whitespace tokens are included in the output.
    pub keep_whitespace: bool,

    /// The category ID for space characters, used when keep_whitespace is false.
    space_category_id: Option<CategoryId>,
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
        // Get SPACE category ID for MeCab compatibility (ignore whitespace by default)
        let space_category_id = dictionary.character_definition.category_id_by_name("SPACE");

        Self {
            mode,
            dictionary,
            user_dictionary,
            keep_whitespace: false, // Default: ignore whitespace for MeCab compatibility
            space_category_id,
        }
    }

    /// Builder method to set whether to keep whitespace tokens in output.
    ///
    /// When `keep_whitespace` is false (default), whitespace is ignored for MeCab compatibility.
    /// When true, whitespace tokens are included in the output.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included in the output.
    ///
    /// # Example
    ///
    /// ```
    /// use lindera::mode::Mode;
    /// use lindera::dictionary::load_dictionary;
    /// use lindera::segmenter::Segmenter;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "embed-ipadic")]
    /// # {
    /// let dictionary = load_dictionary("embedded://ipadic")?;
    /// let segmenter = Segmenter::new(Mode::Normal, dictionary, None)
    ///     .keep_whitespace(true);
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn keep_whitespace(mut self, keep_whitespace: bool) -> Self {
        self.keep_whitespace = keep_whitespace;
        self
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
        let dictionary = load_dictionary(
            config
                .get("dictionary")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("dictionary field is missing"))
                })?,
        )?;

        // Get metadata from the dictionary
        let metadata = &dictionary.metadata;

        // Load the user dictionary from the config
        let user_dictionary_uri = config
            .get("user_dictionary")
            .and_then(Value::as_str)
            .map(String::from);

        let user_dictionary = match user_dictionary_uri {
            Some(uri) => Some(load_user_dictionary(&uri, metadata)?),
            None => None,
        };

        // Load the mode from the config
        let mode: Mode = config.get("mode").map_or_else(
            || Ok(Mode::Normal),
            |v| {
                if let Some(s) = v.as_str() {
                    Mode::from_str(s).map_err(|e| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("mode field is invalid string: {e}"))
                    })
                } else {
                    serde_json::from_value::<Mode>(v.clone()).map_err(|e| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("mode field is invalid object: {e}"))
                    })
                }
            },
        )?;

        // Load the keep_whitespace option from the config
        // Default is false (MeCab compatible - ignore whitespace)
        // Set to true explicitly to include whitespace tokens
        let keep_whitespace = config
            .get("keep_whitespace")
            .and_then(Value::as_bool)
            .unwrap_or(false); // Default: false (ignore whitespace)

        // Get the SPACE category ID if whitespace should be ignored
        let space_category_id = if !keep_whitespace {
            dictionary
                .character_definition
                .category_id_by_name("SPACE")
                .ok_or_else(|| {
                    LinderaErrorKind::Parse.with_error(anyhow::anyhow!(
                        "SPACE category is not defined in the dictionary (char.def)"
                    ))
                })?;
            Some(
                dictionary
                    .character_definition
                    .category_id_by_name("SPACE")
                    .unwrap(),
            )
        } else {
            None
        };

        Ok(Self {
            mode,
            dictionary,
            user_dictionary,
            keep_whitespace,
            space_category_id,
        })
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
        let mut lattice = Lattice::default();
        self.segment_with_lattice(text, &mut lattice)
    }

    /// Segments the input text into tokens based on the dictionary and user-defined rules.
    ///
    /// # Arguments
    ///
    /// * `text` - A `Cow<'a, str>` representing the input text. This can either be borrowed or owned, allowing for efficient text handling depending on the use case.
    /// * `lattice` - A mutable reference to a `Lattice` structure. This allows reusing the lattice across multiple calls to avoid memory allocation.
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
    pub fn segment_with_lattice<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
    ) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();

        let mut position = 0_usize;
        let mut byte_position = 0_usize;

        // Process whole text without splitting first for better performance with borrowed text
        let text_bytes = text.as_bytes();
        let text_len = text.len();
        let mut sentence_start = 0;

        while sentence_start < text_len {
            // Find the end of the current sentence
            let mut sentence_end = sentence_start;
            while sentence_end < text_len {
                let ch = text_bytes[sentence_end];
                sentence_end += 1;
                // Check for sentence delimiters
                if ch == b'\n' || ch == b'\t' {
                    break;
                }
                // Check for Japanese punctuation (multi-byte)
                if sentence_end >= 3 && sentence_end <= text_len {
                    let last_3 = &text_bytes[sentence_end - 3..sentence_end];
                    if last_3 == "。".as_bytes() || last_3 == "、".as_bytes() {
                        break;
                    }
                }
            }

            let sentence = &text[sentence_start..sentence_end];
            if sentence.is_empty() {
                sentence_start = sentence_end;
                continue;
            }

            // Process the sentence through lattice
            lattice.set_text(
                &self.dictionary.prefix_dictionary,
                &self.user_dictionary.as_ref().map(|d| &d.dict),
                &self.dictionary.character_definition,
                &self.dictionary.unknown_dictionary,
                &self.dictionary.connection_cost_matrix,
                sentence,
                &self.mode,
            );
            // Forward Viterbi implementation handles cost calculation within `set_text`.

            let offsets = lattice.tokens_offset();

            for i in 0..offsets.len() {
                let (byte_start, word_id) = offsets[i];
                let byte_end = if i == offsets.len() - 1 {
                    sentence.len()
                } else {
                    let (next_start, _word_id) = offsets[i + 1];
                    next_start
                };

                // Calculate absolute position in the original text
                let absolute_start = sentence_start + byte_start;
                let absolute_end = sentence_start + byte_end;

                // Skip whitespace tokens if keep_whitespace is false (default MeCab behavior)
                if !self.keep_whitespace
                    && let Some(space_category_id) = self.space_category_id
                {
                    // Check if this token consists only of whitespace characters
                    let token_text = &sentence[byte_start..byte_end];
                    let is_space = token_text.chars().all(|c| {
                        self.dictionary
                            .character_definition
                            .lookup_categories(c)
                            .contains(&space_category_id)
                    });

                    if is_space {
                        // Update byte_position to maintain correct offsets
                        byte_position += byte_end - byte_start;
                        continue;
                    }
                }

                // Create surface Cow efficiently - avoid unnecessary string allocation for owned strings
                let surface_cow = match &text {
                    Cow::Borrowed(s) => Cow::Borrowed(&s[absolute_start..absolute_end]),
                    Cow::Owned(s) => {
                        // Use slice from owned string instead of creating new string
                        Cow::Owned(s[absolute_start..absolute_end].to_owned())
                    }
                };

                // compute the token's absolute byte positions
                let token_start = byte_position;
                byte_position += byte_end - byte_start;
                let token_end = byte_position;

                tokens.push(Token::new(
                    surface_cow,
                    token_start,
                    token_end,
                    position,
                    word_id,
                    &self.dictionary,
                    self.user_dictionary.as_ref(),
                ));

                position += 1;
            }

            sentence_start = sentence_end;
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    #[cfg(any(
        feature = "embed-ipadic",
        feature = "embed-ipadic-neologd",
        feature = "embed-unidic",
        feature = "embed-ko-dic",
        feature = "embed-cc-cedict"
    ))]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    // use crate::mode::{Mode, Penalty};

    #[cfg(any(
        feature = "embed-ipadic",
        feature = "embed-unidic",
        feature = "embed-ko-dic",
        feature = "embed-cc-cedict"
    ))]
    use crate::segmenter::{Segmenter, SegmenterConfig};

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segmenter_config_ipadic_normal() {
        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;

        let result: Result<SegmenterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segmenter_config_ipadic_decompose() {
        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
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
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_ipadic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
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
            assert_eq!(token.surface, "日本語");
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
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "形態素");
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
            assert_eq!(token.surface, "解析");
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
            assert_eq!(token.surface, "を");
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
            assert_eq!(token.surface, "行う");
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
            assert_eq!(token.surface, "こと");
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
            assert_eq!(token.surface, "が");
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
            assert_eq!(token.surface, "でき");
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
            assert_eq!(token.surface, "ます");
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
            assert_eq!(token.surface, "。");
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
            assert_eq!(token.surface, "テスト");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-unidic")]
    fn test_segment_unidic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://unidic",
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
            assert_eq!(token.surface, "日本");
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
            assert_eq!(token.surface, "語");
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
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "形態");
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
            assert_eq!(token.surface, "素");
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
            assert_eq!(token.surface, "解析");
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
            assert_eq!(token.surface, "を");
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
            assert_eq!(token.surface, "行う");
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
            assert_eq!(token.surface, "こと");
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
            assert_eq!(token.surface, "が");
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
            assert_eq!(token.surface, "でき");
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
            assert_eq!(token.surface, "ます");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-ko-dic")]
    fn test_segment_ko_dic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ko-dic",
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
            assert_eq!(token.surface, "한국어");
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
            assert_eq!(token.surface, "의");
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
            assert_eq!(token.surface, "형태");
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
            assert_eq!(token.surface, "해석");
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
            assert_eq!(token.surface, "을");
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
            assert_eq!(token.surface, "실시");
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
            assert_eq!(token.surface, "할");
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
            assert_eq!(token.surface, "수");
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
            assert_eq!(token.surface, "있");
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
            assert_eq!(token.surface, "습니다");
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
            assert_eq!(token.surface, ".");
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
    #[cfg(feature = "embed-cc-cedict")]
    fn test_segment_cc_cedict() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://cc-cedict",
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
            assert_eq!(token.surface, "可以");
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
            assert_eq!(token.surface, "进行");
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
            assert_eq!(token.surface, "中文");
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
            assert_eq!(token.surface, "形态学");
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
            assert_eq!(token.surface, "分析");
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_simple_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
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
            assert_eq!(token.surface, "東京スカイツリー");
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
                    "*",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "最寄り駅");
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
            assert_eq!(token.surface, "は");
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
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
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
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "です");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-unidic")]
    fn test_segment_with_simple_userdic_unidic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("unidic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://unidic",
            "user_dictionary": userdic_file.to_str().unwrap(),
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
            assert_eq!(token.surface, "東京スカイツリー");
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
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "最寄り");
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
            assert_eq!(token.surface, "駅");
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
            assert_eq!(token.surface, "は");
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
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
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
            assert_eq!(token.surface, "です");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-ko-dic")]
    fn test_segment_with_simple_userdic_ko_dic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ko-dic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ko-dic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("하네다공항한정토트백."))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "하네다공항");
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
            assert_eq!(token.surface, "한정");
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
            assert_eq!(token.surface, "토트백");
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
            assert_eq!(token.surface, ".");
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
    #[cfg(feature = "embed-cc-cedict")]
    fn test_segment_with_simple_userdic_cc_cedict() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("cc-cedict_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://cc-cedict",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田机场限定托特包。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田机场");
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
            assert_eq!(token.surface, "限定");
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
            assert_eq!(token.surface, "托特");
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
            assert_eq!(token.surface, "包");
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_simple_userdic_bin_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
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
            assert_eq!(token.surface, "東京スカイツリー");
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
                    "*",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "最寄り駅");
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
            assert_eq!(token.surface, "は");
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
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
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
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "です");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-unidic")]
    fn test_segment_with_simple_userdic_bin_unidic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("unidic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": "embedded://unidic",
            "user_dictionary": userdic_file.to_str().unwrap(),
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
            assert_eq!(token.surface, "東京スカイツリー");
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
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "最寄り");
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
            assert_eq!(token.surface, "駅");
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
            assert_eq!(token.surface, "は");
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
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
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
            assert_eq!(token.surface, "です");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-ko-dic")]
    fn test_segment_with_simple_userdic_bin_ko_dic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ko-dic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": "embedded://ko-dic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("하네다공항한정토트백."))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "하네다공항");
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
            assert_eq!(token.surface, "한정");
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
            assert_eq!(token.surface, "토트백");
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
            assert_eq!(token.surface, ".");
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
    #[cfg(feature = "embed-cc-cedict")]
    fn test_segment_with_simple_userdic_bin_cc_cedict() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("cc-cedict_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": "embedded://cc-cedict",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田机场限定托特包。"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田机场");
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
            assert_eq!(token.surface, "限定");
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
            assert_eq!(token.surface, "托特");
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
            assert_eq!(token.surface, "包");
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_detailed_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_detailed_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
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
            assert_eq!(token.surface, "東京スカイツリー");
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
            assert_eq!(token.surface, "の");
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
            assert_eq!(token.surface, "最寄り駅");
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
            assert_eq!(token.surface, "は");
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
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
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
            assert_eq!(token.surface, "です");
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
            assert_eq!(token.surface, "。");
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
    #[cfg(feature = "embed-ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_nomal_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田空港");
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
            assert_eq!(token.surface, "限定");
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_decompose_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
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
            assert_eq!(token.surface, "羽田");
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
            assert_eq!(token.surface, "空港");
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
            assert_eq!(token.surface, "限定");
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_decompose_mode_default_penalty() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "decompose"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();

        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田");
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
            assert_eq!(token.surface, "空港");
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
            assert_eq!(token.surface, "限定");
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.details(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_ignores_space() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // Default behavior: should have 2 tokens, space is ignored (MeCab compatible)
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_keep_whitespace() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal",
            "keep_whitespace": true
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // With keep_whitespace=true: should have 3 tokens including space
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, " ");
        assert_eq!(tokens[2].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_builder_keep_whitespace() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // With keep_whitespace=true: should have 3 tokens including space
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, " ");
        assert_eq!(tokens[2].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_multiple_spaces() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京   都")).unwrap();

        // Should have 2 tokens: "東京" and "都", multiple spaces are ignored
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_leading_trailing() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();

        // Leading spaces - "   東京都" is segmented as "東京" and "都" (not "東京都")
        let tokens = segmenter.segment(Cow::Borrowed("   東京都")).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");

        // Trailing spaces - "東京都   " is also segmented as "東京" and "都"
        let tokens = segmenter.segment(Cow::Borrowed("東京都   ")).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
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
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();

        let tokens = segmenter
            .segment(Cow::Borrowed(large_text.as_str()))
            .unwrap();
        assert!(!tokens.is_empty());
    }
}
