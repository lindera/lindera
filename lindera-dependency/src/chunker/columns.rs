//! The columns of a morpheme that a template reads.
//!
//! A CRF++ template addresses its input by row and column: the rows are the
//! morphemes of a sentence, and the columns are made here, from the surface
//! and the feature columns of the dictionary, through its [`Schema`].

use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::schema::Schema;

/// The value of a column that the morpheme or its dictionary does not have.
const MISSING: &str = "*";

/// A column of the input of a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Column {
    /// The surface of the morpheme, as the tokenizer gives it.
    Surface,
    /// The part of speech: its levels joined by `-`, up to the first level
    /// that is `*` or missing, as in `名詞-固有名詞-人名-姓`. This is the
    /// second column CaboCha gives its chunker.
    PartOfSpeech,
    /// The conjugation type.
    ConjugationType,
    /// The conjugation form.
    ConjugationForm,
    /// The base form.
    BaseForm,
}

impl Column {
    /// Every column, with its name and its code in the bytes of a model.
    const ALL: [(Column, &'static str, u8); 5] = [
        (Column::Surface, "surface", 0),
        (Column::PartOfSpeech, "pos", 1),
        (Column::ConjugationType, "ctype", 2),
        (Column::ConjugationForm, "cform", 3),
        (Column::BaseForm, "base", 4),
    ];

    /// The name of the column: `surface`, `pos`, `ctype`, `cform` or `base`.
    ///
    /// # Returns
    ///
    /// The name.
    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(column, _, _)| *column == self)
            .map_or("", |(_, name, _)| name)
    }

    /// The column with a name.
    ///
    /// # Arguments
    ///
    /// * `name` - A name that [`Column::name`] gives.
    ///
    /// # Returns
    ///
    /// The column, or `None` for another name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, other, _)| *other == name)
            .map(|(column, _, _)| *column)
    }

    /// The code of the column in the bytes of a model.
    ///
    /// # Returns
    ///
    /// The code.
    pub(crate) fn code(self) -> u8 {
        Self::ALL
            .iter()
            .find(|(column, _, _)| *column == self)
            .map_or(u8::MAX, |(_, _, code)| *code)
    }

    /// The column with a code.
    ///
    /// # Arguments
    ///
    /// * `code` - A code that [`Column::code`] gives.
    ///
    /// # Returns
    ///
    /// The column, or `None` for another code.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, _, other)| *other == code)
            .map(|(column, _, _)| *column)
    }

    /// The value of the column for a morpheme.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morpheme.
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// The value; `*` for a role the schema does not have or a feature
    /// column the morpheme does not have, and an empty string for a part of
    /// speech without a first level.
    pub(crate) fn value<M: Morpheme>(self, schema: &Schema, morpheme: &M) -> String {
        let feature = |index: Option<usize>| {
            index
                .and_then(|index| morpheme.feature(index))
                .unwrap_or(MISSING)
                .to_string()
        };
        match self {
            Column::Surface => morpheme.surface().to_string(),
            Column::PartOfSpeech => schema.part_of_speech_of(morpheme),
            Column::ConjugationType => feature(schema.conjugation_type),
            Column::ConjugationForm => feature(schema.conjugation_form),
            Column::BaseForm => feature(schema.base_form),
        }
    }
}

/// Reads a list of columns.
///
/// # Arguments
///
/// * `text` - The names of the columns, separated by commas, as in
///   `surface,pos,cform`.
///
/// # Returns
///
/// The columns, or [`Error::Chunker`] for a name that is none.
pub fn parse_columns(text: &str) -> Result<Vec<Column>> {
    text.split(',')
        .map(|name| {
            let name = name.trim();
            Column::from_name(name).ok_or_else(|| {
                Error::Chunker(format!(
                    "unknown column `{name}`: expected surface, pos, ctype, cform or base"
                ))
            })
        })
        .collect()
}

/// The columns of every morpheme of a sentence.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morphemes.
/// * `columns` - The columns to make.
/// * `morphemes` - The morphemes.
///
/// # Returns
///
/// For each morpheme, the value of each column.
pub(crate) fn rows<M: Morpheme>(
    schema: &Schema,
    columns: &[Column],
    morphemes: &[M],
) -> Vec<Vec<String>> {
    morphemes
        .iter()
        .map(|morpheme| {
            columns
                .iter()
                .map(|column| column.value(schema, morpheme))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::LatticeMorpheme;

    /// Every column of a morpheme with the given features, with the schema
    /// of IPADIC.
    fn values(surface: &str, features: &str) -> Vec<String> {
        let morpheme = LatticeMorpheme::new(surface, features);
        let all: Vec<Column> = Column::ALL.iter().map(|(column, _, _)| *column).collect();
        rows(&Schema::ipadic(), &all, &[morpheme]).remove(0)
    }

    #[test]
    fn the_columns_of_a_verb() {
        assert_eq!(
            values("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
            ["読ん", "動詞-自立", "五段・マ行", "連用タ接続", "読む"]
        );
    }

    #[test]
    fn the_part_of_speech_stops_at_the_first_missing_level() {
        assert_eq!(
            values("田中", "名詞,固有名詞,人名,姓,*,*,田中,タナカ,タナカ")[1],
            "名詞-固有名詞-人名-姓"
        );
        // A level after `*` is not taken.
        assert_eq!(values("x", "名詞,*,一般,*")[1], "名詞");
        assert_eq!(values("x", "*,一般")[1], "");
    }

    #[test]
    fn the_columns_come_from_the_schema() {
        // A dictionary with two levels of part of speech and the base form
        // right after them.
        let schema = Schema {
            part_of_speech: 0..2,
            conjugation_type: None,
            conjugation_form: None,
            base_form: Some(2),
            ..Schema::default()
        };
        let morpheme = LatticeMorpheme::new("読ん", "動詞,自立,読む,連用");
        let columns = [
            Column::PartOfSpeech,
            Column::ConjugationForm,
            Column::BaseForm,
        ];
        assert_eq!(
            rows(&schema, &columns, &[morpheme]),
            [["動詞-自立", "*", "読む"]]
        );
    }

    #[test]
    fn a_morpheme_with_too_few_columns_does_not_panic() {
        assert_eq!(values("x", "名詞"), ["x", "名詞", "*", "*", "*"]);
        assert_eq!(values("x", ""), ["x", "", "*", "*", "*"]);
    }

    #[test]
    fn columns_have_names_and_codes() {
        for (column, name, code) in Column::ALL {
            assert_eq!(column.name(), name);
            assert_eq!(Column::from_name(name), Some(column));
            assert_eq!(column.code(), code);
            assert_eq!(Column::from_code(code), Some(column));
        }
        assert_eq!(Column::from_name("lemma"), None);
        assert_eq!(Column::from_code(9), None);
    }

    #[test]
    fn reads_a_list_of_columns() {
        assert_eq!(
            parse_columns("surface, pos,cform").unwrap(),
            [
                Column::Surface,
                Column::PartOfSpeech,
                Column::ConjugationForm
            ]
        );
        assert_eq!(
            parse_columns("surface,lemma").unwrap_err().to_string(),
            "unknown column `lemma`: expected surface, pos, ctype, cform or base"
        );
    }
}
