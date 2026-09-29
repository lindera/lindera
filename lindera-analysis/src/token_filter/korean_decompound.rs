use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera_segmenter::LinderaResult;
use lindera_segmenter::dictionary::{Dictionary, Schema, UserDictionary, WordId};
use lindera_segmenter::error::LinderaErrorKind;
use lindera_segmenter::token::Token;

pub const KOREAN_DECOMPOUND_TOKEN_FILTER_NAME: &str = "korean_decompound";

pub type KoreanDecompoundTokenFilterConfig = Value;

/// The `type` value ko-dic gives a compound noun, the only type whose
/// fragments carry their own offsets (see [`KoreanDecompoundTokenFilter`]).
const TYPE_COMPOUND: &str = "Compound";

/// Every `type` value ko-dic gives an entry that carries a decomposition, and
/// so every value `types` may name.
const KNOWN_TYPES: [&str; 3] = [TYPE_COMPOUND, "Inflect", "Preanalysis"];

/// ko-dic's placeholder for an absent field.
const ABSENT: &str = "*";

/// Dictionary schema fields this filter reads and rewrites. `type`,
/// `expression` and `part_of_speech_tag` are required; the rest are absent
/// from some schemas and simply not rewritten then.
const FIELD_TYPE: &str = "type";
const FIELD_EXPRESSION: &str = "expression";
const FIELD_POS_TAG: &str = "part_of_speech_tag";
const FIELD_MEANING: &str = "meaning";
const FIELD_READING: &str = "reading";
const FIELD_PRESENCE: &str = "presence_absence";
const FIELD_FIRST_POS: &str = "first_part_of_speech";
const FIELD_LAST_POS: &str = "last_part_of_speech";

/// Number of leading CSV columns (`surface`, `left_context_id`,
/// `right_context_id`, `cost`) that precede a token's detail fields.
const COMMON_FIELD_COUNT: usize = 4;

/// How a decompounded token is represented in the output stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KoreanDecompoundMode {
    /// Replace the compound with its fragments (nori's `discard`, the
    /// default there and here).
    #[default]
    Discard,
    /// Keep the compound *and* emit its fragments (nori's `mixed`).
    Mixed,
}

impl KoreanDecompoundMode {
    /// Parses the `mode` config value.
    ///
    /// # Arguments
    ///
    /// * `value` - `"discard"` or `"mixed"`.
    ///
    /// # Returns
    ///
    /// The mode, or an error for any other string.
    fn parse(value: &str) -> LinderaResult<Self> {
        match value {
            "discard" => Ok(Self::Discard),
            "mixed" => Ok(Self::Mixed),
            other => Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "mode must be \"discard\" or \"mixed\", got {other:?}"
            ))),
        }
    }
}

/// One fragment parsed out of a `expression` field, e.g. `무궁/NNG/*`.
#[derive(Clone, Debug)]
struct Fragment {
    /// The fragment's surface form.
    surface: String,
    /// The fragment's part-of-speech tag.
    pos: String,
    /// The fragment's semantic class, `*` when it carries none.
    meaning: String,
}

/// The parts of a compound token a fragment inherits besides its details,
/// copied out so the compound itself can be moved into the output first.
#[derive(Clone, Copy)]
struct Source<'a> {
    word_id: WordId,
    dictionary: &'a Dictionary,
    user_dictionary: Option<&'a UserDictionary>,
}

/// Detail-field indexes resolved once per `apply` call from the dictionary
/// schema, so no per-token name lookup is needed.
#[derive(Clone, Copy, Debug)]
struct Fields {
    type_: usize,
    expression: usize,
    pos_tag: usize,
    meaning: Option<usize>,
    reading: Option<usize>,
    presence: Option<usize>,
    first_pos: Option<usize>,
    last_pos: Option<usize>,
}

impl Fields {
    /// Resolves the detail indexes this filter needs.
    ///
    /// # Arguments
    ///
    /// * `schema` - The system dictionary schema.
    ///
    /// # Returns
    ///
    /// The indexes, or `None` when the schema lacks `type`, `expression` or
    /// `part_of_speech_tag` — i.e. the dictionary is not ko-dic-shaped and
    /// nothing can be decompounded.
    fn resolve(schema: &Schema) -> Option<Self> {
        // Detail indexes are schema indexes minus the four common columns.
        let detail = |name: &str| {
            schema
                .get_field_index(name)
                .and_then(|i| i.checked_sub(COMMON_FIELD_COUNT))
        };
        Some(Self {
            type_: detail(FIELD_TYPE)?,
            expression: detail(FIELD_EXPRESSION)?,
            pos_tag: detail(FIELD_POS_TAG)?,
            meaning: detail(FIELD_MEANING),
            reading: detail(FIELD_READING),
            presence: detail(FIELD_PRESENCE),
            first_pos: detail(FIELD_FIRST_POS),
            last_pos: detail(FIELD_LAST_POS),
        })
    }
}

/// Splits ko-dic compound, inflected and pre-analyzed tokens into the
/// morphemes their `expression` field lists, reproducing Lucene nori's
/// `DecompoundMode`.
///
/// ko-dic marks such an entry with a `type` of `Compound`, `Inflect` or
/// `Preanalysis` and spells the decomposition out in `expression`, as
/// `surface/POS/semantic-class` fragments joined by `+`:
///
/// ```text
/// 무궁화  NNG,*,F,무궁화,Compound,*,*,무궁/NNG/*+화/NNG/*
/// 갔      VV+EP,*,T,갔,Inflect,VV,EP,가/VV/*+았/EP/*
/// ```
///
/// Lindera's `Mode::Decompose` does not read these fields — it applies a
/// length penalty tuned for Japanese kanji — so `무궁화` stays whole in every
/// mode without this filter.
///
/// # Offsets
///
/// Only `Compound` fragments get their own byte range, carved out of the
/// token's span from the right by each fragment's own length. Every other
/// type shares the whole token's span, because an inflected form's fragments
/// need not be substrings of it at all (`갔` decomposes to `가` + `았`). This
/// is what nori does in `Viterbi`, including the direction the `Compound`
/// walk runs in, which is what decides where the discrepancy lands for the
/// handful of ko-dic entries whose fragment lengths fall short of the surface
/// length (`그레이맨` = `그레이` + an empty fragment).
///
/// Where nori's walk would produce an offset outside the token or inside a
/// character, a `Compound` token falls back to the shared span instead: when
/// the fragments are longer than the surface, when a carved boundary is not a
/// character boundary of the surface, or when an earlier filter has rewritten
/// the surface so its bytes no longer match the span. No bundled ko-dic entry
/// reaches any of these, so the bundled output is nori's exactly; the guard is
/// for user dictionaries, where an entry such as `프린터3D` =
/// `프린터` + `쓰리디` would otherwise yield an offset in the middle of `프`
/// and make any caller slicing the text by it panic.
///
/// # Positions
///
/// `Discard` gives the fragments consecutive positions starting at the
/// compound's own. `Mixed` keeps the compound at that position with a
/// `position_length` spanning all its fragments, then emits the fragments
/// from the same position onward — the graph nori builds with a zero position
/// increment on the first fragment. Later tokens shift by the number of
/// positions the expansion added, which preserves any gaps an earlier filter
/// (such as `korean_stop_tags`) left behind.
///
/// Only tokens spanning exactly one position are expanded. A longer token has
/// already been expanded (the compound a `Mixed` pass keeps) or merged by
/// another filter, and splitting it would duplicate its fragments. Put this
/// filter in a chain once; a single-fragment entry kept by `Mixed` still spans
/// one position, and a second pass would expand it again.
///
/// # Ordering
///
/// `Mixed` output and the fragments of a non-`Compound` token overlap in byte
/// range by design. Place a merging filter (`korean_compound_word`) before this
/// one when both are used: `merge_consecutive_tokens` ends a run at a byte
/// overlap, so nothing breaks either way, but merging first is what lets it
/// see the tokens the segmenter produced.
///
/// # Details
///
/// A fragment inherits the compound's detail fields, then overrides
/// `part_of_speech_tag` and `meaning` with its own, and blanks the fields
/// that describe the compound rather than the fragment: `type`, `expression`,
/// `first_part_of_speech`, `last_part_of_speech`, `reading` and
/// `presence_absence`. Blanking the reading matches nori, whose
/// `DecompoundToken::getReading` returns `null`; `korean_reading_form` skips
/// `*`, so a fragment keeps its own surface there. The fragment keeps the
/// compound's `word_id`, which stays a usable back-reference to the entry it
/// came from.
///
/// Applied to a dictionary whose schema has no `type`/`expression` fields
/// (every non-ko-dic dictionary), the filter passes tokens through unchanged
/// and warns once per configured filter. Clones share the warning, so a
/// tokenizer and the workers made from it warn once between them, while two
/// separately configured pipelines each warn.
#[derive(Clone, Debug)]
pub struct KoreanDecompoundTokenFilter {
    mode: KoreanDecompoundMode,
    types: HashSet<String>,
    /// Set once the unsupported-schema warning has been logged.
    unsupported_warned: Arc<AtomicBool>,
}

impl KoreanDecompoundTokenFilter {
    /// Creates a filter.
    ///
    /// # Arguments
    ///
    /// * `mode` - Whether to replace the compound or keep it alongside the
    ///   fragments.
    /// * `types` - The `type` values to decompound. Taken as given;
    ///   [`Self::from_config`] is where a config's values are validated.
    ///
    /// # Returns
    ///
    /// The filter.
    pub fn new(mode: KoreanDecompoundMode, types: HashSet<String>) -> Self {
        Self {
            mode,
            types,
            unsupported_warned: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The types decompounded when the config names none: every type ko-dic
    /// defines a decomposition for.
    ///
    /// # Returns
    ///
    /// `{Compound, Inflect, Preanalysis}`.
    pub fn default_types() -> HashSet<String> {
        KNOWN_TYPES.into_iter().map(str::to_string).collect()
    }

    /// Builds a filter from its JSON config.
    ///
    /// `mode` defaults to `"discard"` and `types` to
    /// [`Self::default_types`].
    ///
    /// # Arguments
    ///
    /// * `config` - `{"mode": "discard"|"mixed", "types": [...]}`.
    ///
    /// # Returns
    ///
    /// The filter, or an error for an unknown mode, a malformed `types`, an
    /// empty `types`, or a `types` value ko-dic never assigns. The last two
    /// are rejected rather than accepted as a filter that silently does
    /// nothing, the same way an unknown `mode` is; values are case-sensitive
    /// because they are compared with the dictionary's own field.
    pub fn from_config(config: &KoreanDecompoundTokenFilterConfig) -> LinderaResult<Self> {
        let mode = match config.get("mode") {
            None | Some(Value::Null) => KoreanDecompoundMode::default(),
            Some(value) => {
                let value = value.as_str().ok_or_else(|| {
                    LinderaErrorKind::Deserialize
                        .with_error(anyhow::anyhow!("mode must be a string"))
                })?;
                KoreanDecompoundMode::parse(value)?
            }
        };

        let types = match config.get("types") {
            None | Some(Value::Null) => Self::default_types(),
            Some(value) => {
                let known = KNOWN_TYPES.join(", ");
                let array = value.as_array().ok_or_else(|| {
                    LinderaErrorKind::Deserialize
                        .with_error(anyhow::anyhow!("types must be an array of strings"))
                })?;
                if array.is_empty() {
                    return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                        "types must name at least one of {known}; an empty list would \
                         decompound nothing"
                    )));
                }
                let mut types = HashSet::with_capacity(array.len());
                for value in array {
                    let token_type = value.as_str().ok_or_else(|| {
                        LinderaErrorKind::Deserialize
                            .with_error(anyhow::anyhow!("type must be a string"))
                    })?;
                    if !KNOWN_TYPES.contains(&token_type) {
                        return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                            "unknown type {token_type:?}; expected one of {known} \
                             (case-sensitive)"
                        )));
                    }
                    types.insert(token_type.to_string());
                }
                types
            }
        };

        Ok(Self::new(mode, types))
    }

    /// Parses `expression` into fragments, dropping the empty-surface ones a
    /// few ko-dic entries carry (an elided copula in `고양이로소이다`, for
    /// instance) since a zero-width token would be meaningless.
    ///
    /// # Arguments
    ///
    /// * `expression` - The raw field, e.g. `무궁/NNG/*+화/NNG/*`.
    ///
    /// # Returns
    ///
    /// The fragments, or `None` when the field is absent, malformed, or holds
    /// nothing but empty surfaces.
    fn parse_fragments(expression: &str) -> Option<Vec<Fragment>> {
        if expression.is_empty() || expression == ABSENT {
            return None;
        }

        let mut fragments = Vec::new();
        for part in expression.split('+') {
            let mut pieces = part.split('/');
            let surface = pieces.next()?;
            // A fragment without a part of speech is not one this filter can
            // describe; treat the whole field as malformed rather than
            // guessing a tag.
            let pos = pieces.next()?;
            let meaning = pieces.next().unwrap_or(ABSENT);
            if surface.is_empty() {
                continue;
            }
            fragments.push(Fragment {
                surface: surface.to_string(),
                pos: pos.to_string(),
                meaning: meaning.to_string(),
            });
        }

        (!fragments.is_empty()).then_some(fragments)
    }

    /// Returns the fragments `token` should be split into, if any.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to inspect (mutable only because reading a
    ///   detail field materializes the token's details).
    /// * `fields` - The resolved schema indexes.
    ///
    /// # Returns
    ///
    /// `Some((fragments, is_compound))` when the token's type is configured
    /// and its expression parses, `None` otherwise.
    fn fragments_of(
        &self,
        token: &mut Token<'_>,
        fields: &Fields,
    ) -> Option<(Vec<Fragment>, bool)> {
        // Read the type first and copy what is needed out of the borrow, so
        // the expression can be read next.
        let (wanted, is_compound) = {
            let token_type = token.get_detail(fields.type_)?;
            (self.types.contains(token_type), token_type == TYPE_COMPOUND)
        };
        if !wanted {
            return None;
        }

        let fragments = {
            let expression = token.get_detail(fields.expression)?;
            Self::parse_fragments(expression)?
        };

        Some((fragments, is_compound))
    }

    /// Assigns each fragment its byte range within the compound's span.
    ///
    /// # Arguments
    ///
    /// * `fragments` - The fragments, in reading order.
    /// * `surface` - The compound's surface, used to check that every carved
    ///   boundary is a character boundary.
    /// * `byte_start` - The compound's start offset.
    /// * `byte_end` - The compound's end offset.
    /// * `is_compound` - Whether to carve the span up (`Compound`) or hand
    ///   every fragment the whole span (every other type).
    ///
    /// # Returns
    ///
    /// One `(start, end)` pair per fragment, in reading order. Every offset
    /// lies inside the compound's span and on a character boundary.
    fn spans(
        fragments: &[Fragment],
        surface: &str,
        byte_start: usize,
        byte_end: usize,
        is_compound: bool,
    ) -> Vec<(usize, usize)> {
        let shared = || vec![(byte_start, byte_end); fragments.len()];

        // A surface an earlier filter rewrote (`korean_reading_form`,
        // `mapping`) no longer matches the span byte for byte, so boundaries
        // cannot be checked against it; share the span rather than guess.
        if !is_compound || byte_end.checked_sub(byte_start) != Some(surface.len()) {
            return shared();
        }

        // Walk from the end, as nori does: the last fragment is anchored to
        // the compound's end, so a shortfall lands on the first fragment's
        // start rather than on the last fragment's end. Where nori's walk
        // would leave the token (fragments longer than the surface) or cut a
        // character in two (fragments that are not substrings of it), share
        // the span instead. No bundled ko-dic entry reaches either case.
        let mut spans = vec![(byte_start, byte_end); fragments.len()];
        let mut end = byte_end;
        for (i, fragment) in fragments.iter().enumerate().rev() {
            let Some(start) = end
                .checked_sub(fragment.surface.len())
                .filter(|&start| start >= byte_start)
            else {
                return shared();
            };
            if !surface.is_char_boundary(start - byte_start) {
                return shared();
            }
            spans[i] = (start, end);
            end = start;
        }
        spans
    }

    /// Builds one fragment token.
    ///
    /// # Arguments
    ///
    /// * `fragment` - The fragment to build a token for, consumed so its
    ///   strings move into the token.
    /// * `span` - Its `(byte_start, byte_end)`.
    /// * `position` - Its position in the token stream.
    /// * `source` - The dictionary references and word id of the compound
    ///   the fragment came from.
    /// * `base_details` - The compound's materialized details, inherited.
    /// * `fields` - The resolved schema indexes.
    ///
    /// # Returns
    ///
    /// The fragment token.
    fn fragment_token<'a>(
        fragment: Fragment,
        span: (usize, usize),
        position: usize,
        source: Source<'a>,
        base_details: &[Cow<'a, str>],
        fields: &Fields,
    ) -> Token<'a> {
        let Fragment {
            surface,
            pos,
            meaning,
        } = fragment;
        let mut details = base_details.to_vec();
        let mut set = |index: usize, value: Cow<'a, str>| {
            if let Some(slot) = details.get_mut(index) {
                *slot = value;
            }
        };

        set(fields.pos_tag, Cow::Owned(pos));
        set(fields.type_, Cow::Borrowed(ABSENT));
        set(fields.expression, Cow::Borrowed(ABSENT));
        if let Some(index) = fields.meaning {
            set(index, Cow::Owned(meaning));
        }
        // Fields that describe the compound, not the fragment. The reading is
        // blanked rather than guessed, matching nori.
        for index in [
            fields.reading,
            fields.presence,
            fields.first_pos,
            fields.last_pos,
        ]
        .into_iter()
        .flatten()
        {
            set(index, Cow::Borrowed(ABSENT));
        }

        Token {
            surface: Cow::Owned(surface),
            byte_start: span.0,
            byte_end: span.1,
            position,
            position_length: 1,
            word_id: source.word_id,
            dictionary: source.dictionary,
            user_dictionary: source.user_dictionary,
            details: Some(details),
        }
    }
}

impl TokenFilter for KoreanDecompoundTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_DECOMPOUND_TOKEN_FILTER_NAME
    }

    /// Splits every configured compound token into the morphemes its
    /// `expression` field lists.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The tokens to filter, replaced in place.
    ///
    /// # Returns
    ///
    /// `Ok(())`. A dictionary without the required schema fields is a no-op
    /// (warned once per configured filter), not an error, so a filter chain
    /// shared across dictionaries keeps working.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        let Some(first) = tokens.first() else {
            return Ok(());
        };

        // Every token in a call comes from the same segmenter, so the schema
        // is resolved once rather than per token.
        let metadata = &first.dictionary.metadata;
        let Some(fields) = Fields::resolve(&metadata.dictionary_schema) else {
            if !self.unsupported_warned.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "{KOREAN_DECOMPOUND_TOKEN_FILTER_NAME}: dictionary '{}' has no \
                     '{FIELD_TYPE}'/'{FIELD_EXPRESSION}'/'{FIELD_POS_TAG}' fields, so no token \
                     can be decompounded; the filter is doing nothing. It expects a ko-dic-style \
                     schema.",
                    metadata.name
                );
            }
            return Ok(());
        };

        let source = std::mem::take(tokens);
        let mut out: Vec<Token<'_>> = Vec::with_capacity(source.len());
        // Positions already in the stream are preserved; only the positions
        // this filter adds shift the tokens after them, so gaps left by an
        // earlier filter survive.
        let mut shift = 0usize;

        for mut token in source {
            token.position += shift;

            // Only a token occupying exactly one position is expanded; the
            // shift below assumes it. A longer one is the compound an earlier
            // `Mixed` pass kept, or a merge, and expanding it would duplicate
            // its fragments.
            if token.position_length != 1 {
                out.push(token);
                continue;
            }

            let Some((fragments, is_compound)) = self.fragments_of(&mut token, &fields) else {
                out.push(token);
                continue;
            };

            let spans = Self::spans(
                &fragments,
                &token.surface,
                token.byte_start,
                token.byte_end,
                is_compound,
            );
            let base_position = token.position;
            let fragment_count = fragments.len();
            let source = Source {
                word_id: token.word_id,
                dictionary: token.dictionary,
                user_dictionary: token.user_dictionary,
            };

            // `fragments_of` read the type and expression through
            // `get_detail`, which materialized the details. `Discard` drops
            // the compound, so its details can be taken rather than copied.
            let base_details = match self.mode {
                KoreanDecompoundMode::Discard => token.details.take(),
                KoreanDecompoundMode::Mixed => token.details.clone(),
            }
            .unwrap_or_default();

            if self.mode == KoreanDecompoundMode::Mixed {
                // The compound spans every position its fragments occupy.
                token.position_length = fragment_count;
                out.push(token);
            }

            for (i, (fragment, span)) in fragments.into_iter().zip(spans).enumerate() {
                out.push(Self::fragment_token(
                    fragment,
                    span,
                    base_position + i,
                    source,
                    &base_details,
                    &fields,
                ));
            }

            shift += fragment_count - 1;
        }

        *tokens = out;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::token_filter::korean_decompound::{
        KoreanDecompoundMode, KoreanDecompoundTokenFilter, KoreanDecompoundTokenFilterConfig,
    };

    #[test]
    fn test_korean_decompound_token_filter_config_defaults() {
        let config: KoreanDecompoundTokenFilterConfig = serde_json::from_str("{}").unwrap();
        let filter = KoreanDecompoundTokenFilter::from_config(&config).unwrap();

        assert_eq!(filter.mode, KoreanDecompoundMode::Discard);
        assert_eq!(filter.types, KoreanDecompoundTokenFilter::default_types());
    }

    #[test]
    fn test_korean_decompound_token_filter_config_explicit() {
        let config_str = r#"
        {
            "mode": "mixed",
            "types": [
                "Compound"
            ]
        }
        "#;
        let config: KoreanDecompoundTokenFilterConfig = serde_json::from_str(config_str).unwrap();
        let filter = KoreanDecompoundTokenFilter::from_config(&config).unwrap();

        assert_eq!(filter.mode, KoreanDecompoundMode::Mixed);
        assert_eq!(filter.types.len(), 1);
        assert!(filter.types.contains("Compound"));
    }

    #[test]
    fn test_korean_decompound_token_filter_config_rejects_bad_values() {
        for config_str in [
            r#"{"mode": "none"}"#,
            r#"{"mode": 1}"#,
            r#"{"types": "Compound"}"#,
            r#"{"types": [1]}"#,
            // Would otherwise build a filter that silently does nothing.
            r#"{"types": []}"#,
            r#"{"types": ["compound"]}"#,
            r#"{"types": ["Compound", "Noun"]}"#,
        ] {
            let config: KoreanDecompoundTokenFilterConfig =
                serde_json::from_str(config_str).unwrap();
            assert!(
                KoreanDecompoundTokenFilter::from_config(&config).is_err(),
                "{config_str}"
            );
        }
    }

    /// An expression whose fragments carry an empty surface (9 ko-dic
    /// entries, e.g. the elided copula in `고양이로소이다`) drops just that
    /// fragment; an absent or malformed field yields nothing at all.
    #[test]
    fn test_parse_fragments_edge_cases() {
        let parse = KoreanDecompoundTokenFilter::parse_fragments;

        let frags = parse("무궁/NNG/*+화/NNG/*").unwrap();
        assert_eq!(frags.len(), 2);
        assert_eq!(frags[0].surface, "무궁");
        assert_eq!(frags[0].pos, "NNG");
        assert_eq!(frags[0].meaning, "*");

        // Semantic class is carried through when the entry has one.
        let frags = parse("에르난/NNP/인명+뻬셰라노/NNP/인명").unwrap();
        assert_eq!(frags[1].meaning, "인명");

        // Empty-surface fragment dropped, the rest kept.
        let frags = parse("고양이/NNG/*+/VCP/*+로소이다/EC/*").unwrap();
        assert_eq!(
            frags.iter().map(|f| f.surface.as_str()).collect::<Vec<_>>(),
            vec!["고양이", "로소이다"]
        );

        assert!(parse("*").is_none());
        assert!(parse("").is_none());
        // Nothing but empty surfaces.
        assert!(parse("/NNG/*").is_none());
        // No part of speech.
        assert!(parse("무궁").is_none());
    }

    #[cfg(feature = "embed-ko-dic")]
    mod ko_dic {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::korean_decompound::{
            KoreanDecompoundTokenFilter, KoreanDecompoundTokenFilterConfig,
        };
        use lindera_segmenter::dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{
            Dictionary, DictionaryKind, WordId, load_embedded_dictionary,
        };
        use lindera_segmenter::token::Token;

        fn filter(config_str: &str) -> KoreanDecompoundTokenFilter {
            let config: KoreanDecompoundTokenFilterConfig =
                serde_json::from_str(config_str).unwrap();
            KoreanDecompoundTokenFilter::from_config(&config).unwrap()
        }

        /// Builds a token carrying ko-dic's eight detail fields.
        #[allow(clippy::too_many_arguments)]
        fn token<'a>(
            dictionary: &'a Dictionary,
            surface: &'a str,
            byte_start: usize,
            position: usize,
            pos_tag: &'a str,
            token_type: &'a str,
            expression: &'a str,
        ) -> Token<'a> {
            Token {
                surface: Cow::Borrowed(surface),
                byte_start,
                byte_end: byte_start + surface.len(),
                position,
                position_length: 1,
                word_id: WordId::new(LexType::System, 0),
                dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed(pos_tag),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("F"),
                    Cow::Borrowed(surface),
                    Cow::Borrowed(token_type),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed(expression),
                ]),
            }
        }

        fn rendered(tokens: &mut [Token<'_>]) -> Vec<(String, usize, usize, usize, usize)> {
            tokens
                .iter()
                .map(|t| {
                    (
                        t.surface.to_string(),
                        t.byte_start,
                        t.byte_end,
                        t.position,
                        t.position_length,
                    )
                })
                .collect()
        }

        /// `discard` replaces the compound with its fragments, which carve up
        /// its byte span and take consecutive positions.
        #[test]
        fn test_discard_replaces_compound_with_fragments() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let mut tokens = vec![token(
                &dictionary,
                "무궁화",
                0,
                0,
                "NNG",
                "Compound",
                "무궁/NNG/*+화/NNG/*",
            )];

            filter(r#"{"mode": "discard"}"#).apply(&mut tokens).unwrap();

            assert_eq!(
                rendered(&mut tokens),
                vec![
                    ("무궁".to_string(), 0, 6, 0, 1),
                    ("화".to_string(), 6, 9, 1, 1),
                ]
            );
        }

        /// `mixed` keeps the compound, spanning every position its fragments
        /// occupy, and emits the fragments from the same position onward.
        #[test]
        fn test_mixed_keeps_compound_and_spans_its_fragments() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let mut tokens = vec![token(
                &dictionary,
                "무궁화",
                0,
                0,
                "NNG",
                "Compound",
                "무궁/NNG/*+화/NNG/*",
            )];

            filter(r#"{"mode": "mixed"}"#).apply(&mut tokens).unwrap();

            assert_eq!(
                rendered(&mut tokens),
                vec![
                    ("무궁화".to_string(), 0, 9, 0, 2),
                    ("무궁".to_string(), 0, 6, 0, 1),
                    ("화".to_string(), 6, 9, 1, 1),
                ]
            );
        }

        /// A fragment takes its own part of speech and semantic class, and
        /// blanks the fields that describe the compound. The reading is
        /// blanked rather than guessed, matching nori's
        /// `DecompoundToken.getReading() == null`.
        #[test]
        fn test_fragment_details_are_rewritten() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let mut tokens = vec![token(
                &dictionary,
                "가곡역",
                0,
                0,
                "NNP",
                "Compound",
                "가곡/NNP/지명+역/NNG/*",
            )];

            filter("{}").apply(&mut tokens).unwrap();

            let details: Vec<Vec<String>> = tokens
                .iter_mut()
                .map(|t| t.details().iter().map(|d| d.to_string()).collect())
                .collect();

            // part_of_speech_tag, meaning, presence_absence, reading,
            // type, first_part_of_speech, last_part_of_speech, expression
            assert_eq!(
                details[0],
                vec!["NNP", "지명", "*", "*", "*", "*", "*", "*"]
            );
            assert_eq!(details[1], vec!["NNG", "*", "*", "*", "*", "*", "*", "*"]);
            // The fragments keep the compound's word id as a back-reference.
            assert!(tokens.iter().all(|t| t.word_id.id() == 0));
        }

        /// Only `Compound` fragments carve up the span. An inflected form's
        /// fragments need not be substrings of it (`갔` = `가` + `았`), so
        /// they all share the whole token's offsets, as nori does.
        #[test]
        fn test_inflect_fragments_share_the_whole_span() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let mut tokens = vec![token(
                &dictionary,
                "갔",
                0,
                0,
                "VV+EP",
                "Inflect",
                "가/VV/*+았/EP/*",
            )];

            filter("{}").apply(&mut tokens).unwrap();

            assert_eq!(
                rendered(&mut tokens),
                vec![
                    ("가".to_string(), 0, 3, 0, 1),
                    ("았".to_string(), 0, 3, 1, 1),
                ]
            );
        }

        /// Expanding a token shifts the tokens after it by the positions the
        /// expansion added, and only by those, so a gap an earlier filter
        /// left (here 1 -> 5) survives.
        #[test]
        fn test_following_positions_shift_and_preserve_gaps() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let mut tokens = vec![
                token(
                    &dictionary,
                    "무궁화",
                    0,
                    0,
                    "NNG",
                    "Compound",
                    "무궁/NNG/*+화/NNG/*",
                ),
                token(&dictionary, "꽃", 9, 5, "NNG", "*", "*"),
            ];

            filter("{}").apply(&mut tokens).unwrap();

            assert_eq!(
                rendered(&mut tokens),
                vec![
                    ("무궁".to_string(), 0, 6, 0, 1),
                    ("화".to_string(), 6, 9, 1, 1),
                    ("꽃".to_string(), 9, 12, 6, 1),
                ]
            );
        }

        /// A token with no decomposition, and a type the config leaves out,
        /// both pass through untouched.
        #[test]
        fn test_tokens_without_a_configured_decomposition_are_untouched() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

            let mut tokens = vec![token(&dictionary, "꽃", 0, 0, "NNG", "*", "*")];
            filter("{}").apply(&mut tokens).unwrap();
            assert_eq!(rendered(&mut tokens), vec![("꽃".to_string(), 0, 3, 0, 1)]);

            // Compound excluded from `types`.
            let mut tokens = vec![token(
                &dictionary,
                "무궁화",
                0,
                0,
                "NNG",
                "Compound",
                "무궁/NNG/*+화/NNG/*",
            )];
            filter(r#"{"types": ["Inflect"]}"#)
                .apply(&mut tokens)
                .unwrap();
            assert_eq!(
                rendered(&mut tokens),
                vec![("무궁화".to_string(), 0, 9, 0, 1)]
            );
        }

        /// End to end through the segmenter: the filter reads the details the
        /// dictionary actually supplies, not hand-written ones.
        #[test]
        fn test_apply_to_segmented_text() {
            use std::borrow::Cow;

            use lindera_segmenter::mode::Mode;
            use lindera_segmenter::segmenter::Segmenter;

            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let mut tokens = segmenter
                .segment(Cow::Borrowed("무궁화꽃이 피었습니다."))
                .unwrap();

            let before: Vec<String> = tokens.iter().map(|t| t.surface.to_string()).collect();
            assert_eq!(before[0], "무궁화");

            filter("{}").apply(&mut tokens).unwrap();

            let after: Vec<String> = tokens.iter().map(|t| t.surface.to_string()).collect();
            assert_eq!(&after[..3], &["무궁", "화", "꽃"]);
            // Positions stay strictly increasing across the expansion.
            assert!(tokens.windows(2).all(|w| w[0].position < w[1].position));
            // Offsets still index the original text.
            let text = "무궁화꽃이 피었습니다.";
            for token in &tokens {
                assert_eq!(&text[token.byte_start..token.byte_end], token.surface);
            }
        }

        /// A compound defined only in a user dictionary decompounds too: the
        /// filter reads the token's details, whichever lexicon they came from.
        #[test]
        fn test_apply_to_user_dictionary_compound() {
            use std::borrow::Cow;
            use std::path::PathBuf;

            use lindera_segmenter::dictionary::load_user_dictionary_from_csv;
            use lindera_segmenter::mode::Mode;
            use lindera_segmenter::segmenter::Segmenter;

            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources/user_dict/ko-dic_detailed_userdic.csv");
            let user_dictionary =
                load_user_dictionary_from_csv(&dictionary.metadata, path.as_path()).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

            let mut tokens = segmenter.segment(Cow::Borrowed("세종시에 산다")).unwrap();
            assert_eq!(tokens[0].surface, "세종시");

            filter("{}").apply(&mut tokens).unwrap();

            let surfaces: Vec<String> = tokens.iter().map(|t| t.surface.to_string()).collect();
            assert_eq!(&surfaces[..2], &["세종", "시"]);
            assert_eq!(tokens[0].position, 0);
            assert_eq!(tokens[1].position, 1);
        }

        /// A compound whose fragments are not substrings of it would have its
        /// span carved in the middle of a character (`프린터3D` = `프린터` +
        /// `쓰리디` puts a boundary at byte 2, inside `프`), and any caller
        /// slicing the text by that offset would panic. Such a compound falls
        /// back to the shared span, as do fragments longer than the surface
        /// and a surface an earlier filter rewrote. Every offset stays on a
        /// character boundary of the text.
        #[test]
        fn test_compound_spans_never_split_a_character() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let text = "프린터3D";

            let mut tokens = vec![token(
                &dictionary,
                text,
                0,
                0,
                "NNG",
                "Compound",
                "프린터/NNG/*+쓰리디/NNG/*",
            )];
            filter("{}").apply(&mut tokens).unwrap();
            assert_eq!(
                rendered(&mut tokens),
                vec![
                    ("프린터".to_string(), 0, 11, 0, 1),
                    ("쓰리디".to_string(), 0, 11, 1, 1),
                ]
            );
            for token in &tokens {
                assert!(text.is_char_boundary(token.byte_start), "{}", token.surface);
                assert!(text.is_char_boundary(token.byte_end), "{}", token.surface);
            }

            // Fragments longer than the surface: nori's walk would leave the
            // token; the span is shared instead.
            let mut tokens = vec![token(
                &dictionary,
                "역사",
                0,
                0,
                "NNG",
                "Compound",
                "역사/NNG/*+관/NNG/*",
            )];
            filter("{}").apply(&mut tokens).unwrap();
            assert!(tokens.iter().all(|t| (t.byte_start, t.byte_end) == (0, 6)));

            // A surface an earlier filter rewrote no longer matches the span
            // byte for byte, so it cannot be carved.
            let mut rewritten = token(
                &dictionary,
                "무궁화",
                0,
                0,
                "NNG",
                "Compound",
                "무궁/NNG/*+화/NNG/*",
            );
            rewritten.byte_end = 12;
            let mut tokens = vec![rewritten];
            filter("{}").apply(&mut tokens).unwrap();
            assert!(tokens.iter().all(|t| (t.byte_start, t.byte_end) == (0, 12)));
        }

        /// Only a token spanning one position is expanded. The compound a
        /// `mixed` pass keeps spans all its fragments, so a second pass leaves
        /// it alone instead of duplicating them (`무궁화 무궁 화 무궁 화`).
        #[test]
        fn test_mixed_output_is_not_expanded_again() {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let input = || {
                vec![
                    token(
                        &dictionary,
                        "무궁화",
                        0,
                        0,
                        "NNG",
                        "Compound",
                        "무궁/NNG/*+화/NNG/*",
                    ),
                    token(&dictionary, "꽃", 9, 1, "NNG", "*", "*"),
                ]
            };
            let mixed = filter(r#"{"mode": "mixed"}"#);

            let mut once = input();
            mixed.apply(&mut once).unwrap();
            let mut twice = input();
            mixed.apply(&mut twice).unwrap();
            mixed.apply(&mut twice).unwrap();

            assert_eq!(rendered(&mut twice), rendered(&mut once));
            assert_eq!(
                rendered(&mut once),
                vec![
                    ("무궁화".to_string(), 0, 9, 0, 2),
                    ("무궁".to_string(), 0, 6, 0, 1),
                    ("화".to_string(), 6, 9, 1, 1),
                    ("꽃".to_string(), 9, 12, 2, 1),
                ]
            );
        }

        /// The reviewer's chain: `mixed` output followed by a merge on a tag
        /// the fragments carry. The merge ends its run at the overlap, so the
        /// compound's text is not duplicated into `무궁화무궁화꽃`, and every
        /// surface still matches the text its offsets cover.
        #[test]
        fn test_mixed_output_survives_a_following_merge() {
            use std::borrow::Cow;

            use crate::token_filter::korean_compound_word::KoreanCompoundWordTokenFilter;
            use lindera_segmenter::mode::Mode;
            use lindera_segmenter::segmenter::Segmenter;

            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let text = "무궁화꽃";
            let mut tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();

            filter(r#"{"mode": "mixed"}"#).apply(&mut tokens).unwrap();
            let merge_config = serde_json::json!({ "tags": ["NNG"], "new_tag": "NNG" });
            KoreanCompoundWordTokenFilter::from_config(&merge_config)
                .unwrap()
                .apply(&mut tokens)
                .unwrap();

            let surfaces: Vec<String> = tokens.iter().map(|t| t.surface.to_string()).collect();
            assert!(
                !surfaces.iter().any(|s| s.contains("무궁화무궁화")),
                "{surfaces:?}"
            );
            for token in &tokens {
                assert_eq!(&text[token.byte_start..token.byte_end], token.surface);
            }
        }
    }

    /// A dictionary whose schema has no `type`/`expression` fields cannot be
    /// decompounded; the filter passes every token through rather than
    /// failing the chain.
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_non_korean_dictionary_is_a_no_op() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::korean_decompound::{
            KoreanDecompoundTokenFilter, KoreanDecompoundTokenFilterConfig,
        };
        use lindera_segmenter::dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let mut tokens: Vec<Token> = vec![Token {
            surface: Cow::Borrowed("関西国際空港"),
            byte_start: 0,
            byte_end: 18,
            position: 0,
            position_length: 1,
            word_id: WordId::new(LexType::System, 0),
            dictionary: &dictionary,
            user_dictionary: None,
            details: None,
        }];

        let config: KoreanDecompoundTokenFilterConfig = serde_json::from_str("{}").unwrap();
        let filter = KoreanDecompoundTokenFilter::from_config(&config).unwrap();
        let clone = filter.clone();
        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].surface, "関西国際空港");
        assert_eq!(tokens[0].position, 0);

        // The warning is per configured filter: a clone (a worker made from
        // the same tokenizer) shares it, a separately configured filter does
        // not, so a second misconfigured pipeline still gets logged.
        use std::sync::atomic::Ordering;
        assert!(filter.unsupported_warned.load(Ordering::Relaxed));
        assert!(clone.unsupported_warned.load(Ordering::Relaxed));
        let separate = KoreanDecompoundTokenFilter::from_config(&config).unwrap();
        assert!(!separate.unsupported_warned.load(Ordering::Relaxed));
    }
}
