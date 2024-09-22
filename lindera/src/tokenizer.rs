use std::borrow::Cow;

use lindera_core::dictionary::{Dictionary, UserDictionary};
use lindera_core::mode::Mode;
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::character_filter::{correct_offset, BoxCharacterFilter, CharacterFilterLoader};
use crate::segmenter::{Segmenter, SegmenterConfig};
use crate::token::Token;
use crate::token_filter::{BoxTokenFilter, TokenFilterLoader};

pub type TokenizerConfig = Value;

pub struct Tokenizer {
    /// Segmenter
    /// The `segmenter` field is an instance of the `Segmenter` struct, which is responsible for
    /// segmenting text into tokens. This is a core component of the tokenizer, enabling it to
    /// break down input text into manageable and meaningful units for further processing.
    pub segmenter: Segmenter,

    /// Character filters
    /// A vector of boxed character filters that will be applied to the input text
    /// before tokenization. Each character filter is responsible for transforming
    /// the input text in a specific way, such as normalizing characters or removing
    /// unwanted characters.
    pub character_filters: Vec<BoxCharacterFilter>,

    /// Token filters
    /// A vector of boxed token filters that will be applied to the tokens during tokenization.
    /// Each token filter is a boxed trait object implementing the `TokenFilter` trait, allowing
    /// for various transformations and processing steps to be applied to the tokens.
    pub token_filters: Vec<BoxTokenFilter>,
}

impl Tokenizer {
    /// Creates a `Tokenizer` instance from the provided JSON configuration (`TokenizerConfig`).
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to a `TokenizerConfig` (which is a `serde_json::Value`). This JSON object should include settings for the segmenter, character filters, and token filters.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<Self>`, which is an instance of the `Tokenizer` on success, or an error if the configuration is invalid or cannot be parsed.
    ///
    /// # Errors
    ///
    /// - Returns an error if:
    ///   - The `segmenter` configuration is missing or cannot be deserialized.
    ///   - The segmenter configuration in JSON is malformed or missing required fields.
    ///   - Any character or token filter configuration is missing or contains invalid settings.
    ///
    /// # Detailed Process
    ///
    /// 1. **Loading Segmenter Configuration**:
    ///    - The function extracts the `segmenter` section from the JSON `TokenizerConfig`.
    ///    - It converts the segmenter configuration to a byte array using `serde_json::to_vec`.
    ///    - The byte array is deserialized into a `SegmenterConfig` struct.
    ///    - A `Segmenter` is created using the deserialized segmenter configuration.
    ///
    /// 2. **Creating the Tokenizer**:
    ///    - A new `Tokenizer` instance is created from the segmenter.
    ///
    /// 3. **Loading Character Filters**:
    ///    - If the `character_filters` section in the `TokenizerConfig` is not empty, the function iterates through the array of character filter settings.
    ///    - For each filter, it extracts the `kind` and `args` to determine which filter to load and its arguments.
    ///    - The corresponding character filter is loaded and appended to the tokenizer.
    ///
    /// 4. **Loading Token Filters**:
    ///    - If the `token_filters` section in the `TokenizerConfig` is not empty, the function iterates through the array of token filter settings.
    ///    - Similar to character filters, it extracts the `kind` and `args` to load and append each token filter to the tokenizer.
    pub fn from_config(config: &TokenizerConfig) -> LinderaResult<Self> {
        // Load a JSON object for segmenter config from the tokenizer config.
        let args_value = config["segmenter"].as_object().ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing segmenter config."))
        })?;
        let arg_bytes = serde_json::to_vec(args_value)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        // Load a segmenter config from the segmenter config JSON object.
        let segmenter_config = serde_json::from_slice::<SegmenterConfig>(&arg_bytes)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        // Create a segmenter from the segmenter config.
        let segmenter = Segmenter::from_config(segmenter_config)?;

        // Create a tokenizer from the segmenter.
        let mut tokenizer = Tokenizer::from_segmenter(segmenter);

        // Load character filter settings from the tokenizer config if it is not empty.
        if let Some(character_filter_settings) = config["character_filters"].as_array() {
            for character_filter_setting in character_filter_settings {
                let character_filter_name = character_filter_setting["kind"].as_str();
                if let Some(character_filter_name) = character_filter_name {
                    // Append a character filter to the tokenizer.
                    tokenizer.append_character_filter(CharacterFilterLoader::load_from_value(
                        character_filter_name,
                        &character_filter_setting["args"],
                    )?);
                }
            }
        }

        // Load token filter settings from the tokenizer config if it is not empty.
        if let Some(token_filter_settings) = config["token_filters"].as_array() {
            for token_filter_setting in token_filter_settings {
                let token_filter_name = token_filter_setting["kind"].as_str();
                if let Some(token_filter_name) = token_filter_name {
                    // Append a token filter to the tokenizer.
                    tokenizer.append_token_filter(TokenFilterLoader::load_from_value(
                        token_filter_name,
                        &token_filter_setting["args"],
                    )?);
                }
            }
        }

        Ok(tokenizer)
    }

    /// Creates a new instance of `Tokenizer`.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `Mode` in which the tokenizer will operate. This typically defines how aggressively tokens are segmented (e.g., normal or aggressive mode).
    /// * `dictionary` - A `Dictionary` object that provides the tokenization rules and dictionary data.
    /// * `user_dictionary` - An optional `UserDictionary` that provides additional or custom tokenization rules. If `None`, only the main dictionary will be used.
    ///
    /// # Returns
    ///
    /// Returns a new `Tokenizer` instance that is configured using the provided `mode`, `dictionary`, and `user_dictionary`.
    pub fn new(
        mode: Mode,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        Tokenizer::from_segmenter(Segmenter::new(mode, dictionary, user_dictionary))
    }

    /// Creates a new `Tokenizer` instance from a provided `Segmenter`.
    ///
    /// # Arguments
    ///
    /// * `segmenter` - An instance of the `Segmenter` struct, which is responsible for the core tokenization process.
    ///
    /// # Returns
    ///
    /// Returns a new `Tokenizer` instance that uses the provided `segmenter` for tokenization, with empty character and token filters.
    ///
    /// # Details
    ///
    /// - `segmenter`: The segmenter is responsible for handling the actual segmentation and tokenization of text. It is passed into the `Tokenizer` during initialization.
    /// - `character_filters`: This is initialized as an empty vector and can be modified later to include character filters.
    /// - `token_filters`: This is also initialized as an empty vector and can be modified later to include token filters.
    pub fn from_segmenter(segmenter: Segmenter) -> Self {
        Self {
            segmenter,
            character_filters: Vec::new(),
            token_filters: Vec::new(),
        }
    }

    /// Appends a character filter to the tokenizer.
    ///
    /// # Arguments
    ///
    /// * `character_filter` - A `BoxCharacterFilter` that will be added to the tokenizer. This filter will be applied to the text during the tokenization process.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to `Self`, allowing for method chaining.
    ///
    /// # Details
    ///
    /// - This method adds a new character filter to the `Tokenizer`'s `character_filters` vector.
    /// - It returns a mutable reference to `self`, allowing multiple character filters to be appended in a chain of method calls.
    pub fn append_character_filter(&mut self, character_filter: BoxCharacterFilter) -> &mut Self {
        self.character_filters.push(character_filter);

        self
    }

    /// Appends a token filter to the tokenizer.
    ///
    /// # Arguments
    ///
    /// * `token_filter` - A `BoxTokenFilter` that will be added to the tokenizer. This filter will be applied to the tokens after they are segmented.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to `Self`, allowing for method chaining.
    ///
    /// # Details
    ///
    /// - This method adds a new token filter to the `Tokenizer`'s `token_filters` vector.
    /// - It returns a mutable reference to `self`, allowing multiple token filters to be appended in a chain of method calls.
    pub fn append_token_filter(&mut self, token_filter: BoxTokenFilter) -> &mut Self {
        self.token_filters.push(token_filter);

        self
    }

    /// Tokenizes the input text using the tokenizer's segmenter, character filters, and token filters.
    ///
    /// # Arguments
    ///
    /// * `text` - A reference to the input text (`&str`) that will be tokenized.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult` containing a vector of `Token`s, where each `Token` represents a segment of the tokenized text.
    ///
    /// # Process
    ///
    /// 1. **Apply character filters**:
    ///    - If any character filters are defined, they are applied to the input text before tokenization.
    ///    - The `offsets`, `diffs`, and `text_len` are recorded for each character filter.
    /// 2. **Segment the text**:
    ///    - The `segmenter` divides the (potentially filtered) text into tokens.
    /// 3. **Apply token filters**:
    ///    - If any token filters are defined, they are applied to the segmented tokens.
    /// 4. **Correct token offsets**:
    ///    - If character filters were applied, the byte offsets of each token are corrected to account for changes introduced by those filters.
    ///
    /// # Errors
    ///
    /// - Returns an error if any of the character or token filters fail during processing.
    /// - Returns an error if the segmentation process fails.
    ///
    /// # Details
    ///
    /// - `Cow<'a, str>` is used for the `normalized_text`, allowing the function to either borrow the original text or create an owned version if the text needs modification.
    /// - If no character filters are applied, the original `text` is used as-is for segmentation.
    /// - Token offsets are adjusted after the tokenization process if character filters were applied to ensure the byte positions of each token are accurate relative to the original text.
    pub fn tokenize<'a>(&'a self, text: &'a str) -> LinderaResult<Vec<Token<'a>>> {
        let mut normalized_text: Cow<'a, str> = Cow::Borrowed(text);

        let mut text_len_vec: Vec<usize> = Vec::new();
        let mut offsets_vec: Vec<Vec<usize>> = Vec::new();
        let mut diffs_vec: Vec<Vec<i64>> = Vec::new();

        // Appy character filters to the text if it is not empty.
        for character_filter in &self.character_filters {
            let (offsets, diffs, text_len) = character_filter.apply(normalized_text.to_mut())?;

            if !offsets.is_empty() {
                // Record the offsets of each character filter.
                offsets_vec.insert(0, offsets);

                // Record the diffs of each character filter.
                diffs_vec.insert(0, diffs);

                // Record the length of the text after each character filter is applied.
                text_len_vec.insert(0, text_len);
            }
        }

        // Setment a text.
        let mut tokens = self.segmenter.segment(normalized_text)?;

        // Apply token filters to the tokens if they are not empty.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tokens)?;
        }

        // Correct token offsets if character filters are applied.
        if !offsets_vec.is_empty() {
            for token in tokens.iter_mut() {
                // Override details.
                for (i, offsets) in offsets_vec.iter().enumerate() {
                    // Override start.
                    token.byte_start =
                        correct_offset(token.byte_start, offsets, &diffs_vec[i], text_len_vec[i]);
                    // Override end.
                    token.byte_end =
                        correct_offset(token.byte_end, offsets, &diffs_vec[i], text_len_vec[i]);
                }
            }
        }

        Ok(tokens)
    }
}

impl Clone for Tokenizer {
    /// Creates a deep clone of the `Tokenizer` instance, including all character filters, token filters, and the segmenter.
    ///
    /// # Returns
    ///
    /// Returns a new `Tokenizer` instance that is a deep clone of the current instance. All internal filters and the segmenter are cloned.
    ///
    /// # Details
    ///
    /// - **Character Filters**: Each character filter is cloned by calling its `box_clone` method, which ensures that any dynamically dispatched filters are properly cloned.
    /// - **Token Filters**: Similarly, each token filter is cloned using the `box_clone` method to handle dynamic dispatch.
    /// - **Segmenter**: The segmenter is cloned using its `clone` method.
    ///
    /// # Notes
    ///
    /// - This method performs deep cloning, meaning that all internal filters and segmenter instances are fully duplicated.
    /// - The `box_clone` method is used to clone the dynamically dispatched filter objects (`BoxCharacterFilter` and `BoxTokenFilter`).
    fn clone(&self) -> Self {
        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();
        for character_filter in self.character_filters.iter() {
            character_filters.push(character_filter.box_clone());
        }

        let mut token_filters: Vec<BoxTokenFilter> = Vec::new();
        for token_filter in self.token_filters.iter() {
            token_filters.push(token_filter.box_clone());
        }

        Tokenizer {
            character_filters,
            segmenter: self.segmenter.clone(),
            token_filters,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "ipadic")]
    use crate::tokenizer::{Tokenizer, TokenizerConfig};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenizer_config_from_slice() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "segmenter": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;

        let result: Result<TokenizerConfig, _> = serde_json::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenizer_config_clone() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "segmenter": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;

        let tokenizer_config: TokenizerConfig =
            serde_json::from_slice(config_str.as_bytes()).unwrap();

        let cloned_tokenizer_config = tokenizer_config.clone();

        assert_eq!(tokenizer_config, cloned_tokenizer_config);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_ipadic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "japanese_iteration_mark",
                    "args": {
                        "normalize_kanji": true,
                        "normalize_kana": true
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "segmenter": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_compound_word",
                    "args": {
                        "kind": "ipadic",
                        "tags": [
                            "名詞,数",
                            "名詞,接尾,助数詞"
                        ]
                    }
                },
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let tokenizer_config: TokenizerConfig =
            serde_json::from_slice(config_str.as_bytes()).unwrap();

        let analyzer = Tokenizer::from_config(&tokenizer_config).unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("Lindera"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 15);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, Some(vec![Cow::Borrowed("UNK")]));
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("形態素"));
                assert_eq!(token.byte_start, 18);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("形態素"),
                        Cow::Borrowed("ケイタイソ"),
                        Cow::Borrowed("ケイタイソ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("解析"));
                assert_eq!(token.byte_start, 27);
                assert_eq!(token.byte_end, 33);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("サ変接続"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("解析"),
                        Cow::Borrowed("カイセキ"),
                        Cow::Borrowed("カイセキ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("エンジン"));
                assert_eq!(token.byte_start, 33);
                assert_eq!(token.byte_end, 48);
                assert_eq!(token.position, 4);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("エンジン"),
                        Cow::Borrowed("エンジン"),
                        Cow::Borrowed("エンジン"),
                    ])
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("Lindera"));
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("10"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, Some(vec![Cow::Borrowed("UNK")]));
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ガロン"));
                assert_eq!(token.byte_start, 6);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 1);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("接尾"),
                        Cow::Borrowed("助数詞"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ガロン"),
                        Cow::Borrowed("ガロン"),
                        Cow::Borrowed("ガロン"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ガソリン"));
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ガソリン"),
                        Cow::Borrowed("ガソリン"),
                        Cow::Borrowed("ガソリン"),
                    ])
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("10"));
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("ガロン"));
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, Cow::Borrowed("ガソリン"));
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }

        {
            let text = "お釣りは百三十四円です。";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("お釣り"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("お釣り"),
                        Cow::Borrowed("オツリ"),
                        Cow::Borrowed("オツリ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("百三十四円"));
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 5);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("複合語"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                    ])
                );
            }
        }

        {
            let text = "ここは騒々しい";
            let mut tokens = analyzer.tokenize(text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("ここ"));
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("代名詞"),
                        Cow::Borrowed("一般"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("ここ"),
                        Cow::Borrowed("ココ"),
                        Cow::Borrowed("ココ"),
                    ])
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, Cow::Borrowed("騒騒しい"));
                assert_eq!(token.byte_start, 9);
                assert_eq!(token.byte_end, 21);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("形容詞"),
                        Cow::Borrowed("自立"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("形容詞・イ段"),
                        Cow::Borrowed("基本形"),
                        Cow::Borrowed("騒騒しい"),
                        Cow::Borrowed("ソウゾウシイ"),
                        Cow::Borrowed("ソーゾーシイ"),
                    ])
                );
            }
        }
    }
}
