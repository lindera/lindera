/// This module defines character filters and utilities for handling text transformations
/// in the Lindera library. It includes various character filters such as Japanese iteration
/// mark filter, mapping filter, regex filter, and unicode normalization filter. The module
/// also provides functionality to load character filters from configuration values or CLI flags,
/// and utilities to manage text offsets during transformations.
///
/// # Modules
/// - `japanese_iteration_mark`: Contains the Japanese iteration mark character filter.
/// - `mapping`: Contains the mapping character filter.
/// - `regex`: Contains the regex character filter.
/// - `unicode_normalize`: Contains the unicode normalization character filter.
///
/// # Traits
/// - `CharacterFilter`: A trait for character filters that can be applied to text.
/// - `CharacterFilterClone`: A trait for cloning character filters.
///
/// # Structs
/// - `BoxCharacterFilter`: A boxed character filter that implements `Deref` to `CharacterFilter`.
/// - `CharacterFilterLoader`: A loader for character filters from configuration values or CLI flags.
///
/// # Functions
/// - `add_offset_diff`: Adds an offset difference to the given offsets and diffs vectors.
/// - `correct_offset`: Corrects the given offset based on the provided offsets and diffs.
///
/// # Tests
/// - `test_correct_offset`: Tests the `correct_offset` function with various cases.
pub mod japanese_iteration_mark;
pub mod mapping;
pub mod regex;
pub mod unicode_normalize;

use std::ops::Deref;

use serde_json::Value;

use crate::LinderaResult;
use crate::character_filter::japanese_iteration_mark::{
    JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME, JapaneseIterationMarkCharacterFilter,
};
use crate::character_filter::mapping::{MAPPING_CHARACTER_FILTER_NAME, MappingCharacterFilter};
use crate::character_filter::regex::{REGEX_CHARACTER_FILTER_NAME, RegexCharacterFilter};
use crate::character_filter::unicode_normalize::{
    UNICODE_NORMALIZE_CHARACTER_FILTER_NAME, UnicodeNormalizeCharacterFilter,
};
use crate::error::LinderaErrorKind;
use crate::parse_cli_flag;

/// The `CharacterFilter` trait defines an interface for filters that preprocess text before tokenization.
///
/// # Required Methods
///
/// - `name(&self) -> &str`:
///   - Returns the name of the character filter. This can be used for identification or logging purposes.
///
/// - `apply(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>, usize)>`:
///   - Applies the character filter to the provided mutable string `text`.
///   - It returns a result containing a tuple of:
///     - A vector of offsets (`Vec<usize>`) which represent positions in the text where modifications were made.
///     - A vector of differences (`Vec<i64>`) which indicates the change in text length at those positions.
///     - The final length of the modified text (`usize`).
///
/// # Trait Bounds
///
/// - `'static`: The filter must have a `'static` lifetime, meaning it does not contain any references with shorter lifetimes.
/// - `Send` and `Sync`: These bounds ensure that the filter can be safely used in multi-threaded contexts, allowing filters to be shared or sent across threads.
///
/// # Cloneability
///
/// - This trait requires the `CharacterFilterClone` trait, which is typically used to allow cloning of trait objects that implement `CharacterFilter`. This enables dynamic dispatch of cloned filters.
pub trait CharacterFilter: 'static + Send + Sync + CharacterFilterClone {
    fn name(&self) -> &str;
    fn apply(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>, usize)>;
}

/// A struct that holds a boxed `CharacterFilter` trait object.
///
/// `BoxCharacterFilter` wraps a `Box<dyn CharacterFilter + 'static + Send + Sync>`, allowing for dynamic dispatch of character filters while ensuring they are thread-safe and have a `'static` lifetime.
///
/// # Fields
///
/// - `0: Box<dyn CharacterFilter + 'static + Send + Sync>`:
///   - The boxed character filter trait object, which can be any type that implements the `CharacterFilter` trait. This allows for runtime polymorphism, meaning different character filter implementations can be stored in the same collection or passed around generically.
///
/// # Trait Bounds
///
/// - `CharacterFilter`: The wrapped object must implement the `CharacterFilter` trait, which defines the interface for applying filters to text.
/// - `'static`: The wrapped object must have a `'static` lifetime, meaning it can live for the duration of the program and does not borrow from temporary data.
/// - `Send` and `Sync`: These bounds ensure the filter can be shared between threads and sent across thread boundaries, making it safe for concurrent use.
///
/// # Example Usage
///
/// `BoxCharacterFilter` allows you to store and use different types of character filters dynamically, making it easier to apply multiple filters without needing to know their concrete types at compile time.
pub struct BoxCharacterFilter(Box<dyn CharacterFilter + 'static + Send + Sync>);

impl Deref for BoxCharacterFilter {
    type Target = dyn CharacterFilter;

    fn deref(&self) -> &dyn CharacterFilter {
        &*self.0
    }
}

impl<T: CharacterFilter> From<T> for BoxCharacterFilter {
    fn from(character_filter: T) -> BoxCharacterFilter {
        BoxCharacterFilter(Box::new(character_filter))
    }
}

pub trait CharacterFilterClone {
    fn box_clone(&self) -> BoxCharacterFilter;
}

impl<T: CharacterFilter + Clone + 'static> CharacterFilterClone for T {
    fn box_clone(&self) -> BoxCharacterFilter {
        BoxCharacterFilter::from(self.clone())
    }
}

pub fn add_offset_diff(offsets: &mut Vec<usize>, diffs: &mut Vec<i64>, offset: usize, diff: i64) {
    match offsets.last() {
        Some(&last_offset) => {
            if last_offset == offset {
                // Replace the last diff.
                diffs.pop();
                diffs.push(diff);
            } else {
                offsets.push(offset);
                diffs.push(diff);
            }
        }
        None => {
            // First offset.
            offsets.push(offset);
            diffs.push(diff);
        }
    }
}

pub fn correct_offset(offset: usize, offsets: &[usize], diffs: &[i64], text_len: usize) -> usize {
    // If `offsets` is empty, the `offset` specified is the correct offset.
    if offsets.is_empty() {
        return offset;
    }

    // Finds the `index` containing the specified `offset` from the `offsets`.
    let index = match offsets.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => {
            if i != 0 {
                // If `i` is greater than `0`, then `i - 1` is the `index` for the `diff` of the specified `offset`.
                i - 1
            } else if i >= text_len {
                text_len
            } else {
                // If the `offset` is not found and `i` is 0,
                // the specified `offset` is the correct offset.
                return offset;
            }
        }
    };

    // The correct offset value can be calculated by adding `diff[index]` to the given `offset`.
    (offset as i64 + diffs[index]) as usize
}

pub struct CharacterFilterLoader {}

impl CharacterFilterLoader {
    /// Loads a character filter based on the specified kind and configuration value.
    ///
    /// # Arguments
    ///
    /// * `kind` - A string slice representing the type of the character filter to be loaded. This string must match one of the supported filter types.
    /// * `value` - A `serde_json::Value` that contains the configuration for the filter. The structure of this value depends on the filter type.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<BoxCharacterFilter>`, which is a boxed character filter, or an error if the filter type is unsupported or if the configuration fails to load.
    ///
    /// # Supported Filters
    ///
    /// - `JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME`: Loads a `JapaneseIterationMarkCharacterFilter`.
    /// - `MAPPING_CHARACTER_FILTER_NAME`: Loads a `MappingCharacterFilter`.
    /// - `REGEX_CHARACTER_FILTER_NAME`: Loads a `RegexCharacterFilter`.
    /// - `UNICODE_NORMALIZE_CHARACTER_FILTER_NAME`: Loads a `UnicodeNormalizeCharacterFilter`.
    ///
    /// # Errors
    ///
    /// - If the `kind` does not match any of the supported filters, an error is returned.
    /// - If the configuration (`value`) for a filter is invalid, an error is returned during deserialization.
    ///
    /// # Details
    ///
    /// - This function uses the `kind` argument to determine which specific character filter to load. It matches the `kind` string to a filter name, deserializes the `value` into the appropriate filter configuration, and then constructs the corresponding filter.
    /// - If the `kind` does not match any supported filters, the function returns a deserialization error with an appropriate error message.
    pub fn load_from_value(kind: &str, value: &Value) -> LinderaResult<BoxCharacterFilter> {
        let character_filter = match kind {
            JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME => {
                BoxCharacterFilter::from(JapaneseIterationMarkCharacterFilter::from_config(value)?)
            }
            MAPPING_CHARACTER_FILTER_NAME => {
                BoxCharacterFilter::from(MappingCharacterFilter::from_config(value)?)
            }
            REGEX_CHARACTER_FILTER_NAME => {
                BoxCharacterFilter::from(RegexCharacterFilter::from_config(value)?)
            }
            UNICODE_NORMALIZE_CHARACTER_FILTER_NAME => {
                BoxCharacterFilter::from(UnicodeNormalizeCharacterFilter::from_config(value)?)
            }
            _ => {
                return Err(LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("unsupported character filter: {}", kind)));
            }
        };

        Ok(character_filter)
    }

    /// Loads a character filter based on a CLI flag string.
    ///
    /// # Arguments
    ///
    /// * `cli_flag` - A string slice representing the command-line interface (CLI) flag used to specify the character filter. The flag typically contains both the filter kind and its arguments.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<BoxCharacterFilter>`, which is a boxed character filter, or an error if the CLI flag is invalid or the filter configuration cannot be loaded.
    ///
    /// # Process
    ///
    /// 1. **Parse CLI flag**:
    ///    - The `parse_cli_flag` function is called to extract the filter kind and its arguments from the `cli_flag` string.
    /// 2. **Load filter from parsed values**:
    ///    - The filter kind and arguments are passed to `load_from_value`, which constructs the appropriate character filter based on the parsed values.
    ///
    /// # Errors
    ///
    /// - If the CLI flag cannot be parsed, an error is returned.
    /// - If the filter kind or its configuration is invalid, an error is returned during the filter loading process.
    ///
    /// # Details
    ///
    /// - The CLI flag is parsed into a filter kind and arguments. These are then used to load the appropriate character filter using the `load_from_value` function.
    pub fn load_from_cli_flag(cli_flag: &str) -> LinderaResult<BoxCharacterFilter> {
        let (kind, args) = parse_cli_flag(cli_flag)?;

        let character_filter = Self::load_from_value(kind, &args)?;

        Ok(character_filter)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_correct_offset() {
        let text = "ABCDEFG";
        let filterd_text = "AbbbCdddFgggg";

        let text_len = filterd_text.len();
        let offsets = vec![2, 3, 7, 10, 11, 12];
        let diffs = vec![-1, -2, -3, -4, -5, -6];

        let start_b = 1;
        let end_b = 4;
        assert_eq!("bbb", &filterd_text[start_b..end_b]);
        let correct_start_b = super::correct_offset(start_b, &offsets, &diffs, text_len);
        let correct_end_b = super::correct_offset(end_b, &offsets, &diffs, text_len);
        assert_eq!(1, correct_start_b);
        assert_eq!(2, correct_end_b);
        assert_eq!("B", &text[correct_start_b..correct_end_b]);

        let start_g = 9;
        let end_g = 13;
        assert_eq!("gggg", &filterd_text[start_g..end_g]);
        let correct_start_g = super::correct_offset(start_g, &offsets, &diffs, text_len);
        let correct_end_g = super::correct_offset(end_g, &offsets, &diffs, text_len);
        assert_eq!(6, correct_start_g);
        assert_eq!(7, correct_end_g);
        assert_eq!("G", &text[correct_start_g..correct_end_g]);

        let start = 0;
        let end = 13;
        assert_eq!("AbbbCdddFgggg", &filterd_text[start..end]);
        let correct_start = super::correct_offset(start, &offsets, &diffs, text_len);
        let correct_end = super::correct_offset(end, &offsets, &diffs, text_len);
        assert_eq!(0, correct_start);
        assert_eq!(7, correct_end);
        assert_eq!("ABCDEFG", &text[correct_start..correct_end]);
    }
}
