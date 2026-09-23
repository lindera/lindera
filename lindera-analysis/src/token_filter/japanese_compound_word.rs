use std::borrow::Cow;
use std::collections::HashSet;
use std::iter;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::token_filter::compound::{
    merge_consecutive_tokens, parse_new_tag, write_compound_details,
};
use crate::token_filter::tags::{
    normalize_japanese_tag, normalize_japanese_tags, parse_tags, part_of_speech_offset_of,
    write_japanese_pos_key,
};
use lindera::LinderaResult;
use lindera::token::Token;

pub const JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME: &str = "japanese_compound_word";

pub type JapaneseCompoundWordTokenFilterConfig = Value;

/// The part-of-speech tag a merged token gets when `new_tag` is not configured.
const DEFAULT_NEW_TAG: &str = "複合語";

/// Compound consecutive tokens that have specified part-of-speech tags into a single token.
///
/// A token takes part in a merge when its four part-of-speech detail fields, joined with `,`,
/// equal one of `tags` (each padded to four parts with `*`), the same key the Japanese
/// keep/stop tag filters use. The fields start at the dictionary schema's `part_of_speech`
/// field (the first detail when the schema names no such field), so the filter also works with
/// SudachiDict, whose `display_surface` precedes the part-of-speech columns. The merge itself
/// is shared with `korean_compound_word`.
///
#[derive(Clone, Debug)]
pub struct JapaneseCompoundWordTokenFilter {
    /// The four-part part-of-speech tags whose tokens are merged with their neighbours.
    tags: HashSet<String>,
    /// The four-part tag given to a merged token; `None` selects `複合語`.
    new_tag: Option<String>,
}

impl JapaneseCompoundWordTokenFilter {
    /// Creates the filter, padding `tags` and `new_tag` to four comma-separated parts.
    ///
    /// # 引数
    ///
    /// * `tags` - Part-of-speech tags, one to four comma-separated levels each.
    /// * `new_tag` - The tag assigned to a merged token, or `None` for `複合語`.
    ///
    /// # 戻り値
    ///
    /// The configured filter.
    pub fn new(tags: HashSet<String>, new_tag: Option<String>) -> Self {
        Self {
            tags: normalize_japanese_tags(tags),
            new_tag: new_tag.map(|tag| normalize_japanese_tag(&tag)),
        }
    }

    /// Builds the filter from its JSON configuration: `tags` (required, array of strings) and
    /// `new_tag` (optional, string).
    ///
    /// # 引数
    ///
    /// * `config` - The filter's JSON configuration.
    ///
    /// # 戻り値
    ///
    /// The configured filter, or a deserialization error when `tags` is missing or either
    /// argument has the wrong type.
    pub fn from_config(config: &JapaneseCompoundWordTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new(parse_tags(config)?, parse_new_tag(config)?))
    }
}

impl TokenFilter for JapaneseCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    /// Merges each run of consecutive tokens whose part-of-speech tag is in the configured
    /// tags into one token.
    ///
    /// The merged token has the concatenated surface, the first token's `byte_start` and
    /// `position`, the last token's `byte_end`, the summed `position_length`, and details
    /// carrying `new_tag` (or `複合語`) at the schema's part-of-speech position, with every
    /// other field (any leading non-part-of-speech field such as SudachiDict's
    /// `display_surface`, and the rest up to the dictionary's field count) set to `*`. A
    /// matching token with no matching neighbour is left as it is.
    ///
    /// # 引数
    ///
    /// * `tokens` - The tokens to merge, modified in place.
    ///
    /// # 戻り値
    ///
    /// `Ok(())`; this filter cannot fail.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        // Where the part-of-speech hierarchy starts in the details, resolved
        // once from the dictionary schema: it is both where each token's key
        // is read from and where the merged token's tag is written to.
        let offset = part_of_speech_offset_of(tokens);
        merge_consecutive_tokens(
            tokens,
            &self.tags,
            |token, key| write_japanese_pos_key(token, offset, key),
            |token| {
                // Fields before the part-of-speech position (SudachiDict's
                // `display_surface`) get `*`, as unknown words carry there.
                let leading = iter::repeat_n(Cow::Borrowed("*"), offset);
                match &self.new_tag {
                    // The four-part tag becomes the part-of-speech fields, one per part.
                    Some(new_tag) => write_compound_details(
                        token,
                        leading.chain(new_tag.split(',').map(|part| Cow::Owned(part.to_owned()))),
                    ),
                    None => write_compound_details(
                        token,
                        leading.chain(iter::once(Cow::Borrowed(DEFAULT_NEW_TAG))),
                    ),
                }
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    #[test]
    fn test_japanese_compound_word_token_filter_config_ipadic() {
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilterConfig;

        let config_str = r#"
        {
            "tags": [
                "名詞,数",
                "名詞,接尾,助数詞"
            ],
            "new_tag": "複合語"
        }
        "#;
        let result: Result<JapaneseCompoundWordTokenFilterConfig, _> =
            serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[cfg(feature = "embed-ipadic")]
    #[test]
    fn test_japanese_compound_word_token_filter_ipadic() {
        use crate::token_filter::japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
        };

        let config_str = r#"
        {
            "tags": [
                "名詞,数",
                "名詞,接尾,助数詞"
            ],
            "new_tag": "複合語"
        }
        "#;
        let config: JapaneseCompoundWordTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let result = JapaneseCompoundWordTokenFilter::from_config(&config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_compound_word_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
        };
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
        {
            "tags": [
                "名詞,数",
                "名詞,接尾,助数詞"
            ],
            "new_tag": "複合語"
        }
        "#;
        let config: JapaneseCompoundWordTokenFilterConfig =
            serde_json::from_str(config_str).unwrap();
        let filter = JapaneseCompoundWordTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("１"),
                byte_start: 0,
                byte_end: 3,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 391174),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("１"),
                    Cow::Borrowed("イチ"),
                    Cow::Borrowed("イチ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("０"),
                byte_start: 3,
                byte_end: 6,
                position: 1,
                position_length: 1,
                word_id: WordId::new(LexType::System, 391171),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("０"),
                    Cow::Borrowed("ゼロ"),
                    Cow::Borrowed("ゼロ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("０"),
                byte_start: 6,
                byte_end: 9,
                position: 2,
                position_length: 1,
                word_id: WordId::new(LexType::System, 391171),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("数"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("０"),
                    Cow::Borrowed("ゼロ"),
                    Cow::Borrowed("ゼロ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("円"),
                byte_start: 9,
                byte_end: 12,
                position: 3,
                position_length: 1,
                word_id: WordId::new(LexType::System, 137904),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("助数詞"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("円"),
                    Cow::Borrowed("エン"),
                    Cow::Borrowed("エン"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("玉"),
                byte_start: 12,
                byte_end: 15,
                position: 4,
                position_length: 1,
                word_id: WordId::new(LexType::System, 287427),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("玉"),
                    Cow::Borrowed("ダマ"),
                    Cow::Borrowed("ダマ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("を"),
                byte_start: 15,
                byte_end: 18,
                position: 5,
                position_length: 1,
                word_id: WordId::new(LexType::System, 80582),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("玉"),
                    Cow::Borrowed("ダマ"),
                    Cow::Borrowed("ダマ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("拾う"),
                byte_start: 18,
                byte_end: 24,
                position: 6,
                position_length: 1,
                word_id: WordId::new(LexType::System, 228047),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("動詞"),
                    Cow::Borrowed("自立"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("五段・ワ行促音便"),
                    Cow::Borrowed("基本形"),
                    Cow::Borrowed("拾う"),
                    Cow::Borrowed("ヒロウ"),
                    Cow::Borrowed("ヒロウ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].surface, "１００円".to_string());
        assert_eq!(tokens[0].byte_start, 0);
        assert_eq!(tokens[0].byte_end, 12);
        assert_eq!(tokens[0].position, 0);
        assert_eq!(tokens[0].position_length, 4);
        assert_eq!(tokens[1].surface, "玉".to_string());
        assert_eq!(tokens[1].byte_start, 12);
        assert_eq!(tokens[1].byte_end, 15);
        assert_eq!(tokens[1].position, 4);
        assert_eq!(tokens[1].position_length, 1);
        assert_eq!(tokens[2].surface, "を".to_string());
        assert_eq!(tokens[2].byte_start, 15);
        assert_eq!(tokens[2].byte_end, 18);
        assert_eq!(tokens[2].position, 5);
        assert_eq!(tokens[2].position_length, 1);
        assert_eq!(tokens[3].surface, "拾う".to_string());
        assert_eq!(tokens[3].byte_start, 18);
        assert_eq!(tokens[3].byte_end, 24);
        assert_eq!(tokens[3].position, 6);
        assert_eq!(tokens[3].position_length, 1);

        assert_eq!(
            tokens[0].details(),
            vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*",]
        );
    }

    #[test]
    fn test_japanese_compound_word_token_filter_from_config_errors() {
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;

        // `tags` is required and must be an array of strings.
        assert!(JapaneseCompoundWordTokenFilter::from_config(&serde_json::json!({})).is_err());
        assert!(
            JapaneseCompoundWordTokenFilter::from_config(&serde_json::json!({ "tags": "名詞,数" }))
                .is_err()
        );
        assert!(
            JapaneseCompoundWordTokenFilter::from_config(&serde_json::json!({ "tags": [1] }))
                .is_err()
        );

        // `new_tag`, when present, must be a string.
        assert!(
            JapaneseCompoundWordTokenFilter::from_config(
                &serde_json::json!({ "tags": ["名詞,数"], "new_tag": 1 })
            )
            .is_err()
        );
    }

    /// The details a merged token gets for each `new_tag` variant, and that a
    /// matching token with no matching neighbour keeps its own.
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_compound_word_token_filter_new_tag_variants_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
        use lindera::dictionary::{Dictionary, DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        /// `一万円` as IPADIC tokenizes it: two numerals, then a counter suffix.
        fn tokens(dictionary: &Dictionary) -> Vec<Token<'_>> {
            [
                ("一", ["名詞", "数", "*", "*"]),
                ("万", ["名詞", "数", "*", "*"]),
                ("円", ["名詞", "接尾", "助数詞", "*"]),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (surface, pos))| Token {
                surface: Cow::Borrowed(surface),
                byte_start: i * 3,
                byte_end: i * 3 + 3,
                position: i,
                position_length: 1,
                word_id: WordId::new(LexType::System, i as u32),
                dictionary,
                user_dictionary: None,
                details: Some(pos.into_iter().chain(["*"; 5]).map(Cow::Borrowed).collect()),
            })
            .collect()
        }

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let counter_details = vec!["名詞", "接尾", "助数詞", "*", "*", "*", "*", "*", "*"];

        // Omitted: the merged token is tagged 複合語 and the counter is untouched.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": ["名詞,数"] }),
        )
        .unwrap();
        let mut tokens_omitted = tokens(&dictionary);
        filter.apply(&mut tokens_omitted).unwrap();
        assert_eq!(tokens_omitted.len(), 2);
        assert_eq!(tokens_omitted[0].surface, "一万");
        assert_eq!(tokens_omitted[0].byte_end, 6);
        assert_eq!(tokens_omitted[0].position_length, 2);
        assert_eq!(
            tokens_omitted[0].details(),
            vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*"]
        );
        assert_eq!(tokens_omitted[1].surface, "円");
        assert_eq!(tokens_omitted[1].position, 2);
        assert_eq!(tokens_omitted[1].details(), counter_details);

        // Given with two levels: padded to four parts, one detail field per part.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": ["名詞,数"], "new_tag": "名詞,数" }),
        )
        .unwrap();
        let mut tokens_two_levels = tokens(&dictionary);
        filter.apply(&mut tokens_two_levels).unwrap();
        assert_eq!(
            tokens_two_levels[0].details(),
            vec!["名詞", "数", "*", "*", "*", "*", "*", "*", "*"]
        );

        // Given with more than four levels: only the first four are kept, as before.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": ["名詞,数"], "new_tag": "名詞,数,*,*,余分" }),
        )
        .unwrap();
        let mut tokens_five_levels = tokens(&dictionary);
        filter.apply(&mut tokens_five_levels).unwrap();
        assert_eq!(
            tokens_five_levels[0].details(),
            vec!["名詞", "数", "*", "*", "*", "*", "*", "*", "*"]
        );

        // A matching token with no matching neighbour keeps its details.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": ["名詞,接尾,助数詞"] }),
        )
        .unwrap();
        let mut tokens_lone = tokens(&dictionary);
        filter.apply(&mut tokens_lone).unwrap();
        assert_eq!(tokens_lone.len(), 3);
        assert_eq!(tokens_lone[2].surface, "円");
        assert_eq!(tokens_lone[2].position_length, 1);
        assert_eq!(tokens_lone[2].details(), counter_details);
    }

    /// With the SudachiDict schema the part-of-speech hierarchy is details
    /// `1..5` (`display_surface` comes first): the tags are matched there,
    /// and the merged token's tag is written there, with `*` in the display
    /// surface slot as SudachiDict's own unknown words have (#997).
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_japanese_compound_word_token_filter_apply_sudachidict_schema() {
        use crate::token_filter::TokenFilter;
        use crate::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
        use crate::token_filter::tags::test_support::{sudachidict_schema_dictionary, tokens};

        // `令和五年に` as SudachiDict tokenizes it; a SudachiDict token has 15 details.
        let rows: [(&str, &[&str]); 4] = [
            (
                "令和",
                &[
                    "令和",
                    "名詞",
                    "固有名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "レイワ",
                    "令和",
                    "*",
                    "A",
                    "*",
                    "*",
                    "*",
                    "*",
                ],
            ),
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
            (
                "に",
                &[
                    "に",
                    "助詞",
                    "格助詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "ニ",
                    "に",
                    "*",
                    "A",
                    "*",
                    "*",
                    "*",
                    "*",
                ],
            ),
        ];
        let dictionary = sudachidict_schema_dictionary();
        let tags = serde_json::json!(["名詞,数詞", "名詞,普通名詞,助数詞可能"]);

        // Omitted `new_tag`: `複合語` goes into the part-of-speech slot, `*` before it.
        let filter =
            JapaneseCompoundWordTokenFilter::from_config(&serde_json::json!({ "tags": tags }))
                .unwrap();
        let mut merged = tokens(&dictionary, &rows);
        filter.apply(&mut merged).unwrap();
        let surfaces: Vec<&str> = merged.iter().map(|t| t.surface.as_ref()).collect();
        assert_eq!(surfaces, ["令和", "五年", "に"]);
        assert_eq!(merged[1].byte_start, "令和".len());
        assert_eq!(merged[1].byte_end, "令和五年".len());
        assert_eq!(merged[1].position, 1);
        assert_eq!(merged[1].position_length, 2);
        let mut expected = vec!["*", "複合語"];
        expected.resize(15, "*");
        assert_eq!(merged[1].details(), expected);
        // The neighbours keep their own details, display surface included.
        assert_eq!(merged[0].details(), rows[0].1);
        assert_eq!(merged[2].details(), rows[3].1);

        // A multi-level `new_tag` is laid out from the part-of-speech slot on.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": tags, "new_tag": "名詞,数詞" }),
        )
        .unwrap();
        let mut merged = tokens(&dictionary, &rows);
        filter.apply(&mut merged).unwrap();
        let mut expected = vec!["*", "名詞", "数詞", "*", "*"];
        expected.resize(15, "*");
        assert_eq!(merged[1].details(), expected);

        // A tag spelled the way the old positional key came out (display
        // surface first) must not match anymore, so nothing is merged.
        let filter = JapaneseCompoundWordTokenFilter::from_config(
            &serde_json::json!({ "tags": ["五,名詞,数詞", "年,名詞,普通名詞,助数詞可能"] }),
        )
        .unwrap();
        let mut untouched = tokens(&dictionary, &rows);
        filter.apply(&mut untouched).unwrap();
        assert_eq!(untouched.len(), rows.len());
    }
}
