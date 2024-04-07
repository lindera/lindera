pub type LinderaResult<T> = lindera_core::LinderaResult<T>;
pub type LinderaError = lindera_core::error::LinderaError;
pub type LinderaErrorKind = lindera_core::error::LinderaErrorKind;
pub type Mode = lindera_core::mode::Mode;
pub type Penalty = lindera_core::mode::Penalty;
pub type DictionaryConfig = lindera_dictionary::DictionaryConfig;
pub type DictionaryKind = lindera_dictionary::DictionaryKind;
pub type UserDictionaryConfig = lindera_dictionary::UserDictionaryConfig;
pub type Tokenizer = lindera_tokenizer::tokenizer::Tokenizer;
pub type TokenizerConfig = lindera_tokenizer::tokenizer::TokenizerConfig;
pub type DictionaryBuilderResolver = lindera_dictionary::DictionaryBuilderResolver;
pub type Dictionary = lindera_core::dictionary::Dictionary;
pub type UserDictionary = lindera_core::dictionary::UserDictionary;
#[cfg(feature = "filter")]
pub type Analyzer = lindera_analyzer::analyzer::Analyzer;
#[cfg(feature = "filter")]
pub type AnalyzerConfig = lindera_analyzer::analyzer::AnalyzerConfig;
pub type AnalyzerToken = lindera_analyzer::token::Token;
#[cfg(feature = "filter")]
pub type BoxCharacterFilter = lindera_analyzer::character_filter::BoxCharacterFilter;
#[cfg(feature = "filter")]
pub type JapaneseIterationMarkCharacterFilter = lindera_analyzer::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
#[cfg(feature = "filter")]
pub type JapaneseIterationMarkCharacterFilterConfig = lindera_analyzer::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilterConfig;
#[cfg(feature = "filter")]
pub type MappingCharacter = lindera_analyzer::character_filter::mapping::MappingCharacterFilter;
#[cfg(feature = "filter")]
pub type MappingCharacterFilterConfig =
    lindera_analyzer::character_filter::mapping::MappingCharacterFilterConfig;
#[cfg(feature = "filter")]
pub type RegexCharacterFilter = lindera_analyzer::character_filter::regex::RegexCharacterFilter;
#[cfg(feature = "filter")]
pub type RegexCharacterFilterConfig =
    lindera_analyzer::character_filter::regex::RegexCharacterFilterConfig;
#[cfg(feature = "filter")]
pub type UnicodeNormalizeCharacterFilter =
    lindera_analyzer::character_filter::unicode_normalize::UnicodeNormalizeCharacterFilter;
#[cfg(feature = "filter")]
pub type UnicodeNormalizeCharacterFilterConfig =
    lindera_analyzer::character_filter::unicode_normalize::UnicodeNormalizeCharacterFilterConfig;
#[cfg(feature = "filter")]
pub type UnicodeNormalizeKind =
    lindera_analyzer::character_filter::unicode_normalize::UnicodeNormalizeKind;
#[cfg(feature = "filter")]
pub type BoxTokenFilter = lindera_analyzer::token_filter::BoxTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseBaseFormTokenFilter =
    lindera_analyzer::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseBaseFormTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_base_form::JapaneseBaseFormTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseCompoundWordTokenFilter =
    lindera_analyzer::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseCompoundWordTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseKanaTokenFilter =
    lindera_analyzer::token_filter::japanese_kana::JapaneseKanaTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseKanaTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_kana::JapaneseKanaTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseKatakanaStemTokenFilter =
    lindera_analyzer::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseKatakanaStemTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseKeepTagsTokenFilter =
    lindera_analyzer::token_filter::japanese_keep_tags::JapaneseKeepTagsTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseKeepTagsTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_keep_tags::JapaneseKeepTagsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseNumberTokenFilter =
    lindera_analyzer::token_filter::japanese_number::JapaneseNumberTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseNumberTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_number::JapaneseNumberTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseReadingFormTokenFilter =
    lindera_analyzer::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseReadingFormTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilterConfig;
#[cfg(feature = "filter")]
pub type JapaneseStopTagsTokenFilter =
    lindera_analyzer::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
#[cfg(feature = "filter")]
pub type JapaneseStopTagsTokenFilterConfig =
    lindera_analyzer::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type KeepWordsTokenFilter = lindera_analyzer::token_filter::keep_words::KeepWordsTokenFilter;
#[cfg(feature = "filter")]
pub type KeepWordsTokenFilterConfig =
    lindera_analyzer::token_filter::keep_words::KeepWordsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type KoreanKeepTagsTokenFilter =
    lindera_analyzer::token_filter::korean_keep_tags::KoreanKeepTagsTokenFilter;
#[cfg(feature = "filter")]
pub type KoreanKeepTagsTokenFilterConfig =
    lindera_analyzer::token_filter::korean_keep_tags::KoreanKeepTagsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type KoreanReadingFormTokenFilter =
    lindera_analyzer::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
#[cfg(feature = "filter")]
pub type KoreanStopTagsTokenFilter =
    lindera_analyzer::token_filter::korean_stop_tags::KoreanStopTagsTokenFilter;
#[cfg(feature = "filter")]
pub type KoreanStopTagsTokenFilterConfig =
    lindera_analyzer::token_filter::korean_stop_tags::KoreanStopTagsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type LengthTokenFilter = lindera_analyzer::token_filter::length::LengthTokenFilter;
#[cfg(feature = "filter")]
pub type LengthTokenFilterConfig = lindera_analyzer::token_filter::length::LengthTokenFilterConfig;
#[cfg(feature = "filter")]
pub type LowercaseTokenFilter = lindera_analyzer::token_filter::lowercase::LowercaseTokenFilter;
#[cfg(feature = "filter")]
pub type MappingTokenFilter = lindera_analyzer::token_filter::mapping::MappingTokenFilter;
#[cfg(feature = "filter")]
pub type MappingTokenFilterConfig =
    lindera_analyzer::token_filter::mapping::MappingTokenFilterConfig;
#[cfg(feature = "filter")]
pub type StopWordsTokenFilter = lindera_analyzer::token_filter::stop_words::StopWordsTokenFilter;
#[cfg(feature = "filter")]
pub type StopWordsTokenFilterConfig =
    lindera_analyzer::token_filter::stop_words::StopWordsTokenFilterConfig;
#[cfg(feature = "filter")]
pub type UppercaseTokenFilter = lindera_analyzer::token_filter::uppercase::UppercaseTokenFilter;

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    #[cfg(any(
        feature = "ipadic",
        feature = "ipadic-neologd",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    use crate::{
        DictionaryConfig, DictionaryKind, Mode, Penalty, Tokenizer, TokenizerConfig,
        UserDictionaryConfig,
    };

    #[cfg(feature = "filter")]
    use crate::{Analyzer, AnalyzerConfig};

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_config_ipadic_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": "normal"
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::IPADIC));
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_config_ipadic_decompose() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ipadic"
            },
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::IPADIC));
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。テスト。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "日本語");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "日本語",
                    "ニホンゴ",
                    "ニホンゴ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "形態素");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "形態素",
                    "ケイタイソ",
                    "ケイタイソ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "解析",
                    "カイセキ",
                    "カイセキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "を", "ヲ", "ヲ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "自立",
                    "*",
                    "*",
                    "五段・ワ行促音便",
                    "基本形",
                    "行う",
                    "オコナウ",
                    "オコナウ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "非自立",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "こと",
                    "コト",
                    "コト"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "が", "ガ", "ガ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "自立",
                    "*",
                    "*",
                    "一段",
                    "連用形",
                    "できる",
                    "デキ",
                    "デキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・マス",
                    "基本形",
                    "ます",
                    "マス",
                    "マス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "テスト");
            assert_eq!(token.byte_start, 60);
            assert_eq!(token.byte_end, 69);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "テスト",
                    "テスト",
                    "テスト"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 69);
            assert_eq!(token.byte_end, 72);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_decompose_mode_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Decompose(Penalty::default()),
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田空港限定トートバッグ").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "人名",
                    "姓",
                    "*",
                    "*",
                    "羽田",
                    "ハタ",
                    "ハタ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "空港");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "空港",
                    "クウコウ",
                    "クーコー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "サ変接続",
                    "*",
                    "*",
                    "*",
                    "*",
                    "限定",
                    "ゲンテイ",
                    "ゲンテイ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_simple_userdic_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_tokenize_with_simple_userdic_bin_ipadic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_long_text_ipadic() {
        let mut large_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut large_text = String::new();
        let _size = large_file.read_to_string(&mut large_text).unwrap();

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();

        let tokens = tokenizer.tokenize(large_text.as_str()).unwrap();
        assert!(!tokens.is_empty());
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_config_unidic_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "unidic"
            },
            "mode": "normal"
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::UniDic));
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_config_unidic_decompose() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "unidic"
            },
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::UniDic));
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("日本語の形態素解析を行うことができます。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "日本");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "地名",
                    "国",
                    "*",
                    "*",
                    "ニッポン",
                    "日本",
                    "日本",
                    "ニッポン",
                    "日本",
                    "ニッポン",
                    "固",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "語");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ゴ",
                    "語",
                    "語",
                    "ゴ",
                    "語",
                    "ゴ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "形態");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ケイタイ",
                    "形態",
                    "形態",
                    "ケータイ",
                    "形態",
                    "ケータイ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "素");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "接尾辞",
                    "名詞的",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "ソ",
                    "素",
                    "素",
                    "ソ",
                    "素",
                    "ソ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "サ変可能",
                    "*",
                    "*",
                    "*",
                    "カイセキ",
                    "解析",
                    "解析",
                    "カイセキ",
                    "解析",
                    "カイセキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ヲ",
                    "を",
                    "を",
                    "オ",
                    "を",
                    "オ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "一般",
                    "*",
                    "*",
                    "五段-ワア行",
                    "連体形-一般",
                    "オコナウ",
                    "行う",
                    "行う",
                    "オコナウ",
                    "行う",
                    "オコナウ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "コト",
                    "事",
                    "こと",
                    "コト",
                    "こと",
                    "コト",
                    "和",
                    "コ濁",
                    "基本形",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ガ",
                    "が",
                    "が",
                    "ガ",
                    "が",
                    "ガ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "動詞",
                    "非自立可能",
                    "*",
                    "*",
                    "上一段-カ行",
                    "連用形-一般",
                    "デキル",
                    "出来る",
                    "でき",
                    "デキ",
                    "できる",
                    "デキル",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-マス",
                    "終止形-一般",
                    "マス",
                    "ます",
                    "ます",
                    "マス",
                    "ます",
                    "マス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_with_simple_userdic_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "モヨリ",
                    "最寄り",
                    "最寄り",
                    "モヨリ",
                    "最寄り",
                    "モヨリ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "エキ",
                    "駅",
                    "駅",
                    "エキ",
                    "駅",
                    "エキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "係助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ハ",
                    "は",
                    "は",
                    "ワ",
                    "は",
                    "ワ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-デス",
                    "終止形-一般",
                    "デス",
                    "です",
                    "です",
                    "デス",
                    "です",
                    "デス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "unidic")]
    fn test_tokenize_with_simple_userdic_bin_unidic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ノ",
                    "の",
                    "の",
                    "ノ",
                    "の",
                    "ノ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "モヨリ",
                    "最寄り",
                    "最寄り",
                    "モヨリ",
                    "最寄り",
                    "モヨリ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "駅");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "普通名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "エキ",
                    "駅",
                    "駅",
                    "エキ",
                    "駅",
                    "エキ",
                    "漢",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助詞",
                    "係助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ハ",
                    "は",
                    "は",
                    "ワ",
                    "は",
                    "ワ",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "助動詞-デス",
                    "終止形-一般",
                    "デス",
                    "です",
                    "です",
                    "デス",
                    "です",
                    "デス",
                    "和",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "補助記号",
                    "句点",
                    "*",
                    "*",
                    "*",
                    "*",
                    "",
                    "。",
                    "。",
                    "",
                    "。",
                    "",
                    "記号",
                    "*",
                    "*",
                    "*",
                    "*"
                ]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_config_ko_dic_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ko-dic"
            },
            "mode": "normal"
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::KoDic));
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_config_ko_dic_decompose() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "ko-dic"
            },
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::KoDic));
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("한국어의형태해석을실시할수있습니다.")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한국어");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "F",
                    "한국어",
                    "Compound",
                    "*",
                    "*",
                    "한국/NNG/*+어/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "의");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["JKG", "*", "F", "의", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "형태");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "F", "형태", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "해석");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "행위", "T", "해석", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "을");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["JKO", "*", "T", "을", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "실시");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "행위", "F", "실시", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "할");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "XSV+ETM",
                    "*",
                    "T",
                    "할",
                    "Inflect",
                    "XSV",
                    "ETM",
                    "하/XSV/*+ᆯ/ETM/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "수");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNB", "*", "F", "수", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "있");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["VV", "*", "T", "있", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "습니다");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["EF", "*", "F", "습니다", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 52);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_with_simple_userdic_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "T",
                    "토트백",
                    "Compound",
                    "*",
                    "*",
                    "토트/NNP/인명+백/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ko-dic")]
    fn test_tokenize_with_simple_userdic_bin_ko_dic() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("하네다공항한정토트백.").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "하네다공항");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 15);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNP", "*", "*", "하네다공항", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "한정");
            assert_eq!(token.byte_start, 15);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["NNG", "*", "T", "한정", "*", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "토트백");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "NNG",
                    "*",
                    "T",
                    "토트백",
                    "Compound",
                    "*",
                    "*",
                    "토트/NNP/인명+백/NNG/*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, ".");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 31);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["SF", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_config_cc_cedict_normal() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "cc-cedict"
            },
            "mode": "normal"
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::CcCedict));
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_config_cc_cedict_decompose() {
        let config_str = r#"
        {
            "dictionary": {
                "kind": "cc-cedict"
            },
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        }
        "#;

        let config: TokenizerConfig = serde_json::from_str(config_str).unwrap();
        assert_eq!(config.dictionary.kind, Some(DictionaryKind::CcCedict));
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_cc_cedict() {
        let dictionary = lindera_dictionary::DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("可以进行中文形态学分析。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "可以");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "ke3 yi3",
                    "可以",
                    "可以",
                    "can/may/possible/able to/not bad/pretty good/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "进行");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "jin4 xing2",
                    "進行",
                    "进行",
                    "to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "中文");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "Zhong1 wen2",
                    "中文",
                    "中文",
                    "Chinese language/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "形态学");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xing2 tai4 xue2",
                    "形態學",
                    "形态学",
                    "morphology (in biology or linguistics)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "分析");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 33);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "fen1 xi1",
                    "分析",
                    "分析",
                    "to analyze/analysis/CL:個|个[ge4]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 33);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_with_simple_userdic_cc_cedict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xian4 ding4",
                    "限定",
                    "限定",
                    "to restrict to/to limit/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "tuo1 te4",
                    "托特",
                    "托特",
                    "(loanword) tote (bag)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "bao1",
                    "包",
                    "包",
                    "to cover/to wrap/to hold/to include/to take charge of/to contract (to or for)/package/wrapper/container/bag/to hold or embrace/bundle/packet/CL:個|个[ge4]",
                    "隻|只[zhi1]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "cc-cedict")]
    fn test_tokenize_with_simple_userdic_bin_cc_cedict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.bin");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer.tokenize("羽田机场限定托特包。").unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "羽田机场");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["*", "*", "*", "*", "Yu3 tian2 ji1 chang3", "*", "*", "*"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "xian4 ding4",
                    "限定",
                    "限定",
                    "to restrict to/to limit/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "托特");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "tuo1 te4",
                    "托特",
                    "托特",
                    "(loanword) tote (bag)/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "包");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "*",
                    "*",
                    "*",
                    "*",
                    "bao1",
                    "包",
                    "包",
                    "to cover/to wrap/to hold/to include/to take charge of/to contract (to or for)/package/wrapper/container/bag/to hold or embrace/bundle/packet/CL:個|个[ge4]",
                    "隻|只[zhi1]/"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(token.get_details().unwrap(), vec!["UNK"]);
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    fn test_mixed_user_dict() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_mixed_userdic.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(config).unwrap();
        let mut tokens = tokenizer
            .tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。")
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "カスタム名詞",
                    "*",
                    "*",
                    "東京スカイツリー",
                    "トウキョウスカイツリー",
                    "トウキョウスカイツリー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "*",
                    "最寄り駅",
                    "モヨリエキ",
                    "モヨリエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec![
                    "助動詞",
                    "*",
                    "*",
                    "*",
                    "特殊・デス",
                    "基本形",
                    "です",
                    "デス",
                    "デス"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.text, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.get_details().unwrap(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        Tokenizer::from_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: userdic_file,
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        Tokenizer::from_config(config).unwrap();
    }

    #[test]
    #[cfg(feature = "filter")]
    fn test_analyzer_config_from_slice() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let result = AnalyzerConfig::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(feature = "filter")]
    fn test_analyzer_config_clone() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let analyzer_config = AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let cloned_analyzer_config = analyzer_config.clone();

        assert_eq!(analyzer_config, cloned_analyzer_config);
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
    fn test_analyzer_from_config() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let analyzer_config =
            lindera_analyzer::analyzer::AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let result = lindera_analyzer::analyzer::Analyzer::from_config(&analyzer_config);

        assert_eq!(true, result.is_ok());
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
    fn test_analyzer_from_wrong_config() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal",
                "with_details": false
            },
            "token_filters": [
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "unexisting_filter",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let analyzer_config =
            lindera_analyzer::analyzer::AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let result = lindera_analyzer::analyzer::Analyzer::from_config(&analyzer_config);

        assert_eq!(true, result.is_err());
    }

    #[test]
    #[cfg(all(feature = "ipadic", feature = "filter",))]
    fn test_analyze_ipadic() {
        let config_str = r#"
        {
            "character_filters": [
                {
                    "kind": "unicode_normalize",
                    "args": {
                        "kind": "nfkc"
                    }
                },
                {
                    "kind": "japanese_iteration_mark",
                    "args": {
                        "normalize_kanji": true,
                        "normalize_kana": true
                    }
                },
                {
                    "kind": "mapping",
                    "args": {
                        "mapping": {
                            "リンデラ": "Lindera"
                        }
                    }
                }
            ],
            "tokenizer": {
                "dictionary": {
                    "kind": "ipadic"
                },
                "mode": "normal"
            },
            "token_filters": [
                {
                    "kind": "japanese_compound_word",
                    "args": {
                        "kind": "ipadic",
                        "tags": [
                            "名詞,数",
                            "名詞,接尾,助数詞"
                        ]
                    }
                },
                {
                    "kind": "japanese_stop_tags",
                    "args": {
                        "tags": [
                            "接続詞",
                            "助詞",
                            "助詞,格助詞",
                            "助詞,格助詞,一般",
                            "助詞,格助詞,引用",
                            "助詞,格助詞,連語",
                            "助詞,係助詞",
                            "助詞,副助詞",
                            "助詞,間投助詞",
                            "助詞,並立助詞",
                            "助詞,終助詞",
                            "助詞,副助詞／並立助詞／終助詞",
                            "助詞,連体化",
                            "助詞,副詞化",
                            "助詞,特殊",
                            "助動詞",
                            "記号",
                            "記号,一般",
                            "記号,読点",
                            "記号,句点",
                            "記号,空白",
                            "記号,括弧閉",
                            "その他,間投",
                            "フィラー",
                            "非言語音"
                        ]
                    }
                },
                {
                    "kind": "japanese_katakana_stem",
                    "args": {
                        "min": 3
                    }
                }
            ]
        }
        "#;
        let analyzer_config = AnalyzerConfig::from_slice(config_str.as_bytes()).unwrap();

        let analyzer = Analyzer::from_config(&analyzer_config).unwrap();

        {
            let text = "ﾘﾝﾃﾞﾗは形態素解析ｴﾝｼﾞﾝです。".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "Lindera".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 15);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, vec!["UNK".to_string()]);
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "形態素".to_string());
                assert_eq!(token.byte_start, 18);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "形態素".to_string(),
                        "ケイタイソ".to_string(),
                        "ケイタイソ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "解析".to_string());
                assert_eq!(token.byte_start, 27);
                assert_eq!(token.byte_end, 33);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "サ変接続".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "解析".to_string(),
                        "カイセキ".to_string(),
                        "カイセキ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "エンジン".to_string());
                assert_eq!(token.byte_start, 33);
                assert_eq!(token.byte_end, 48);
                assert_eq!(token.position, 4);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "エンジン".to_string(),
                        "エンジン".to_string(),
                        "エンジン".to_string()
                    ]
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "Lindera".to_string());
                assert_eq!(&text[start..end], "ﾘﾝﾃﾞﾗ");
            }
        }

        {
            let text = "１０㌎のｶﾞｿﾘﾝ".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "10".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(token.details, vec!["UNK".to_string()]);
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ガロン".to_string());
                assert_eq!(token.byte_start, 6);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 1);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "接尾".to_string(),
                        "助数詞".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ガロン".to_string(),
                        "ガロン".to_string(),
                        "ガロン".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ガソリン".to_string());
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 3);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ガソリン".to_string(),
                        "ガソリン".to_string(),
                        "ガソリン".to_string()
                    ]
                );
            }

            let mut tokens_iter = tokens.iter();
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "10".to_string());
                assert_eq!(&text[start..end], "１０");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガロン".to_string());
                assert_eq!(&text[start..end], "㌎");
            }
            {
                let token = tokens_iter.next().unwrap();
                let start = token.byte_start;
                let end = token.byte_end;
                assert_eq!(token.text, "ガソリン".to_string());
                assert_eq!(&text[start..end], "ｶﾞｿﾘﾝ");
            }
        }

        {
            let text = "お釣りは百三十四円です。".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "お釣り".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 9);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "お釣り".to_string(),
                        "オツリ".to_string(),
                        "オツリ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "百三十四円".to_string());
                assert_eq!(token.byte_start, 12);
                assert_eq!(token.byte_end, 27);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 5);
                assert_eq!(
                    token.details,
                    vec![
                        "複合語".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string()
                    ]
                );
            }
        }

        {
            let text = "ここは騒々しい".to_string();
            let mut analyze_text = text.clone();
            let mut tokens = analyzer.analyze(&mut analyze_text).unwrap();
            let mut tokens_iter = tokens.iter_mut();
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "ここ".to_string());
                assert_eq!(token.byte_start, 0);
                assert_eq!(token.byte_end, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "名詞".to_string(),
                        "代名詞".to_string(),
                        "一般".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "ここ".to_string(),
                        "ココ".to_string(),
                        "ココ".to_string()
                    ]
                );
            }
            {
                let token = tokens_iter.next().unwrap();
                assert_eq!(token.text, "騒騒しい".to_string());
                assert_eq!(token.byte_start, 9);
                assert_eq!(token.byte_end, 21);
                assert_eq!(token.position, 2);
                assert_eq!(token.position_length, 1);
                assert_eq!(
                    token.details,
                    vec![
                        "形容詞".to_string(),
                        "自立".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        "形容詞・イ段".to_string(),
                        "基本形".to_string(),
                        "騒騒しい".to_string(),
                        "ソウゾウシイ".to_string(),
                        "ソーゾーシイ".to_string()
                    ]
                );
            }
        }
    }
}
