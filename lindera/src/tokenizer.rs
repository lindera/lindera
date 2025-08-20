use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde_json::{Value, json};

use crate::LinderaResult;
use crate::character_filter::{BoxCharacterFilter, CharacterFilterLoader, OffsetMapping};
use crate::error::LinderaErrorKind;
use crate::mode::Mode;
use crate::segmenter::Segmenter;
use crate::token::Token;
use crate::token_filter::{BoxTokenFilter, TokenFilterLoader};

pub type TokenizerConfig = Value;

fn yaml_to_config(file_path: &Path) -> LinderaResult<TokenizerConfig> {
    let mut input_read = File::open(file_path).map_err(|err| {
        LinderaErrorKind::Io.with_error(err).add_context(format!(
            "Failed to open tokenizer config file: {}",
            file_path.display()
        ))
    })?;

    let mut buffer = Vec::new();
    input_read.read_to_end(&mut buffer).map_err(|err| {
        LinderaErrorKind::Io.with_error(err).add_context(format!(
            "Failed to read tokenizer config file: {}",
            file_path.display()
        ))
    })?;

    match serde_yaml::from_slice::<serde_yaml::Value>(&buffer) {
        Ok(value) => {
            // Check if the value is a mapping.
            match value {
                serde_yaml::Value::Mapping(_) => {
                    Ok(serde_json::to_value(value).map_err(|err| {
                        LinderaErrorKind::Deserialize
                            .with_error(err)
                            .add_context(format!(
                                "Failed to convert YAML to JSON for config file: {}",
                                file_path.display()
                            ))
                    })?)
                }
                _ => Err(LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("Invalid YAML"))
                    .add_context(format!(
                        "Config file must contain a YAML mapping: {}",
                        file_path.display()
                    ))),
            }
        }
        Err(err) => Err(LinderaErrorKind::Deserialize
            .with_error(err)
            .add_context(format!(
                "Failed to parse YAML config file: {}",
                file_path.display()
            ))),
    }
}

/// Returns the default configuration as a `serde_json::Value`.
fn empty_config() -> Value {
    json!({
        "segmenter": {},
        "character_filters": [],
        "token_filters": []
    })
}

/// Ensures that the configuration contains the required keys with default values if absent.
fn ensure_keys(mut config: Value) -> Value {
    if config.get("segmenter").is_none() {
        config["segmenter"] = json!({});
    }

    if config.get("character_filters").is_none() {
        config["character_filters"] = json!([]);
    }

    if config.get("token_filters").is_none() {
        config["token_filters"] = json!([]);
    }

    config
}

#[derive(Debug)]
pub struct TokenizerBuilder {
    config: TokenizerConfig,
}

impl TokenizerBuilder {
    pub fn new() -> LinderaResult<Self> {
        if let Ok(config_path) = env::var("LINDERA_CONFIG_PATH") {
            Self::from_file(Path::new(&config_path))
        } else {
            Ok(Self {
                config: empty_config(),
            })
        }
    }

    pub fn from_file(file_path: &Path) -> LinderaResult<Self> {
        let config = yaml_to_config(file_path)?;

        Ok(TokenizerBuilder {
            config: ensure_keys(config),
        })
    }

    pub fn from_config(config: TokenizerConfig) -> LinderaResult<Self> {
        Ok(TokenizerBuilder {
            config: ensure_keys(config),
        })
    }

    pub fn set_segmenter_mode(&mut self, mode: &Mode) -> &mut Self {
        self.config["segmenter"]["mode"] = json!(mode.as_str());
        self
    }

    pub fn set_segmenter_dictionary(&mut self, uri: &str) -> &mut Self {
        self.config["segmenter"]["dictionary"] = json!(uri);
        self
    }

    pub fn set_segmenter_user_dictionary(&mut self, uri: &str) -> &mut Self {
        self.config["segmenter"]["user_dictionary"] = json!(uri);
        self
    }

    pub fn append_character_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        if let Some(array) = self.config["character_filters"].as_array_mut() {
            array.push(json!({ "kind": kind, "args": args }));
        }
        self
    }

    pub fn append_token_filter(&mut self, kind: &str, args: &Value) -> &mut Self {
        if let Some(array) = self.config["token_filters"].as_array_mut() {
            array.push(json!({ "kind": kind, "args": args }));
        }
        self
    }

    pub fn build(&self) -> LinderaResult<Tokenizer> {
        Tokenizer::from_config(&self.config).map_err(|err| {
            LinderaErrorKind::Parse
                .with_error(anyhow::anyhow!("failed to build tokenizer: {}", err))
        })
    }
}

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
    pub fn new(segmenter: Segmenter) -> Self {
        Self {
            segmenter,
            character_filters: Vec::new(),
            token_filters: Vec::new(),
        }
    }

    pub fn from_config(config: &TokenizerConfig) -> LinderaResult<Self> {
        let segmenter_config = config.get("segmenter").ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("missing segmenter config."))
        })?;
        let segmenter = Segmenter::from_config(segmenter_config)?;

        // Create a tokenizer from the segmenter.
        let mut tokenizer = Tokenizer::new(segmenter);

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

        let mut offset_mappings: Vec<OffsetMapping> =
            Vec::with_capacity(self.character_filters.len());

        // Apply character filters to the text if it is not empty.
        // Optimize: Only convert to mutable when we have filters to apply
        if !self.character_filters.is_empty() {
            // Convert to owned string once for all filters
            let text_mut = normalized_text.to_mut();

            for character_filter in &self.character_filters {
                let mapping = character_filter.apply(text_mut)?;

                if !mapping.is_empty() {
                    // Record the offset mapping of each character filter in reverse order
                    // since we need to apply corrections in reverse order
                    offset_mappings.push(mapping);
                }
            }
        }

        // Store the final text length for offset correction
        let final_text_len = normalized_text.len();

        // Segment a text.
        let mut tokens = self.segmenter.segment(normalized_text)?;

        // Apply token filters to the tokens if they are not empty.
        for token_filter in &self.token_filters {
            token_filter.apply(&mut tokens)?;
        }

        // Correct token offsets if character filters are applied.
        // Apply corrections in reverse order (last filter first)
        if !offset_mappings.is_empty() {
            for token in tokens.iter_mut() {
                // Apply corrections in reverse order to undo the transformations
                for mapping in offset_mappings.iter().rev() {
                    // Override start.
                    token.byte_start = mapping.correct_offset(token.byte_start, final_text_len);
                    // Override end.
                    token.byte_end = mapping.correct_offset(token.byte_end, final_text_len);
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
        let character_filters: Vec<BoxCharacterFilter> = self
            .character_filters
            .iter()
            .map(|filter| filter.box_clone())
            .collect();

        let token_filters: Vec<BoxTokenFilter> = self
            .token_filters
            .iter()
            .map(|filter| filter.box_clone())
            .collect();

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
    #[test]
    fn test_tokenizer_config_from_slice() {
        use std::path::PathBuf;

        use crate::tokenizer::yaml_to_config;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("lindera.yml");

        let result = yaml_to_config(&config_file);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenizer_config_clone() {
        use std::path::PathBuf;

        use crate::tokenizer::yaml_to_config;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("lindera.yml");

        let tokenizer_config = yaml_to_config(&config_file).unwrap();

        let cloned_tokenizer_config = tokenizer_config.clone();

        assert_eq!(tokenizer_config, cloned_tokenizer_config);
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_ipadic() {
        use std::borrow::Cow;
        use std::path::PathBuf;

        use crate::tokenizer::TokenizerBuilder;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("lindera.yml");

        let builder = TokenizerBuilder::from_file(&config_file).unwrap();

        let tokenizer = builder.build().unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。";
            let mut tokens = tokenizer.tokenize(text).unwrap();
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
            let mut tokens = tokenizer.tokenize(text).unwrap();
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
            let mut tokens = tokenizer.tokenize(text).unwrap();
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
                assert_eq!(token.text, Cow::Borrowed("134円"));
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 5);
                assert_eq!(
                    token.details,
                    Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("数"),
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
            let mut tokens = tokenizer.tokenize(text).unwrap();
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

    #[test]
    #[cfg(not(windows))]
    #[should_panic(expected = "No such file or directory")]
    fn test_create_tokenizer_builder_from_non_existent_file() {
        use std::path::PathBuf;

        use crate::tokenizer::TokenizerBuilder;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("non_existent_file.yml");

        TokenizerBuilder::from_file(&config_file).unwrap();
    }

    #[test]
    #[cfg(windows)]
    #[should_panic(expected = "The system cannot find the file specified.")]
    fn test_create_tokenizer_builder_from_non_existent_file() {
        use std::path::PathBuf;

        use crate::tokenizer::TokenizerBuilder;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("non_existent_file.yml");

        TokenizerBuilder::from_file(&config_file).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid YAML")]
    fn test_create_tokenizer_builder_from_invalid_file() {
        use std::path::PathBuf;

        use crate::tokenizer::TokenizerBuilder;

        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("invalid.yml");

        TokenizerBuilder::from_file(&config_file).unwrap();
    }
}
