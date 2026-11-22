//! The morpheme trait, and the feature string of a morpheme.

use std::borrow::Cow;

/// A morpheme: a surface and the feature columns of its dictionary.
///
/// The crate works on any sequence of morphemes, so a type outside Lindera
/// can implement this trait. What a column means depends on the dictionary:
/// in IPADIC, column 0 is the part of speech and column 6 the base form.
///
/// # Examples
///
/// ```
/// use lindera_dependency::Morpheme;
///
/// struct Word {
///     surface: &'static str,
///     features: [&'static str; 2],
/// }
///
/// impl Morpheme for Word {
///     fn surface(&self) -> &str {
///         self.surface
///     }
///
///     fn feature(&self, index: usize) -> Option<&str> {
///         self.features.get(index).copied()
///     }
/// }
///
/// let word = Word { surface: "本", features: ["名詞", "一般"] };
/// assert_eq!(word.feature(1), Some("一般"));
/// assert_eq!(word.features(), "名詞,一般");
/// ```
pub trait Morpheme {
    /// The surface of the morpheme.
    ///
    /// # Returns
    ///
    /// The surface, as it stands in the text.
    fn surface(&self) -> &str;

    /// A feature column of the morpheme.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the column, from 0.
    ///
    /// # Returns
    ///
    /// The column, or `None` past the last one. The columns have no gap: when
    /// an index gives `None`, so does every larger one.
    fn feature(&self, index: usize) -> Option<&str>;

    /// The feature columns as the one string that follows the surface in the
    /// lattice format.
    ///
    /// The default joins the columns with commas, and quotes a column that
    /// holds a comma or a double quote, as CSV does. An implementation that
    /// has the string as it was read returns that instead, so that writing a
    /// sentence gives back the bytes it was read from.
    ///
    /// # Returns
    ///
    /// The feature string.
    fn features(&self) -> Cow<'_, str> {
        let mut joined = String::new();
        let mut index = 0;
        while let Some(column) = self.feature(index) {
            if index > 0 {
                joined.push(',');
            }
            push_column(&mut joined, column);
            index += 1;
        }
        Cow::Owned(joined)
    }

    /// The named entity tag of the morpheme: the third column of its line in
    /// the lattice format.
    ///
    /// # Returns
    ///
    /// The tag, or `None` (the default) for a morpheme without one.
    fn named_entity(&self) -> Option<&str> {
        None
    }
}

/// Appends a feature column to a feature string, quoted when it holds a comma
/// or a double quote.
///
/// # Arguments
///
/// * `joined` - The feature string so far.
/// * `column` - The column to append.
fn push_column(joined: &mut String, column: &str) {
    if column.contains([',', '"']) {
        joined.push('"');
        for c in column.chars() {
            if c == '"' {
                joined.push('"');
            }
            joined.push(c);
        }
        joined.push('"');
    } else {
        joined.push_str(column);
    }
}

/// Splits a feature string into its columns, as a row of CSV.
///
/// A column that starts with a double quote runs to the closing quote, and
/// two quotes inside it stand for one, so that a quoted comma does not end
/// the column (MeCab writes the columns of its dictionary's CSV this way). A
/// quote elsewhere, and an opening quote without a closing one, are kept as
/// they are: the function gives columns for any string.
///
/// # Arguments
///
/// * `features` - The feature string.
///
/// # Returns
///
/// The columns. The empty string has none.
pub(crate) fn split_features(features: &str) -> Vec<String> {
    let mut columns = Vec::new();
    if features.is_empty() {
        return columns;
    }
    let mut rest = features;
    loop {
        let (column, after) = split_column(rest);
        columns.push(column);
        match after {
            Some(after) => rest = after,
            None => break,
        }
    }
    columns
}

/// Takes the first column of a feature string.
///
/// # Arguments
///
/// * `rest` - The feature string from the start of a column.
///
/// # Returns
///
/// The column, and what follows its comma, or `None` when the column is the
/// last one.
fn split_column(rest: &str) -> (String, Option<&str>) {
    if let Some(quoted) = rest.strip_prefix('"')
        && let Some((column, after)) = split_quoted(quoted)
    {
        return (column, after);
    }
    match rest.split_once(',') {
        Some((column, after)) => (column.to_string(), Some(after)),
        None => (rest.to_string(), None),
    }
}

/// Takes a quoted column, from after its opening quote.
///
/// # Arguments
///
/// * `quoted` - The feature string from after the opening quote.
///
/// # Returns
///
/// The column without its quotes, and what follows its comma (`None` for the
/// last column); or `None` when there is no closing quote, or when the
/// closing quote is followed by something other than a comma.
fn split_quoted(quoted: &str) -> Option<(String, Option<&str>)> {
    let mut column = String::new();
    let mut chars = quoted.char_indices();
    while let Some((at, c)) = chars.next() {
        if c != '"' {
            column.push(c);
            continue;
        }
        let after = &quoted[at + 1..];
        if after.starts_with('"') {
            column.push('"');
            chars.next();
        } else if after.is_empty() {
            return Some((column, None));
        } else {
            let after = after.strip_prefix(',')?;
            return Some((column, Some(after)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Columns(Vec<&'static str>);

    impl Morpheme for Columns {
        fn surface(&self) -> &str {
            "surface"
        }

        fn feature(&self, index: usize) -> Option<&str> {
            self.0.get(index).copied()
        }
    }

    #[test]
    fn splits_plain_columns() {
        assert_eq!(split_features("名詞,一般,*"), ["名詞", "一般", "*"]);
        assert_eq!(split_features("名詞"), ["名詞"]);
        assert!(split_features("").is_empty());
    }

    #[test]
    fn keeps_empty_columns() {
        assert_eq!(split_features("a,,b,"), ["a", "", "b", ""]);
        assert_eq!(split_features(","), ["", ""]);
    }

    #[test]
    fn a_quoted_comma_does_not_end_the_column() {
        assert_eq!(
            split_features(r#"記号,読点,*,*,*,*,",",",",",""#),
            ["記号", "読点", "*", "*", "*", "*", ",", ",", ","]
        );
    }

    #[test]
    fn two_quotes_in_a_quoted_column_are_one() {
        assert_eq!(split_features(r#""a""b",c"#), ["a\"b", "c"]);
        assert_eq!(split_features(r#""""""#), ["\""]);
        assert_eq!(split_features(r#""a""""#), ["a\""]);
    }

    #[test]
    fn an_unquoted_comma_row_has_more_columns() {
        // What `lindera tokenize -o mecab` writes for the morpheme `,`.
        assert_eq!(
            split_features("記号,読点,*,*,*,*,,,,,,"),
            ["記号", "読点", "*", "*", "*", "*", "", "", "", "", "", ""]
        );
    }

    #[test]
    fn a_quote_that_does_not_quote_a_column_is_kept() {
        assert_eq!(split_features(r#"a"b,c"#), ["a\"b", "c"]);
        assert_eq!(split_features(r#""abc"#), ["\"abc"]);
        assert_eq!(split_features(r#""a"b,c"#), ["\"a\"b", "c"]);
    }

    #[test]
    fn the_default_feature_string_joins_the_columns() {
        assert_eq!(Columns(vec!["名詞", "一般", "*"]).features(), "名詞,一般,*");
        assert_eq!(Columns(vec![]).features(), "");
    }

    #[test]
    fn the_default_feature_string_quotes_commas_and_quotes() {
        let morpheme = Columns(vec!["記号", ",", "a\"b", ""]);
        let features = morpheme.features();
        assert_eq!(features, r#"記号,",","a""b","#);
        assert_eq!(split_features(&features), ["記号", ",", "a\"b", ""]);
    }

    #[test]
    fn a_morpheme_has_no_named_entity_by_default() {
        assert_eq!(Columns(vec!["名詞"]).named_entity(), None);
    }
}
