pub mod japanese_iteration_mark;
pub mod mapping;
pub mod regex;
pub mod unicode_normalize;

use serde_json::Value;
use std::ops::Deref;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::character_filter::japanese_iteration_mark::{
    JapaneseIterationMarkCharacterFilter, JapaneseIterationMarkCharacterFilterConfig,
    JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME,
};
use crate::character_filter::mapping::{
    MappingCharacterFilter, MappingCharacterFilterConfig, MAPPING_CHARACTER_FILTER_NAME,
};
use crate::character_filter::regex::{
    RegexCharacterFilter, RegexCharacterFilterConfig, REGEX_CHARACTER_FILTER_NAME,
};
use crate::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeCharacterFilterConfig,
    UNICODE_NORMALIZE_CHARACTER_FILTER_NAME,
};
use crate::parse_cli_flag;

pub trait CharacterFilter: 'static + Send + Sync + CharacterFilterClone {
    fn name(&self) -> &str;
    fn apply(&self, text: &str) -> LinderaResult<(String, Vec<usize>, Vec<i64>)>;
}

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
    pub fn load_from_value(kind: &str, value: &Value) -> LinderaResult<BoxCharacterFilter> {
        let character_filter = match kind {
            JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME => {
                BoxCharacterFilter::from(JapaneseIterationMarkCharacterFilter::new(
                    JapaneseIterationMarkCharacterFilterConfig::from_value(value)?,
                ))
            }
            MAPPING_CHARACTER_FILTER_NAME => {
                let config = MappingCharacterFilterConfig::from_value(value)?;
                BoxCharacterFilter::from(MappingCharacterFilter::new(config)?)
            }
            REGEX_CHARACTER_FILTER_NAME => {
                let config = RegexCharacterFilterConfig::from_value(value)?;
                BoxCharacterFilter::from(RegexCharacterFilter::new(config)?)
            }
            UNICODE_NORMALIZE_CHARACTER_FILTER_NAME => {
                let config = UnicodeNormalizeCharacterFilterConfig::from_value(value)?;
                BoxCharacterFilter::from(UnicodeNormalizeCharacterFilter::new(config))
            }
            _ => {
                return Err(LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("unsupported character filter: {}", kind)));
            }
        };

        Ok(character_filter)
    }

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
