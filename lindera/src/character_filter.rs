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
/// - `OffsetMapping`: A modern structure for tracking position changes during text filtering.
/// - `Transformation`: A record of text transformation with original and filtered positions.
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

/// A transformation record for offset mapping between original and filtered text
#[derive(Debug, Clone, PartialEq)]
pub struct Transformation {
    /// Start position in the original text (in bytes)
    pub original_start: usize,
    /// End position in the original text (in bytes)
    pub original_end: usize,
    /// Start position in the filtered text (in bytes)
    pub filtered_start: usize,
    /// End position in the filtered text (in bytes)
    pub filtered_end: usize,
}

impl Transformation {
    pub fn new(
        original_start: usize,
        original_end: usize,
        filtered_start: usize,
        filtered_end: usize,
    ) -> Self {
        Self {
            original_start,
            original_end,
            filtered_start,
            filtered_end,
        }
    }
}

/// Offset mapping structure for tracking position changes during text filtering
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OffsetMapping {
    /// List of transformations applied to the text
    pub transformations: Vec<Transformation>,
}

impl OffsetMapping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_transformations(transformations: Vec<Transformation>) -> Self {
        Self { transformations }
    }

    /// Add a transformation to the mapping
    pub fn add_transformation(&mut self, transformation: Transformation) {
        self.transformations.push(transformation);
    }

    /// Check if this mapping is empty (no transformations)
    pub fn is_empty(&self) -> bool {
        self.transformations.is_empty()
    }

    /// Convert this OffsetMapping to the legacy format (Vec<usize>, Vec<i64>, usize)
    /// This method ensures backward compatibility with existing code
    pub fn to_legacy_format(&self, text_len: usize) -> (Vec<usize>, Vec<i64>, usize) {
        if self.transformations.is_empty() {
            return (Vec::new(), Vec::new(), text_len);
        }

        let mut offsets = Vec::new();
        let mut diffs = Vec::new();
        let mut prev_diff = 0_i64;

        for transformation in &self.transformations {
            let original_len = transformation.original_end - transformation.original_start;
            let filtered_len = transformation.filtered_end - transformation.filtered_start;
            let diff_len = original_len as i64 - filtered_len as i64;

            if diff_len != 0 {
                if diff_len > 0 {
                    // Replacement is shorter than original
                    let offset =
                        (transformation.original_end as i64 - diff_len - prev_diff) as usize;
                    let diff = prev_diff + diff_len;
                    add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                } else {
                    // Replacement is longer than original
                    let output_offset = (transformation.original_end as i64 - prev_diff) as usize;
                    for extra_idx in 0..diff_len.unsigned_abs() as usize {
                        let offset = output_offset + extra_idx;
                        let diff = prev_diff - extra_idx as i64 - 1;
                        add_offset_diff(&mut offsets, &mut diffs, offset, diff);
                    }
                }
                prev_diff += diff_len;
            }
        }

        (offsets, diffs, text_len)
    }

    /// Correct a position in filtered text to the corresponding position in original text
    pub fn correct_offset(&self, offset: usize, text_len: usize) -> usize {
        let (offsets, diffs, text_len) = self.to_legacy_format(text_len);
        correct_offset(offset, &offsets, &diffs, text_len)
    }

    /// Compose this mapping with another mapping (for chaining filters)
    pub fn compose(self, other: OffsetMapping) -> OffsetMapping {
        if other.transformations.is_empty() {
            return self;
        }
        if self.transformations.is_empty() {
            return other;
        }

        // For now, use a simple approach: convert both to legacy format and merge
        // This can be optimized in the future for better performance
        let mut combined_transformations = self.transformations;
        combined_transformations.extend(other.transformations);

        OffsetMapping {
            transformations: combined_transformations,
        }
    }
}

/// The `CharacterFilter` trait defines an interface for filters that preprocess text before tokenization.
///
/// # Required Methods
///
/// - `name(&self) -> &str`:
///   - Returns the name of the character filter. This can be used for identification or logging purposes.
///
/// - `apply_with_offset_mapping(&self, text: &mut String) -> LinderaResult<OffsetMapping>`:
///   - Applies the character filter to the provided mutable string `text`.
///   - It returns a result containing an `OffsetMapping` which tracks all text transformations
///     performed by the filter, allowing precise position mapping between original and filtered text.
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
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping>;
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
            } else if offset >= text_len {
                // If offset is beyond the text length, use the last available diff
                if diffs.is_empty() {
                    return offset;
                } else {
                    diffs.len() - 1
                }
            } else {
                // If the `offset` is not found and `i` is 0,
                // the specified `offset` is the correct offset.
                return offset;
            }
        }
    };

    // Ensure index is within bounds
    if index >= diffs.len() {
        return offset;
    }

    // The correct offset value can be calculated by adding `diff[index]` to the given `offset`.
    let corrected = offset as i64 + diffs[index];
    if corrected < 0 { 0 } else { corrected as usize }
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
    use super::*;

    #[test]
    fn test_transformation() {
        let transformation = Transformation::new(0, 3, 0, 1);
        assert_eq!(transformation.original_start, 0);
        assert_eq!(transformation.original_end, 3);
        assert_eq!(transformation.filtered_start, 0);
        assert_eq!(transformation.filtered_end, 1);
    }

    #[test]
    fn test_offset_mapping_empty() {
        let mapping = OffsetMapping::new();
        assert!(mapping.is_empty());

        let (offsets, diffs, text_len) = mapping.to_legacy_format(10);
        assert!(offsets.is_empty());
        assert!(diffs.is_empty());
        assert_eq!(text_len, 10);
    }

    #[test]
    fn test_offset_mapping_with_transformation() {
        let mut mapping = OffsetMapping::new();
        mapping.add_transformation(Transformation::new(0, 3, 0, 1));

        assert!(!mapping.is_empty());

        let (offsets, diffs, text_len) = mapping.to_legacy_format(8);
        assert_eq!(offsets, vec![1]);
        assert_eq!(diffs, vec![2]);
        assert_eq!(text_len, 8);
    }

    #[test]
    fn test_offset_mapping_compose() {
        let mut mapping1 = OffsetMapping::new();
        mapping1.add_transformation(Transformation::new(0, 3, 0, 1));

        let mut mapping2 = OffsetMapping::new();
        mapping2.add_transformation(Transformation::new(1, 2, 1, 4));

        let composed = mapping1.compose(mapping2);
        assert_eq!(composed.transformations.len(), 2);
    }

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
