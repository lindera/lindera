/// This module defines character filters and utilities for handling text transformations
/// in the Lindera library. It includes various character filters such as Japanese iteration
/// mark filter, mapping filter, regex filter, and unicode normalization filter. The module
/// also provides functionality to load character filters from configuration values or CLI flags,
/// and utilities to manage text offsets during transformations.
///
/// # Offset Mapping System
///
/// The offset mapping system tracks how character positions change during text filtering,
/// allowing accurate mapping between filtered text positions and original text positions.
/// This is essential for maintaining correct token byte offsets in tokenization.
///
/// ## How Offset Mapping Works
///
/// When text is transformed by character filters, each transformation is recorded as a
/// `Transformation` that captures:
/// - Original text byte range (before filtering)  
/// - Filtered text byte range (after filtering)
///
/// ### Example: "１０㍑" → "10リットル"
///
/// ```text
/// Original:  "１０㍑"
/// Positions:  0-3  3-6  6-9     (byte positions)
///              ↓    ↓    ↓
/// Filtered:  "10リットル"  
/// Positions:  0-1  1-2  2-14    (byte positions)
/// ```
///
/// This creates three transformations:
/// 1. "１" (0-3) → "1" (0-1)
/// 2. "０" (3-6) → "0" (1-2)
/// 3. "㍑" (6-9) → "リットル" (2-14)
///
/// ### Position Correction
///
/// To map a filtered text position back to the original:
/// 1. Find which transformation range contains the position
/// 2. Calculate the corresponding position in the original text
/// 3. Return the original text position
///
/// ```rust
/// // For filtered position 2 ("リットル" start):
/// // → finds transformation[2]: filtered_range(2-14) contains position 2
/// // → returns original_start: 6 (start of "㍑")
/// let original_pos = mapping.correct_offset(2, text.len()); // returns 6
/// ```
///
/// This ensures that tokenizer can provide accurate byte offsets relative to the original
/// input text, even after multiple character transformations.
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
/// No public utility functions are exposed, as all offset management is handled through OffsetMapping.
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

/// A transformation record for offset mapping between original and filtered text.
///
/// This structure captures a single text transformation, recording how a specific
/// segment of the original text maps to a segment in the filtered text.
///
/// # Example
///
/// For the transformation "㍑" → "リットル":
/// ```rust
/// let transformation = Transformation::new(
///     6, 9,    // original: "㍑" at bytes 6-9
///     2, 14    // filtered: "リットル" at bytes 2-14  
/// );
/// ```
///
/// This allows precise mapping between any position in the filtered text
/// back to the corresponding position in the original text.
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

/// Offset mapping structure for tracking position changes during text filtering.
///
/// This structure maintains a list of all text transformations that occurred during
/// character filtering, enabling accurate position mapping between filtered and original text.
///
/// # Usage Pattern
///
/// 1. **Record transformations** during filtering:
/// ```rust
/// let mut mapping = OffsetMapping::new();
/// // When "㍑" → "リットル" transformation occurs:
/// mapping.add_transformation(Transformation::new(6, 9, 2, 14));
/// ```
///
/// 2. **Correct positions** from filtered to original:
/// ```rust
/// let original_pos = mapping.correct_offset(filtered_pos, text.len());
/// ```
///
/// # Multi-Filter Support
///
/// When multiple character filters are applied, their mappings are composed:
/// ```rust
/// let combined_mapping = mapping1.compose(mapping2);
/// ```
///
/// This ensures accurate position tracking through complex filter chains.
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

    /// Correct a position in filtered text to the corresponding position in original text.
    ///
    /// This method maps a byte position in the filtered text back to the corresponding
    /// byte position in the original text, accounting for all recorded transformations.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte position in the filtered text
    /// * `text_len` - Length of the filtered text (used for boundary validation)
    ///
    /// # Returns
    ///
    /// The corresponding byte position in the original text.
    ///
    /// # Algorithm
    ///
    /// 1. If no transformations exist, return the offset unchanged
    /// 2. Find the transformation whose filtered range contains the offset
    /// 3. Map the offset to the corresponding position in the original range
    /// 4. If offset is outside all transformation ranges, adjust by cumulative differences
    ///
    /// # Example
    ///
    /// ```rust
    /// // For "１０㍑" → "10リットル" with transformations recorded
    /// let mapping = /* ... transformations for the above conversion */;
    ///
    /// // Position 2 in "10リットル" ("リットル" start)
    /// let original_pos = mapping.correct_offset(2, 14); // returns 6
    /// // This maps to position 6 in "１０㍑" ("㍑" start)
    /// ```
    pub fn correct_offset(&self, offset: usize, text_len: usize) -> usize {
        if self.transformations.is_empty() {
            return offset;
        }

        // Boundary check: if offset is beyond text length, clamp to text length
        let clamped_offset = offset.min(text_len);

        // Find the transformation that affects this offset
        for transformation in &self.transformations {
            if clamped_offset >= transformation.filtered_start
                && clamped_offset <= transformation.filtered_end
            {
                // Offset is within this transformation range
                let filtered_offset = clamped_offset - transformation.filtered_start;
                let original_len = transformation.original_end - transformation.original_start;
                let filtered_len = transformation.filtered_end - transformation.filtered_start;

                if filtered_len == 0 {
                    // Deletion case
                    return transformation.original_start;
                } else if original_len == 0 {
                    // Insertion case
                    return transformation.original_start;
                } else {
                    // Substitution case - proportionally map within the range
                    let ratio = filtered_offset as f64 / filtered_len as f64;
                    let original_offset = (ratio * original_len as f64).round() as usize;
                    return transformation.original_start + original_offset;
                }
            } else if clamped_offset < transformation.filtered_start {
                // Offset is before this transformation, need to account for previous transformations
                let mut corrected = clamped_offset;
                for prev_transform in &self.transformations {
                    if prev_transform.filtered_start < transformation.filtered_start {
                        let original_len =
                            prev_transform.original_end - prev_transform.original_start;
                        let filtered_len =
                            prev_transform.filtered_end - prev_transform.filtered_start;
                        let diff = original_len as i64 - filtered_len as i64;
                        corrected = (corrected as i64 + diff) as usize;
                    }
                }
                return corrected;
            }
        }

        // Offset is after all transformations - apply cumulative differences
        let mut corrected = clamped_offset;
        for transformation in &self.transformations {
            let original_len = transformation.original_end - transformation.original_start;
            let filtered_len = transformation.filtered_end - transformation.filtered_start;
            let diff = original_len as i64 - filtered_len as i64;
            corrected = (corrected as i64 + diff) as usize;
        }

        // Handle case where original offset was beyond text length
        if offset > text_len {
            // Preserve the overshoot in the original text space
            let overshoot = offset - text_len;
            corrected + overshoot
        } else {
            corrected
        }
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

        // Empty mapping should not change offsets
        assert_eq!(5, mapping.correct_offset(5, 10));
        assert_eq!(0, mapping.correct_offset(0, 10));
    }

    #[test]
    fn test_offset_mapping_with_transformation() {
        let mut mapping = OffsetMapping::new();
        mapping.add_transformation(Transformation::new(0, 3, 0, 1));

        assert!(!mapping.is_empty());

        // Test offset correction for shortening transformation (0-3) -> (0-1)
        assert_eq!(0, mapping.correct_offset(0, 8)); // Start maps to start
        assert_eq!(3, mapping.correct_offset(1, 8)); // End of filtered maps to end of original
        assert_eq!(5, mapping.correct_offset(3, 8)); // After transformation, add diff
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
}
