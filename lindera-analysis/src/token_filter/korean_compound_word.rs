use std::borrow::Cow;
use std::collections::HashSet;
use std::iter;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::token_filter::compound::{
    merge_consecutive_tokens, parse_new_tag, write_compound_details,
};
use crate::token_filter::tags::parse_tags;
use lindera::LinderaResult;
use lindera::token::Token;

pub const KOREAN_COMPOUND_WORD_TOKEN_FILTER_NAME: &str = "korean_compound_word";

pub type KoreanCompoundWordTokenFilterConfig = Value;

/// Compound consecutive tokens that have specified part-of-speech tags into a single token.
///
/// The Korean counterpart of `japanese_compound_word`: a token takes part in a merge when its
/// first detail, the single ko-dic part-of-speech tag, is one of `tags`, the same key the
/// Korean keep/stop tag filters use. ko-dic tokenizes a Sino-Korean numeral into one token per
/// morpheme (`이천이십육년` is `이/NR 천/NR 이/NR 십/NR 육/NR 년/NNBC`), so merging the `SN` and
/// `NR` runs first is what lets `korean_number` turn them into `2026 년`.
///
#[derive(Clone, Debug)]
pub struct KoreanCompoundWordTokenFilter {
    /// The part-of-speech tags whose tokens are merged with their neighbours.
    tags: HashSet<String>,
    /// The part-of-speech tag given to a merged token; `None` selects [`Self::DEFAULT_NEW_TAG`].
    new_tag: Option<String>,
}

impl KoreanCompoundWordTokenFilter {
    /// The part-of-speech tag a merged token gets when `new_tag` is not configured.
    ///
    /// Korean for "compound word", the counterpart of the `複合語` that `japanese_compound_word`
    /// assigns. It is not a ko-dic tag, so a following `korean_number` or `korean_keep_tags` only
    /// picks the merged token up when `new_tag` names a tag it is configured for.
    pub const DEFAULT_NEW_TAG: &'static str = "복합어";

    /// Creates the filter.
    ///
    /// # 引数
    ///
    /// * `tags` - Part-of-speech tags whose consecutive tokens are merged.
    /// * `new_tag` - The tag assigned to a merged token, or `None` for [`Self::DEFAULT_NEW_TAG`].
    ///
    /// # 戻り値
    ///
    /// The configured filter.
    pub fn new(tags: HashSet<String>, new_tag: Option<String>) -> Self {
        Self { tags, new_tag }
    }

    /// Builds the filter from its JSON configuration: `tags` (required, array of strings) and
    /// `new_tag` (optional, string; used verbatim as the merged token's first detail).
    ///
    /// # 引数
    ///
    /// * `config` - The filter's JSON configuration.
    ///
    /// # 戻り値
    ///
    /// The configured filter, or a deserialization error when `tags` is missing or either
    /// argument has the wrong type.
    pub fn from_config(config: &KoreanCompoundWordTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new(parse_tags(config)?, parse_new_tag(config)?))
    }
}

impl TokenFilter for KoreanCompoundWordTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_COMPOUND_WORD_TOKEN_FILTER_NAME
    }

    /// Merges each run of consecutive tokens whose first part-of-speech detail is in the
    /// configured tags into one token.
    ///
    /// The merged token has the concatenated surface, the first token's `byte_start` and
    /// `position`, the last token's `byte_end`, the summed `position_length`, and details of
    /// `new_tag` (or [`Self::DEFAULT_NEW_TAG`]) followed by `*` for every other field. A matching
    /// token with no matching neighbour is left as it is.
    ///
    /// # 引数
    ///
    /// * `tokens` - The tokens to merge, modified in place.
    ///
    /// # 戻り値
    ///
    /// `Ok(())`; this filter cannot fail.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        merge_consecutive_tokens(
            tokens,
            &self.tags,
            |token, key| {
                // ko-dic has a single part-of-speech tag, which is the first detail.
                key.push_str(token.get_detail(0).unwrap_or_default());
            },
            |token| {
                let tag = match &self.new_tag {
                    Some(new_tag) => Cow::Owned(new_tag.clone()),
                    None => Cow::Borrowed(Self::DEFAULT_NEW_TAG),
                };
                write_compound_details(token, iter::once(tag));
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_compound_word::{
        KoreanCompoundWordTokenFilter, KoreanCompoundWordTokenFilterConfig,
    };

    #[test]
    fn test_korean_compound_word_token_filter_config() {
        let config_str = r#"
        {
            "tags": [
                "SN",
                "NR"
            ],
            "new_tag": "NR"
        }
        "#;
        let result: Result<KoreanCompoundWordTokenFilterConfig, _> =
            serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_korean_compound_word_token_filter_from_config() {
        // `tags` with and without `new_tag`.
        let config: KoreanCompoundWordTokenFilterConfig =
            serde_json::json!({ "tags": ["SN", "NR"], "new_tag": "NR" });
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_ok());
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({ "tags": ["NR"] });
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_ok());

        // `tags` is required and must be an array of strings.
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({});
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_err());
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({ "tags": "NR" });
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_err());
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({ "tags": [1] });
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_err());

        // `new_tag`, when present, must be a string.
        let config: KoreanCompoundWordTokenFilterConfig =
            serde_json::json!({ "tags": ["NR"], "new_tag": 1 });
        assert!(KoreanCompoundWordTokenFilter::from_config(&config).is_err());
    }

    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_compound_word_token_filter_apply() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{Dictionary, DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        /// `이천이십육년` as ko-dic tokenizes it: one token per numeral morpheme, then `년`.
        /// Every Hangul syllable is three bytes.
        fn numeral_tokens(dictionary: &Dictionary) -> Vec<Token<'_>> {
            [
                ("이", "NR", "F"),
                ("천", "NR", "T"),
                ("이", "NR", "F"),
                ("십", "NR", "T"),
                ("육", "NR", "T"),
                ("년", "NNBC", "T"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (surface, tag, final_consonant))| Token {
                surface: Cow::Borrowed(surface),
                byte_start: i * 3,
                byte_end: i * 3 + 3,
                position: i,
                position_length: 1,
                word_id: WordId::new(LexType::System, i as u32),
                dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed(tag),
                    Cow::Borrowed("*"),
                    Cow::Borrowed(final_consonant),
                    Cow::Borrowed(surface),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                ]),
            })
            .collect()
        }

        let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

        // With `new_tag`, the merged token carries that tag.
        let config: KoreanCompoundWordTokenFilterConfig =
            serde_json::json!({ "tags": ["SN", "NR"], "new_tag": "NR" });
        let filter = KoreanCompoundWordTokenFilter::from_config(&config).unwrap();
        let mut tokens = numeral_tokens(&dictionary);
        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "이천이십육");
        assert_eq!(tokens[0].byte_start, 0);
        assert_eq!(tokens[0].byte_end, 15);
        assert_eq!(tokens[0].position, 0);
        assert_eq!(tokens[0].position_length, 5);
        assert_eq!(
            tokens[0].details(),
            vec!["NR", "*", "*", "*", "*", "*", "*", "*"]
        );
        assert_eq!(tokens[1].surface, "년");
        assert_eq!(tokens[1].byte_start, 15);
        assert_eq!(tokens[1].byte_end, 18);
        assert_eq!(tokens[1].position, 5);
        assert_eq!(tokens[1].position_length, 1);
        assert_eq!(
            tokens[1].details(),
            vec!["NNBC", "*", "T", "년", "*", "*", "*", "*"]
        );

        // Without `new_tag`, the merged token is tagged with the default marker.
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({ "tags": ["NR"] });
        let filter = KoreanCompoundWordTokenFilter::from_config(&config).unwrap();
        let mut tokens = numeral_tokens(&dictionary);
        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "이천이십육");
        assert_eq!(
            tokens[0].get_detail(0),
            Some(KoreanCompoundWordTokenFilter::DEFAULT_NEW_TAG)
        );
        assert_eq!(tokens[0].get_detail(0), Some("복합어"));

        // A matching token with no matching neighbour keeps its details.
        let config: KoreanCompoundWordTokenFilterConfig = serde_json::json!({ "tags": ["NNBC"] });
        let filter = KoreanCompoundWordTokenFilter::from_config(&config).unwrap();
        let mut tokens = numeral_tokens(&dictionary);
        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[5].surface, "년");
        assert_eq!(tokens[5].position_length, 1);
        assert_eq!(
            tokens[5].details(),
            vec!["NNBC", "*", "T", "년", "*", "*", "*", "*"]
        );
    }

    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_compound_word_token_filter_with_tokenizer() {
        use crate::token_filter::BoxTokenFilter;
        use crate::token_filter::korean_number::KoreanNumberTokenFilter;
        use crate::tokenizer::Tokenizer;
        use lindera::dictionary::load_dictionary;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::token::Token;

        /// A tokenizer over the embedded ko-dic with the given filter chain.
        fn build(filters: Vec<BoxTokenFilter>) -> Tokenizer {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokenizer = Tokenizer::new(segmenter);
            for filter in filters {
                tokenizer.append_token_filter(filter);
            }
            tokenizer
        }
        fn compound(config: serde_json::Value) -> BoxTokenFilter {
            BoxTokenFilter::from(KoreanCompoundWordTokenFilter::from_config(&config).unwrap())
        }
        fn number(config: serde_json::Value) -> BoxTokenFilter {
            BoxTokenFilter::from(KoreanNumberTokenFilter::from_config(&config).unwrap())
        }
        fn surfaces(tokens: &[Token<'_>]) -> Vec<String> {
            tokens.iter().map(|t| t.surface.to_string()).collect()
        }
        let tokenize = |text: &str, filters: Vec<BoxTokenFilter>| -> Vec<String> {
            let tokenizer = build(filters);
            let tokens = tokenizer.tokenize(text).unwrap();
            surfaces(&tokens)
        };

        // The documented chain: merge the numeral runs, then convert them.
        let chain = || {
            vec![
                compound(serde_json::json!({ "tags": ["SN", "NR"], "new_tag": "NR" })),
                number(serde_json::json!({ "tags": ["SN", "NR"] })),
            ]
        };
        assert_eq!(tokenize("이천이십육년", chain()), ["2026", "년"]);
        assert_eq!(tokenize("10만", chain()), ["100000"]);
        assert_eq!(tokenize("2천26", chain()), ["2026"]);
        assert_eq!(tokenize("삼천오백 원", chain()), ["3500", "원"]);
        assert_eq!(tokenize("일억이천만", chain()), ["120000000"]);

        // Ordinary text has no `SN` / `NR` runs and comes through untouched.
        assert_eq!(
            tokenize("이것은 사과입니다", chain()),
            ["이것", "은", "사과", "입니다"]
        );

        // The filter on its own: the merged token spans the whole numeral and carries
        // `new_tag`, and the token after it keeps its own position.
        let tokenizer = build(vec![compound(
            serde_json::json!({ "tags": ["SN", "NR"], "new_tag": "NR" }),
        )]);
        let mut tokens = tokenizer.tokenize("이천이십육년").unwrap();
        assert_eq!(surfaces(&tokens), ["이천이십육", "년"]);
        assert_eq!(tokens[0].byte_start, 0);
        assert_eq!(tokens[0].byte_end, 15);
        assert_eq!(tokens[0].position, 0);
        assert_eq!(tokens[0].position_length, 5);
        assert_eq!(
            tokens[0].details(),
            vec!["NR", "*", "*", "*", "*", "*", "*", "*"]
        );
        assert_eq!(tokens[1].position, 5);
        assert_eq!(tokens[1].get_detail(0), Some("NNBC"));

        // Without `new_tag`, the merged token gets the default marker, which is not a numeral
        // tag, so a following `korean_number` under its default tags leaves it alone.
        let tokenizer = build(vec![compound(serde_json::json!({ "tags": ["NR"] }))]);
        let mut tokens = tokenizer.tokenize("이천이십육년").unwrap();
        assert_eq!(surfaces(&tokens), ["이천이십육", "년"]);
        assert_eq!(tokens[0].get_detail(0), Some("복합어"));
        assert_eq!(
            tokenize(
                "이천이십육년",
                vec![
                    compound(serde_json::json!({ "tags": ["NR"] })),
                    number(serde_json::json!({})),
                ],
            ),
            ["이천이십육", "년"]
        );
    }
}
