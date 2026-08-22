use std::collections::HashSet;

use serde_json::Value;

use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

/// Parses the `"tags"` string array shared by the keep/stop tag token filters.
pub(crate) fn parse_tags(config: &Value) -> LinderaResult<HashSet<String>> {
    config["tags"]
        .as_array()
        .ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("tags is required"))
        })?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| {
                    LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!("tag must be string"))
                })
                .map(|s| s.to_string())
        })
        .collect()
}

/// Normalizes Japanese part-of-speech tags to exactly four comma-separated
/// parts, padding missing trailing parts with `*`.
pub(crate) fn normalize_japanese_tags(tags: HashSet<String>) -> HashSet<String> {
    tags.into_iter()
        .map(|v| {
            let mut tag_parts: Vec<&str> = v.split(',').collect();
            tag_parts.resize(4, "*");
            tag_parts.join(",")
        })
        .collect()
}

/// Whether a tag filter keeps or removes the tokens whose tag matches the set.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TagPolicy {
    /// Keep only tokens whose extracted tag is in the set.
    Keep,
    /// Remove tokens whose extracted tag is in the set.
    Remove,
}

/// Filters `tokens` in place, retaining or removing each token depending on
/// whether the tag produced by `extract_tag` is present in `tags`, per
/// `policy`.
///
/// The tag extraction strategy is supplied by the caller because each filter
/// builds its comparison key differently (Japanese filters join up to four
/// POS parts; Korean filters use only the first part).
pub(crate) fn apply_tag_filter<F>(
    tokens: &mut Vec<Token<'_>>,
    tags: &HashSet<String>,
    policy: TagPolicy,
    extract_tag: F,
) where
    F: Fn(&mut Token<'_>) -> String,
{
    // An empty tag set means every token's tag is trivially "not in the
    // set" -- resolve the outcome directly per policy without calling
    // extract_tag (which typically reads dictionary details) for every
    // token.
    if tags.is_empty() {
        match policy {
            TagPolicy::Keep => tokens.clear(),
            TagPolicy::Remove => {}
        }
        return;
    }

    let mut filtered_tokens = Vec::with_capacity(tokens.len());

    for mut token in tokens.drain(..) {
        let tag = extract_tag(&mut token);
        let matched = tags.contains(&tag);
        let keep = match policy {
            TagPolicy::Keep => matched,
            TagPolicy::Remove => !matched,
        };
        if keep {
            filtered_tokens.push(token);
        }
    }

    *tokens = filtered_tokens;
}

// Every test here builds a `Token` against an embedded IPADIC dictionary, so
// the whole module is gated rather than each test individually -- otherwise
// `use super::*` is unused whenever the feature is off.
#[cfg(all(test, feature = "embed-ipadic"))]
mod tests {
    use super::*;

    #[test]
    fn test_apply_tag_filter_empty_set_keep_policy_removes_all_without_extracting() {
        use std::borrow::Cow;
        use std::cell::Cell;

        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut tokens: Vec<Token> = vec![Token {
            surface: Cow::Borrowed("もも"),
            byte_start: 0,
            byte_end: 6,
            position: 0,
            position_length: 1,
            word_id: WordId::new(LexType::System, 4294967295),
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed("UNK")]),
        }];

        let tags = HashSet::new();
        let extract_called = Cell::new(false);
        apply_tag_filter(&mut tokens, &tags, TagPolicy::Keep, |_| {
            extract_called.set(true);
            String::new()
        });

        assert_eq!(tokens.len(), 0);
        assert!(
            !extract_called.get(),
            "extract_tag must be skipped for an empty tag set"
        );
    }

    #[test]
    fn test_apply_tag_filter_empty_set_remove_policy_keeps_all_without_extracting() {
        use std::borrow::Cow;
        use std::cell::Cell;

        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut tokens: Vec<Token> = vec![Token {
            surface: Cow::Borrowed("もも"),
            byte_start: 0,
            byte_end: 6,
            position: 0,
            position_length: 1,
            word_id: WordId::new(LexType::System, 4294967295),
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed("UNK")]),
        }];

        let tags = HashSet::new();
        let extract_called = Cell::new(false);
        apply_tag_filter(&mut tokens, &tags, TagPolicy::Remove, |_| {
            extract_called.set(true);
            String::new()
        });

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].surface, "もも");
        assert!(
            !extract_called.get(),
            "extract_tag must be skipped for an empty tag set"
        );
    }
}
