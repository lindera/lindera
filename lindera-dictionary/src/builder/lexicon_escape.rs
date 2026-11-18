//! Decoding of the escapes that a lexicon format writes in its fields
//! (#1389).

use std::borrow::Cow;

use csv::StringRecord;

use crate::dictionary::lexicon_format::LexiconFormat;

/// Decodes a lexicon row as `format` reads it.
///
/// MeCab's format takes every field as written. Sudachi's format decodes the
/// escapes in every field (see [`decode_sudachi_escapes`]).
///
/// # Arguments
///
/// * `format` - The format of the lexicon the row comes from.
/// * `record` - The row, as read.
///
/// # Returns
///
/// The row itself when there is nothing to decode, otherwise a new row with
/// every field decoded and the position (the line) of the original; or a
/// message naming an escape that cannot be decoded, for the caller to put
/// the file and the line in front of.
pub(crate) fn decode_record(
    format: LexiconFormat,
    record: StringRecord,
) -> Result<StringRecord, String> {
    if format.is_mecab() || !record.iter().any(has_escape_start) {
        return Ok(record);
    }
    let mut decoded = StringRecord::with_capacity(record.as_slice().len(), record.len());
    for field in record.iter() {
        decoded.push_field(&decode_sudachi_escapes(field)?);
    }
    decoded.set_position(record.position().cloned());
    Ok(decoded)
}

/// Returns whether a field may hold an escape: whether it has a backslash
/// followed by `u`.
///
/// # Arguments
///
/// * `field` - The field.
///
/// # Returns
///
/// `true` when the field has a backslash followed by `u`.
fn has_escape_start(field: &str) -> bool {
    field.contains("\\u")
}

/// Decodes Sudachi's escapes in a field, as `unescape` in sudachi.rs
/// `v0.7.0` (`sudachi/src/dic/build/parse.rs`) does with its regex
/// `\\u(?:\{([0-9a-fA-F]{1,6})\}|([0-9a-fA-F]{4}))`:
///
/// - A backslash, `u` and one to six hex digits in braces, or a backslash,
///   `u` and four hex digits, stands for the code point that the digits
///   name. Braces are tried first, and text that matches neither form stays
///   as written.
/// - Escapes are found left to right and do not overlap, and the text that
///   an escape stands for is not decoded again.
/// - There is no escape for the backslash itself, so a backslash before an
///   escape stays: SudachiDict writes the emoticon `\(^o^)/` with its
///   parentheses and its slash escaped and its backslash as it is.
///
/// # Arguments
///
/// * `field` - The field, as read.
///
/// # Returns
///
/// The field itself when it has no escape, otherwise the decoded text; or a
/// message naming an escape for a value that is not a Unicode scalar value
/// (a surrogate, or a value above U+10FFFF), which Sudachi rejects too, or
/// for NUL, which Sudachi accepts but a dictionary cannot hold in a field,
/// since NUL separates the details of an entry.
fn decode_sudachi_escapes(field: &str) -> Result<Cow<'_, str>, String> {
    let bytes = field.as_bytes();
    let mut decoded = String::new();
    // The end of the text already copied into `decoded`: 0 until the first
    // escape is decoded, and past an escape after that.
    let mut copied = 0;
    // Where to look for the next escape.
    let mut from = 0;
    while let Some(offset) = field[from..].find("\\u") {
        let start = from + offset;
        let Some((code, end)) = parse_escape_digits(bytes, start + 2) else {
            from = start + 1;
            continue;
        };
        let escape = &field[start..end];
        let c = char::from_u32(code).ok_or_else(|| {
            format!("invalid escape {escape}: U+{code:04X} is not a Unicode scalar value")
        })?;
        if c == '\0' {
            return Err(format!(
                "invalid escape {escape}: NUL cannot be stored in a dictionary field, \
                 since it separates the details of an entry"
            ));
        }
        decoded.push_str(&field[copied..start]);
        decoded.push(c);
        copied = end;
        from = end;
    }
    if copied == 0 {
        return Ok(Cow::Borrowed(field));
    }
    decoded.push_str(&field[copied..]);
    Ok(Cow::Owned(decoded))
}

/// Parses the digits of an escape: one to six hex digits in braces, or else
/// four hex digits.
///
/// # Arguments
///
/// * `bytes` - The field.
/// * `at` - Where the digits (or the opening brace) start, right after the
///   backslash and `u`.
///
/// # Returns
///
/// The value that the digits name and the end of the escape, or `None` when
/// the text at `at` is neither form.
fn parse_escape_digits(bytes: &[u8], at: usize) -> Option<(u32, usize)> {
    // The number of hex digits from `from`, counting at most `max`.
    let hex_digits = |from: usize, max: usize| {
        bytes[from..]
            .iter()
            .take(max)
            .take_while(|b| b.is_ascii_hexdigit())
            .count()
    };
    if bytes.get(at) == Some(&b'{') {
        // Seven digits are counted so that a run longer than six is told
        // apart from a run of six. A brace is not a hex digit, so the
        // four-digit form cannot match here either.
        let digits = hex_digits(at + 1, 7);
        return ((1..=6).contains(&digits) && bytes.get(at + 1 + digits) == Some(&b'}'))
            .then(|| (hex_value(&bytes[at + 1..at + 1 + digits]), at + 2 + digits));
    }
    (hex_digits(at, 4) == 4).then(|| (hex_value(&bytes[at..at + 4]), at + 4))
}

/// Returns the value of a run of hex digits.
///
/// # Arguments
///
/// * `digits` - At most six ASCII hex digits, which the caller has checked.
///
/// # Returns
///
/// Their value.
fn hex_value(digits: &[u8]) -> u32 {
    digits.iter().fold(0, |value, &digit| {
        value * 16 + char::from(digit).to_digit(16).unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use csv::StringRecord;

    use super::{decode_record, decode_sudachi_escapes};
    use crate::dictionary::lexicon_format::LexiconFormat;

    /// Decodes a field that must decode.
    fn decoded(field: &str) -> String {
        match decode_sudachi_escapes(field) {
            Ok(text) => text.into_owned(),
            Err(message) => panic!("{field:?} should decode: {message}"),
        }
    }

    /// Returns the message for a field that must not decode.
    fn rejected(field: &str) -> String {
        match decode_sudachi_escapes(field) {
            Ok(text) => panic!("{field:?} should be rejected, decoded to {text:?}"),
            Err(message) => message,
        }
    }

    // The backslashes in these tests are written `\x5c`.

    /// The cases of the `unescape` tests of sudachi.rs `v0.7.0`
    /// (`sudachi/src/dic/build/parse.rs`).
    #[test]
    fn decodes_as_sudachi_does() {
        for (field, expected) in [
            ("", ""),
            ("a", "a"),
            ("あ", "あ"),
            ("\x5cu0020", " "),
            ("\x5cu{20}", " "),
            ("\x5cu{1f49e}", "💞"),
            ("\x5cu020f", "\u{20f}"),
            ("\x5cu{20}f", " f"),
            ("f\x5cu0020", "f "),
            ("f\x5cu{20}", "f "),
            ("\x5cu100056", "\u{1000}56"),
            ("a\x5cu002cc", "a,c"),
            ("a\x5cu{2c}c", "a,c"),
            ("\x5cu{10FFFF}", "\u{10FFFF}"),
        ] {
            assert_eq!(decoded(field), expected, "{field:?}");
        }
    }

    /// SudachiDict's escapes: upper- and lower-case hex digits, several in
    /// one field.
    #[test]
    fn decodes_sudachidict_escapes() {
        for (field, expected) in [
            ("http:\x5cu002f\x5cu002f", "http://"),
            ("http:\x5cu002F\x5cu002F", "http://"),
            ("1\x5cu002c2塁", "1,2塁"),
            ("\x5cu0028株\x5cu0029", "(株)"),
            ("4'33\x5cu0022", "4'33\""),
            ("a\x5cu007cb", "a|b"),
            ("\x5cu{2F}", "/"),
        ] {
            assert_eq!(decoded(field), expected, "{field:?}");
        }
    }

    /// There is no escape for the backslash: a backslash before an escape
    /// stays, as in SudachiDict's emoticons and `cargill\, inc.`.
    #[test]
    fn keeps_a_backslash_before_an_escape() {
        for (field, expected) in [
            ("\x5c\x5cu0028^o^\x5cu0029\x5cu002f", "\x5c(^o^)/"),
            ("cargill\x5c\x5cu002c inc.", "cargill\x5c, inc."),
            ("\x5c\x5c\x5cu0028", "\x5c\x5c("),
        ] {
            assert_eq!(decoded(field), expected, "{field:?}");
        }
    }

    /// The text an escape stands for is not decoded again: an escaped
    /// backslash followed by `u0041` is a backslash and `u0041`.
    #[test]
    fn does_not_decode_the_decoded_text_again() {
        assert_eq!(decoded("\x5cu005cu0041"), "\x5cu0041");
        assert_eq!(decoded("\x5cu{5c}u{41}"), "\x5cu{41}");
    }

    /// Text that matches neither form stays as written, and a field without
    /// an escape is returned without a copy.
    #[test]
    fn leaves_other_text_as_written() {
        for field in [
            "\x5c",
            "\x5cu",
            "\x5cu12",
            "\x5cu12g4",
            "\x5cu{}",
            "\x5cu{12",
            "\x5cu{1234567}",
            "\x5cu{12g}",
            "\x5cx0041",
            "a\x5cnb",
        ] {
            assert!(
                matches!(decode_sudachi_escapes(field), Ok(Cow::Borrowed(text)) if text == field),
                "{field:?}"
            );
        }
        // An escape after text that only looks like one.
        assert_eq!(decoded("\x5cu{}\x5cu0041"), "\x5cu{}A");
    }

    /// Values that are not Unicode scalar values are rejected, as Sudachi
    /// rejects them, and so is NUL, which a dictionary field cannot hold.
    #[test]
    fn rejects_values_that_are_not_characters_and_nul() {
        for (field, expected) in [
            ("\x5cu{110000}", "U+110000 is not a Unicode scalar value"),
            ("\x5cu{FFFFFF}", "U+FFFFFF is not a Unicode scalar value"),
            ("a\x5cuD800b", "U+D800 is not a Unicode scalar value"),
            ("\x5cu{dfff}", "U+DFFF is not a Unicode scalar value"),
            ("\x5cu0000", "NUL cannot be stored"),
            ("x\x5cu{0}", "NUL cannot be stored"),
        ] {
            let message = rejected(field);
            assert!(message.contains(expected), "{field:?}: {message}");
            // The message names the escape as written.
            let escape = &field[field.find('\x5c').unwrap()..];
            let escape = escape.trim_end_matches('b');
            assert!(message.contains(escape), "{field:?}: {message}");
        }
    }

    /// MeCab's format takes the row as written, escapes included.
    #[test]
    fn mecab_rows_are_taken_as_written() {
        let record = StringRecord::from(vec!["b\x5cu002fs", "1", "1", "100", "B\x5cu002FS"]);
        let result = decode_record(LexiconFormat::Mecab, record.clone()).unwrap();
        assert_eq!(result, record);
    }

    /// In Sudachi's format, a row without escapes is the same row, not a
    /// copy, and a row with escapes has every field decoded.
    #[test]
    fn sudachi_rows_are_decoded() {
        let plain = StringRecord::from(vec!["本", "1", "1", "100", "名詞"]);
        let buffer = plain.as_slice().as_ptr();
        let result = decode_record(LexiconFormat::Sudachi, plain).unwrap();
        assert_eq!(result.as_slice().as_ptr(), buffer);

        let record = StringRecord::from(vec![
            "b\x5cu002fs",
            "5139",
            "5139",
            "15000",
            "B\x5cu002FS",
            "名詞",
            "b\x5cu002fs",
            "B\x5cu002FS",
        ]);
        let result = decode_record(LexiconFormat::Sudachi, record).unwrap();
        assert_eq!(
            result,
            StringRecord::from(vec![
                "b/s", "5139", "5139", "15000", "B/S", "名詞", "b/s", "B/S"
            ])
        );
    }

    /// A decoded row keeps the position of the row it was read as, which
    /// the errors of the builders report.
    #[test]
    fn decoded_rows_keep_their_line() {
        let csv = "本,1,1,100\nb\x5cu002fs,1,1,100\n";
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(csv.as_bytes());
        let record = reader.records().nth(1).unwrap().unwrap();
        let line = record.position().map(|position| position.line());
        assert_eq!(line, Some(2));
        let result = decode_record(LexiconFormat::Sudachi, record).unwrap();
        assert_eq!(&result[0], "b/s");
        assert_eq!(result.position().map(|position| position.line()), line);
    }

    /// An escape that cannot be decoded fails the whole row.
    #[test]
    fn a_bad_escape_fails_the_row() {
        let record = StringRecord::from(vec!["a", "1", "1", "100", "\x5cu{110000}"]);
        let message = decode_record(LexiconFormat::Sudachi, record).unwrap_err();
        assert!(message.contains("\x5cu{110000}"), "{message}");
    }
}
