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

/// Initial capacity of the comparison-key buffer [`apply_tag_filter`] reuses
/// across tokens.
///
/// Sized for the longest key the filters build in practice: four
/// comma-separated Japanese part-of-speech fields. The longest distinct
/// 4-field key across the whole IPADIC lexicon is 47 bytes
/// (`助詞,副助詞／並立助詞／終助詞,*,*`), so this leaves headroom without being
/// a meaningful allocation. A longer key is still handled correctly; it just
/// grows the buffer once.
const KEY_BUFFER_CAPACITY: usize = 64;

/// Whether a tag filter keeps or removes the tokens whose tag matches the set.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TagPolicy {
    /// Keep only tokens whose extracted tag is in the set.
    Keep,
    /// Remove tokens whose extracted tag is in the set.
    Remove,
}

/// Filters `tokens` in place, retaining or removing each token depending on
/// whether the tag produced by `write_tag` is present in `tags`, per
/// `policy`.
///
/// The tag extraction strategy is supplied by the caller because each filter
/// builds its comparison key differently (Japanese filters join up to four
/// POS parts; Korean filters use only the first part). `write_tag` writes
/// into a buffer this function owns and reuses for every token, rather than
/// returning a fresh `String`, so a whole `apply` call costs at most one key
/// allocation instead of one per token.
///
/// # 引数
///
/// * `tokens` - The tokens to filter, modified in place.
/// * `tags` - The configured tag set to compare each key against.
/// * `policy` - Whether a match keeps or removes the token.
/// * `write_tag` - Writes a token's comparison key into the supplied buffer.
///   The buffer is cleared before each call.
pub(crate) fn apply_tag_filter<F>(
    tokens: &mut Vec<Token<'_>>,
    tags: &HashSet<String>,
    policy: TagPolicy,
    mut write_tag: F,
) where
    F: FnMut(&mut Token<'_>, &mut String),
{
    // An empty tag set means every token's tag is trivially "not in the
    // set" -- resolve the outcome directly per policy without calling
    // write_tag (which typically reads dictionary details) for every
    // token.
    if tags.is_empty() {
        match policy {
            TagPolicy::Keep => tokens.clear(),
            TagPolicy::Remove => {}
        }
        return;
    }

    // One key buffer for the whole call, reused across tokens. `contains`
    // takes `&str` through `HashSet<String>`'s `Borrow<str>` impl, so no
    // owned `String` is needed for the lookup itself.
    //
    // Presized so the buffer does not grow while the first few keys are
    // written: a Japanese key is at most four comma-separated POS fields,
    // which stays well inside this for every field set the bundled
    // dictionaries define. A longer key still works -- `push_str` grows the
    // buffer as usual, and the growth is paid once per call, not per token.
    let mut key = String::with_capacity(KEY_BUFFER_CAPACITY);

    // `retain_mut` compacts in place, so the second vector the previous
    // drain-into-a-fresh-`Vec` approach allocated is gone. Order is
    // preserved, exactly as before.
    tokens.retain_mut(|token| {
        key.clear();
        write_tag(token, &mut key);
        let matched = tags.contains(key.as_str());
        match policy {
            TagPolicy::Keep => matched,
            TagPolicy::Remove => !matched,
        }
    });
}

/// Writes a token's leading part-of-speech fields into `out`, joined with
/// `,`, as the Japanese tag filters' comparison key.
///
/// Uses at most the first four fields, matching `normalize_japanese_tags`,
/// which pads every configured tag to exactly four parts. A token with fewer
/// details yields a shorter key (and, for zero details, the empty string),
/// which is existing behavior that predates this helper -- such a key simply
/// matches no configured tag.
///
/// `details_iter` is used rather than `details` so the per-token `Vec<&str>`
/// the latter collects is not allocated.
///
/// # 引数
///
/// * `token` - The token whose details form the key.
/// * `out` - The buffer to write into; assumed already cleared.
pub(crate) fn write_japanese_pos_key(token: &mut Token<'_>, out: &mut String) {
    for (i, detail) in token.details_iter().take(4).enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(detail);
    }
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
        apply_tag_filter(&mut tokens, &tags, TagPolicy::Keep, |_, _| {
            extract_called.set(true);
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
        apply_tag_filter(&mut tokens, &tags, TagPolicy::Remove, |_, _| {
            extract_called.set(true);
        });

        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].surface, "もも");
        assert!(
            !extract_called.get(),
            "extract_tag must be skipped for an empty tag set"
        );
    }

    /// The Japanese key builder must produce exactly what the retired
    /// `details[0..len.min(4)].join(",")` produced: no leading or trailing
    /// separator, at most four fields, and the empty string for zero details.
    /// The zero-details case is the #438 crash site.
    #[test]
    fn test_write_japanese_pos_key_matches_join_semantics() {
        use std::borrow::Cow;

        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let cases: [(&[&str], &str); 7] = [
            (&[], ""),
            (&["名詞"], "名詞"),
            (&["名詞", "一般"], "名詞,一般"),
            (&["名詞", "固有名詞", "地域"], "名詞,固有名詞,地域"),
            (
                &["名詞", "固有名詞", "地域", "一般"],
                "名詞,固有名詞,地域,一般",
            ),
            // Only the first four fields participate in the key.
            (
                &["名詞", "固有名詞", "地域", "一般", "余分", "無視"],
                "名詞,固有名詞,地域,一般",
            ),
            // Empty fields are preserved as empty parts, not skipped.
            (&["", "", "*", "*"], ",,*,*"),
        ];

        for (details, expected) in cases {
            let mut token = Token {
                surface: Cow::Borrowed("x"),
                byte_start: 0,
                byte_end: 1,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 0),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(details.iter().map(|d| Cow::Borrowed(*d)).collect()),
            };

            let mut key = String::new();
            write_japanese_pos_key(&mut token, &mut key);
            assert_eq!(key, expected, "details {details:?}");

            // Equivalence with the retired implementation, on the same input.
            let collected = token.details();
            let tags_len = collected.len().min(4);
            assert_eq!(key, collected[0..tags_len].join(","), "details {details:?}");
        }
    }

    /// The key buffer is reused across tokens, so a stale key from a previous
    /// token must never leak into the next one. A short key following a long
    /// one is the case that would break if the buffer were not cleared.
    #[test]
    fn test_key_buffer_is_not_leaked_between_tokens() {
        use std::borrow::Cow;

        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let make = |surface: &'static str, details: &[&'static str], id: u32| Token {
            surface: Cow::Borrowed(surface),
            byte_start: 0,
            byte_end: surface.len(),
            position: 0,
            position_length: 1,
            word_id: WordId::new(LexType::System, id),
            dictionary: &dictionary,
            user_dictionary: None,
            details: Some(details.iter().map(|d| Cow::Borrowed(*d)).collect()),
        };

        // A long key first, then a short one, then an empty one.
        let mut tokens = vec![
            make("long", &["名詞", "固有名詞", "地域", "一般"], 0),
            make("short", &["助詞"], 1),
            make("empty", &[], 2),
        ];

        // Remove only the short one, so the others must survive with their
        // own keys rather than a leftover of the previous token's key.
        let mut tags = HashSet::new();
        tags.insert("助詞".to_string());

        apply_tag_filter(
            &mut tokens,
            &tags,
            TagPolicy::Remove,
            write_japanese_pos_key,
        );

        let surfaces: Vec<&str> = tokens.iter().map(|t| t.surface.as_ref()).collect();
        assert_eq!(surfaces, vec!["long", "empty"]);
    }

    /// `retain_mut` compacts in place; order and contents must match the
    /// retired drain-into-a-new-vector approach for every removal shape.
    #[test]
    fn test_retain_preserves_order_for_every_removal_shape() {
        use std::borrow::Cow;

        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        // "drop" tokens carry the stop tag; "keep" tokens do not.
        let build = |shape: &[bool]| -> Vec<Token<'_>> {
            shape
                .iter()
                .enumerate()
                .map(|(i, drop)| Token {
                    surface: if *drop {
                        Cow::Borrowed("drop")
                    } else {
                        Cow::Borrowed("keep")
                    },
                    byte_start: i,
                    byte_end: i + 4,
                    position: i,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, i as u32),
                    dictionary: &dictionary,
                    user_dictionary: None,
                    details: Some(if *drop {
                        vec![Cow::Borrowed("助詞")]
                    } else {
                        vec![Cow::Borrowed("名詞")]
                    }),
                })
                .collect()
        };

        let mut tags = HashSet::new();
        tags.insert("助詞".to_string());

        // first / last / middle / all / none / empty input
        let shapes: [&[bool]; 6] = [
            &[true, false, false],
            &[false, false, true],
            &[false, true, false],
            &[true, true, true],
            &[false, false, false],
            &[],
        ];

        for shape in shapes {
            let mut tokens = build(shape);
            apply_tag_filter(
                &mut tokens,
                &tags,
                TagPolicy::Remove,
                write_japanese_pos_key,
            );

            let expected: Vec<usize> = shape
                .iter()
                .enumerate()
                .filter(|(_, drop)| !**drop)
                .map(|(i, _)| i)
                .collect();
            let actual: Vec<usize> = tokens.iter().map(|t| t.position).collect();
            assert_eq!(actual, expected, "shape {shape:?}");
            assert!(
                tokens.iter().all(|t| t.surface == "keep"),
                "shape {shape:?}"
            );
        }
    }
}
