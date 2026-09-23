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
/// Each token is converted on its own. ko-dic tokenizes a Sino-Korean numeral into one token per
/// morpheme, so `이천이십육년` arrives as `이/NR 천/NR 이/NR 십/NR 육/NR 년/NNBC` and comes out as
/// `2 1000 2 10 6 년`, not as `2026 년`. Merging those tokens first is tracked in
/// <https://github.com/lindera/lindera/issues/1026>.
///
#[derive(Clone, Debug)]
pub struct KoreanNumberTokenFilter {
    tags: Option<HashSet<String>>,
}

impl KoreanNumberTokenFilter {
    /// The tags the filter is restricted to when `tags` is not given: numbers and numerals.
    ///
    /// A single-character morpheme spelled exactly like a numeral cannot be told apart from the
    /// numeral by the all-numeral rule, only by its tag: without one, the noun `일` ("work") and
    /// the subject particle `이` in `오늘 일이 많다` become `1` and `2`. Restricting the filter to
    /// the numeral tags is the useful default. `"tags": null` asks for every token.
    pub const DEFAULT_TAGS: [&'static str; 2] = ["SN", "NR"];

    pub fn new(tags: Option<HashSet<String>>) -> Self {
        Self { tags }
    }

    pub fn from_config(config: &KoreanNumberTokenFilterConfig) -> LinderaResult<Self> {
        let tags = match config.get("tags") {
            // Absent: restrict the filter to the numeral tags.
            None => Some(Self::DEFAULT_TAGS.iter().map(|s| s.to_string()).collect()),
            // Explicitly null: convert every token.
            Some(Value::Null) => None,
            Some(value) => {
                let array = value.as_array().ok_or_else(|| {
                    LinderaErrorKind::Deserialize
                        .with_error(anyhow::anyhow!("tags must be an array of strings"))
                })?;

                Some(
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
                        .collect::<LinderaResult<HashSet<String>>>()?,
                )
            }
        };

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
    ///    - If no tags are specified (`None`), the conversion is applied to every token. Configurations that
    ///      leave `tags` out get [`KoreanNumberTokenFilter::DEFAULT_TAGS`] instead, because Hangul numerals are
    ///      homographs of common morphemes (`이` is also a subject particle, `만` is also an auxiliary particle).
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

/// Whether the character can take part in a Sino-Korean numeral: a digit, a numeral syllable or
/// one of the position characters, in Hangul, in Hanja or in the financial Hanja forms.
fn is_numeral(c: char) -> bool {
    matches!(
        c,
        '0'..='9'
            | '０'..='９'
            | '영' | '공' | '〇' | '零'
            | '일' | '一' | '壹'
            | '이' | '二' | '貳' | '貮'
            | '삼' | '三' | '參' | '叁'
            | '사' | '四' | '肆'
            | '오' | '五' | '伍'
            | '육' | '륙' | '六' | '陸'
            | '칠' | '七' | '柒'
            | '팔' | '八' | '捌'
            | '구' | '九' | '玖'
            | '십' | '十' | '拾'
            | '백' | '百' | '佰'
            | '천' | '千' | '仟'
            | '만' | '萬' | '万'
            | '억' | '億'
            | '조' | '兆'
            | '경' | '京'
            | '해' | '垓'
    )
}

/// Converts a Sino-Korean numeral into Arabic numerals.
///
/// Both the Hangul spelling (`이천이십육`) and the Hanja spelling (`二千二十六`) are accepted, as are
/// mixed forms (`2천26`) and full width digits.  Characters that are not numerals are copied as they are,
/// so a token that is not a number comes back unchanged.
///
/// A token is converted only when every one of its characters is a numeral, the rule Lucene's
/// `KoreanNumberFilter` applies. Anything else is returned unchanged, which is what keeps native
/// Korean numerals intact: `일곱` ("seven") starts with the Sino-Korean `일` but is not a
/// positional numeral, and converting it character by character would give `1곱`.
fn to_arabic_numerals(from_str: &str) -> String {
    if !from_str.chars().all(is_numeral) {
        return from_str.to_owned();
    }

    let mut num_buf = String::new();
    let mut digit = String::new();

    let from_chars = from_str.chars().rev().collect::<Vec<char>>();

    let mut i = from_chars.iter().peekable();
    while let Some(c) = i.next() {
        match c {
            '0' | '０' | '영' | '공' | '〇' | '零' => num_buf.insert(0, '0'),
            '1' | '１' | '일' | '一' | '壹' => num_buf.insert(0, '1'),
            '2' | '２' | '이' | '二' | '貳' | '貮' => num_buf.insert(0, '2'),
            '3' | '３' | '삼' | '三' | '參' | '叁' => num_buf.insert(0, '3'),
            '4' | '４' | '사' | '四' | '肆' => num_buf.insert(0, '4'),
            '5' | '５' | '오' | '五' | '伍' => num_buf.insert(0, '5'),
            '6' | '６' | '육' | '륙' | '六' | '陸' => num_buf.insert(0, '6'),
            '7' | '７' | '칠' | '七' | '柒' => num_buf.insert(0, '7'),
            '8' | '８' | '팔' | '八' | '捌' => num_buf.insert(0, '8'),
            '9' | '９' | '구' | '九' | '玖' => num_buf.insert(0, '9'),
            '십' | '十' | '拾' => {
                num_buf = adjust_digits(&num_buf, "0", &digit);

                match i.peek() {
                    Some('백') | Some('百') | Some('佰') | Some('천') | Some('千') | Some('仟')
                    | Some('만') | Some('萬') | Some('万') | Some('억') | Some('億')
                    | Some('조') | Some('兆') | Some('경') | Some('京') | Some('해')
                    | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '백' | '百' | '佰' => {
                num_buf = adjust_digits(&num_buf, "00", &digit);

                match i.peek() {
                    Some('천') | Some('千') | Some('仟') | Some('만') | Some('萬') | Some('万')
                    | Some('억') | Some('億') | Some('조') | Some('兆') | Some('경')
                    | Some('京') | Some('해') | Some('垓') | None => {
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '천' | '千' | '仟' => {
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
        // The financial forms, used on cheques and contracts.
        assert_eq!(to_arabic_numerals("壹"), "1");
        assert_eq!(to_arabic_numerals("貳"), "2");
        assert_eq!(to_arabic_numerals("參"), "3");
        assert_eq!(to_arabic_numerals("肆"), "4");
        assert_eq!(to_arabic_numerals("伍"), "5");
        assert_eq!(to_arabic_numerals("陸"), "6");
        assert_eq!(to_arabic_numerals("柒"), "7");
        assert_eq!(to_arabic_numerals("捌"), "8");
        assert_eq!(to_arabic_numerals("玖"), "9");
        assert_eq!(to_arabic_numerals("壹仟貳佰參拾肆"), "1234");
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
        // Native Korean numerals are not positional, so they are left as they are.
        assert_eq!(to_arabic_numerals("하나"), "하나");
        assert_eq!(to_arabic_numerals("스물"), "스물");
        assert_eq!(to_arabic_numerals("여덟"), "여덟");
        // These carry a Sino-Korean syllable, and a character-by-character conversion would
        // turn them into `1곱`, `1흔` and `1고여덟`. The all-numeral rule keeps them whole.
        assert_eq!(to_arabic_numerals("일곱"), "일곱");
        assert_eq!(to_arabic_numerals("일흔"), "일흔");
        assert_eq!(to_arabic_numerals("일고여덟"), "일고여덟");
        // The same rule protects ordinary words that begin with a numeral character.
        assert_eq!(to_arabic_numerals("한국"), "한국");
        assert_eq!(to_arabic_numerals("參加"), "參加");
        assert_eq!(to_arabic_numerals("萬歲"), "萬歲");
        assert_eq!(to_arabic_numerals(""), "");
    }

    #[test]
    fn test_is_numeral_matches_the_conversion_table() {
        use crate::token_filter::korean_number::is_numeral;

        for c in "0123456789０９영공〇零일一壹이二貳貮삼三參叁사四肆오五伍육륙六陸칠七柒팔八捌구九玖십十拾백百佰천千仟만萬万억億조兆경京해垓".chars() {
            assert!(is_numeral(c), "{c} should be a numeral character");
        }
        for c in "가나다곱흔여덟한국字架貨店 -.".chars() {
            assert!(!is_numeral(c), "{c} should not be a numeral character");
        }
    }

    #[test]
    fn test_from_config_tags() {
        use crate::token_filter::korean_number::KoreanNumberTokenFilter;

        // An absent `tags` restricts the filter to the numeral tags.
        let filter = KoreanNumberTokenFilter::from_config(&serde_json::json!({})).unwrap();
        let tags = filter.tags.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains("SN"));
        assert!(tags.contains("NR"));

        // An explicit null asks for every token.
        let filter =
            KoreanNumberTokenFilter::from_config(&serde_json::json!({ "tags": null })).unwrap();
        assert!(filter.tags.is_none());

        // An explicit list is taken as it is.
        let filter =
            KoreanNumberTokenFilter::from_config(&serde_json::json!({ "tags": ["NR"] })).unwrap();
        assert_eq!(filter.tags.unwrap().len(), 1);

        // Anything that is not a list of strings is an error.
        assert!(
            KoreanNumberTokenFilter::from_config(&serde_json::json!({ "tags": "NR" })).is_err()
        );
        assert!(KoreanNumberTokenFilter::from_config(&serde_json::json!({ "tags": [1] })).is_err());
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

        // Without tags every token is converted.
        let filter = KoreanNumberTokenFilter::new(None);
        let mut tokens: Vec<Token> = vec![make_token("육", "NR", 0), make_token("년", "NNB", 1)];
        filter.apply(&mut tokens).unwrap();
        assert_eq!(tokens[0].surface, "6");
        assert_eq!(tokens[1].surface, "년");

        // With tags only the tokens carrying one of them are converted. Hangul numerals are
        // homographs of common morphemes, which is what the tags keep apart here.
        let filter = KoreanNumberTokenFilter::new(Some(HashSet::from(["NR".to_string()])));
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

    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_number_token_filter_with_tokenizer() {
        use crate::token_filter::BoxTokenFilter;
        use crate::token_filter::korean_number::KoreanNumberTokenFilter;
        use crate::tokenizer::Tokenizer;
        use lindera::dictionary::load_dictionary;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;

        let tokenize = |text: &str, filter: KoreanNumberTokenFilter| -> Vec<String> {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokenizer = Tokenizer::new(segmenter);
            tokenizer.append_token_filter(BoxTokenFilter::from(filter));
            tokenizer
                .tokenize(text)
                .unwrap()
                .iter()
                .map(|token| token.surface.to_string())
                .collect()
        };

        let default_filter =
            || KoreanNumberTokenFilter::from_config(&serde_json::json!({})).unwrap();

        // ko-dic tokenizes a Sino-Korean numeral into one token per morpheme, and this filter
        // converts each token on its own, so the tokens do not add up to a single number.
        // Merging them first is tracked in lindera/lindera#1026.
        assert_eq!(
            tokenize("이천이십육년", default_filter()),
            ["2", "1000", "2", "10", "6", "년"]
        );
        assert_eq!(tokenize("10만", default_filter()), ["10", "10000"]);
        assert_eq!(tokenize("2천26", default_filter()), ["2", "1000", "26"]);

        // Ordinary text is left alone under the default tags, even though it is full of
        // morphemes that are spelled like numerals.
        assert_eq!(
            tokenize("이것은 사과입니다", default_filter()),
            ["이것", "은", "사과", "입니다"]
        );

        // ko-dic tags native numerals as NR, so they reach the filter under the default tags.
        // Those that contain a Sino-Korean syllable must still come through untouched.
        assert_eq!(tokenize("일곱 명", default_filter()), ["일곱", "명"]);
        assert_eq!(tokenize("일흔 살", default_filter()), ["일흔", "살"]);
        assert_eq!(
            tokenize("일고여덟 명", default_filter()),
            ["일고여덟", "명"]
        );
        assert_eq!(
            tokenize("오늘 일이 많다", default_filter()),
            ["오늘", "일", "이", "많", "다"]
        );

        // ko-dic ships left-space penalty rules, applied by default since #1027, so a particle
        // reading is not chosen right after a space: `만` in `나는 만 원만 있다` is `만`/NR, a
        // numeral, and is converted under the default tags.
        assert_eq!(
            tokenize("나는 만 원만 있다", default_filter()),
            ["나", "는", "10000", "원만", "있", "다"]
        );

        // With `tags` set to null every token is converted. The all-numeral rule still protects
        // words that merely contain a numeral character, but a single-character morpheme that is
        // spelled exactly like a numeral is indistinguishable from one without its tag: `일`/NNG
        // and `이`/JKS become 1 and 2.
        let every_token =
            || KoreanNumberTokenFilter::from_config(&serde_json::json!({ "tags": null })).unwrap();
        assert_eq!(
            tokenize("이것은 사과입니다", every_token()),
            ["이것", "은", "사과", "입니다"]
        );
        assert_eq!(
            tokenize("오늘 일이 많다", every_token()),
            ["오늘", "1", "2", "많", "다"]
        );

        // ko-dic tags Hanja as SH, so Hanja numerals need that tag to be converted.
        let with_hanja = KoreanNumberTokenFilter::from_config(
            &serde_json::json!({ "tags": ["SN", "NR", "SH"] }),
        )
        .unwrap();
        assert_eq!(tokenize("二千二十六", with_hanja), ["2026"]);
    }
}
