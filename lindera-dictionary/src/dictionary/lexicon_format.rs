use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// The format of a dictionary's lexicon CSV files, which its `metadata.json`
/// selects with `lexicon_format` (#1389). It decides how the builder reads
/// the fields of the system dictionary's lexicon and of the user
/// dictionaries built for it.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum LexiconFormat {
    /// MeCab's format, the default: every field is taken as written.
    #[default]
    Mecab,
    /// Sudachi's format: a backslash followed by `u` and four hex digits, or
    /// by `u` and one to six hex digits in braces, stands for the code point
    /// they name, as in the lexicons of SudachiDict, which write `(`, `)`,
    /// `/`, `,`, `|` and `"` that way. The builder decodes every field of a
    /// row as Sudachi's dictionary builder (sudachi.rs `v0.7.0`) decodes its
    /// string fields: left to right, without decoding the text an escape
    /// stands for again, and with no escape for the backslash itself.
    Sudachi,
}

impl LexiconFormat {
    /// Returns whether this is MeCab's format, the default, which
    /// `metadata.json` leaves out.
    ///
    /// # Returns
    ///
    /// `true` for [`LexiconFormat::Mecab`].
    pub fn is_mecab(&self) -> bool {
        *self == LexiconFormat::Mecab
    }
}

#[cfg(test)]
mod tests {
    use super::LexiconFormat;

    #[test]
    fn mecab_is_the_default() {
        assert_eq!(LexiconFormat::default(), LexiconFormat::Mecab);
        assert!(LexiconFormat::Mecab.is_mecab());
        assert!(!LexiconFormat::Sudachi.is_mecab());
    }

    /// The JSON names are the lower-case variant names.
    #[test]
    fn serializes_in_lower_case() {
        assert_eq!(
            serde_json::to_string(&LexiconFormat::Sudachi).unwrap(),
            "\"sudachi\""
        );
        assert_eq!(
            serde_json::from_str::<LexiconFormat>("\"mecab\"").unwrap(),
            LexiconFormat::Mecab
        );
        assert!(serde_json::from_str::<LexiconFormat>("\"Sudachi\"").is_err());
    }
}
