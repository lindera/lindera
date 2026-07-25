use std::collections::HashMap;

use daachorse::DoubleArrayAhoCorasick;
use daachorse::DoubleArrayAhoCorasickBuilder;
use daachorse::MatchKind;
use serde_json::Value;

use crate::character_filter::{CharacterFilter, OffsetMapping, Transformation};
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;

pub const MAPPING_CHARACTER_FILTER_NAME: &str = "mapping";

pub type MappingCharacterFilterConfig = Value;

#[derive(Clone)]
pub struct MappingCharacterFilter {
    mapping: HashMap<String, String>,
    trie: DoubleArrayAhoCorasick<u32>,
}

impl MappingCharacterFilter {
    /// Create a new `MappingCharacterFilter` from a surface-to-replacement mapping.
    ///
    /// # Arguments
    ///
    /// * `mapping` - A map from surface text to its replacement text. Keys must be
    ///   non-empty; an empty key would match at every byte position under the
    ///   leftmost-longest search strategy used by `apply`, which is never a
    ///   meaningful mapping and is therefore rejected.
    ///
    /// # Returns
    ///
    /// A `MappingCharacterFilter`, or an error if `mapping` contains an empty key or
    /// the underlying Aho-Corasick automaton fails to build.
    pub fn new(mapping: HashMap<String, String>) -> LinderaResult<Self> {
        if mapping.keys().any(|key| key.is_empty()) {
            return Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("mapping key must not be empty.")));
        }

        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let trie = DoubleArrayAhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build_with_values(keyset)
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?;

        Ok(Self { mapping, trie })
    }

    pub fn from_config(config: &MappingCharacterFilterConfig) -> LinderaResult<Self> {
        let mapping = config
            .get("mapping")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("mapping must be an object."))
            })?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<HashMap<String, String>>();

        Self::new(mapping)
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn name(&self) -> &'static str {
        MAPPING_CHARACTER_FILTER_NAME
    }

    /// Apply the filter using the `OffsetMapping` API.
    ///
    /// Performs a single leftmost-longest pass over `text` with the underlying
    /// Aho-Corasick automaton: at each position the longest configured key wins,
    /// matches never overlap, and scanning resumes immediately after each match
    /// (mirroring `docs/src/concepts/filters.md`'s documented "longest-match
    /// search" semantics for this filter).
    ///
    /// # Arguments
    ///
    /// * `text` - The text to filter, replaced in place with the mapped text.
    ///
    /// # Returns
    ///
    /// An `OffsetMapping` recording one `Transformation` per replacement whose
    /// byte length differs from the original (length-preserving replacements are
    /// not recorded), in ascending filtered-offset order.
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
        let mut filtered_text = String::with_capacity(text.len());
        let mut mapping = OffsetMapping::new();

        {
            let source = text.as_str();
            let mut cursor = 0_usize;

            for m in self.trie.leftmost_find_iter(source) {
                // Keys are validated non-empty in `new`, and all keys are valid
                // UTF-8, so matches are non-empty, non-overlapping, strictly
                // ascending, and always land on char boundaries.
                debug_assert!(m.start() >= cursor && m.end() > m.start());

                // Copy the unmatched gap before this match verbatim.
                filtered_text.push_str(&source[cursor..m.start()]);

                let input_start = m.start();
                let input_len = m.end() - m.start();
                let replacement_text = &self.mapping[&source[m.start()..m.end()]];
                let replacement_len = replacement_text.len();

                // Record transformation if text changed
                if input_len != replacement_len {
                    let transformation = Transformation::new(
                        input_start,
                        input_start + input_len,
                        filtered_text.len(),
                        filtered_text.len() + replacement_len,
                    );
                    mapping.add_transformation(transformation);
                }

                filtered_text.push_str(replacement_text);
                cursor = m.end();
            }

            // Copy the trailing unmatched tail.
            filtered_text.push_str(&source[cursor..]);
        }

        *text = filtered_text;
        Ok(mapping)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::character_filter::mapping::{MappingCharacterFilter, MappingCharacterFilterConfig};
    use crate::character_filter::{CharacterFilter, Transformation};

    #[test]
    fn test_mapping_character_filter_config() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result: Result<MappingCharacterFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mapping_character_filter_from_config() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

        let result = MappingCharacterFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mapping_character_filter_apply() {
        {
            let config_str = r#"
            {
                "mapping": {
                    "ｱ": "ア",
                    "ｲ": "イ",
                    "ｳ": "ウ",
                    "ｴ": "エ",
                    "ｵ": "オ"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();

            let original_text = "ｱｲｳｴｵ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("アイウエオ", text.as_str());
            assert!(mapping.is_empty());

            // Test text fragments
            let start = 3;
            let end = 6;
            assert_eq!("イ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(3, correct_start);
            assert_eq!(6, correct_end);
            assert_eq!("ｲ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "ﾘ": "リ",
                    "ﾝ": "ン",
                    "ﾃﾞ": "デ",
                    "ﾗ": "ラ"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());

            // Verify transformation: "ﾃﾞ"(6-12) → "デ"(6-9)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(6, transform.original_start);
            assert_eq!(12, transform.original_end);
            assert_eq!(6, transform.filtered_start);
            assert_eq!(9, transform.filtered_end);

            // Test text fragments
            let start = 6;
            let end = 9;
            assert_eq!("デ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(12, correct_end);
            assert_eq!("ﾃﾞ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "ﾘﾝﾃﾞﾗ": "リンデラ"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "ﾘﾝﾃﾞﾗ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("リンデラ", text.as_str());

            // Verify transformation: "ﾘﾝﾃﾞﾗ"(0-15) → "リンデラ"(0-12)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(0, transform.original_start);
            assert_eq!(15, transform.original_end);
            assert_eq!(0, transform.filtered_start);
            assert_eq!(12, transform.filtered_end);

            // Test text fragments
            let start = 0;
            let end = 12;
            assert_eq!("リンデラ", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(0, correct_start);
            assert_eq!(15, correct_end);
            assert_eq!("ﾘﾝﾃﾞﾗ", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "リンデラ": "Lindera"
                }
            }
            "#;
            let config = serde_json::from_str::<MappingCharacterFilterConfig>(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "Rust製形態素解析器リンデラで日本語を形態素解析する。";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!(
                "Rust製形態素解析器Linderaで日本語を形態素解析する。",
                text.as_str()
            );

            // Verify transformation: "リンデラ"(25-37) → "Lindera"(25-32)
            assert_eq!(1, mapping.transformations.len());
            let transform = &mapping.transformations[0];
            assert_eq!(25, transform.original_start);
            assert_eq!(37, transform.original_end);
            assert_eq!(25, transform.filtered_start);
            assert_eq!(32, transform.filtered_end);

            // Test text fragments
            let start = 25;
            let end = 32;
            assert_eq!("Lindera", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(25, correct_start);
            assert_eq!(37, correct_end);
            assert_eq!("リンデラ", &original_text[correct_start..correct_end]);

            let start = 35;
            let end = 44;
            assert_eq!("日本語", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(40, correct_start);
            assert_eq!(49, correct_end);
            assert_eq!("日本語", &original_text[correct_start..correct_end]);
        }

        {
            let config_str = r#"
            {
                "mapping": {
                    "１": "1",
                    "０": "0",
                    "㍑": "リットル"
                }
            }
            "#;
            let config = serde_json::from_str(config_str).unwrap();

            let filter = MappingCharacterFilter::from_config(&config).unwrap();
            let original_text = "１０㍑";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            assert_eq!("10リットル", text.as_str());

            // All three replacements are recorded because of byte length differences
            assert_eq!(3, mapping.transformations.len());

            // Verify the last transformation: "㍑"(6-9) → "リットル"(2-14)
            let transform = &mapping.transformations[2];
            assert_eq!(6, transform.original_start);
            assert_eq!(9, transform.original_end);
            assert_eq!(2, transform.filtered_start);
            assert_eq!(14, transform.filtered_end);

            // Test text fragments
            let start = 2;
            let end = 14;
            assert_eq!("リットル", &text[start..end]);
            let correct_start = mapping.correct_offset(start, text.len());
            let correct_end = mapping.correct_offset(end, text.len());
            assert_eq!(6, correct_start);
            assert_eq!(9, correct_end);
            assert_eq!("㍑", &original_text[correct_start..correct_end]);
        }
    }

    #[test]
    fn test_mapping_character_filter_apply_longest_match() {
        let mut mapping = HashMap::new();
        mapping.insert("ab".to_string(), "X".to_string());
        mapping.insert("abc".to_string(), "YY".to_string());
        mapping.insert("b".to_string(), "Z".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();

        let mut text = "abcabx".to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("YYXx", text.as_str());

        // "abc" (longest match at 0) wins over "ab"/"b"; "ab" (longest at 3) wins over "b".
        assert_eq!(2, mapping.transformations.len());
        assert_eq!(Transformation::new(0, 3, 0, 2), mapping.transformations[0]);
        assert_eq!(Transformation::new(3, 5, 2, 3), mapping.transformations[1]);
    }

    #[test]
    fn test_mapping_character_filter_apply_backtrack() {
        // "abcd" fails to match "abx", but the shorter key "b" hiding inside the
        // failed prefix must still be found via the automaton's fail links.
        let mut mapping = HashMap::new();
        mapping.insert("abcd".to_string(), "1".to_string());
        mapping.insert("b".to_string(), "22".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();

        let mut text = "abx".to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("a22x", text.as_str());
        assert_eq!(1, mapping.transformations.len());
        assert_eq!(Transformation::new(1, 2, 1, 3), mapping.transformations[0]);
    }

    #[test]
    fn test_mapping_character_filter_apply_leftmost_wins() {
        // The long leftmost match consumes "h", so the overlapping key "hz"
        // starting inside it must not fire.
        let mut mapping = HashMap::new();
        mapping.insert("abcdefgh".to_string(), "1".to_string());
        mapping.insert("hz".to_string(), "2".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();

        let mut text = "abcdefghz".to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("1z", text.as_str());
        assert_eq!(1, mapping.transformations.len());
    }

    #[test]
    fn test_mapping_character_filter_apply_shared_prefix() {
        // "ﾃﾞ" and "ﾗ" share the "EF BE" lead byte pair; this exercises fail-link
        // recovery mid-multibyte-character and proves the gap copy never slices
        // across a char boundary.
        let mut mapping = HashMap::new();
        mapping.insert("ﾃﾞ".to_string(), "デ".to_string());
        mapping.insert("ﾗ".to_string(), "ラ".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();

        let mut text = "ﾃﾗ".to_string();
        let mapping = filter.apply(&mut text).unwrap();
        assert_eq!("ﾃラ", text.as_str());
        // "ﾗ" -> "ラ" is a same-byte-length substitution (3 -> 3 bytes).
        assert!(mapping.is_empty());
    }

    #[test]
    fn test_mapping_character_filter_empty_key_rejected() {
        let mut mapping = HashMap::new();
        mapping.insert(String::new(), "x".to_string());
        assert!(MappingCharacterFilter::new(mapping).is_err());

        let mut mapping = HashMap::new();
        mapping.insert("a".to_string(), "b".to_string());
        assert!(MappingCharacterFilter::new(mapping).is_ok());
    }

    #[test]
    fn test_mapping_character_filter_apply_large_input() {
        let mut mapping = HashMap::new();
        mapping.insert("リンデラ".to_string(), "Lindera".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();

        // Large, entirely non-matching input: the worst case for the previous
        // O(n^2) implementation. A generous absolute wall-clock ceiling is used
        // instead of a two-point scaling ratio, since a linear implementation
        // finishes in low single-digit milliseconds here (leaving a huge margin)
        // while the previous quadratic implementation would take several seconds
        // at this size, making the ceiling a reliable regression guard without
        // being sensitive to CI timing noise.
        let mut text = "あ".repeat(100_000);
        let original_len = text.len();

        let start = std::time::Instant::now();
        let mapping = filter.apply(&mut text).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(original_len, text.len());
        assert!(mapping.is_empty());
        assert!(
            elapsed.as_secs() < 3,
            "apply() took too long ({elapsed:?}); the quadratic-scan regression may have returned"
        );
    }
}
