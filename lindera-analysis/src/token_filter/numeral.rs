use std::iter;

/// What a character means inside a numeral.
///
/// The value is produced by a per-language classifier, so the engine below never needs to know
/// which script it is reading: `japanese_number` maps the kanji forms, `korean_number` the Hangul
/// and Hanja ones, and both arrive here as the same two cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Numeral {
    /// A digit, given as the Arabic digit it stands for.
    Digit(char),
    /// A position character, given as its rank: 1 for ten, 2 for hundred, 3 for thousand, then 4
    /// for the first myriad unit (万 / 만) and one more for each myriad after it, up to 8.
    ///
    /// Ranks 1 to 3 pad the buffer to their own width and leave the current myriad scale alone;
    /// ranks 4 and above set that scale to four zeros per myriad. In both cases a leading `1` is
    /// inserted when nothing follows the character or when what follows is a higher position,
    /// which is how `십만` and `百億` come out as `100000` and `10000000000` rather than `00000`
    /// and `0000000000`.
    Position(u8),
}

/// Classifies a character for one language, or returns `None` when it takes no part in a numeral.
pub(crate) type Classifier = fn(char) -> Option<Numeral>;

/// The number of zeros a position of this rank contributes.
fn zeros_for(rank: u8) -> usize {
    if rank <= 3 {
        rank as usize
    } else {
        4 * (rank as usize - 3)
    }
}

/// Left-pads `num` with zeros until it is as wide as `base` plus `digit`, counting characters.
///
/// Returns `num` unchanged when it is already at least that wide. The count is in characters
/// rather than bytes: a byte count makes one non-ASCII character weigh three digits, which is how
/// `万歳` used to become `10歳` before the all-numeral rule made that buffer impossible.
fn adjust_digits(num: &str, base: usize, digit: usize) -> String {
    let width = base + digit;
    let len = num.chars().count();

    if width <= len {
        return num.to_owned();
    }

    let mut padded = String::with_capacity(width);
    padded.extend(iter::repeat_n('0', width - len));
    padded.push_str(num);
    padded
}

/// Whether every character of `text` takes part in a numeral.
///
/// This is the rule Lucene's `JapaneseNumberFilter` and `KoreanNumberFilter` apply through
/// `isNumeral(String)`, and the reason a token is either converted whole or not at all.
pub(crate) fn is_all_numeral(text: &str, classify: Classifier) -> bool {
    text.chars().all(|c| classify(c).is_some())
}

/// Converts a numeral written in digits, in Sino-Japanese or in Sino-Korean into Arabic numerals.
///
/// The token is converted only when [`is_all_numeral`] holds for it; anything else is returned as
/// it is. Without that rule a token that merely contains a numeral character is rewritten in
/// place, which turns the Japanese `一部` ("a part") into `1部` and the Korean `일곱` ("seven")
/// into `1곱`.
///
/// The scan itself runs right to left, keeping the digits seen so far in `num_buf` and the current
/// myriad scale in `digit`. Reaching a position character pads the buffer to that position's
/// width, so `二千二十六` builds `6`, then `26`, then `026`, then `2026`.
pub(crate) fn to_arabic_numerals(from_str: &str, classify: Classifier) -> String {
    if !is_all_numeral(from_str, classify) {
        return from_str.to_owned();
    }

    let mut num_buf = String::new();
    let mut digit = 0usize;

    let chars = from_str.chars().rev().collect::<Vec<char>>();
    let mut i = chars.iter().peekable();

    while let Some(&c) = i.next() {
        match classify(c) {
            Some(Numeral::Digit(d)) => num_buf.insert(0, d),
            Some(Numeral::Position(rank)) => {
                let zeros = zeros_for(rank);
                let base = if rank <= 3 { zeros } else { 0 };
                if rank > 3 {
                    digit = zeros;
                }

                num_buf = adjust_digits(&num_buf, base, digit);

                // A position with nothing after it, or with a higher position after it, has had
                // its leading `1` left out: `십` is 10 and `십만` is 100000.
                let followed_by_higher = match i.peek().and_then(|&&next| classify(next)) {
                    Some(Numeral::Position(next_rank)) => next_rank > rank,
                    _ => false,
                };
                if i.peek().is_none() || followed_by_higher {
                    num_buf.insert(0, '1');
                }
            }
            // Unreachable for real input because of the all-numeral rule above, and kept so the
            // engine is total on its own.
            None => {
                num_buf.insert(0, c);
                digit = 0;
            }
        }
    }

    num_buf
}

#[cfg(test)]
mod tests {
    use crate::token_filter::numeral::{
        Numeral, adjust_digits, is_all_numeral, to_arabic_numerals,
    };

    /// A classifier with one digit and one position, enough to drive the engine.
    fn classify(c: char) -> Option<Numeral> {
        let numeral = match c {
            '0'..='9' => Numeral::Digit(c),
            '一' => Numeral::Digit('1'),
            '二' => Numeral::Digit('2'),
            '十' => Numeral::Position(1),
            '百' => Numeral::Position(2),
            '万' => Numeral::Position(4),
            _ => return None,
        };

        Some(numeral)
    }

    #[test]
    fn test_adjust_digits_counts_characters_not_bytes() {
        // A byte count would make this three digits wide and leave it unpadded.
        assert_eq!(adjust_digits("歳", 2, 0), "0歳");
        assert_eq!(adjust_digits("十歳", 4, 0), "00十歳");

        // ASCII buffers are unaffected, and a buffer at least as wide as the target is returned
        // as it is.
        assert_eq!(adjust_digits("1", 2, 0), "01");
        assert_eq!(adjust_digits("123", 2, 0), "123");
        assert_eq!(adjust_digits("", 0, 4), "0000");
    }

    #[test]
    fn test_is_all_numeral() {
        assert!(is_all_numeral("二十一", classify));
        assert!(is_all_numeral("", classify));
        assert!(!is_all_numeral("二十歳", classify));
    }

    #[test]
    fn test_to_arabic_numerals_returns_mixed_tokens_unchanged() {
        assert_eq!(to_arabic_numerals("二十歳", classify), "二十歳");
        assert_eq!(to_arabic_numerals("万歳", classify), "万歳");
    }

    #[test]
    fn test_to_arabic_numerals_positions() {
        assert_eq!(to_arabic_numerals("十", classify), "10");
        assert_eq!(to_arabic_numerals("二十一", classify), "21");
        assert_eq!(to_arabic_numerals("百", classify), "100");
        assert_eq!(to_arabic_numerals("万", classify), "10000");
        // A position followed by a higher one has its leading 1 left out in the source.
        assert_eq!(to_arabic_numerals("十万", classify), "100000");
        assert_eq!(to_arabic_numerals("二百万", classify), "2000000");
    }
}
