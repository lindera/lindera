//! Lexicon CSV files in MeCab's format (`surface,left_id,right_id,cost,feature...`),
//! written so that `lindera build`, which reads `lex.csv` with the `csv`
//! crate, reads every field back as it was.

use std::borrow::Cow;

/// Returns a field value as it must be written in a lexicon CSV file.
///
/// A value that contains a field separator (`,`), a quote (`"`) or a line
/// break is enclosed in quotes, with its quotes doubled, as MeCab's
/// `escape_csv_element` does for `,` and `"`; a line break would otherwise
/// end the record. Any other value is returned as it is. The `csv` crate,
/// which `lindera build` reads `lex.csv` with, reads the result back as
/// `value` (#1170).
///
/// # Arguments
///
/// * `value` - The field value, such as a surface.
///
/// # Returns
///
/// The value as a CSV field.
pub(crate) fn csv_field(value: &str) -> Cow<'_, str> {
    if value.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads `content` as `lindera build` reads `lex.csv`.
    fn read_as_dictionary_builder(content: &str) -> Vec<Vec<String>> {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content.as_bytes())
            .records()
            .map(|record| record.unwrap().iter().map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn csv_field_quotes_only_when_needed() {
        assert_eq!(csv_field("東京"), "東京");
        assert!(matches!(csv_field("東京"), Cow::Borrowed(_)));
        assert_eq!(csv_field(""), "");
        assert_eq!(csv_field("#"), "#");
        assert_eq!(csv_field(" x "), " x ");
        assert_eq!(csv_field(","), "\",\"");
        assert_eq!(csv_field("\""), "\"\"\"\"");
        assert_eq!(csv_field("a\"b,c"), "\"a\"\"b,c\"");
        assert_eq!(csv_field("a\nb"), "\"a\nb\"");
        assert_eq!(csv_field("a\rb"), "\"a\rb\"");
    }

    #[test]
    fn csv_field_reads_back_as_the_dictionary_builder_reads_it() {
        let values = [
            ",",
            "\"",
            "a\"b,c",
            "\"abc\"def",
            "#",
            "a\nb",
            "a\r\nb",
            " x",
            "",
        ];
        let content: String = values
            .iter()
            .map(|value| format!("{},1,2,3,名詞,一般\n", csv_field(value)))
            .collect();

        let records = read_as_dictionary_builder(&content);

        assert_eq!(records.len(), values.len(), "{content}");
        for (record, value) in records.iter().zip(values) {
            assert_eq!(record, &[value, "1", "2", "3", "名詞", "一般"]);
        }
    }
}
