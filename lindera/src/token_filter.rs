/// This module defines various token filters and provides functionality to load them.
///
/// # Modules
/// - `japanese_base_form`: Contains the Japanese base form token filter.
/// - `japanese_compound_word`: Contains the Japanese compound word token filter.
/// - `japanese_kana`: Contains the Japanese kana token filter.
/// - `japanese_katakana_stem`: Contains the Japanese katakana stem token filter.
/// - `japanese_keep_tags`: Contains the Japanese keep tags token filter.
/// - `japanese_number`: Contains the Japanese number token filter.
/// - `japanese_reading_form`: Contains the Japanese reading form token filter.
/// - `japanese_stop_tags`: Contains the Japanese stop tags token filter.
/// - `keep_words`: Contains the keep words token filter.
/// - `korean_keep_tags`: Contains the Korean keep tags token filter.
/// - `korean_reading_form`: Contains the Korean reading form token filter.
/// - `korean_stop_tags`: Contains the Korean stop tags token filter.
/// - `length`: Contains the length token filter.
/// - `lowercase`: Contains the lowercase token filter.
/// - `mapping`: Contains the mapping token filter.
/// - `remove_diacritical_mark`: Contains the remove diacritical mark token filter.
/// - `stop_words`: Contains the stop words token filter.
/// - `uppercase`: Contains the uppercase token filter.
///
/// # Traits
/// - `TokenFilter`: A trait for token filters that can be applied to a vector of tokens.
/// - `TokenFilterClone`: A trait for cloning boxed token filters.
///
/// # Structs
/// - `BoxTokenFilter`: A boxed token filter that implements `TokenFilter`.
/// - `TokenFilterLoader`: A loader for creating token filters from configuration values.
///
/// # Usage
/// The `TokenFilterLoader` struct provides methods to load token filters from configuration values
/// or command-line flags. The `TokenFilter` trait defines the interface for token filters, and
/// `BoxTokenFilter` is a boxed implementation of a token filter.
pub mod japanese_base_form;
pub mod japanese_compound_word;
pub mod japanese_kana;
pub mod japanese_katakana_stem;
pub mod japanese_keep_tags;
pub mod japanese_number;
pub mod japanese_reading_form;
pub mod japanese_stop_tags;
pub mod keep_words;
pub mod korean_keep_tags;
pub mod korean_reading_form;
pub mod korean_stop_tags;
pub mod length;
pub mod lowercase;
pub mod mapping;
pub mod remove_diacritical_mark;
pub mod stop_words;
pub mod uppercase;

use serde_json::Value;
use std::ops::Deref;

use crate::parse_cli_flag;
use crate::token::Token;
use crate::token_filter::japanese_base_form::{
    JapaneseBaseFormTokenFilter, JAPANESE_BASE_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_compound_word::{
    JapaneseCompoundWordTokenFilter, JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_kana::{
    JapaneseKanaTokenFilter, JAPANESE_KANA_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_katakana_stem::{
    JapaneseKatakanaStemTokenFilter, JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_keep_tags::{
    JapaneseKeepTagsTokenFilter, JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_number::{
    JapaneseNumberTokenFilter, JAPANESE_NUMBER_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_reading_form::{
    JapaneseReadingFormTokenFilter, JAPANESE_READING_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_stop_tags::{
    JapaneseStopTagsTokenFilter, JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::keep_words::{KeepWordsTokenFilter, KEEP_WORDS_TOKEN_FILTER_NAME};
use crate::token_filter::korean_keep_tags::{
    KoreanKeepTagsTokenFilter, KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::korean_reading_form::{
    KoreanReadingFormTokenFilter, KOREAN_READING_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::korean_stop_tags::{
    KoreanStopTagsTokenFilter, KOREAN_STOP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::length::{LengthTokenFilter, LENGTH_TOKEN_FILTER_NAME};
use crate::token_filter::lowercase::{LowercaseTokenFilter, LOWERCASE_TOKEN_FILTER_NAME};
use crate::token_filter::mapping::{MappingTokenFilter, MAPPING_TOKEN_FILTER_NAME};
use crate::token_filter::remove_diacritical_mark::{
    RemoveDiacriticalMarkTokenFilter, REMOVE_DIACRITICAL_TOKEN_FILTER_NAME,
};
use crate::token_filter::stop_words::{StopWordsTokenFilter, STOP_WORDS_TOKEN_FILTER_NAME};
use crate::token_filter::uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME};
use crate::{LinderaErrorKind, LinderaResult};

/// A trait for token filters that can be applied to a vector of tokens.
///
/// This trait requires the implementor to be `'static`, `Send`, `Sync`, and
/// implement the `TokenFilterClone` trait. It provides methods to get the
/// name of the filter and to apply the filter to a mutable vector of tokens.
///
/// # Required Methods
///
/// - `name`: Returns the name of the token filter as a static string slice.
/// - `apply`: Applies the token filter to a mutable vector of tokens, returning
///   a `LinderaResult<()>`.
pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn name(&self) -> &'static str;
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()>;
}

/// A `BoxTokenFilter` is a wrapper around a boxed trait object that implements
/// the `TokenFilter` trait. This allows for dynamic dispatch of different
/// `TokenFilter` implementations at runtime. The `BoxTokenFilter` ensures that
/// the contained `TokenFilter` is thread-safe (`Send` and `Sync`) and has a
/// static lifetime.
pub struct BoxTokenFilter(Box<dyn TokenFilter + 'static + Send + Sync>);

impl Deref for BoxTokenFilter {
    type Target = dyn TokenFilter;

    fn deref(&self) -> &dyn TokenFilter {
        &*self.0
    }
}

impl<T: TokenFilter> From<T> for BoxTokenFilter {
    fn from(token_filter: T) -> BoxTokenFilter {
        BoxTokenFilter(Box::new(token_filter))
    }
}

/// A trait for cloning token filters.
///
/// This trait provides a method `box_clone` which allows for cloning
/// a token filter and returning it as a boxed trait object.
pub trait TokenFilterClone {
    fn box_clone(&self) -> BoxTokenFilter;
}

impl<T: TokenFilter + Clone + 'static> TokenFilterClone for T {
    fn box_clone(&self) -> BoxTokenFilter {
        BoxTokenFilter::from(self.clone())
    }
}

pub struct TokenFilterLoader {}

impl TokenFilterLoader {
    pub fn load_from_value(kind: &str, value: &Value) -> LinderaResult<BoxTokenFilter> {
        // Creates a `BoxTokenFilter` based on the provided `kind` and `value`.
        //
        // The function matches the `kind` against various predefined token filter names
        // and constructs the corresponding token filter using the configuration derived
        // from `value`. If the `kind` does not match any of the predefined names, an error
        // is returned.
        //
        // # Parameters
        // - `kind`: A string slice that specifies the type of token filter to create.
        // - `value`: A `serde_json::Value` that contains the configuration for the token filter.
        //
        // # Returns
        // - `Result<BoxTokenFilter, LinderaError>`: A boxed token filter if the `kind` is recognized,
        //   otherwise an error indicating that the token filter is unsupported.
        //
        // # Errors
        // - Returns `LinderaErrorKind::Deserialize` if the `kind` is not supported or if there is an
        //   error in creating the token filter from the provided `value`.
        let token_filter = match kind {
            JAPANESE_BASE_FORM_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseBaseFormTokenFilter::from_config(value)?)
            }
            JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseCompoundWordTokenFilter::from_config(value)?)
            }
            JAPANESE_KANA_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseKanaTokenFilter::from_config(value)?)
            }
            JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseKatakanaStemTokenFilter::from_config(value)?)
            }
            JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseKeepTagsTokenFilter::from_config(value)?)
            }
            JAPANESE_NUMBER_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseNumberTokenFilter::from_config(value)?)
            }
            JAPANESE_READING_FORM_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseReadingFormTokenFilter::from_config(value)?)
            }
            JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(JapaneseStopTagsTokenFilter::from_config(value)?)
            }
            KEEP_WORDS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(KeepWordsTokenFilter::from_config(value)?)
            }
            KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(KoreanKeepTagsTokenFilter::from_config(value)?)
            }
            KOREAN_READING_FORM_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(KoreanReadingFormTokenFilter::from_config(value)?)
            }
            KOREAN_STOP_TAGS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(KoreanStopTagsTokenFilter::from_config(value)?)
            }
            LENGTH_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(LengthTokenFilter::from_config(value)?)
            }
            LOWERCASE_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(LowercaseTokenFilter::from_config(value)?)
            }
            MAPPING_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(MappingTokenFilter::from_config(value)?)
            }
            REMOVE_DIACRITICAL_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(RemoveDiacriticalMarkTokenFilter::from_config(value)?)
            }
            STOP_WORDS_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(StopWordsTokenFilter::from_config(value)?)
            }
            UPPERCASE_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(UppercaseTokenFilter::from_config(value)?)
            }
            _ => {
                return Err(LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("unsupported token filter: {}", kind)));
            }
        };

        Ok(token_filter)
    }

    /// Loads a token filter based on a CLI flag string.
    ///
    /// # Arguments
    ///
    /// * `cli_flag` - A string slice representing the command-line interface (CLI) flag used to specify the token filter. The flag typically contains both the filter kind and its arguments.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<BoxTokenFilter>`, which is a boxed token filter, or an error if the CLI flag is invalid or the filter configuration cannot be loaded.
    ///
    /// # Process
    ///
    /// 1. **Parse CLI flag**:
    ///    - The `parse_cli_flag` function is called to extract the filter kind and its arguments from the `cli_flag` string.
    /// 2. **Load filter from parsed values**:
    ///    - The filter kind and arguments are passed to `load_from_value`, which constructs the appropriate token filter based on the parsed values.
    ///
    /// # Errors
    ///
    /// - If the CLI flag cannot be parsed, an error is returned.
    /// - If the filter kind or its configuration is invalid, an error is returned during the filter loading process.
    ///
    /// # Details
    ///
    /// - The CLI flag is parsed into a filter kind and arguments. These are then used to load the appropriate token filter using the `load_from_value` function.
    pub fn load_from_cli_flag(cli_flag: &str) -> LinderaResult<BoxTokenFilter> {
        let (kind, args) = parse_cli_flag(cli_flag)?;

        let character_filter = Self::load_from_value(kind, &args)?;

        Ok(character_filter)
    }
}
