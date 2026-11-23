//! The schema of a dictionary: which feature column holds what.
//!
//! The feature columns of a morpheme differ from one dictionary to another.
//! What reads them by role (the chunker, and later the parser and the
//! output) takes the column numbers from a [`Schema`], and holds none of its
//! own.

use std::ops::Range;

/// Which feature columns of a dictionary hold the part of speech, the
/// conjugation and the base form.
///
/// Columns are counted from 0, as [`Morpheme::feature`](crate::Morpheme::feature)
/// does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// The columns of the part of speech, from the coarsest level to the
    /// finest.
    pub part_of_speech: Range<usize>,
    /// The column of the conjugation type, when the dictionary has one.
    pub conjugation_type: Option<usize>,
    /// The column of the conjugation form, when the dictionary has one.
    pub conjugation_form: Option<usize>,
    /// The column of the base form, when the dictionary has one.
    pub base_form: Option<usize>,
}

impl Schema {
    /// The schema of IPADIC: four levels of part of speech, then the
    /// conjugation type, the conjugation form and the base form.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn ipadic() -> Self {
        Self {
            part_of_speech: 0..4,
            conjugation_type: Some(4),
            conjugation_form: Some(5),
            base_form: Some(6),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipadic_names_the_columns_of_mecab_ipadic() {
        // 読ん: 動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
        let features = [
            "動詞",
            "自立",
            "*",
            "*",
            "五段・マ行",
            "連用タ接続",
            "読む",
            "ヨン",
            "ヨン",
        ];
        let schema = Schema::ipadic();
        assert_eq!(
            features[schema.part_of_speech.clone()],
            ["動詞", "自立", "*", "*"]
        );
        assert_eq!(features[schema.conjugation_type.unwrap()], "五段・マ行");
        assert_eq!(features[schema.conjugation_form.unwrap()], "連用タ接続");
        assert_eq!(features[schema.base_form.unwrap()], "読む");
    }
}
