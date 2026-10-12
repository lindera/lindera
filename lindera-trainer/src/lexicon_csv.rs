//! Lexicon CSV files in MeCab's format (`surface,left_id,right_id,cost,feature...`),
//! read and written as `lindera build` reads `lex.csv`: with the CSV rules of
//! the `csv` crate, with no comment lines (#1170).

use std::borrow::Cow;

use anyhow::{Result, anyhow, bail};
use csv_core::{ReadFieldResult, Reader};

/// A row of a lexicon CSV file: a record with at least five fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexiconRow {
    /// The surface, the first field, unquoted.
    pub(crate) surface: String,
    /// The left context id, the second field, unquoted and not parsed.
    pub(crate) left_id: String,
    /// The right context id, the third field, unquoted and not parsed.
    pub(crate) right_id: String,
    /// The cost, the fourth field, unquoted and not parsed.
    pub(crate) cost: String,
    /// The feature: the text of the record after the fourth field separator,
    /// as written, quotes included, without the line break that ends the
    /// record. MeCab keeps it the same way when the fifth field is not quoted
    /// (`tokenizeCSV(line, col, 5)` leaves the rest of the line in
    /// `col[4]`), and written out as it is, it reads back in `lindera build`
    /// as the source does. A feature rebuilt from the parsed fields would
    /// lose the quotes of a column such as Jieba's `"authorized full-service
    /// car dealership (offering sales, ...)"`, whose commas would then split
    /// it. The trainer splits the feature into columns with
    /// [`split_feature`](crate::feature_csv::split_feature), as MeCab's
    /// learner does (#1184).
    pub(crate) feature: String,
}

/// Reads the rows of a lexicon CSV file, such as the seed lexicon or a user
/// lexicon.
///
/// The records and their fields are read as `lindera build` reads `lex.csv`:
/// `csv_core::Reader::new()` has the defaults of `csv::ReaderBuilder::new()`
/// (RFC 4180 quoting, no comment character). So a quoted field such as
/// mecab-ko-dic's `","` surface is unquoted, and a line that starts with `#`,
/// such as UniDic's `#` surface, is a row; the line-based reader before
/// #1170 split the first on `,` and skipped the second as a comment.
///
/// A record with fewer than five fields is skipped, and so are blank lines.
/// The rows keep their order, which gives the seed rows their label ids.
///
/// # Arguments
///
/// * `content` - The text of the file.
/// * `name` - What the file is, such as `seed lexicon`, for error messages.
///
/// # Returns
///
/// The rows in file order, or an error when a quoted field spans lines.
/// MeCab reads one record per line, and such a field is most likely a quote
/// that is never closed, which would swallow the rows after it.
pub(crate) fn read_lexicon_rows(content: &str, name: &str) -> Result<Vec<LexiconRow>> {
    let bytes = content.as_bytes();
    let mut reader = Reader::new();
    let mut output = vec![0u8; 256];
    let mut rows = Vec::new();

    // Offsets in `bytes` and in `output`. A field can end in a later call
    // than the one that read its bytes, so both carry over `InputEmpty`.
    let mut pos = 0;
    let mut output_len = 0;
    // Where the field being read starts, and where the feature starts: after
    // the fourth field separator of the record.
    let mut field_start = 0;
    let mut feature_start = None;
    // The first four fields of the record, and how many fields it has.
    let mut fields: Vec<String> = Vec::with_capacity(4);
    let mut field_count = 0;

    loop {
        // An empty `&bytes[pos..]` tells the reader that the input ended.
        let (result, nin, nout) = reader.read_field(&bytes[pos..], &mut output[output_len..]);
        pos += nin;
        output_len += nout;
        match result {
            ReadFieldResult::InputEmpty => {}
            ReadFieldResult::OutputFull => output.resize(output.len() * 2, 0),
            ReadFieldResult::Field { record_end } => {
                let value = &output[..output_len];
                if value.contains(&b'\n') || value.contains(&b'\r') {
                    bail!(
                        "{name} line {}: a quoted field spans lines (is a quote left unclosed?)",
                        line_number(content, field_start)
                    );
                }
                if field_count < 4 {
                    let value = std::str::from_utf8(value).map_err(|err| {
                        anyhow!("{name} line {}: {err}", line_number(content, field_start))
                    })?;
                    fields.push(value.to_string());
                }
                field_count += 1;
                output_len = 0;
                if field_count == 4 && !record_end {
                    feature_start = Some(pos);
                }
                if record_end {
                    if let Some(start) = feature_start.take()
                        && let Ok([surface, left_id, right_id, cost]) =
                            <[String; 4]>::try_from(std::mem::take(&mut fields))
                    {
                        // The record ends in `\n`, or in `\r` of `\r\n`, whose
                        // `\n` the next call skips; the last one may end in
                        // neither.
                        let feature = content
                            .get(start..pos)
                            .ok_or_else(|| anyhow!("{name}: invalid feature offsets"))?
                            .trim_end_matches(['\r', '\n']);
                        rows.push(LexiconRow {
                            surface,
                            left_id,
                            right_id,
                            cost,
                            feature: feature.to_string(),
                        });
                    }
                    fields.clear();
                    field_count = 0;
                }
                field_start = pos;
            }
            ReadFieldResult::End => break,
        }
    }

    Ok(rows)
}

/// Returns the line of `content` that a field starting at `offset` is on.
///
/// The bytes the reader consumed before the first field of a record include
/// the blank lines before it, and the `\n` of a `\r\n` that ended the record
/// before, so they are skipped first.
///
/// # Arguments
///
/// * `content` - The text of the file.
/// * `offset` - The byte offset where the reader started the field.
///
/// # Returns
///
/// The 1-based line number.
fn line_number(content: &str, offset: usize) -> usize {
    let rest = content.get(offset..).unwrap_or_default();
    let start = offset + (rest.len() - rest.trim_start_matches(['\r', '\n']).len());
    content
        .get(..start)
        .map_or(0, |before| before.matches('\n').count())
        + 1
}

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

    /// Returns the `(surface, feature)` pairs of `content`.
    fn surfaces_and_features(content: &str) -> Vec<(String, String)> {
        read_lexicon_rows(content, "seed lexicon")
            .unwrap()
            .into_iter()
            .map(|row| (row.surface, row.feature))
            .collect()
    }

    #[test]
    fn reads_a_quoted_surface() {
        let rows = read_lexicon_rows(
            "\",\",1792,3558,788,SC,*,*,*,*,*,*,*\n\"\"\"\",1,2,3,SY\n",
            "seed lexicon",
        )
        .unwrap();

        assert_eq!(
            rows,
            [
                LexiconRow {
                    surface: ",".to_string(),
                    left_id: "1792".to_string(),
                    right_id: "3558".to_string(),
                    cost: "788".to_string(),
                    feature: "SC,*,*,*,*,*,*,*".to_string(),
                },
                LexiconRow {
                    surface: "\"".to_string(),
                    left_id: "1".to_string(),
                    right_id: "2".to_string(),
                    cost: "3".to_string(),
                    feature: "SY".to_string(),
                },
            ]
        );
    }

    #[test]
    fn reads_a_hash_line_as_a_row() {
        assert_eq!(
            surfaces_and_features("#,5969,5969,2588,補助記号,一般\n# a,1,1,1,F\nx,1,1,1,G\n"),
            [
                ("#".to_string(), "補助記号,一般".to_string()),
                ("# a".to_string(), "F".to_string()),
                ("x".to_string(), "G".to_string()),
            ]
        );
    }

    /// The feature is the text after the fourth separator as written: a
    /// quoted column keeps its quotes and its commas (Jieba), and so does a
    /// column with literal quotes in it (CC-CEDICT).
    #[test]
    fn keeps_the_feature_as_written() {
        let jieba = "4S店,0,0,746,n,4S dian4,4S店,4S店,\"authorized full-service car \
                     dealership (offering sales, spare parts, after-sales service and \
                     customer support)\"";
        let cedict = "ACG,0,0,-2078,*,*,*,*,A C G,ACG,ACG,acronym for \"anime, comics \
                      and games\"/";
        let doubled = "一,0,0,260,m,yi1,一,一,\"one/\"\"one\"\" radical\"";

        let rows = surfaces_and_features(&format!("{jieba}\n{cedict}\n{doubled}\n"));

        assert_eq!(
            rows,
            [
                (
                    "4S店".to_string(),
                    jieba["4S店,0,0,746,".len()..].to_string()
                ),
                (
                    "ACG".to_string(),
                    cedict["ACG,0,0,-2078,".len()..].to_string()
                ),
                ("一".to_string(), doubled["一,0,0,260,".len()..].to_string()),
            ]
        );
    }

    #[test]
    fn reads_a_bom_crlf_blank_lines_and_no_final_line_break() {
        assert_eq!(
            surfaces_and_features("\u{feff}a,1,2,3,x,y\r\n\r\n\nb,1,2,3,z\r\n\nc,1,2,3,w"),
            [
                ("a".to_string(), "x,y".to_string()),
                ("b".to_string(), "z".to_string()),
                ("c".to_string(), "w".to_string()),
            ]
        );
    }

    #[test]
    fn skips_records_with_fewer_than_five_fields() {
        assert_eq!(
            surfaces_and_features("a,1,2,3\nb,1,2,3,\n   \nc\nd,1,2,3,x\n"),
            [
                ("b".to_string(), String::new()),
                ("d".to_string(), "x".to_string()),
            ]
        );
    }

    /// A field longer than the reader's initial output buffer.
    #[test]
    fn reads_long_fields() {
        let surface = "長".repeat(200);
        let feature = format!("名詞,{}", "x".repeat(1000));

        assert_eq!(
            surfaces_and_features(&format!("{surface},1,2,3,{feature}\n")),
            [(surface, feature)]
        );
    }

    #[test]
    fn rejects_a_quoted_field_that_spans_lines() {
        for (content, line) in [
            ("a,1,1,1,F\n\",1,1,0,0,記号\nb,1,1,1,G\n", 2),
            ("a,1,1,1,F\n\n\r\nb,1,1,1,\"x\ny\"\n", 4),
            ("a,1,1,1,F\r\n\"b\rc\",1,1,1,G\r\n", 2),
        ] {
            let err = match read_lexicon_rows(content, "user lexicon") {
                Ok(rows) => panic!("{content:?} must be an error, got {rows:?}"),
                Err(err) => err.to_string(),
            };
            assert!(
                err.starts_with(&format!(
                    "user lexicon line {line}: a quoted field spans lines"
                )),
                "{content:?}: {err}"
            );
        }
    }

    /// A row read and written back reads the same in `lindera build`.
    #[test]
    fn rows_written_back_read_the_same_in_the_dictionary_builder() {
        let content = "\",\",1792,3558,788,SC,*,*\n#,1,1,1,記号,\"a, b\",\"c\"\"d\"\n\
                       \"a\"\"b\",1,1,1,x\n";
        let rows = read_lexicon_rows(content, "seed lexicon").unwrap();
        let written: String = rows
            .iter()
            .map(|row| {
                format!(
                    "{},{},{},{},{}\n",
                    csv_field(&row.surface),
                    row.left_id,
                    row.right_id,
                    row.cost,
                    row.feature
                )
            })
            .collect();

        assert_eq!(written, content);
        assert_eq!(
            read_as_dictionary_builder(&written),
            read_as_dictionary_builder(content)
        );
    }
}
