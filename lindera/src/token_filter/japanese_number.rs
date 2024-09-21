use std::borrow::Cow;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use lindera_core::error::LinderaErrorKind;
use lindera_core::LinderaResult;

use crate::token::Token;
use crate::token_filter::TokenFilter;

pub const JAPANESE_NUMBER_TOKEN_FILTER_NAME: &str = "japanese_number";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseNumberTokenFilterConfig {
    tags: Option<HashSet<String>>,
}

impl JapaneseNumberTokenFilterConfig {
    pub fn new(tags: Option<HashSet<String>>) -> Self {
        match tags {
            Some(tags) => {
                let mut formatted_tags: HashSet<String> = HashSet::new();
                for tag in tags.iter() {
                    let mut formatted_tag = ["*", "*", "*", "*"];

                    let tag_array: Vec<&str> = tag.split(',').collect();
                    for (i, j) in tag_array.iter().enumerate() {
                        formatted_tag[i] = j;
                    }

                    formatted_tags.insert(formatted_tag.join(","));
                }

                Self {
                    tags: Some(formatted_tags),
                }
            }
            None => Self { tags: None },
        }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let args = serde_json::from_slice::<Value>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))?;
        Self::from_value(&args)
    }

    pub fn from_value(value: &serde_json::Value) -> LinderaResult<Self> {
        let tags = if let Some(t) = value.get("tags") {
            if t.is_array() {
                Some(
                    t.as_array()
                        .ok_or_else(|| {
                            LinderaErrorKind::Deserialize
                                .with_error(anyhow::anyhow!("tags is required"))
                        })?
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| {
                                    LinderaErrorKind::Deserialize
                                        .with_error(anyhow::anyhow!("tag must be string"))
                                })
                                .map(|s| s.to_string())
                        })
                        .collect::<LinderaResult<HashSet<String>>>()?,
                )
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self::new(tags))
    }
}

/// Convert tokens representing Japanese numerals, including Kanji numerals, to Arabic numerals.
///
#[derive(Clone, Debug)]
pub struct JapaneseNumberTokenFilter {
    config: JapaneseNumberTokenFilterConfig,
}

impl JapaneseNumberTokenFilter {
    pub fn new(config: JapaneseNumberTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseNumberTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseNumberTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_NUMBER_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let details = token.details();

            // If the length of the details is greater than or equal to 4,
            // the tag length is 4, otherwise 1 is assigned to tags_len.
            let tags_len = if details.len() >= 4 { 4 } else { 1 };

            // Make a string of the part-of-speech tags.
            let tag = details[0..tags_len].join(",");

            let should_convert = match self.config.tags {
                Some(ref tags) => tags.contains(&tag),
                None => true, // If no tags are provided, apply to all tokens
            };

            if should_convert {
                let text = token.text.as_ref();
                token.text = Cow::Owned(to_arabic_numerals(text));
            }
        }

        Ok(())
    }
}

fn adjust_digits(num: &str, base: &str, digit: &str) -> String {
    let zero_str = format!("{}{}", base, digit);
    let zero_len = zero_str.len() - num.len();
    let zeros = &zero_str[0..zero_len];

    let mut num_str = num.to_owned();
    num_str.insert_str(0, zeros);
    num_str
}

fn to_arabic_numerals(from_str: &str) -> String {
    let mut num_buf = String::new();
    let mut digit = String::new();

    let from_chars = from_str.chars().rev().collect::<Vec<char>>();

    let mut i = from_chars.iter().peekable();
    while let Some(c) = i.next() {
        match c {
            '0' | '０' | '〇' | '零' => num_buf.insert(0, '0'),
            '1' | '１' | '一' | '壱' => num_buf.insert(0, '1'),
            '2' | '２' | '二' | '弐' => num_buf.insert(0, '2'),
            '3' | '３' | '三' | '参' => num_buf.insert(0, '3'),
            '4' | '４' | '四' => num_buf.insert(0, '4'),
            '5' | '５' | '五' => num_buf.insert(0, '5'),
            '6' | '６' | '六' => num_buf.insert(0, '6'),
            '7' | '７' | '七' => num_buf.insert(0, '7'),
            '8' | '８' | '八' => num_buf.insert(0, '8'),
            '9' | '９' | '九' => num_buf.insert(0, '9'),
            '十' | '拾' => {
                num_buf = adjust_digits(&num_buf, "0", &digit);

                match i.peek() {
                    Some('百') | Some('千') | Some('万') | Some('億') | Some('兆') | Some('京')
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
            '百' => {
                num_buf = adjust_digits(&num_buf, "00", &digit);

                match i.peek() {
                    Some('千') | Some('万') | Some('億') | Some('兆') | Some('京') | Some('垓')
                    | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '千' => {
                num_buf = adjust_digits(&num_buf, "000", &digit);

                match i.peek() {
                    Some('万') | Some('億') | Some('兆') | Some('京') | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '万' => {
                digit = "0000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('億') | Some('兆') | Some('京') | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '億' => {
                digit = "00000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('兆') | Some('京') | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '兆' => {
                digit = "000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('京') | Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '京' => {
                digit = "0000000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                match i.peek() {
                    Some('垓') | None => {
                        // If the first character is a '0', the '1' has been omitted.
                        // Therefore, insert a leading '1'.
                        num_buf.insert(0, '1');
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
            '垓' => {
                digit = "00000000000000000000".to_string();

                num_buf = adjust_digits(&num_buf, "", &digit);

                if i.peek().is_none() {
                    // If the first character is a '0', the '1' has been omitted.
                    // Therefore, insert a leading '1'.
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

    #[test]
    fn test_to_number_str_ipadic() {
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
    fn test_japanese_number_token_filter_config_from_slice_ipadic() {
        use crate::token_filter::japanese_number::JapaneseNumberTokenFilterConfig;

        {
            let config_str = r#"
                {
                    "tags": null
                }
                "#;
            let config =
                JapaneseNumberTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

            assert!(config.tags.is_none());
        }

        {
            let config_str = r#"
                {
                }
                "#;
            let config =
                JapaneseNumberTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

            assert!(config.tags.is_none());
        }

        {
            let config_str = r#"
                {
                    "tags": [
                        "名詞,数"
                    ]
                }
                "#;
            let config =
                JapaneseNumberTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

            assert!(config.tags.is_some());

            assert!(config.tags.unwrap().contains("名詞,数,*,*"));
        }
    }

    #[test]
    fn test_japanese_number_token_filter_from_slice_ipadic() {
        use crate::token_filter::japanese_number::JapaneseNumberTokenFilter;

        {
            // test empty tags
            let config_str = r#"
                {
                }
                "#;
            let result = JapaneseNumberTokenFilter::from_slice(config_str.as_bytes());

            assert_eq!(true, result.is_ok());
        }

        {
            let config_str = r#"
                {
                    "tags": [
                        "名詞,数"
                    ]
                }
                "#;
            let result = JapaneseNumberTokenFilter::from_slice(config_str.as_bytes());

            assert_eq!(true, result.is_ok());
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_number_token_filter_apply_numbers_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token_filter::japanese_number::JapaneseNumberTokenFilter;
        use crate::{token::Token, token_filter::TokenFilter};

        let config_str = r#"
            {
                "tags": [
                    "名詞,数"
                ]
            }
            "#;
        let filter = JapaneseNumberTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("一"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId(102657, true),
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
            assert_eq!(&tokens[0].text, "1");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("一二三"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(102657, true),
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
            assert_eq!(&tokens[0].text, "123");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    text: Cow::Borrowed("一千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四"),
                    byte_start: 0,
                    byte_end: 129,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(102657, true),
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
                },
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].text, "123456789012345678901234");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    text: Cow::Borrowed("鈴木"),
                    byte_start: 0,
                    byte_end: 6,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(368893, true),
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
                    text: Cow::Borrowed("一郎"),
                    byte_start: 6,
                    byte_end: 12,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(103913, true),
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
            assert_eq!(&tokens[0].text, "鈴木");
            assert_eq!(&tokens[1].text, "一郎");
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_japanese_number_token_filter_apply_empty_ipadic() {
        use std::borrow::Cow;

        use lindera_core::dictionary::word_entry::WordId;

        use crate::dictionary::{DictionaryKind, DictionaryLoader};
        use crate::token_filter::japanese_number::JapaneseNumberTokenFilter;
        use crate::{token::Token, token_filter::TokenFilter};

        let config_str = r#"
            {
            }
            "#;
        let filter = JapaneseNumberTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary =
            DictionaryLoader::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("一"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId(102657, true),
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
            assert_eq!(&tokens[0].text, "1");
        }

        {
            let mut tokens: Vec<Token> = vec![Token {
                text: Cow::Borrowed("一二三"),
                byte_start: 0,
                byte_end: 9,
                position: 0,
                position_length: 1,
                word_id: WordId(102657, true),
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
            assert_eq!(&tokens[0].text, "123");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    text: Cow::Borrowed("一千二百三十四垓五千六百七十八京九千十二兆三千四百五十六億七千八百九十万一千二百三十四"),
                    byte_start: 0,
                    byte_end: 129,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(102657, true),
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
                },
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 1);
            assert_eq!(&tokens[0].text, "123456789012345678901234");
        }

        {
            let mut tokens: Vec<Token> = vec![
                Token {
                    text: Cow::Borrowed("鈴木"),
                    byte_start: 0,
                    byte_end: 6,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(368893, true),
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
                    text: Cow::Borrowed("一郎"),
                    byte_start: 6,
                    byte_end: 12,
                    position: 0,
                    position_length: 1,
                    word_id: WordId(103913, true),
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
            assert_eq!(&tokens[0].text, "鈴木");
            assert_eq!(&tokens[1].text, "1郎");
        }
    }
}
