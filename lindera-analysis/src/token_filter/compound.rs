use std::borrow::Cow;
use std::collections::HashSet;

use serde_json::Value;

use crate::token_filter::tags::KEY_BUFFER_CAPACITY;
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

/// Parses the optional `"new_tag"` string shared by the compound-word token
/// filters.
///
/// # 引数
///
/// * `config` - The filter's JSON configuration.
///
/// # 戻り値
///
/// `Ok(None)` when `new_tag` is absent, `Ok(Some(tag))` when it is a string,
/// and a deserialization error when it is present with any other type.
pub(crate) fn parse_new_tag(config: &Value) -> LinderaResult<Option<String>> {
    config
        .get("new_tag")
        .map(|v| {
            v.as_str()
                .ok_or_else(|| {
                    LinderaErrorKind::Deserialize
                        .with_error(anyhow::anyhow!("new_tag must be a string"))
                })
                .map(|s| s.to_string())
        })
        .transpose()
}

/// Merges every run of two or more consecutive tokens whose comparison key
/// is in `tags` into a single token, in place.
///
/// The merged token keeps the first token's `byte_start` and `position`,
/// takes the last token's `byte_end`, sums the `position_length`s and
/// concatenates the surfaces in order. `set_merged_details` is then called
/// once on it so the caller can rewrite its details. A matching token with
/// no matching neighbour is left untouched, details included, and an empty
/// tag set leaves the whole list untouched without extracting a single key.
///
/// The key extraction strategy is supplied by the caller for the same reason
/// as in [`super::tags::apply_tag_filter`]: Japanese filters join up to four
/// part-of-speech parts, Korean filters use only the first. `write_tag`
/// writes into one buffer this function owns and reuses for every token, so
/// a whole `apply` call costs at most one key allocation.
///
/// The list is compacted with a read index and a write index and truncated
/// at the end, so no second vector is allocated and unmerged tokens are
/// moved, never cloned. Each merged surface is built with a single
/// allocation sized from the surfaces it joins.
///
/// # 引数
///
/// * `tokens` - The tokens to merge, modified in place; order is preserved.
/// * `tags` - The configured tag set a token's key must be in to take part
///   in a merge.
/// * `write_tag` - Writes a token's comparison key into the supplied buffer.
///   The buffer is cleared before each call.
/// * `set_merged_details` - Rewrites the details of a token that absorbed at
///   least one neighbour. Called after its surface and offsets are updated.
pub(crate) fn merge_consecutive_tokens<'a, W, D>(
    tokens: &mut Vec<Token<'a>>,
    tags: &HashSet<String>,
    mut write_tag: W,
    mut set_merged_details: D,
) where
    W: FnMut(&mut Token<'a>, &mut String),
    D: FnMut(&mut Token<'a>),
{
    // Nothing can match an empty set, so skip the key extraction (which
    // typically reads dictionary details) for every token.
    if tags.is_empty() {
        return;
    }

    // One key buffer for the whole call, reused across tokens; see
    // `apply_tag_filter` for the sizing rationale.
    let mut key = String::with_capacity(KEY_BUFFER_CAPACITY);
    let mut matches = |token: &mut Token<'a>| {
        key.clear();
        write_tag(token, &mut key);
        tags.contains(key.as_str())
    };

    let len = tokens.len();
    let mut write = 0;
    let mut read = 0;
    while read < len {
        // `end` is one past the run that starts at `read`: the token alone
        // when it does not match, otherwise it and every matching token
        // that follows. Each token's key is extracted exactly once.
        let mut end = read + 1;
        if matches(&mut tokens[read]) {
            while end < len && matches(&mut tokens[end]) {
                end += 1;
            }
        }

        if end - read > 1 {
            let (head, tail) = tokens.split_at_mut(read + 1);
            let merged = &mut head[read];
            let absorbed = &tail[..end - read - 1];

            let mut surface = String::with_capacity(
                merged.surface.len() + absorbed.iter().map(|t| t.surface.len()).sum::<usize>(),
            );
            surface.push_str(&merged.surface);
            for next in absorbed {
                surface.push_str(&next.surface);
                merged.byte_end = next.byte_end;
                merged.position_length += next.position_length;
            }
            merged.surface = Cow::Owned(surface);
            set_merged_details(merged);
        }

        // Move the run's surviving token down to the write cursor. Every
        // slot at or past `write` is either still to be read or already
        // absorbed, so the swap never displaces a live token.
        if write != read {
            tokens.swap(write, read);
        }
        write += 1;
        read = end;
    }

    tokens.truncate(write);
}

/// Replaces `token`'s details with `leading` followed by `*` up to the
/// dictionary schema's custom field count, the width every token's details
/// are padded to, so the merged token stays indexable like any other.
/// Fields of `leading` beyond that width are dropped.
///
/// # 引数
///
/// * `token` - The merged token whose details are replaced.
/// * `leading` - The leading detail fields, typically the new part-of-speech
///   tag or its comma-separated parts.
pub(crate) fn write_compound_details<'a, I>(token: &mut Token<'a>, leading: I)
where
    I: IntoIterator<Item = Cow<'a, str>>,
{
    let width = token
        .dictionary
        .metadata
        .dictionary_schema
        .get_custom_fields()
        .len();

    let mut details: Vec<Cow<'a, str>> = Vec::with_capacity(width);
    details.extend(leading.into_iter().take(width));
    details.resize(width, Cow::Borrowed("*"));
    token.details = Some(details);
}

#[cfg(test)]
mod config_tests {
    use super::parse_new_tag;

    #[test]
    fn test_parse_new_tag() {
        assert_eq!(parse_new_tag(&serde_json::json!({})).unwrap(), None);
        assert_eq!(
            parse_new_tag(&serde_json::json!({ "new_tag": "NR" })).unwrap(),
            Some("NR".to_string())
        );
        assert!(parse_new_tag(&serde_json::json!({ "new_tag": 1 })).is_err());
        assert!(parse_new_tag(&serde_json::json!({ "new_tag": null })).is_err());
        assert!(parse_new_tag(&serde_json::json!({ "new_tag": ["NR"] })).is_err());
    }
}

// Every test here builds a `Token` against an embedded IPADIC dictionary, so
// the whole module is gated rather than each test individually, as in
// `tags.rs`.
#[cfg(all(test, feature = "embed-ipadic"))]
mod tests {
    use std::borrow::Cow;
    use std::cell::Cell;
    use std::collections::HashSet;

    use lindera::dictionary::{Dictionary, DictionaryKind, WordId, load_embedded_dictionary};
    use lindera::token::Token;
    use lindera_dictionary::viterbi::LexType;

    use super::*;
    use crate::token_filter::tags::write_japanese_pos_key;

    /// The tag that takes part in merges throughout these tests.
    const MATCH: &str = "名詞";
    /// A tag that never takes part in a merge.
    const OTHER: &str = "助詞";
    /// What `set_merged_details` writes, so a test can tell a merged token
    /// from one whose details were left alone.
    const MERGED: &str = "MERGED";

    /// Builds the `i`-th token of a test list: surface `s<i>` (two bytes),
    /// contiguous byte offsets, position `i`, and a single-field detail that
    /// either matches or not.
    fn make(dictionary: &Dictionary, i: usize, matching: bool) -> Token<'_> {
        Token {
            surface: Cow::Owned(format!("s{i}")),
            byte_start: i * 2,
            byte_end: i * 2 + 2,
            position: i,
            position_length: 1,
            word_id: WordId::new(LexType::System, i as u32),
            dictionary,
            user_dictionary: None,
            details: Some(vec![Cow::Borrowed(if matching { MATCH } else { OTHER })]),
        }
    }

    fn match_set() -> HashSet<String> {
        HashSet::from([MATCH.to_string()])
    }

    fn mark_merged(token: &mut Token<'_>) {
        token.details = Some(vec![Cow::Borrowed(MERGED)]);
    }

    /// A merged token, as a reference implementation of the run semantics
    /// would produce it, for comparison with the helper's output.
    #[derive(Debug, PartialEq)]
    struct Expected {
        surface: String,
        byte_start: usize,
        byte_end: usize,
        position: usize,
        position_length: usize,
        first_detail: &'static str,
    }

    /// Groups `shape` into runs the way the helper is specified to: a run is
    /// a maximal sequence of matching tokens, and only a run of two or more
    /// is merged (and marked).
    fn expected_runs(shape: &[bool]) -> Vec<Expected> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < shape.len() {
            let mut end = i + 1;
            if shape[i] {
                while end < shape.len() && shape[end] {
                    end += 1;
                }
            }
            let count = end - i;
            out.push(Expected {
                surface: (i..end).map(|j| format!("s{j}")).collect(),
                byte_start: i * 2,
                byte_end: (end - 1) * 2 + 2,
                position: i,
                position_length: count,
                first_detail: if count > 1 {
                    MERGED
                } else if shape[i] {
                    MATCH
                } else {
                    OTHER
                },
            });
            i = end;
        }
        out
    }

    #[test]
    fn test_merge_consecutive_tokens_every_run_shape() {
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let tags = match_set();

        // empty / single / run at start, end, middle / whole list / lone
        // matches separated by others / two runs / no match at all
        let shapes: [&[bool]; 13] = [
            &[],
            &[false],
            &[true],
            &[true, true],
            &[true, true, false],
            &[false, true, true],
            &[false, true, true, false],
            &[true, true, true],
            &[true, false, true],
            &[true, false, false, true],
            &[true, true, false, true, true],
            &[true, true, false, true, false, true, true, true],
            &[false, false, false],
        ];

        for shape in shapes {
            let mut tokens: Vec<Token<'_>> = shape
                .iter()
                .enumerate()
                .map(|(i, matching)| make(&dictionary, i, *matching))
                .collect();

            merge_consecutive_tokens(
                &mut tokens,
                &tags,
                |token, key| write_japanese_pos_key(token, 0, key),
                mark_merged,
            );

            let actual: Vec<Expected> = tokens
                .iter_mut()
                .map(|t| Expected {
                    surface: t.surface.to_string(),
                    byte_start: t.byte_start,
                    byte_end: t.byte_end,
                    position: t.position,
                    position_length: t.position_length,
                    first_detail: match t.get_detail(0).unwrap() {
                        MERGED => MERGED,
                        MATCH => MATCH,
                        OTHER => OTHER,
                        other => panic!("unexpected detail {other}"),
                    },
                })
                .collect();
            assert_eq!(actual, expected_runs(shape), "shape {shape:?}");
        }
    }

    #[test]
    fn test_merge_consecutive_tokens_empty_tag_set_touches_nothing() {
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut tokens = vec![
            make(&dictionary, 0, true),
            make(&dictionary, 1, true),
            make(&dictionary, 2, false),
        ];
        let tags = HashSet::new();
        let extract_called = Cell::new(false);
        let details_called = Cell::new(false);

        merge_consecutive_tokens(
            &mut tokens,
            &tags,
            |_, _| extract_called.set(true),
            |_| details_called.set(true),
        );

        assert_eq!(tokens.len(), 3);
        assert!(
            !extract_called.get(),
            "write_tag must be skipped for an empty tag set"
        );
        assert!(!details_called.get());
        assert_eq!(tokens[0].surface, "s0");
        assert_eq!(tokens[0].position_length, 1);
    }

    /// The key buffer is reused across tokens, so a stale key from a previous
    /// token must never leak into the next one: a long key followed by the
    /// short keys of a run is the case that would break if the buffer were
    /// not cleared.
    #[test]
    fn test_merge_consecutive_tokens_key_buffer_is_not_leaked_between_tokens() {
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut tokens = vec![
            make(&dictionary, 0, false),
            make(&dictionary, 1, true),
            make(&dictionary, 2, true),
        ];
        tokens[0].details = Some(
            ["名詞", "固有名詞", "地域", "一般"]
                .into_iter()
                .map(Cow::Borrowed)
                .collect(),
        );
        let tags = match_set();

        merge_consecutive_tokens(
            &mut tokens,
            &tags,
            |token, key| write_japanese_pos_key(token, 0, key),
            mark_merged,
        );

        let surfaces: Vec<&str> = tokens.iter().map(|t| t.surface.as_ref()).collect();
        assert_eq!(surfaces, vec!["s0", "s1s2"]);
        assert_eq!(tokens[0].get_detail(0), Some("名詞"));
        assert_eq!(tokens[1].get_detail(0), Some(MERGED));
    }

    #[test]
    fn test_write_compound_details_pads_and_truncates_to_the_schema_width() {
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let width = dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();
        assert_eq!(width, 9, "IPADIC has nine custom fields");

        let mut token = make(&dictionary, 0, true);

        // One leading field is padded out with `*`.
        write_compound_details(&mut token, std::iter::once(Cow::Borrowed("複合語")));
        assert_eq!(
            token.details(),
            vec!["複合語", "*", "*", "*", "*", "*", "*", "*", "*"]
        );

        // Several leading fields keep their order.
        write_compound_details(
            &mut token,
            ["名詞", "数", "*", "*"].into_iter().map(Cow::Borrowed),
        );
        assert_eq!(
            token.details(),
            vec!["名詞", "数", "*", "*", "*", "*", "*", "*", "*"]
        );

        // More leading fields than the schema has are dropped, never
        // widening the token.
        let too_many: Vec<Cow<'_, str>> = (0..width + 3)
            .map(|i| Cow::Owned(format!("f{i}")))
            .collect();
        write_compound_details(&mut token, too_many);
        assert_eq!(token.details().len(), width);
        assert_eq!(token.get_detail(width - 1), Some("f8"));

        // No leading field at all still yields a full-width row of `*`.
        write_compound_details(&mut token, std::iter::empty());
        assert_eq!(token.details(), vec!["*"; width]);
    }
}
