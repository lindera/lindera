//! Feature strings split into columns and joined back as MeCab's learner
//! does (#1184).
//!
//! A feature is the text of a lexicon row after its fourth field, as written
//! (see `LexiconRow::feature`), or the text after the tab of a corpus line.
//! The rewrite rules, the feature templates (`%F[n]`, `%L[n]`, `%R[n]`),
//! unknown-word matching and context-id inference read it column by column.
//! MeCab's learner splits it with `tokenizeCSV` (`src/utils.h`), and so does
//! [`split_feature`], so a `feature.def` or `rewrite.def` written for MeCab
//! reads the same columns. These are not the rules of the `csv` crate, which
//! `lindera build` and [`crate::lexicon_csv`] read lexicon records with; the
//! differences are listed on [`split_feature`].

use std::borrow::Cow;

/// Splits a feature string into its columns, as MeCab's `tokenizeCSV` does.
///
/// - Spaces and tabs before a column are skipped.
/// - A column that then starts with `"` is quoted: it ends at the next `"`
///   that is not doubled, and `""` in it is `"`. The text from the closing
///   quote to the next `,` is dropped. A quote that is never closed takes the
///   rest of the string.
/// - Any other column ends at the next `,`, with the spaces at its end kept.
/// - A `,` at the end of the string does not start an empty last column, and
///   an empty string has no columns.
///
/// So `n,"shop, store",1` has the three columns `n`, `shop, store` and `1`,
/// where a split on `,` gives four. The `csv` crate would keep the spaces
/// before a column and the text after a closing quote, and an empty last
/// column. Unlike MeCab's feature templates, which read at most 64 columns,
/// the number of columns has no limit.
///
/// # Arguments
///
/// * `feature` - The feature string.
///
/// # Returns
///
/// The columns, borrowed from `feature` except a quoted column with `""` in
/// it.
pub(crate) fn split_feature(feature: &str) -> Vec<Cow<'_, str>> {
    let bytes = feature.as_bytes();
    let mut columns = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'"') {
            let (column, end) = quoted_column(feature, pos + 1);
            columns.push(column);
            pos = next_comma(bytes, end);
        } else {
            let end = next_comma(bytes, pos);
            columns.push(Cow::Borrowed(&feature[pos..end]));
            pos = end;
        }
        // Past the `,` that ends the column.
        pos += 1;
    }
    columns
}

/// Returns the position of the first `,` at or after `from`.
///
/// Every byte that [`split_feature`] stops at is ASCII, so each position it
/// slices the feature at is a character boundary.
///
/// # Arguments
///
/// * `bytes` - The bytes of the feature string.
/// * `from` - Where to start looking, at most `bytes.len()`.
///
/// # Returns
///
/// The position of the `,`, or `bytes.len()` when there is none.
fn next_comma(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|&byte| byte == b',')
        .map_or(bytes.len(), |offset| from + offset)
}

/// Reads a quoted column, as `tokenizeCSV` does.
///
/// # Arguments
///
/// * `feature` - The feature string.
/// * `start` - Where the text of the column starts, just after its opening
///   quote.
///
/// # Returns
///
/// The text of the column with `""` read as `"`, and the position just after
/// its closing quote, or the end of `feature` when the quote is never closed.
fn quoted_column(feature: &str, start: usize) -> (Cow<'_, str>, usize) {
    let bytes = feature.as_bytes();
    // The text read so far, kept only once a `""` makes it differ from a
    // slice of `feature`, and where the text not yet copied into it starts.
    let mut owned: Option<String> = None;
    let mut rest = start;
    let mut pos = start;
    let end = loop {
        match bytes[pos..].iter().position(|&byte| byte == b'"') {
            Some(offset) => {
                let quote = pos + offset;
                if bytes.get(quote + 1) == Some(&b'"') {
                    // A doubled quote: one `"` of text.
                    owned
                        .get_or_insert_with(String::new)
                        .push_str(&feature[rest..=quote]);
                    rest = quote + 2;
                    pos = quote + 2;
                } else {
                    break quote;
                }
            }
            None => break bytes.len(),
        }
    };
    let text = &feature[rest..end];
    let column = match owned {
        Some(mut column) => {
            column.push_str(text);
            Cow::Owned(column)
        }
        None => Cow::Borrowed(text),
    };
    (column, (end + 1).min(bytes.len()))
}

/// Joins columns into a feature string, as MeCab's rewriter joins the
/// columns it rewrites.
///
/// A column that contains `,` or `"` is quoted, with its quotes doubled, as
/// MeCab's `escape_csv_element` does, so that [`split_feature`] reads it back
/// as one column. As in MeCab, the spaces and tabs at the start of a column
/// that is not quoted, and an empty last column, are not read back.
///
/// # Arguments
///
/// * `columns` - The columns.
///
/// # Returns
///
/// The columns joined with `,`.
pub(crate) fn join_feature<S: AsRef<str>>(columns: &[S]) -> String {
    let mut feature = String::new();
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            feature.push(',');
        }
        let column = column.as_ref();
        if column.contains([',', '"']) {
            feature.push('"');
            feature.push_str(&column.replace('"', "\"\""));
            feature.push('"');
        } else {
            feature.push_str(column);
        }
    }
    feature
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the columns of `feature` as strings.
    fn columns(feature: &str) -> Vec<String> {
        split_feature(feature)
            .into_iter()
            .map(Cow::into_owned)
            .collect()
    }

    // The expected columns are what MeCab's own `tokenizeCSV`, compiled from
    // `src/utils.h`, gives for the same strings.

    #[test]
    fn splits_plain_columns() {
        assert_eq!(columns("名詞,一般,*,*"), ["名詞", "一般", "*", "*"]);
        assert_eq!(columns("a,,b"), ["a", "", "b"]);
        assert_eq!(columns(",a"), ["", "a"]);
    }

    #[test]
    fn a_quoted_column_with_a_comma_is_one_column() {
        assert_eq!(
            columns("n,dian4,店,店,\"shop, store\",1"),
            ["n", "dian4", "店", "店", "shop, store", "1"]
        );
    }

    #[test]
    fn a_doubled_quote_is_one_quote() {
        assert_eq!(columns("a,\"x\"\"y\",z"), ["a", "x\"y", "z"]);
        assert_eq!(columns("\"abc\"\""), ["abc\""]);
        assert_eq!(columns("\"a\"\"\""), ["a\""]);
    }

    #[test]
    fn skips_spaces_and_tabs_before_a_column() {
        assert_eq!(columns(" a,\tb,  c "), ["a", "b", "c "]);
        assert_eq!(columns("a, \"b,c\""), ["a", "b,c"]);
        assert_eq!(columns("a, "), ["a", ""]);
    }

    #[test]
    fn drops_the_text_after_a_closing_quote() {
        assert_eq!(
            columns("\"bamboo\" radical in Chinese characters,x"),
            ["bamboo", "x"]
        );
    }

    #[test]
    fn a_quote_inside_a_column_is_text() {
        assert_eq!(
            columns("acronym for \"anime, comics and games\"/"),
            ["acronym for \"anime", "comics and games\"/"]
        );
    }

    #[test]
    fn an_unclosed_quote_takes_the_rest() {
        assert_eq!(columns("a,\"unclosed,b"), ["a", "unclosed,b"]);
    }

    #[test]
    fn a_trailing_comma_has_no_empty_column() {
        assert_eq!(columns("a,b,"), ["a", "b"]);
        assert!(columns("").is_empty());
    }

    #[test]
    fn borrows_all_but_quoted_columns_with_doubled_quotes() {
        let split = split_feature("a,\"b,c\",\"d\"\"e\"");
        assert!(matches!(split[0], Cow::Borrowed("a")));
        assert!(matches!(split[1], Cow::Borrowed("b,c")));
        assert!(matches!(&split[2], Cow::Owned(column) if column == "d\"e"));
    }

    #[test]
    fn join_quotes_columns_with_a_comma_or_a_quote() {
        assert_eq!(
            join_feature(&["n", "shop, store", "1"]),
            "n,\"shop, store\",1"
        );
        assert_eq!(join_feature(&["x\"y"]), "\"x\"\"y\"");
        assert_eq!(join_feature(&["a", "", "b"]), "a,,b");
        assert_eq!(join_feature(&["名詞", "一般"]), "名詞,一般");
        assert_eq!(join_feature::<&str>(&[]), "");
    }

    #[test]
    fn split_reads_back_what_join_writes() {
        let original = [",", "\"", "a\"b,c", "x y", "", "z"];
        assert_eq!(columns(&join_feature(&original)), original);
    }
}
