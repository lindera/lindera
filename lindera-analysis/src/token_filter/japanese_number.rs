use std::borrow::Cow;
use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::token_filter::numeral::{self, Numeral};
use crate::token_filter::tags::{
    KEY_BUFFER_CAPACITY, normalize_japanese_tags, part_of_speech_offset_of, write_japanese_pos_key,
};
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

pub const JAPANESE_NUMBER_TOKEN_FILTER_NAME: &str = "japanese_number";

pub type JapaneseNumberTokenFilterConfig = Value;

/// Convert tokens representing Japanese numerals, including Kanji numerals, to Arabic numerals.
///
#[derive(Clone, Debug)]
pub struct JapaneseNumberTokenFilter {
    /// The four-part part-of-speech tags a token must carry to be converted,
    /// or `None` to convert every token.
    tags: Option<HashSet<String>>,
}

impl JapaneseNumberTokenFilter {
    /// Creates the filter, padding each tag to four comma-separated parts.
    ///
    /// # 引数
    ///
    /// * `tags` - Part-of-speech tags, one to four comma-separated levels
    ///   each, or `None` to convert every token.
    ///
    /// # 戻り値
    ///
    /// The configured filter.
    pub fn new(tags: Option<HashSet<String>>) -> Self {
        Self {
            tags: tags.map(normalize_japanese_tags),
        }
    }

    pub fn from_config(config: &JapaneseNumberTokenFilterConfig) -> LinderaResult<Self> {
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

impl TokenFilter for JapaneseNumberTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_NUMBER_TOKEN_FILTER_NAME
    }

    /// Converts token text to Arabic numerals if the token's part-of-speech tags match the specified configuration.
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
    ///    - The function iterates over the tokens and extracts the part-of-speech tags from each token's details.
    ///    - The four details starting at the schema's `part_of_speech` field form the tag, exactly as in `japanese_keep_tags`; a token with fewer details yields a shorter tag, which matches nothing.
    ///
    /// 2. **Tag Matching**:
    ///    - If the configuration contains specific tags (`self.tags`), the function checks whether the token's tag matches any of them.
    ///    - If no tags are specified (`None`), the function applies the conversion to all tokens without reading their details.
    ///
    /// 3. **Text Conversion**:
    ///    - For tokens that match the criteria, the text is converted to Arabic numerals using the `to_arabic_numerals` function and stored as `Cow::Owned`.
    ///
    /// # Errors
    ///
    /// If any issue arises during token processing or text conversion, the function will return an error in the form of `LinderaResult`.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        let Some(tags) = &self.tags else {
            // No tag restriction: every token is converted, so no key is built.
            for token in tokens.iter_mut() {
                token.surface = Cow::Owned(to_arabic_numerals(&token.surface));
            }
            return Ok(());
        };

        // The key is the same as the keep/stop tag filters': the up-to-4
        // part-of-speech levels from where the dictionary schema puts them
        // (resolved once per call), written into one buffer reused across
        // tokens.
        let offset = part_of_speech_offset_of(tokens);
        let mut key = String::with_capacity(KEY_BUFFER_CAPACITY);
        for token in tokens.iter_mut() {
            key.clear();
            write_japanese_pos_key(token, offset, &mut key);
            if tags.contains(key.as_str()) {
                token.surface = Cow::Owned(to_arabic_numerals(&token.surface));
            }
        }

        Ok(())
    }
}

/// Maps a character to its meaning inside a Japanese numeral.
///
/// Covers the ASCII and fullwidth digits, the kanji digits with their formal (大字) variants, and
/// the position characters from 十 to 垓.
fn classify(c: char) -> Option<Numeral> {
    let numeral = match c {
        '0' | '０' | '〇' | '零' => Numeral::Digit('0'),
        '1' | '１' | '一' | '壱' => Numeral::Digit('1'),
        '2' | '２' | '二' | '弐' => Numeral::Digit('2'),
        '3' | '３' | '三' | '参' => Numeral::Digit('3'),
        '4' | '４' | '四' => Numeral::Digit('4'),
        '5' | '５' | '五' => Numeral::Digit('5'),
        '6' | '６' | '六' => Numeral::Digit('6'),
        '7' | '７' | '七' => Numeral::Digit('7'),
        '8' | '８' | '八' => Numeral::Digit('8'),
        '9' | '９' | '九' => Numeral::Digit('9'),
        '十' | '拾' => Numeral::Position(1),
        '百' => Numeral::Position(2),
        '千' => Numeral::Position(3),
        '万' => Numeral::Position(4),
        '億' => Numeral::Position(5),
        '兆' => Numeral::Position(6),
        '京' => Numeral::Position(7),
        '垓' => Numeral::Position(8),
        _ => return None,
    };

    Some(numeral)
}

/// Converts a Japanese numeral into Arabic numerals.
///
/// A token is converted only when every one of its characters takes part in a numeral, so an
/// ordinary word that begins with one (`一部`, `万歳`) is returned unchanged. See
/// [`super::numeral::to_arabic_numerals`].
fn to_arabic_numerals(from_str: &str) -> String {
    numeral::to_arabic_numerals(from_str, classify)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    use crate::token_filter::japanese_number::{
        JapaneseNumberTokenFilter, JapaneseNumberTokenFilterConfig,
    };

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_to_number_str() {
        use std::str::FromStr;

        use crate::token_filter::japanese_number::to_arabic_numerals;

        {
            let s = "０";
            assert_eq!(to_arabic_numerals(s), String::from_str("0").unwrap());
        }

        {
            let s = "〇";
            assert_eq!(to_arabic_numerals(s), String::from_str("0").unwrap());
        }

        {
            let s = "零";
            assert_eq!(to_arabic_numerals(s), String::from_str("0").unwrap());
        }

        {
            let s = "１";
            assert_eq!(to_arabic_numerals(s), String::from_str("1").unwrap());
        }

        {
            let s = "一";
            assert_eq!(to_arabic_numerals(s), String::from_str("1").unwrap());
        }

        {
            let s = "壱";
            assert_eq!(to_arabic_numerals(s), String::from_str("1").unwrap());
        }

        {
            let s = "２";
            assert_eq!(to_arabic_numerals(s), String::from_str("2").unwrap());
        }

        {
            let s = "二";
            assert_eq!(to_arabic_numerals(s), String::from_str("2").unwrap());
        }

        {
            let s = "弐";
            assert_eq!(to_arabic_numerals(s), String::from_str("2").unwrap());
        }

        {
            let s = "３";
            assert_eq!(to_arabic_numerals(s), String::from_str("3").unwrap());
        }

        {
            let s = "三";
            assert_eq!(to_arabic_numerals(s), String::from_str("3").unwrap());
        }

        {
            let s = "参";
            assert_eq!(to_arabic_numerals(s), String::from_str("3").unwrap());
        }

        {
            let s = "４";
            assert_eq!(to_arabic_numerals(s), String::from_str("4").unwrap());
        }

        {
            let s = "四";
            assert_eq!(to_arabic_numerals(s), String::from_str("4").unwrap());
        }

        {
            let s = "５";
            assert_eq!(to_arabic_numerals(s), String::from_str("5").unwrap());
        }

        {
            let s = "五";
            assert_eq!(to_arabic_numerals(s), String::from_str("5").unwrap());
        }

        {
            let s = "６";
            assert_eq!(to_arabic_numerals(s), String::from_str("6").unwrap());
        }

        {
            let s = "六";
            assert_eq!(to_arabic_numerals(s), String::from_str("6").unwrap());
        }

        {
            let s = "７";
            assert_eq!(to_arabic_numerals(s), String::from_str("7").unwrap());
        }

        {
            let s = "七";
            assert_eq!(to_arabic_numerals(s), String::from_str("7").unwrap());
        }

        {
            let s = "８";
            assert_eq!(to_arabic_numerals(s), String::from_str("8").unwrap());
        }

        {
            let s = "八";
            assert_eq!(to_arabic_numerals(s), String::from_str("8").unwrap());
        }

        {
            let s = "９";
            assert_eq!(to_arabic_numerals(s), String::from_str("9").unwrap());
        }

        {
            let s = "九";
            assert_eq!(to_arabic_numerals(s), String::from_str("9").unwrap());
        }

        {
            let s = "十";
            assert_eq!(to_arabic_numerals(s), String::from_str("10").unwrap());
        }

        {
            let s = "拾";
            assert_eq!(to_arabic_numerals(s), String::from_str("10").unwrap());
        }

        {
            let s = "百";
            assert_eq!(to_arabic_numerals(s), String::from_str("100").unwrap());
        }

        {
            let s = "千";
            assert_eq!(to_arabic_numerals(s), String::from_str("1000").unwrap());
        }

        {
            let s = "万";
            assert_eq!(to_arabic_numerals(s), String::from_str("10000").unwrap());
        }

        {
            let s = "億";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("100000000").unwrap()
            );
        }

        {
            let s = "兆";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1000000000000").unwrap()
            );
        }

        {
            let s = "京";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("10000000000000000").unwrap()
            );
        }

        {
            let s = "垓";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("100000000000000000000").unwrap()
            );
        }

        {
            let s = "百一";
            assert_eq!(to_arabic_numerals(s), String::from_str("101").unwrap());
        }

        {
            let s = "百十";
            assert_eq!(to_arabic_numerals(s), String::from_str("110").unwrap());
        }

        {
            let s = "千百十";
            assert_eq!(to_arabic_numerals(s), String::from_str("1110").unwrap());
        }

        {
            let s = "万千百十";
            assert_eq!(to_arabic_numerals(s), String::from_str("11110").unwrap());
        }

        {
            let s = "十万千百十";
            assert_eq!(to_arabic_numerals(s), String::from_str("101110").unwrap());
        }

        {
            let s = "千十";
            assert_eq!(to_arabic_numerals(s), String::from_str("1010").unwrap());
        }

        {
            let s = "十二";
            assert_eq!(to_arabic_numerals(s), String::from_str("12").unwrap());
        }

        {
            let s = "一十二";
            assert_eq!(to_arabic_numerals(s), String::from_str("12").unwrap());
        }

        {
            let s = "百二十三";
            assert_eq!(to_arabic_numerals(s), String::from_str("123").unwrap());
        }

        {
            let s = "一百二十三";
            assert_eq!(to_arabic_numerals(s), String::from_str("123").unwrap());
        }

        {
            let s = "千二百三十四";
            assert_eq!(to_arabic_numerals(s), String::from_str("1234").unwrap());
        }

        {
            let s = "一千二百三十四";
            assert_eq!(to_arabic_numerals(s), String::from_str("1234").unwrap());
        }

        {
            let s = "万二千三百四十五";
            assert_eq!(to_arabic_numerals(s), String::from_str("12345").unwrap());
        }

        {
            let s = "一万二千三百四十五";
            assert_eq!(to_arabic_numerals(s), String::from_str("12345").unwrap());
        }

        {
            let s = "十二万三千四百五十六";
            assert_eq!(to_arabic_numerals(s), String::from_str("123456").unwrap());
        }

        {
            let s = "一十二万三千四百五十六";
            assert_eq!(to_arabic_numerals(s), String::from_str("123456").unwrap());
        }

        {
            let s = "百二十三万四千五百六十七";
            assert_eq!(to_arabic_numerals(s), String::from_str("1234567").unwrap());
        }

        {
            let s = "一百二十三万四千五百六十七";
            assert_eq!(to_arabic_numerals(s), String::from_str("1234567").unwrap());
        }

        {
            let s = "千二百三十四万五千六百七十八";
            assert_eq!(to_arabic_numerals(s), String::from_str("12345678").unwrap());
        }

        {
            let s = "一千二百三十四万五千六百七十八";
            assert_eq!(to_arabic_numerals(s), String::from_str("12345678").unwrap());
        }

        {
            let s = "億二千三百四十五万六千七百八十九";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789").unwrap()
            );
        }

        {
            let s = "一億二千三百四十五万六千七百八十九";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789").unwrap()
            );
        }

        {
            let s = "十二億三千四百五十六万七千八百九十";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890").unwrap()
            );
        }

        {
            let s = "一十二億三千四百五十六万七千八百九十";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890").unwrap()
            );
        }

        {
            let s = "百二十三億四千五百六十七万八千九百一";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901").unwrap()
            );
        }

        {
            let s = "一百二十三億四千五百六十七万八千九百一";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901").unwrap()
            );
        }

        {
            let s = "千二百三十四億五千六百七十八万九千十二";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012").unwrap()
            );
        }

        {
            let s = "一千二百三十四億五千六百七十八万九千十二";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012").unwrap()
            );
        }

        {
            let s = "兆二千三百四十五億六千七百八十九万百二十三";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123").unwrap()
            );
        }

        {
            let s = "一兆二千三百四十五億六千七百八十九万百二十三";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123").unwrap()
            );
        }

        {
            let s = "十二兆三千四百五十六億七千八百九十万千二百三十四";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234").unwrap()
            );
        }

        {
            let s = "一十二兆三千四百五十六億七千八百九十万千二百三十四";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234").unwrap()
            );
        }

        {
            let s = "百二十三兆四千五百六十七億八千九百一万二千三百四十五";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345").unwrap()
            );
        }

        {
            let s = "一百二十三兆四千五百六十七億八千九百一万二千三百四十五";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345").unwrap()
            );
        }

        {
            let s = "千二百三十四兆五千六百七十八億九千十二万三千四百五十六";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456").unwrap()
            );
        }

        {
            let s = "一千二百三十四兆五千六百七十八億九千十二万三千四百五十六";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456").unwrap()
            );
        }

        {
            let s = "京二千三百四十五兆六千七百八十九億百二十三万四千五百六十七";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567").unwrap()
            );
        }

        {
            let s = "一京二千三百四十五兆六千七百八十九億百二十三万四千五百六十七";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567").unwrap()
            );
        }

        {
            let s = "十二京三千四百五十六兆七千八百九十億千二百三十四万五千六百七十八";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678").unwrap()
            );
        }

        {
            let s = "一十二京三千四百五十六兆七千八百九十億千二百三十四万五千六百七十八";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678").unwrap()
            );
        }

        {
            let s = "百二十三京四千五百六十七兆八千九百一億二千三百四十五万六千七百八十九";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456789").unwrap()
            );
        }

        {
            let s = "一百二十三京四千五百六十七兆八千九百一億二千三百四十五万六千七百八十九";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456789").unwrap()
            );
        }

        {
            let s = "千二百三十四京五千六百七十八兆九千十二億三千四百五十六万七千八百九十";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567890").unwrap()
            );
        }

        {
            let s = "一千二百三十四京五千六百七十八兆九千十二億三千四百五十六万七千八百九十";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567890").unwrap()
            );
        }

        {
            let s = "垓二千三百四十五京六千七百八十九兆百二十三億四千五百六十七万八千九百一";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678901").unwrap()
            );
        }

        {
            let s = "一垓二千三百四十五京六千七百八十九兆百二十三億四千五百六十七万八千九百一";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678901").unwrap()
            );
        }

        {
            let s = "十二垓三千四百五十六京七千八百九十兆千二百三十四億五千六百七十八万九千十二";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456789012").unwrap()
            );
        }

        {
            let s = "一十二垓三千四百五十六京七千八百九十兆千二百三十四億五千六百七十八万九千十二";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("1234567890123456789012").unwrap()
            );
        }

        {
            let s =
                "百二十三垓四千五百六十七京八千九百一兆二千三百四十五億六千七百八十九万百二十三";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567890123").unwrap()
            );
        }

        {
            let s =
                "一百二十三垓四千五百六十七京八千九百一兆二千三百四十五億六千七百八十九万百二十三";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("12345678901234567890123").unwrap()
            );
        }

        {
            let s = "千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678901234").unwrap()
            );
        }

        {
            let s = "一千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四";
            assert_eq!(
                to_arabic_numerals(s),
                String::from_str("123456789012345678901234").unwrap()
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter_config() {
        {
            let config_str = r#"
                {
                    "tags": null
                }
                "#;
            let result: Result<JapaneseNumberTokenFilterConfig, _> =
                serde_json::from_str(config_str);
            assert!(result.is_ok());
        }
        {
            let config_str = r#"
                {
                }
                "#;
            let result: Result<JapaneseNumberTokenFilterConfig, _> =
                serde_json::from_str(config_str);
            assert!(result.is_ok());
        }

        {
            let config_str = r#"
                {
                    "tags": [
                        "名詞,数"
                    ]
                }
                "#;
            let result: Result<JapaneseNumberTokenFilterConfig, _> =
                serde_json::from_str(config_str);
            assert!(result.is_ok());
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter() {
        {
            // test empty tags
            let config_str = r#"
                {
                }
                "#;
            let config: JapaneseNumberTokenFilterConfig = serde_json::from_str(config_str).unwrap();
            let result = JapaneseNumberTokenFilter::from_config(&config);

            assert!(result.is_ok());
        }

        {
            let config_str = r#"
                {
                    "tags": [
                        "名詞,数"
                    ]
                }
                "#;
            let config: JapaneseNumberTokenFilterConfig = serde_json::from_str(config_str).unwrap();
            let result = JapaneseNumberTokenFilter::from_config(&config);

            assert!(result.is_ok());
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter_apply_numbers_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
            {
                "tags": [
                    "名詞,数"
                ]
            }
            "#;
        let config: JapaneseNumberTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseNumberTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("一"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("一"),
                    Cow::Borrowed("イチ"),
                    Cow::Borrowed("イチ"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "1");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("一二三"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "123");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed(
                    "一千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四",
                ),
                byte_start: 0,
                byte_end: 129,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "123456789012345678901234");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    surface: Cow::Borrowed("鈴木"),
                    byte_start: 0,
                    byte_end: 6,
                    position: 0,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, 368893),
                    dictionary: &dictionary,
                    user_dictionary: None,
                    details: Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("固有名詞"),
                        Cow::Borrowed("人名"),
                        Cow::Borrowed("姓"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("鈴木"),
                        Cow::Borrowed("スズキ"),
                        Cow::Borrowed("スズキ"),
                    ]),
                },
                Token {
                    surface: Cow::Borrowed("一郎"),
                    byte_start: 6,
                    byte_end: 12,
                    position: 0,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, 103913),
                    dictionary: &dictionary,
                    user_dictionary: None,
                    details: Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("固有名詞"),
                        Cow::Borrowed("人名"),
                        Cow::Borrowed("名"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("一郎"),
                        Cow::Borrowed("イチロウ"),
                        Cow::Borrowed("イチロー"),
                    ]),
                },
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 2);
            assert_eq!(&tokens[0].surface, "鈴木");
            assert_eq!(&tokens[1].surface, "一郎");
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter_apply_empty_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
            {
            }
            "#;
        let config: JapaneseNumberTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = JapaneseNumberTokenFilter::from_config(&config).unwrap();
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("一"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("一"),
                    Cow::Borrowed("イチ"),
                    Cow::Borrowed("イチ"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "1");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed("一二三"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "123");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                surface: Cow::Borrowed(
                    "一千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四",
                ),
                byte_start: 0,
                byte_end: 129,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 102657),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            }];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].surface, "123456789012345678901234");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    surface: Cow::Borrowed("鈴木"),
                    byte_start: 0,
                    byte_end: 6,
                    position: 0,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, 368893),
                    dictionary: &dictionary,
                    user_dictionary: None,
                    details: Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("固有名詞"),
                        Cow::Borrowed("人名"),
                        Cow::Borrowed("姓"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("鈴木"),
                        Cow::Borrowed("スズキ"),
                        Cow::Borrowed("スズキ"),
                    ]),
                },
                Token {
                    surface: Cow::Borrowed("一郎"),
                    byte_start: 6,
                    byte_end: 12,
                    position: 0,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, 103913),
                    dictionary: &dictionary,
                    user_dictionary: None,
                    details: Some(vec![
                        Cow::Borrowed("名詞"),
                        Cow::Borrowed("固有名詞"),
                        Cow::Borrowed("人名"),
                        Cow::Borrowed("名"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("*"),
                        Cow::Borrowed("一郎"),
                        Cow::Borrowed("イチロウ"),
                        Cow::Borrowed("イチロー"),
                    ]),
                },
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 2);
            // An empty configuration converts every token regardless of its part of speech, but
            // only tokens that are numerals through and through: `一郎` is a name that begins
            // with a numeral character, so it is returned unchanged rather than as `1郎`.
            assert_eq!(&tokens[0].surface, "鈴木");
            assert_eq!(&tokens[1].surface, "一郎");
        }
    }

    #[test]
    fn test_to_number_str_leaves_mixed_tokens_alone() {
        use crate::token_filter::japanese_number::to_arabic_numerals;

        // A token is converted only when every one of its characters takes part in a numeral.
        // Without that rule these are rewritten in place: `一部` becomes `1部`, `万歳` becomes
        // `10歳` (the counter read as digits by the old byte-based padding) and `何億兆回`
        // becomes `何000000000回`, which is the residue of the panic reported in #326.
        assert_eq!(to_arabic_numerals("一部"), "一部");
        assert_eq!(to_arabic_numerals("万歳"), "万歳");
        assert_eq!(to_arabic_numerals("一万円"), "一万円");
        assert_eq!(to_arabic_numerals("何億兆回"), "何億兆回");
        assert_eq!(to_arabic_numerals("一郎"), "一郎");
        assert_eq!(to_arabic_numerals("鈴木"), "鈴木");

        // All-numeral tokens are unaffected by the rule.
        assert_eq!(to_arabic_numerals("一"), "1");
        assert_eq!(to_arabic_numerals("一二三"), "123");
        assert_eq!(to_arabic_numerals("二千二十六"), "2026");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter_with_tokenizer() {
        use std::collections::HashSet;

        use crate::token_filter::BoxTokenFilter;
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
        use crate::token_filter::japanese_number::JapaneseNumberTokenFilter;
        use crate::tokenizer::Tokenizer;
        use lindera::dictionary::load_dictionary;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;

        // The documented recipe: merge the numeral tokens first, then convert. The counter has
        // to stay a separate token; merged into the number it would be read as digits.
        let tokenize = |text: &str| -> Vec<String> {
            let dictionary = load_dictionary("embedded://ipadic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokenizer = Tokenizer::new(segmenter);
            tokenizer.append_token_filter(BoxTokenFilter::from(
                JapaneseCompoundWordTokenFilter::new(
                    HashSet::from(["名詞,数".to_string()]),
                    Some("名詞,数".to_string()),
                ),
            ));
            tokenizer.append_token_filter(BoxTokenFilter::from(JapaneseNumberTokenFilter::new(
                Some(HashSet::from(["名詞,数".to_string()])),
            )));
            tokenizer
                .tokenize(text)
                .unwrap()
                .iter()
                .map(|token| token.surface.to_string())
                .collect()
        };

        assert_eq!(tokenize("一万円"), ["10000", "円"]);
        assert_eq!(tokenize("二千二十六年"), ["2026", "年"]);
        assert_eq!(tokenize("三千五百円"), ["3500", "円"]);
        assert_eq!(tokenize("百五十人"), ["150", "人"]);
        assert_eq!(tokenize("2026年"), ["2026", "年"]);
    }

    /// With the SudachiDict schema the part-of-speech hierarchy is details
    /// `1..5` (`display_surface` comes first), so `tags` must be matched
    /// there: the numeral is converted and the counter, which is not a
    /// `名詞,数詞`, is left alone (#997).
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_number_token_filter_apply_sudachidict_schema() {
        use std::collections::HashSet;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::tags::test_support::{sudachidict_schema_dictionary, tokens};

        let dictionary = sudachidict_schema_dictionary();
        let rows: [(&str, &[&str]); 2] = [
            (
                "五",
                &[
                    "五", "名詞", "数詞", "*", "*", "*", "*", "ゴ", "五", "*", "A", "*", "*", "*",
                    "017040",
                ],
            ),
            (
                "年",
                &[
                    "年",
                    "名詞",
                    "普通名詞",
                    "助数詞可能",
                    "*",
                    "*",
                    "*",
                    "ネン",
                    "年",
                    "*",
                    "A",
                    "*",
                    "*",
                    "*",
                    "*",
                ],
            ),
        ];

        let filter = JapaneseNumberTokenFilter::new(Some(HashSet::from(["名詞,数詞".to_string()])));
        let mut restricted = tokens(&dictionary, &rows);
        filter.apply(&mut restricted).unwrap();
        let surfaces: Vec<&str> = restricted.iter().map(|t| t.surface.as_ref()).collect();
        assert_eq!(surfaces, ["5", "年"]);

        // A tag spelled the way the old positional key came out (display
        // surface first) must not match anymore.
        let filter =
            JapaneseNumberTokenFilter::new(Some(HashSet::from(["五,名詞,数詞".to_string()])));
        let mut untouched = tokens(&dictionary, &rows);
        filter.apply(&mut untouched).unwrap();
        assert_eq!(untouched[0].surface, "五");

        // Without `tags` every token is converted regardless of the schema.
        let filter = JapaneseNumberTokenFilter::new(None);
        let mut all = tokens(&dictionary, &rows);
        filter.apply(&mut all).unwrap();
        assert_eq!(all[0].surface, "5");
    }
}
