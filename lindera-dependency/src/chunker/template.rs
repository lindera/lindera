//! Feature templates in the format of CRF++.
//!
//! A template says which feature strings a position of a sentence has. Each
//! unigram line, such as `U05:%x[-1,0]/%x[0,0]`, gives one string: its text
//! with every `%x[row,column]` replaced by the value of a column of the
//! morpheme `row` positions away. A position before the sentence gives
//! `_B-1`, `_B-2`, ..., and one after it `_B+1`, `_B+2`, ..., as in CRF++.
//! The line `B` asks for the bigram of labels.

use std::fmt;

use crate::error::{Error, Result};

/// The template of CaboCha's chunker (`chunk_crfpp_template`): the surface
/// (column 0) and the part of speech (column 1) of the two morphemes before
/// and after, alone and in pairs and triples of neighbors, and the bigram of
/// labels.
const CABOCHA: &str = "\
U00:%x[-2,0]
U01:%x[-1,0]
U02:%x[0,0]
U03:%x[1,0]
U04:%x[2,0]
U05:%x[-1,0]/%x[0,0]
U06:%x[0,0]/%x[1,0]
U10:%x[-2,1]
U11:%x[-1,1]
U12:%x[0,1]
U13:%x[1,1]
U14:%x[2,1]
U15:%x[-2,1]/%x[-1,1]
U16:%x[-1,1]/%x[0,1]
U17:%x[0,1]/%x[1,1]
U18:%x[1,1]/%x[2,1]
U20:%x[-2,1]/%x[-1,1]/%x[0,1]
U21:%x[-1,1]/%x[0,1]/%x[1,1]
U22:%x[0,1]/%x[1,1]/%x[2,1]
B
";

/// What the default template adds to CaboCha's: the conjugation form (column
/// 2) of the morpheme, of the one before and of the one after, and the part
/// of speech with the conjugation form, for the morpheme and for the one
/// before. Chosen by cross-validation on UD Japanese-GSD (#1421).
const CONJUGATION_FORM: &str = "\
U31:%x[-1,2]
U32:%x[0,2]
U33:%x[1,2]
U50:%x[0,1]/%x[0,2]
U51:%x[-1,1]/%x[-1,2]
";

/// What the template for UniDic adds to CaboCha's: the conjugation form
/// (column 2) with the lines the template of CaboCha for named entities has
/// for its third column, and the base form (column 3) of the morpheme, of
/// the one before and of the one after, and with the part of speech. Chosen
/// by cross-validation on UD Japanese-GSD (#1441).
const UNIDIC: &str = "\
U30:%x[-2,2]
U31:%x[-1,2]
U32:%x[0,2]
U33:%x[1,2]
U34:%x[2,2]
U35:%x[-2,2]/%x[-1,2]
U36:%x[-1,2]/%x[0,2]
U37:%x[0,2]/%x[1,2]
U38:%x[1,2]/%x[2,2]
U40:%x[-2,2]/%x[-1,2]/%x[0,2]
U41:%x[-1,2]/%x[0,2]/%x[1,2]
U42:%x[0,2]/%x[1,2]/%x[2,2]
U50:%x[0,1]/%x[0,2]
U61:%x[-1,3]
U62:%x[0,3]
U63:%x[1,3]
U70:%x[0,1]/%x[0,3]
";

/// A piece of a unigram line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    /// Text copied as it is.
    Text(String),
    /// `%x[row,column]`: the value of a column of the morpheme `row`
    /// positions away.
    Cell {
        /// The offset of the row from the position.
        row: isize,
        /// The column.
        column: usize,
    },
}

/// A feature template: unigram lines and the bigram of labels.
///
/// [`fmt::Display`] writes it back in the format of CRF++.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// The unigram lines, each as its pieces.
    unigrams: Vec<Vec<Part>>,
}

impl Template {
    /// Reads a template in the format of CRF++.
    ///
    /// Empty lines and lines that start with `#` are skipped. A line that
    /// starts with `U` is a unigram line; the line `B` is the bigram of
    /// labels, which the chunker always has, so a template without it is
    /// refused. Bigram lines with features (`B01:%x[0,0]`) are not
    /// supported.
    ///
    /// # Arguments
    ///
    /// * `text` - The template.
    ///
    /// # Returns
    ///
    /// The template, or [`Error::Chunker`] with the number of the line that
    /// cannot be read.
    pub fn parse(text: &str) -> Result<Self> {
        let mut unigrams = Vec::new();
        let mut bigram = false;
        for (index, line) in text.lines().enumerate() {
            let number = index + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "B" {
                bigram = true;
            } else if line.starts_with('U') {
                unigrams.push(parse_line(line, number)?);
            } else {
                return Err(Error::Chunker(format!(
                    "template line {number}: `{line}` is neither a unigram line (U...) nor `B`"
                )));
            }
        }
        if !bigram {
            return Err(Error::Chunker(
                "the template has no `B` line for the bigram of labels".to_string(),
            ));
        }
        if unigrams.is_empty() {
            return Err(Error::Chunker(
                "the template has no unigram line".to_string(),
            ));
        }
        Ok(Self { unigrams })
    }

    /// The template of CaboCha's chunker, over two columns: the surface and
    /// the part of speech.
    ///
    /// # Returns
    ///
    /// The template.
    pub fn cabocha() -> Self {
        Self::constant(CABOCHA.trim_end_matches("B\n"))
    }

    /// The template the chunker is trained with for UniDic: the one of
    /// CaboCha over the surface and the part of speech, the conjugation
    /// form as a third column and the base form as a fourth. The base form
    /// of UniDic is the lexeme (語彙素), which is the same for the ways a
    /// word is written; on UD Japanese-GSD with UniDic the template is
    /// significantly better than the default one (#1441).
    ///
    /// # Returns
    ///
    /// The template.
    pub fn unidic() -> Self {
        let mut template = Self::cabocha();
        template.unigrams.extend(Self::constant(UNIDIC).unigrams);
        template
    }

    /// The lines of a template, read without a check: for the constants of
    /// this module, which the tests read with [`Template::parse`].
    ///
    /// # Arguments
    ///
    /// * `text` - Unigram lines.
    ///
    /// # Returns
    ///
    /// The template; the lines that cannot be read are left out.
    fn constant(text: &str) -> Self {
        Self {
            unigrams: text
                .lines()
                .enumerate()
                .filter_map(|(index, line)| parse_line(line, index + 1).ok())
                .collect(),
        }
    }

    /// The number of columns the template needs: one more than the largest
    /// column it reads.
    ///
    /// # Returns
    ///
    /// The number of columns; 0 for a template that reads none.
    pub fn columns(&self) -> usize {
        self.unigrams
            .iter()
            .flatten()
            .filter_map(|part| match part {
                Part::Cell { column, .. } => Some(column + 1),
                Part::Text(_) => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// The number of unigram lines: the number of feature strings of a
    /// position.
    ///
    /// # Returns
    ///
    /// The number of lines.
    pub fn len(&self) -> usize {
        self.unigrams.len()
    }

    /// Whether the template has no unigram line.
    ///
    /// # Returns
    ///
    /// `true` when it has none.
    pub fn is_empty(&self) -> bool {
        self.unigrams.is_empty()
    }

    /// The feature strings of a position.
    ///
    /// # Arguments
    ///
    /// * `rows` - The columns of each morpheme of the sentence. Every row
    ///   has to hold at least [`Template::columns`] values.
    /// * `position` - The position, an index into `rows`.
    ///
    /// # Returns
    ///
    /// One string per unigram line, in the order of the lines.
    pub fn expand(&self, rows: &[Vec<String>], position: usize) -> Vec<String> {
        self.unigrams
            .iter()
            .map(|parts| {
                let mut feature = String::new();
                for part in parts {
                    match part {
                        Part::Text(text) => feature.push_str(text),
                        Part::Cell { row, column } => {
                            push_cell(&mut feature, rows, position, *row, *column);
                        }
                    }
                }
                feature
            })
            .collect()
    }
}

impl Default for Template {
    /// The template the chunker is trained with unless another is given:
    /// the one of CaboCha, and the conjugation form as a third column, for
    /// the morpheme, the one before and the one after, and with the part of
    /// speech. On UD Japanese-GSD with IPADIC it is significantly better
    /// than CaboCha's alone (#1421).
    fn default() -> Self {
        let mut template = Self::cabocha();
        template
            .unigrams
            .extend(Self::constant(CONJUGATION_FORM).unigrams);
        template
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for parts in &self.unigrams {
            for part in parts {
                match part {
                    Part::Text(text) => f.write_str(text)?,
                    Part::Cell { row, column } => write!(f, "%x[{row},{column}]")?,
                }
            }
            f.write_str("\n")?;
        }
        f.write_str("B\n")
    }
}

/// Appends the value of a cell to a feature string.
///
/// # Arguments
///
/// * `feature` - The string to append to.
/// * `rows` - The columns of each morpheme of the sentence.
/// * `position` - The position the feature is made for.
/// * `row` - The offset of the row from the position.
/// * `column` - The column.
fn push_cell(
    feature: &mut String,
    rows: &[Vec<String>],
    position: usize,
    row: isize,
    column: usize,
) {
    let index = position as isize + row;
    if index < 0 {
        // `_B-1` for the position just before the sentence.
        feature.push_str(&format!("_B{index}"));
    } else if index as usize >= rows.len() {
        // `_B+1` for the position just after the sentence.
        feature.push_str(&format!("_B+{}", index as usize - rows.len() + 1));
    } else if let Some(value) = rows[index as usize].get(column) {
        feature.push_str(value);
    }
}

/// Reads a unigram line.
///
/// # Arguments
///
/// * `line` - The line, trimmed.
/// * `number` - The number of the line, from 1.
///
/// # Returns
///
/// The pieces of the line, or [`Error::Chunker`] for a `%x[` that is not
/// followed by a row, a comma, a column and `]`.
fn parse_line(line: &str, number: usize) -> Result<Vec<Part>> {
    const MACRO: &str = "%x[";
    let malformed = |what: &str| {
        Error::Chunker(format!(
            "template line {number}: {what} in `{line}`: expected %x[row,column]"
        ))
    };
    let mut parts = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find(MACRO) {
        if start > 0 {
            parts.push(Part::Text(rest[..start].to_string()));
        }
        let after = &rest[start + MACRO.len()..];
        let end = after.find(']').ok_or_else(|| malformed("no `]`"))?;
        let (row, column) = after[..end]
            .split_once(',')
            .ok_or_else(|| malformed("no comma"))?;
        let row = row
            .trim()
            .parse::<isize>()
            .map_err(|_| malformed("the row is not a number"))?;
        let column = column
            .trim()
            .parse::<usize>()
            .map_err(|_| malformed("the column is not a number"))?;
        parts.push(Part::Cell { row, column });
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        parts.push(Part::Text(rest.to_string()));
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows of two columns, from `(surface, part of speech)`.
    fn rows(values: &[(&str, &str)]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|(surface, pos)| vec![surface.to_string(), pos.to_string()])
            .collect()
    }

    /// The error of a template that cannot be read.
    fn error(text: &str) -> String {
        Template::parse(text).unwrap_err().to_string()
    }

    #[test]
    fn the_template_of_cabocha_has_its_19_lines_and_two_columns() {
        let template = Template::cabocha();
        assert_eq!(template.len(), 19);
        assert!(!template.is_empty());
        assert_eq!(template.columns(), 2);
        // Written back, it is the constant.
        assert_eq!(template.to_string(), CABOCHA);
        assert_eq!(Template::parse(CABOCHA).unwrap(), template);
    }

    #[test]
    fn the_default_template_adds_the_conjugation_form() {
        let template = Template::default();
        assert_eq!(template.len(), 24);
        assert_eq!(template.columns(), 3);
        let text = format!("{}{CONJUGATION_FORM}B\n", CABOCHA.trim_end_matches("B\n"));
        assert_eq!(template.to_string(), text);
        assert_eq!(Template::parse(&text).unwrap(), template);

        let rows = vec![
            vec![
                "読ん".to_string(),
                "動詞-自立".to_string(),
                "連用タ接続".to_string(),
            ],
            vec!["だ".to_string(), "助動詞".to_string(), "基本形".to_string()],
        ];
        let features = template.expand(&rows, 1);
        assert_eq!(
            features[19..],
            [
                "U31:連用タ接続",
                "U32:基本形",
                "U33:_B+1",
                "U50:助動詞/基本形",
                "U51:動詞-自立/連用タ接続"
            ]
        );
    }

    #[test]
    fn the_template_for_unidic_adds_the_conjugation_form_and_the_base_form() {
        let template = Template::unidic();
        // The 19 lines of CaboCha, 13 for the conjugation form and 4 for
        // the base form.
        assert_eq!(template.len(), 36);
        assert_eq!(template.columns(), 4);
        let text = format!("{}{UNIDIC}B\n", CABOCHA.trim_end_matches("B\n"));
        assert_eq!(template.to_string(), text);
        assert_eq!(Template::parse(&text).unwrap(), template);

        let row = |values: [&str; 4]| values.map(str::to_string).to_vec();
        let rows = vec![
            row(["読ん", "動詞-一般", "連用形-撥音便", "読む"]),
            row(["だ", "助動詞", "終止形-一般", "た"]),
        ];
        let features = template.expand(&rows, 1);
        assert_eq!(
            features[31..],
            [
                "U50:助動詞/終止形-一般",
                "U61:読む",
                "U62:た",
                "U63:_B+1",
                "U70:助動詞/た"
            ]
        );
    }

    #[test]
    fn expands_cells_and_text() {
        let template =
            Template::parse("U00:%x[0,0]\nU01:%x[-1,1]/%x[0,1]\nU02:x\nU03:%x[1,0]%x[0,1]!\nB\n")
                .unwrap();
        let rows = rows(&[("彼", "名詞"), ("は", "助詞"), ("読む", "動詞")]);
        assert_eq!(
            template.expand(&rows, 1),
            ["U00:は", "U01:名詞/助詞", "U02:x", "U03:読む助詞!"]
        );
    }

    #[test]
    fn positions_outside_the_sentence_have_names() {
        let template = Template::parse(
            "U00:%x[-2,0]\nU01:%x[-1,0]\nU02:%x[0,0]\nU03:%x[1,0]\nU04:%x[2,0]\nB\n",
        )
        .unwrap();
        let rows = rows(&[("彼", "名詞"), ("は", "助詞"), ("読む", "動詞")]);
        assert_eq!(
            template.expand(&rows, 0),
            ["U00:_B-2", "U01:_B-1", "U02:彼", "U03:は", "U04:読む"]
        );
        assert_eq!(
            template.expand(&rows, 2),
            ["U00:彼", "U01:は", "U02:読む", "U03:_B+1", "U04:_B+2"]
        );
        // A sentence of one morpheme has both sides outside.
        let one = &rows[..1];
        assert_eq!(
            template.expand(one, 0),
            ["U00:_B-2", "U01:_B-1", "U02:彼", "U03:_B+1", "U04:_B+2"]
        );
    }

    #[test]
    fn the_template_of_cabocha_on_a_sentence() {
        let rows = rows(&[("彼", "名詞-代名詞-一般"), ("は", "助詞-係助詞")]);
        let features = Template::cabocha().expand(&rows, 1);
        assert_eq!(features.len(), 19);
        assert_eq!(features[0], "U00:_B-1");
        assert_eq!(features[5], "U05:彼/は");
        assert_eq!(features[16], "U20:_B-1/名詞-代名詞-一般/助詞-係助詞");
        assert_eq!(features[18], "U22:助詞-係助詞/_B+1/_B+2");
    }

    #[test]
    fn skips_comments_and_empty_lines_and_counts_columns() {
        let template = Template::parse("# a comment\n\n  U00:%x[0,3]  \nB\n").unwrap();
        assert_eq!(template.len(), 1);
        assert_eq!(template.columns(), 4);
        assert_eq!(template.to_string(), "U00:%x[0,3]\nB\n");
        assert_eq!(Template::parse("U00:fixed\nB\n").unwrap().columns(), 0);
    }

    #[test]
    fn a_missing_column_expands_to_nothing() {
        let template = Template::parse("U00:%x[0,5]/x\nB\n").unwrap();
        assert_eq!(template.expand(&rows(&[("彼", "名詞")]), 0), ["U00:/x"]);
    }

    #[test]
    fn refuses_a_malformed_template_with_the_line() {
        assert_eq!(
            error("U00:%x[0,0]\nX00:%x[0,0]\nB\n"),
            "template line 2: `X00:%x[0,0]` is neither a unigram line (U...) nor `B`"
        );
        assert_eq!(
            error("U00:%x[0,0]\nB01:%x[0,0]\n"),
            "template line 2: `B01:%x[0,0]` is neither a unigram line (U...) nor `B`"
        );
        assert_eq!(
            error("U00:%x[0,0\nB\n"),
            "template line 1: no `]` in `U00:%x[0,0`: expected %x[row,column]"
        );
        assert_eq!(
            error("B\nU00:%x[0]\n"),
            "template line 2: no comma in `U00:%x[0]`: expected %x[row,column]"
        );
        assert_eq!(
            error("U00:%x[a,0]\nB\n"),
            "template line 1: the row is not a number in `U00:%x[a,0]`: expected %x[row,column]"
        );
        assert_eq!(
            error("U00:%x[0,-1]\nB\n"),
            "template line 1: the column is not a number in `U00:%x[0,-1]`: expected %x[row,column]"
        );
        assert_eq!(
            error("U00:%x[0,0]\n"),
            "the template has no `B` line for the bigram of labels"
        );
        assert_eq!(error("B\n"), "the template has no unigram line");
    }
}
