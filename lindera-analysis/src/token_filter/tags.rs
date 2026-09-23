use std::collections::HashSet;

use serde_json::Value;

use lindera::LinderaResult;
use lindera::dictionary::Dictionary;
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

/// Normalizes one Japanese part-of-speech tag to exactly four comma-separated
/// parts, padding missing trailing parts with `*` and dropping any part
/// beyond the fourth.
///
/// # 引数
///
/// * `tag` - The tag as configured, with one or more comma-separated parts.
///
/// # 戻り値
///
/// The tag in the four-part form the Japanese filters compare against.
pub(crate) fn normalize_japanese_tag(tag: &str) -> String {
    let mut tag_parts: Vec<&str> = tag.split(',').collect();
    tag_parts.resize(4, "*");
    tag_parts.join(",")
}

/// Normalizes Japanese part-of-speech tags to exactly four comma-separated
/// parts each, with [`normalize_japanese_tag`].
///
/// # 引数
///
/// * `tags` - The tags as configured.
///
/// # 戻り値
///
/// The same tags, each in the four-part form.
pub(crate) fn normalize_japanese_tags(tags: HashSet<String>) -> HashSet<String> {
    tags.into_iter()
        .map(|tag| normalize_japanese_tag(&tag))
        .collect()
}

/// Initial capacity of the comparison-key buffer [`apply_tag_filter`] and
/// the compound-word merge helper reuse across tokens.
///
/// Sized for the longest key the filters build in practice: four
/// comma-separated Japanese part-of-speech fields. The longest distinct
/// 4-field key across the whole IPADIC lexicon is 47 bytes
/// (`助詞,副助詞／並立助詞／終助詞,*,*`), so this leaves headroom without being
/// a meaningful allocation. A longer key is still handled correctly; it just
/// grows the buffer once.
pub(crate) const KEY_BUFFER_CAPACITY: usize = 64;

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

/// Name of the dictionary schema field at which the Japanese part-of-speech
/// hierarchy starts (`part_of_speech`, followed by its subcategory fields).
///
/// Every bundled Japanese dictionary names its first custom field so, but a
/// schema may put other columns first: SudachiDict's `display_surface`
/// precedes the part-of-speech columns, which is why the position is
/// resolved from the schema rather than assumed to be the first detail
/// (#997).
pub(crate) const PART_OF_SPEECH_FIELD: &str = "part_of_speech";

/// Resolves the index within a token's details at which the part-of-speech
/// hierarchy starts, from the dictionary schema.
///
/// A schema without a `part_of_speech` field (ko-dic's `part_of_speech_tag`,
/// the legacy default schema's `major_pos`) yields `0`, so the hierarchy is
/// read from the first detail exactly as before this lookup existed.
///
/// The lookup is a hash-map probe, so the filters resolve it once per
/// `apply` call rather than per token (see [`part_of_speech_offset_of`]).
///
/// # 引数
///
/// * `dictionary` - The dictionary the tokens were segmented with.
///
/// # 戻り値
///
/// The details index of the `part_of_speech` field, or `0` when the schema
/// does not define one.
pub(crate) fn part_of_speech_offset(dictionary: &Dictionary) -> usize {
    dictionary
        .metadata
        .dictionary_schema
        .get_custom_field_index(PART_OF_SPEECH_FIELD)
        .unwrap_or(0)
}

/// Resolves [`part_of_speech_offset`] for a whole token list from its first
/// token, once per `apply` call.
///
/// Every token a filter receives comes from the same segmenter, hence the
/// same dictionary, so the first token's schema speaks for the list. An empty
/// list yields `0`; no filter reads a detail of an empty list anyway.
///
/// # 引数
///
/// * `tokens` - The tokens an `apply` call received.
///
/// # 戻り値
///
/// The details index the part-of-speech hierarchy starts at for these tokens.
pub(crate) fn part_of_speech_offset_of(tokens: &[Token<'_>]) -> usize {
    tokens
        .first()
        .map_or(0, |token| part_of_speech_offset(token.dictionary))
}

/// Writes a token's part-of-speech fields into `out`, joined with `,`, as
/// the Japanese tag filters' comparison key.
///
/// The fields are the four details starting at `offset`, the position of the
/// schema's `part_of_speech` field as resolved by [`part_of_speech_offset`].
/// Four matches `normalize_japanese_tags`, which pads every configured tag
/// to exactly four parts. A token with fewer details past the offset yields
/// a shorter key (and, for none, the empty string), which is existing
/// behavior that predates this helper -- such a key simply matches no
/// configured tag.
///
/// `details_iter` is used rather than `details` so the per-token `Vec<&str>`
/// the latter collects is not allocated.
///
/// # 引数
///
/// * `token` - The token whose details form the key.
/// * `offset` - The details index the part-of-speech hierarchy starts at.
/// * `out` - The buffer to write into; assumed already cleared.
pub(crate) fn write_japanese_pos_key(token: &mut Token<'_>, offset: usize, out: &mut String) {
    for (i, detail) in token.details_iter().skip(offset).take(4).enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(detail);
    }
}

/// Test support shared by the token filter tests: dictionaries whose schema
/// differs from IPADIC's, built without any dictionary but the embedded one.
#[cfg(all(test, feature = "embed-ipadic"))]
pub(crate) mod test_support {
    use std::borrow::Cow;
    use std::sync::Arc;

    use lindera::dictionary::{
        Dictionary, DictionaryKind, Schema, WordId, load_embedded_dictionary,
    };
    use lindera::token::Token;
    use lindera_dictionary::viterbi::LexType;

    /// The SudachiDict schema (`lindera-sudachidict/metadata.json`): the
    /// display surface precedes the part-of-speech columns, so the
    /// part-of-speech hierarchy is details `1..5` and a token has 15 details.
    pub(crate) const SUDACHIDICT_FIELDS: [&str; 19] = [
        "surface",
        "left_context_id",
        "right_context_id",
        "cost",
        "display_surface",
        "part_of_speech",
        "part_of_speech_subcategory_1",
        "part_of_speech_subcategory_2",
        "part_of_speech_subcategory_3",
        "conjugation_type",
        "conjugation_form",
        "reading",
        "normalized_form",
        "dictionary_form_id",
        "split_mode",
        "split_a",
        "split_b",
        "word_structure",
        "synonym_group_ids",
    ];

    /// The embedded IPADIC dictionary with its schema replaced by `fields`.
    ///
    /// Only the schema is swapped; the lexicon is still IPADIC's, so tokens
    /// built against this dictionary must carry explicit `details` laid out
    /// per `fields` rather than have them loaded from the lexicon.
    pub(crate) fn dictionary_with_schema(fields: &[&str]) -> Dictionary {
        let mut dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut metadata = (*dictionary.metadata).clone();
        metadata.dictionary_schema = Schema::new(fields.iter().map(|f| f.to_string()).collect());
        dictionary.metadata = Arc::new(metadata);
        dictionary
    }

    /// A dictionary with the SudachiDict schema; see [`dictionary_with_schema`].
    pub(crate) fn sudachidict_schema_dictionary() -> Dictionary {
        dictionary_with_schema(&SUDACHIDICT_FIELDS)
    }

    /// Builds contiguous tokens from `(surface, details)` rows: byte offsets
    /// follow the surfaces, positions count from zero, and each row's details
    /// are used verbatim, not padded.
    pub(crate) fn tokens<'a>(
        dictionary: &'a Dictionary,
        rows: &[(&'static str, &[&'static str])],
    ) -> Vec<Token<'a>> {
        let mut byte_start = 0;
        rows.iter()
            .enumerate()
            .map(|(i, (surface, details))| {
                let token = Token {
                    surface: Cow::Borrowed(surface),
                    byte_start,
                    byte_end: byte_start + surface.len(),
                    position: i,
                    position_length: 1,
                    word_id: WordId::new(LexType::System, i as u32),
                    dictionary,
                    user_dictionary: None,
                    details: Some(details.iter().map(|d| Cow::Borrowed(*d)).collect()),
                };
                byte_start += surface.len();
                token
            })
            .collect()
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
            write_japanese_pos_key(&mut token, 0, &mut key);
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

        apply_tag_filter(&mut tokens, &tags, TagPolicy::Remove, |token, key| {
            write_japanese_pos_key(token, 0, key)
        });

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
            apply_tag_filter(&mut tokens, &tags, TagPolicy::Remove, |token, key| {
                write_japanese_pos_key(token, 0, key)
            });

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

    /// The part-of-speech offset follows the schema: IPADIC's first custom
    /// field is `part_of_speech`, SudachiDict's is `display_surface` with
    /// `part_of_speech` one field later, and a schema that does not name the
    /// field at all (ko-dic, the legacy default schema) falls back to the
    /// first detail, which keeps those dictionaries on their old behavior.
    #[test]
    fn test_part_of_speech_offset_follows_the_schema() {
        use lindera::dictionary::{DictionaryKind, Schema, load_embedded_dictionary};

        use super::test_support::{dictionary_with_schema, sudachidict_schema_dictionary};

        let ipadic = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        assert_eq!(part_of_speech_offset(&ipadic), 0);

        let sudachidict = sudachidict_schema_dictionary();
        assert_eq!(part_of_speech_offset(&sudachidict), 1);

        let ko_dic_like = dictionary_with_schema(&[
            "surface",
            "left_context_id",
            "right_context_id",
            "cost",
            "part_of_speech_tag",
            "meaning",
        ]);
        assert_eq!(part_of_speech_offset(&ko_dic_like), 0);

        let legacy_schema = Schema::default();
        let legacy_fields: Vec<&str> = legacy_schema
            .get_all_fields()
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(legacy_fields[4], "major_pos");
        let legacy = dictionary_with_schema(&legacy_fields);
        assert_eq!(part_of_speech_offset(&legacy), 0);

        // An empty list has no dictionary to consult and no detail to read.
        assert_eq!(part_of_speech_offset_of(&[]), 0);
    }

    /// With an offset the key skips the leading non-part-of-speech fields
    /// and still takes exactly four: SudachiDict's `display_surface` never
    /// leaks into the key, and a token with no detail past the offset yields
    /// the empty key rather than panicking.
    #[test]
    fn test_write_japanese_pos_key_starts_at_the_offset() {
        use super::test_support::{sudachidict_schema_dictionary, tokens};

        let dictionary = sudachidict_schema_dictionary();
        let mut tokens = tokens(
            &dictionary,
            &[
                (
                    "五",
                    &[
                        "五", "名詞", "数詞", "*", "*", "*", "*", "ゴ", "五", "*", "A", "*", "*",
                        "*", "017040",
                    ],
                ),
                ("短", &["短", "名詞", "数詞"]),
                ("表層のみ", &["表層のみ"]),
                ("空", &[]),
            ],
        );
        let expected = ["名詞,数詞,*,*", "名詞,数詞", "", ""];

        let mut key = String::new();
        for (token, expected) in tokens.iter_mut().zip(expected) {
            key.clear();
            write_japanese_pos_key(token, 1, &mut key);
            assert_eq!(key, expected, "surface {}", token.surface);
        }
    }
}
