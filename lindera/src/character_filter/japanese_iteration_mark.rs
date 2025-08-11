use std::collections::BTreeMap;

use serde_json::Value;

use crate::LinderaResult;
use crate::character_filter::{CharacterFilter, OffsetMapping, Transformation};
use crate::error::LinderaErrorKind;

pub const JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME: &str = "japanese_iteration_mark";

const KANJI_ITERATION_MARK: char = '々';
const HIRAGANA_ITERATION_MARK: char = 'ゝ';
const HIRAGANA_DAKUON_ITERATION_MARK: char = 'ゞ';
const KATAKANA_ITERATION_MARK: char = 'ヽ';
const KATAKANA_DAKUON_ITERATION_MARK: char = 'ヾ';

pub type JapaneseIterationMarkCharacterFilterConfig = Value;

fn hiragana_add_dakuon(c: &char) -> char {
    let codepoint = *c as u32;
    // Unsafe code is okay, because we know that all the characters within these ranges exist.
    match codepoint {
        0x304b..=0x3062 if codepoint % 2 == 1 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        0x3064..=0x3069 if codepoint % 2 == 0 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        0x306f..=0x307d if codepoint % 3 == 0 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        _ => *c,
    }
}

fn hiragana_remove_dakuon(c: &char) -> char {
    let codepoint = *c as u32;
    // Unsafe code is okay, because we know that all the characters within these ranges exist.
    match codepoint {
        0x304b..=0x3062 if codepoint % 2 == 0 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        0x3064..=0x3069 if codepoint % 2 == 1 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        0x306f..=0x307d if codepoint % 3 == 1 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        _ => *c,
    }
}

fn katakana_add_dakuon(c: &char) -> char {
    let codepoint = *c as u32;
    match codepoint {
        0x30ab..=0x30c2 if codepoint % 2 == 1 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        0x30c4..=0x30c9 if codepoint % 2 == 0 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        0x30cf..=0x30dd if codepoint % 3 == 0 => unsafe { char::from_u32_unchecked(codepoint + 1) },
        _ => *c,
    }
}

fn katakana_remove_dakuon(c: &char) -> char {
    let codepoint = *c as u32;
    match codepoint {
        0x30ab..=0x30c2 if codepoint % 2 == 0 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        0x30c4..=0x30c9 if codepoint % 2 == 1 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        0x30cf..=0x30dd if codepoint % 3 == 1 => unsafe { char::from_u32_unchecked(codepoint - 1) },
        _ => *c,
    }
}

#[derive(Clone, Debug)]
pub struct JapaneseIterationMarkCharacterFilter {
    pub normalize_kanji: bool,
    pub normalize_kana: bool,
}

impl JapaneseIterationMarkCharacterFilter {
    pub fn new(normalize_kanji: bool, normalize_kana: bool) -> Self {
        Self {
            normalize_kanji,
            normalize_kana,
        }
    }

    pub fn from_config(config: &JapaneseIterationMarkCharacterFilterConfig) -> LinderaResult<Self> {
        let normalize_kanji = config
            .get("normalize_kanji")
            .map_or(Ok(false), |v| v.as_bool().ok_or(LinderaErrorKind::Args))
            .unwrap_or(false);
        let normalize_kana = config
            .get("normalize_kana")
            .map_or(Ok(false), |v| v.as_bool().ok_or(LinderaErrorKind::Args))
            .unwrap_or(false);

        Ok(Self::new(normalize_kanji, normalize_kana))
    }

    /// Normalizes iteration marks in the text and returns the normalized string.
    ///
    /// # Arguments
    ///
    /// * `iter_marks` - A `BTreeMap` that contains the positions and corresponding iteration mark characters (`&char`) that need to be normalized.
    /// * `text_chars` - A reference to a slice of characters (`&[char]`) representing the original text from which normalization should occur.
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the normalized version of the text, where the iteration marks have been replaced by their corresponding normalized forms.
    ///
    /// # Process
    ///
    /// 1. **Iterating through iteration marks**:
    ///    - The function loops through the `iter_marks` to identify and process each iteration mark.
    ///    - The position of each iteration mark is compared to the previous characters in the text to generate a normalized version.
    ///
    /// 2. **Character Replacement**:
    ///    - Depending on the type of iteration mark (`KANJI_ITERATION_MARK`, `HIRAGANA_ITERATION_MARK`, etc.), the corresponding normalized character is chosen from `text_chars`.
    ///    - For kanji iteration marks, the character at the calculated position is replaced if `normalize_kanji` is enabled in the configuration.
    ///    - For hiragana and katakana iteration marks, the function determines whether dakuon (voiced sound mark) should be added or removed, based on the mark type and the `normalize_kana` flag.
    ///
    /// 3. **Handling Position Differences**:
    ///    - The function calculates the position difference (`pos_diff`) between the first iteration mark and the number of iteration marks, ensuring that the correct character is used as the replacement.
    ///
    /// 4. **Final String Construction**:
    ///    - For each character or iteration mark, the corresponding normalized form (or the original character if no normalization is required) is appended to `normalized_str`.
    ///
    /// # Special Cases
    ///
    /// - If a character for replacement is out of bounds in `text_chars`, the original iteration mark is used as the replacement to avoid panicking.
    ///
    /// # Errors
    ///
    /// This function does not return errors but instead falls back to using the iteration mark itself if the expected replacement character is out of bounds.
    fn normalize(&self, iter_marks: &BTreeMap<usize, &char>, text_chars: &[char]) -> String {
        let mut normalized_str = String::new();

        let first_iter_mark_pos = iter_marks.first_key_value().map(|(k, _)| *k).unwrap_or(0);
        let pos_diff = if first_iter_mark_pos < iter_marks.len() {
            first_iter_mark_pos
        } else {
            iter_marks.len()
        };

        for (iter_mark_pos, iter_mark) in iter_marks.iter() {
            let iter_mark_index = *iter_mark_pos - pos_diff;
            match *(*iter_mark) {
                KANJI_ITERATION_MARK if self.normalize_kanji => {
                    let replacement = text_chars.get(iter_mark_index).unwrap_or(iter_mark);
                    normalized_str.push(*replacement);
                }
                HIRAGANA_ITERATION_MARK if self.normalize_kana => {
                    let replacement = text_chars.get(iter_mark_index).unwrap_or(iter_mark);
                    normalized_str.push(hiragana_remove_dakuon(replacement));
                }
                HIRAGANA_DAKUON_ITERATION_MARK if self.normalize_kana => {
                    let replacement = text_chars.get(iter_mark_index).unwrap_or(iter_mark);
                    let replacement = hiragana_add_dakuon(replacement);
                    normalized_str.push(replacement);
                }
                KATAKANA_ITERATION_MARK if self.normalize_kana => {
                    let replacement = text_chars.get(iter_mark_index).unwrap_or(iter_mark);
                    normalized_str.push(katakana_remove_dakuon(replacement));
                }
                KATAKANA_DAKUON_ITERATION_MARK if self.normalize_kana => {
                    let replacement = text_chars.get(iter_mark_index).unwrap_or(iter_mark);
                    let replacement = katakana_add_dakuon(replacement);
                    normalized_str.push(replacement);
                }
                _ => {
                    normalized_str.push(**iter_mark);
                }
            }
        }

        normalized_str
    }
}

impl CharacterFilter for JapaneseIterationMarkCharacterFilter {
    fn name(&self) -> &'static str {
        JAPANESE_ITERATION_MARK_CHARACTER_FILTER_NAME
    }

    /// Apply the filter using the OffsetMapping API
    fn apply(&self, text: &mut String) -> LinderaResult<OffsetMapping> {
        
        let mut filtered_text = String::with_capacity(text.len());
        let mut mapping = OffsetMapping::new();
        
        let text_chars = text.chars().collect::<Vec<char>>();
        let mut iter_marks = BTreeMap::new();
        let mut byte_pos = 0_usize;
        
        for (i, c) in text_chars.iter().enumerate() {
            let char_byte_len = c.len_utf8();
            
            match c {
                &KANJI_ITERATION_MARK if self.normalize_kanji => {
                    iter_marks.insert(i, c);
                }
                &HIRAGANA_ITERATION_MARK
                | &HIRAGANA_DAKUON_ITERATION_MARK
                | &KATAKANA_ITERATION_MARK
                | &KATAKANA_DAKUON_ITERATION_MARK
                    if self.normalize_kana =>
                {
                    iter_marks.insert(i, c);
                }
                _ => {
                    if !iter_marks.is_empty() {
                        let normalized_text = self.normalize(&iter_marks, &text_chars);
                        let original_len: usize = iter_marks.keys().map(|&idx| text_chars[idx].len_utf8()).sum();
                        let replacement_len = normalized_text.len();
                        
                        // Record transformation if text changed
                        if original_len != replacement_len {
                            let transformation = Transformation::new(
                                byte_pos,
                                byte_pos + original_len,
                                filtered_text.len(),
                                filtered_text.len() + replacement_len,
                            );
                            mapping.add_transformation(transformation);
                        }
                        
                        filtered_text.push_str(&normalized_text);
                        byte_pos += original_len;
                        iter_marks.clear();
                    }
                    filtered_text.push(*c);
                    byte_pos += char_byte_len;
                }
            }
        }

        // Handle remaining iteration marks at the end
        if !iter_marks.is_empty() {
            let normalized_text = self.normalize(&iter_marks, &text_chars);
            let original_len: usize = iter_marks.keys().map(|&idx| text_chars[idx].len_utf8()).sum();
            let replacement_len = normalized_text.len();
            
            // Record transformation if text changed
            if original_len != replacement_len {
                let transformation = Transformation::new(
                    byte_pos,
                    byte_pos + original_len,
                    filtered_text.len(),
                    filtered_text.len() + replacement_len,
                );
                mapping.add_transformation(transformation);
            }
            
            filtered_text.push_str(&normalized_text);
        }

        *text = filtered_text;
        Ok(mapping)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use once_cell::sync::Lazy;

    use crate::character_filter::CharacterFilter;
    use crate::character_filter::japanese_iteration_mark::{
        JapaneseIterationMarkCharacterFilter, JapaneseIterationMarkCharacterFilterConfig,
        hiragana_add_dakuon, hiragana_remove_dakuon, katakana_add_dakuon, katakana_remove_dakuon,
    };

    fn hiragana_has_dakuon(c: &char) -> bool {
        let codepoint = *c as u32;
        // か…ぢ
        ((0x304b..=0x3062).contains(&codepoint) && codepoint % 2 == 0) ||
        // つ…ど
        ((0x3064..=0x3069).contains(&codepoint) && codepoint % 2 == 1) ||
        // は…ぽ
        ((0x306f..=0x307d).contains(&codepoint) && codepoint % 3 == 1)
    }

    fn katakana_has_dakuon(c: &char) -> bool {
        let codepoint = *c as u32;
        // カ…ヂ
        ((0x30ab..=0x30c2).contains(&codepoint) && codepoint % 2 == 0) ||
        // ツ…ド
        ((0x30c4..=0x30c9).contains(&codepoint) && codepoint % 2 == 1) ||
        // ハ…ポ
        ((0x30cf..=0x30dd).contains(&codepoint) && codepoint % 3 == 1)
    }

    static HIRAGANA_DAKUON_MAP: Lazy<HashMap<char, char>> = Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert('か', 'が');
        m.insert('が', 'が');
        m.insert('き', 'ぎ');
        m.insert('ぎ', 'ぎ');
        m.insert('く', 'ぐ');
        m.insert('ぐ', 'ぐ');
        m.insert('け', 'げ');
        m.insert('げ', 'げ');
        m.insert('こ', 'ご');
        m.insert('ご', 'ご');
        m.insert('さ', 'ざ');
        m.insert('ざ', 'ざ');
        m.insert('し', 'じ');
        m.insert('じ', 'じ');
        m.insert('す', 'ず');
        m.insert('ず', 'ず');
        m.insert('せ', 'ぜ');
        m.insert('ぜ', 'ぜ');
        m.insert('そ', 'ぞ');
        m.insert('ぞ', 'ぞ');
        m.insert('た', 'だ');
        m.insert('だ', 'だ');
        m.insert('ち', 'ぢ');
        m.insert('ぢ', 'ぢ');
        m.insert('つ', 'づ');
        m.insert('づ', 'づ');
        m.insert('て', 'で');
        m.insert('で', 'で');
        m.insert('と', 'ど');
        m.insert('ど', 'ど');
        m.insert('は', 'ば');
        m.insert('ば', 'ば');
        m.insert('ひ', 'び');
        m.insert('び', 'び');
        m.insert('ふ', 'ぶ');
        m.insert('ぶ', 'ぶ');
        m.insert('へ', 'べ');
        m.insert('べ', 'べ');
        m.insert('ほ', 'ぼ');
        m.insert('ぼ', 'ぼ');
        m
    });

    static KATAKANA_DAKUON_MAP: Lazy<HashMap<char, char>> = Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert('カ', 'ガ');
        m.insert('ガ', 'ガ');
        m.insert('キ', 'ギ');
        m.insert('ギ', 'ギ');
        m.insert('ク', 'グ');
        m.insert('グ', 'グ');
        m.insert('ケ', 'ゲ');
        m.insert('ゲ', 'ゲ');
        m.insert('コ', 'ゴ');
        m.insert('ゴ', 'ゴ');
        m.insert('サ', 'ザ');
        m.insert('ザ', 'ザ');
        m.insert('シ', 'ジ');
        m.insert('ジ', 'ジ');
        m.insert('ス', 'ズ');
        m.insert('ズ', 'ズ');
        m.insert('セ', 'ゼ');
        m.insert('ゼ', 'ゼ');
        m.insert('ソ', 'ゾ');
        m.insert('ゾ', 'ゾ');
        m.insert('タ', 'ダ');
        m.insert('ダ', 'ダ');
        m.insert('チ', 'ヂ');
        m.insert('ヂ', 'ヂ');
        m.insert('ツ', 'ヅ');
        m.insert('ヅ', 'ヅ');
        m.insert('テ', 'デ');
        m.insert('デ', 'デ');
        m.insert('ト', 'ド');
        m.insert('ド', 'ド');
        m.insert('ハ', 'バ');
        m.insert('バ', 'バ');
        m.insert('ヒ', 'ビ');
        m.insert('ビ', 'ビ');
        m.insert('フ', 'ブ');
        m.insert('ブ', 'ブ');
        m.insert('ヘ', 'ベ');
        m.insert('ベ', 'ベ');
        m.insert('ホ', 'ボ');
        m.insert('ボ', 'ボ');
        m
    });

    #[test]
    fn test_japanese_iteration_mark_character_filter_config() {
        let config_str = r#"
        {
            "normalize_kanji": true,
            "normalize_kana": true
        }
        "#;
        let result: Result<JapaneseIterationMarkCharacterFilterConfig, _> =
            serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_japanese_iteration_mark_character_filter_from_config() {
        let config_str = r#"
        {
            "normalize_kanji": true,
            "normalize_kana": true
        }
        "#;
        let config =
            serde_json::from_str::<JapaneseIterationMarkCharacterFilterConfig>(config_str).unwrap();

        let result = JapaneseIterationMarkCharacterFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_japanese_iteration_mark_character_filter_apply() {
        let config_str = r#"
        {
            "normalize_kanji": true,
            "normalize_kana": true
        }
        "#;

        let config =
            serde_json::from_str::<JapaneseIterationMarkCharacterFilterConfig>(config_str).unwrap();

        let filter = JapaneseIterationMarkCharacterFilter::from_config(&config).unwrap();

        {
            let original_text = "ここは騒々しい";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ここは騒騒しい", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 21);
        }

        {
            let original_text = "祇園 さゝ木";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("祇園 ささ木", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 16);
        }

        {
            let original_text = "いすゞ自動車株式会社";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("いすず自動車株式会社", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 30);
        }

        {
            let original_text = "サヽキ印刷";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ササキ印刷", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 15);
        }

        {
            let original_text = "愛知県岡崎市牧平町マカヾイツ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("愛知県岡崎市牧平町マカガイツ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 42);
        }

        {
            let original_text = "馬鹿々々しい";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("馬鹿馬鹿しい", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 18);
        }

        {
            let original_text = "ところゞゝゝ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ところどころ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 18);
        }

        {
            let original_text = "じゝ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("じし", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 6);
        }

        {
            let original_text = "じゞ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("じじ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 6);
        }

        {
            let original_text = "ジヽ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ジシ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 6);
        }

        {
            let original_text = "ジヾ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ジジ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 6);
        }

        {
            let original_text = "ところゞゝゝゞゝゝ";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ところどころゞゝゝ", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 27);
        }

        {
            let original_text = "ところゞゝゝ馬鹿々々しく騒々しい";
            let mut text = original_text.to_string();
            let mapping = filter.apply(&mut text).unwrap();
            let (offsets, diffs, text_len) = mapping.to_legacy_format(text.len());
            assert_eq!("ところどころ馬鹿馬鹿しく騒騒しい", text);
            assert!(offsets.is_empty());
            assert!(diffs.is_empty());
            assert_eq!(text_len, 48);
        }
    }

    #[test]
    fn test_katakana_has_dakuon() {
        for (k, v) in KATAKANA_DAKUON_MAP.iter() {
            if *k == *v {
                assert!(katakana_has_dakuon(v));
            } else {
                assert!(!katakana_has_dakuon(k));
                assert!(katakana_has_dakuon(v));
            }
        }
    }

    #[test]
    fn test_katakana_add_dakuon() {
        for (k, v) in KATAKANA_DAKUON_MAP.iter() {
            if *k == *v {
                assert_eq!(katakana_add_dakuon(v), *v);
            } else {
                assert_eq!(katakana_add_dakuon(k), *v, "{k}->{v}");
            }
        }
    }

    #[test]
    fn test_katakana_remove_dakuon() {
        for (k, v) in KATAKANA_DAKUON_MAP.iter() {
            if *k != *v {
                assert_eq!(katakana_remove_dakuon(v), *k);
                assert_eq!(katakana_remove_dakuon(k), *k);
            } else {
                assert_ne!(katakana_remove_dakuon(v), *v);
            }
        }
    }

    #[test]
    fn test_hiragana_has_dakuon() {
        for (k, v) in HIRAGANA_DAKUON_MAP.iter() {
            if *k == *v {
                assert!(hiragana_has_dakuon(v));
            } else {
                assert!(!hiragana_has_dakuon(k));
                assert!(hiragana_has_dakuon(v));
            }
        }
    }

    #[test]
    fn test_hiragana_add_dakuon() {
        for (k, v) in HIRAGANA_DAKUON_MAP.iter() {
            if *k == *v {
                assert_eq!(hiragana_add_dakuon(v), *v);
            } else {
                assert_eq!(hiragana_add_dakuon(k), *v);
            }
        }
    }

    #[test]
    fn test_hiragana_remove_dakuon() {
        for (k, v) in HIRAGANA_DAKUON_MAP.iter() {
            if *k != *v {
                assert_eq!(hiragana_remove_dakuon(v), *k);
                assert_eq!(hiragana_remove_dakuon(k), *k);
            } else {
                assert_ne!(hiragana_remove_dakuon(v), *v);
            }
        }
    }
}
