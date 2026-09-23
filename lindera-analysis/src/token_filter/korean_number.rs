use std::borrow::Cow;
use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

pub const KOREAN_NUMBER_TOKEN_FILTER_NAME: &str = "korean_number";

pub type KoreanNumberTokenFilterConfig = Value;

/// Convert tokens representing Sino-Korean numerals, written in Hangul or in Hanja, to Arabic numerals.
///
#[derive(Clone, Debug)]
pub struct KoreanNumberTokenFilter {
    tags: Option<HashSet<String>>,
}

impl KoreanNumberTokenFilter {
    pub fn new(tags: Option<HashSet<String>>) -> Self {
        Self { tags }
    }

    pub fn from_config(config: &KoreanNumberTokenFilterConfig) -> LinderaResult<Self> {
        let tags = config
            .get("tags")
            .and_then(|t| t.as_array())
            .map_or(Ok(None), |array| {
                array
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| {
                                LinderaErrorKind::Deserialize
                                    .with_error(anyhow::anyhow!("tag must be a string"))
                            })
                            .map(|s| s.to_string())
                    })
                    .collect::<LinderaResult<HashSet<String>>>()
                    .map(Some)
            })?;

        Ok(Self::new(tags))
    }
}

impl TokenFilter for KoreanNumberTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_NUMBER_TOKEN_FILTER_NAME
    }

    /// Converts token text to Arabic numerals if the token's part-of-speech tag matches the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A mutable reference to a vector of tokens. The text field of each token may be modified if it matches the criteria.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<()>` indicating whether the operation was successful.
    ///
    /// # Process
    ///
    /// 1. **Token Tag Evaluation**:
    ///    - ko-dic gives a single part-of-speech tag per token, so the first detail is used as the tag.
    ///
    /// 2. **Tag Matching**:
    ///    - If the configuration contains specific tags (`tags`), the function checks whether the token's tag is one of them.
    ///    - If no tags are specified (`None`), the conversion is applied to every token.  Hangul numerals are
    ///      homographs of common morphemes (`이` is also a subject particle, `만` is also an auxiliary particle),
    ///      so restricting this filter to `SN` and `NR` is recommended.
    ///
    /// 3. **Text Conversion**:
    ///    - For tokens that match the criteria, the text is converted to Arabic numerals using the `to_arabic_numerals` function and stored as `Cow::Owned`.
    ///
    /// # Errors
    ///
    /// If any issue arises during token processing or text conversion, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            // Determine whether to convert the token based on the config tags.
            // ko-dic has a single part-of-speech tag, which is the first detail.
            let should_convert = {
                let tag = token.get_detail(0).unwrap_or_default();
                self.tags.as_ref().is_none_or(|tags| tags.contains(tag))
            };

            // If conversion is required, apply the Arabic numeral conversion.
            if should_convert {
                let text = token.surface.as_ref();
                token.surface = Cow::Owned(to_arabic_numerals(text));
            }
        }

        Ok(())
    }
}

fn adjust_digits(num: &str, base: &str, digit: &str) -> String {
    let zero_str = format!("{base}{digit}");

    // If the number is less than the base, return the number as is.
    if zero_str.len() < num.len() {
        return num.to_owned();
    }

    let zero_len = zero_str.len() - num.len();
    let zeros = &zero_str[0..zero_len];

    let mut num_str = num.to_owned();
    num_str.insert_str(0, zeros);
    num_str
}

/// Converts a Sino-Korean numeral into Arabic numerals.
///
/// Both the Hangul spelling (`이천이십육`) and the Hanja spelling (`二千二十六`) are accepted, as are
/// mixed forms (`2천26`) and full width digits.  Characters that are not numerals are copied as they are,
/// so a token that is not a number comes back unchanged.
///
/// Native Korean numerals (`하나`, `둘`, `열`, `스물`, ...) are not positional and are left untouched.
fn to_arabic_numerals(from_str: &str) -> String {
    let mut num_buf = String::new();
    let mut digit = String::new();

    let from_chars = from_str.chars().rev().collect::<Vec<char>>();

    let mut i = from_chars.iter().peekable();
    while let Some(c) = i.next() {
        match c {
            '0' | '０' | '영' | '공' | '〇' | '零' => num_buf.insert(0, '0'),
            '1' | '１' | '일' | '一' | '壹' => num_buf.insert(0, '1'),
            '2' | '２' | '이' | '二' | '貳' => num_buf.insert(0, '2'),
            '3' | '３' | '삼' | '三' | '參' => num_buf.insert(0, '3'),
            '4' | '４' | '사' | '四' => num_buf.insert(0, '4'),
            '5' | '５' | '오' | '五' => num_buf.insert(0, '5'),
            '6' | '６' | '육' | '륙' | '六' => num_buf.insert(0, '6'),
            '7' | '７' | '칠' | '七' => num_buf.insert(0, '7'),
            '8' | '８' | '팔' | '八' => num_buf.insert(0, '8'),
            '9' | '９' | '구' | '九' => num_buf.insert(0, '9'),
            '십' | '十' | '拾' => {
                num_buf = adjust_digits(&num_buf, "0", &digit);

                match i.peek() {
                    Some('백') | Some('百') | Some('천') | Some('千') | Some('만') | Some('萬')
                    | Some('万') | Some('억') | Some('億') | Some('조') | Some('兆')
                    | Some('경') | Some('京') | Some('해') | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '백' | '百' => {
                num_buf = adjust_digits(&num_buf, "00", &digit);

                match i.peek() {
                    Some('천') | Some('千') | Some('만') | Some('萬') | Some('万') | Some('억')
                    | Some('億') | Some('조') | Some('兆') | Some('경') | Some('京')
                    | Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '천' | '千' => {
                num_buf = adjust_digits(&num_buf, "000", &digit);

                match i.peek() {
                    Some('만') | Some('萬') | Some('万') | Some('억') | Some('億') | Some('조')
                    | Some('兆') | Some('경') | Some('京') | Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '만' | '萬' | '万' => {
                digit = "0000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('억') | Some('億') | Some('조') | Some('兆') | Some('경') | Some('京')
                    | Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '억' | '億' => {
                digit = "00000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('조') | Some('兆') | Some('경') | Some('京') | Some('해') | Some('垓')
                    | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '조' | '兆' => {
                digit = "000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('경') | Some('京') | Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '경' | '京' => {
                digit = "0000000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '해' | '垓' => {
                digit = "00000000000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                if i.peek().is_none() {
                    num_buf.insert(0, '1');
                }
            }
            _ => {
                num_buf.insert(0, *c);
                digit.clear();
            }
        }
    }

    num_buf
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_number::to_arabic_numerals;

    #[test]
    fn test_to_arabic_numerals_digits() {
        assert_eq!(to_arabic_numerals("영"), "0");
        assert_eq!(to_arabic_numerals("공"), "0");
        assert_eq!(to_arabic_numerals("일"), "1");
        assert_eq!(to_arabic_numerals("이"), "2");
        assert_eq!(to_arabic_numerals("삼"), "3");
        assert_eq!(to_arabic_numerals("사"), "4");
        assert_eq!(to_arabic_numerals("오"), "5");
        assert_eq!(to_arabic_numerals("육"), "6");
        assert_eq!(to_arabic_numerals("륙"), "6");
        assert_eq!(to_arabic_numerals("칠"), "7");
        assert_eq!(to_arabic_numerals("팔"), "8");
        assert_eq!(to_arabic_numerals("구"), "9");
        assert_eq!(to_arabic_numerals("０"), "0");
        assert_eq!(to_arabic_numerals("９"), "9");
    }

    #[test]
    fn test_to_arabic_numerals_positions() {
        assert_eq!(to_arabic_numerals("십"), "10");
        assert_eq!(to_arabic_numerals("백"), "100");
        assert_eq!(to_arabic_numerals("천"), "1000");
        assert_eq!(to_arabic_numerals("만"), "10000");
        assert_eq!(to_arabic_numerals("억"), "100000000");
        assert_eq!(to_arabic_numerals("조"), "1000000000000");
        assert_eq!(to_arabic_numerals("경"), "10000000000000000");
        assert_eq!(to_arabic_numerals("해"), "100000000000000000000");
    }

    #[test]
    fn test_to_arabic_numerals_compound() {
        assert_eq!(to_arabic_numerals("십일"), "11");
        assert_eq!(to_arabic_numerals("이십"), "20");
        assert_eq!(to_arabic_numerals("이십삼"), "23");
        assert_eq!(to_arabic_numerals("구십구"), "99");
        assert_eq!(to_arabic_numerals("삼천오백"), "3500");
        assert_eq!(to_arabic_numerals("구천구백구십구"), "9999");
        assert_eq!(to_arabic_numerals("이천이십육"), "2026");
        assert_eq!(to_arabic_numerals("십만"), "100000");
        assert_eq!(to_arabic_numerals("일억이천만"), "120000000");
        assert_eq!(to_arabic_numerals("삼백육십오"), "365");
    }

    #[test]
    fn test_to_arabic_numerals_hanja() {
        assert_eq!(to_arabic_numerals("一"), "1");
        assert_eq!(to_arabic_numerals("十"), "10");
        assert_eq!(to_arabic_numerals("二千二十六"), "2026");
        assert_eq!(to_arabic_numerals("三千五百"), "3500");
        assert_eq!(to_arabic_numerals("壹"), "1");
    }

    #[test]
    fn test_to_arabic_numerals_mixed_with_arabic_digits() {
        assert_eq!(to_arabic_numerals("3천"), "3000");
        assert_eq!(to_arabic_numerals("2천26"), "2026");
        assert_eq!(to_arabic_numerals("2026"), "2026");
        assert_eq!(to_arabic_numerals("10만"), "100000");
    }

    #[test]
    fn test_to_arabic_numerals_leaves_other_text_alone() {
        // 고유어 수사는 자릿수 구조가 없으므로 그대로 둔다.
        assert_eq!(to_arabic_numerals("하나"), "하나");
        assert_eq!(to_arabic_numerals("스물"), "스물");
        assert_eq!(to_arabic_numerals("여덟"), "여덟");
        assert_eq!(to_arabic_numerals("한국"), "한국");
        assert_eq!(to_arabic_numerals(""), "");
    }

    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_number_token_filter_apply() {
        use std::borrow::Cow;
        use std::collections::HashSet;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::korean_number::KoreanNumberTokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

        let make_token = |surface: &'static str, tag: &'static str, position: usize| Token {
            surface: Cow::Borrowed(surface),
            byte_start: 0,
            byte_end: surface.len(),
            position,
            position_length: 1,
            word_id: WordId::new(LexType::System, 0),
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![
                Cow::Borrowed(tag),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed(surface),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
        };

        // 태그를 지정하지 않으면 모든 토큰을 변환한다.
        let filter = KoreanNumberTokenFilter::new(None);
        let mut tokens: Vec<Token> = vec![
            make_token("이천이십육", "NR", 0),
            make_token("년", "NNB", 1),
        ];
        filter.apply(&mut tokens).unwrap();
        assert_eq!(tokens[0].surface, "2026");
        assert_eq!(tokens[1].surface, "년");

        // 태그를 지정하면 그 태그의 토큰만 변환한다.
        // 한글 수사는 다른 형태소와 동형이라(`이`/JKS, `만`/JX) 태그 지정을 권장한다.
        let mut tags = HashSet::new();
        tags.insert("NR".to_string());
        let filter = KoreanNumberTokenFilter::new(Some(tags));
        let mut tokens: Vec<Token> = vec![
            make_token("삼", "NR", 0),
            make_token("이", "JKS", 1),
            make_token("만", "JX", 2),
        ];
        filter.apply(&mut tokens).unwrap();
        assert_eq!(tokens[0].surface, "3");
        assert_eq!(tokens[1].surface, "이");
        assert_eq!(tokens[2].surface, "만");
    }
}
