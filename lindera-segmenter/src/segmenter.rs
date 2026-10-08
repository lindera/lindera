mod carried_nbest;
mod exit_dominance;
mod nbest_merge;
mod path_tree;

use self::carried_nbest::CarriedNbest;
use self::exit_dominance::{MarginCache, prune_dominated};
use self::nbest_merge::merge_nbest;
use self::path_tree::{CarryState, NO_NODE, PathTree};

use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;

use lindera_dictionary::mode::Mode;
use log::warn;

use lindera_dictionary::dictionary::{Dictionary, UserDictionary};
use lindera_dictionary::space_penalty::{SpacePenaltyConfig, SpacePenaltyTable};
use lindera_dictionary::viterbi::{
    BosContext, Lattice, LatticeExit, LatticeOptions, NBestPath, TokenOffset,
};
use lindera_dictionary::whitespace::WhitespaceClassifier;
use serde_json::Value;

use crate::LinderaResult;
use crate::dictionary::{load_dictionary_with_options, load_user_dictionary};
use crate::error::LinderaErrorKind;
use crate::token::Token;

pub type SegmenterConfig = Value;

/// Upper bound, in bytes, on a single sentence passed to the Viterbi lattice
/// when no real sentence delimiter (`\n`, `\t`, `。`, `、`) is found.
///
/// Without this bound, delimiter-free input (e.g. minified or line-joined
/// machine-generated text) builds one ever-growing lattice for the whole
/// input. Past a content-dependent point, the accumulated path cost
/// saturates `i32::MAX` and the lattice silently stops accepting edges,
/// collapsing the remainder of the input into a single giant token with no
/// error or warning (see <https://github.com/lindera/lindera/issues/871>).
/// 32 KiB stays well under both that saturation point and downstream
/// consumers' own token-length limits (e.g. tantivy's `MAX_TOKEN_LEN`).
pub(crate) const MAX_SENTENCE_BYTES: usize = 32 * 1024;

/// Finds the end of the next sentence starting at `sentence_start`, scanning
/// for a real sentence delimiter (`\n`, `\t`, `。`, `、`) or, failing that,
/// forcing a cut at [`MAX_SENTENCE_BYTES`] to bound lattice size.
///
/// # 引数
///
/// * `text` - The full input text.
/// * `sentence_start` - The byte offset to start scanning from.
///
/// # 戻り値
///
/// A tuple of the byte offset (exclusive) where the sentence ends, and
/// whether the cut was forced (`true`) rather than a real delimiter
/// (`false`).
fn find_sentence_end(text: &str, sentence_start: usize) -> (usize, bool) {
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();
    let mut sentence_end = sentence_start;
    while sentence_end < text_len {
        let ch = text_bytes[sentence_end];
        sentence_end += 1;
        // Check for sentence delimiters
        if ch == b'\n' || ch == b'\t' {
            return (sentence_end, false);
        }
        // Check for Japanese punctuation (multi-byte). "。" and "、"
        // share the same 2-byte UTF-8 lead (E3 80) and differ only
        // in their last byte (0x82 / 0x81, which is exactly `ch`
        // here), so this cheap pre-check skips the lead comparison for
        // ~254/256 possible byte values. The lead bytes are compared one
        // by one: a 3-byte slice comparison may compile to a `memcmp`
        // call.
        if (ch == 0x81 || ch == 0x82)
            && sentence_end >= 3
            && text_bytes[sentence_end - 3] == 0xE3
            && text_bytes[sentence_end - 2] == 0x80
        {
            return (sentence_end, false);
        }
        // No real delimiter within MAX_SENTENCE_BYTES: force a cut at the
        // next valid char boundary to bound lattice size.
        if sentence_end - sentence_start >= MAX_SENTENCE_BYTES
            && text.is_char_boundary(sentence_end)
        {
            return (sentence_end, true);
        }
    }
    (sentence_end, false)
}

/// One sentence of an input, as [`SentenceWalk`] yields it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sentence {
    /// The sentence's start in the input, in bytes.
    start: usize,
    /// The sentence's end in the input (exclusive), in bytes.
    end: usize,
    /// Whether the cut after the sentence carries the context over to the
    /// next sentence: a cut after `、` or `。`, or a forced cut, that is not
    /// the end of the input. A cut after `\n` or `\t` and the end of the
    /// input carry nothing; they end a segment, a run of sentences that
    /// carry the context from one to the next.
    carries: bool,
}

/// Walks an input sentence by sentence, cutting where
/// [`find_sentence_end`] cuts and logging a warning at every forced cut.
struct SentenceWalk<'t> {
    /// The whole input.
    text: &'t str,
    /// The start of the next sentence, in bytes.
    next_start: usize,
}

impl<'t> SentenceWalk<'t> {
    /// Starts a walk at the beginning of `text`.
    ///
    /// # 引数
    ///
    /// * `text` - The whole input.
    ///
    /// # 戻り値
    ///
    /// The walk.
    fn new(text: &'t str) -> Self {
        Self {
            text,
            next_start: 0,
        }
    }
}

impl Iterator for SentenceWalk<'_> {
    type Item = Sentence;

    /// Returns the next sentence, which is never empty.
    ///
    /// # 戻り値
    ///
    /// The sentence, or `None` at the end of the input.
    #[inline]
    fn next(&mut self) -> Option<Sentence> {
        let text = self.text;
        while self.next_start < text.len() {
            let start = self.next_start;
            let (end, forced_cut) = find_sentence_end(text, start);
            if forced_cut {
                warn!(
                    "no sentence delimiter (\\n, \\t, 。, 、) found within {MAX_SENTENCE_BYTES} bytes from offset {start}; forcing a sentence boundary to bound lattice size (see https://github.com/lindera/lindera/issues/871)"
                );
            }
            self.next_start = end;
            if end == start {
                continue;
            }
            // A sentence that ends with a non-forced cut other than `\n` or
            // `\t` ends with `、` or `。` (or is the end of the input).
            let carries = end < text.len() && {
                let last = text.as_bytes()[end - 1];
                forced_cut || (last != b'\n' && last != b'\t')
            };
            return Some(Sentence {
                start,
                end,
                carries,
            });
        }
        None
    }
}

/// Scratch buffers of the 1-best segmentation, kept by `SegmentWorker`
/// across calls so a call allocates nothing it already has
/// ([`Segmenter::segment`] uses fresh ones).
#[derive(Debug, Default)]
pub(crate) struct SegmentBuffers {
    /// The tokens of the best path of the current sentence, as
    /// `tokens_offset_into` returns them (it clears the buffer first).
    offsets: Vec<TokenOffset>,
    /// The exits of the current sentence, as `exits_into` returns them.
    exits: Vec<LatticeExit>,
    /// The BOS contexts of the current sentence: one per state.
    contexts: Vec<BosContext>,
    /// The states the current sentence starts from, in ascending order of
    /// right id (the order of `contexts`).
    states: Vec<CarryState>,
    /// The states the current sentence ends with, built from its exits.
    next_states: Vec<CarryState>,
    /// The open paths of the states.
    tree: PathTree,
    /// Scratch of the exit pruning: which exits are dominated.
    dominated: Vec<bool>,
}

impl SegmentBuffers {
    /// Releases the capacity of every buffer.
    pub(crate) fn shrink_to_fit(&mut self) {
        self.offsets.shrink_to_fit();
        self.exits.shrink_to_fit();
        self.contexts.shrink_to_fit();
        self.states.shrink_to_fit();
        self.next_states.shrink_to_fit();
        self.tree.shrink_to_fit();
        self.dominated.shrink_to_fit();
    }
}

/// Fills `contexts` with one BOS context per state, in the order of the
/// states, with costs relative to the cheapest state.
///
/// # 引数
///
/// * `states` - The states; not empty.
/// * `contexts` - The buffer to fill, cleared first.
///
/// # 戻り値
///
/// The cost of the cheapest state, which the costs of the lattice's paths
/// are relative to. A relative cost beyond `i32::MAX` is saturated (the
/// lattice clamps BOS costs to its own bound anyway); such a cost gap cannot
/// arise from the bounded word, connection and penalty costs of a
/// dictionary in practice.
fn fill_bos_contexts<I>(states: I, contexts: &mut Vec<BosContext>) -> i64
where
    I: Iterator<Item = (u16, i64)> + Clone,
{
    let base = states.clone().map(|(_, cost)| cost).min().unwrap_or(0);
    contexts.clear();
    contexts.extend(states.map(|(right_id, cost)| BosContext {
        right_id,
        cost: i32::try_from(cost.saturating_sub(base)).unwrap_or(i32::MAX),
    }));
    base
}

/// Segmenter
#[derive(Clone)]
pub struct Segmenter {
    /// The segmentation mode to be used by the segmenter.
    /// This determines how the text will be split into segments.
    pub mode: Mode,

    /// The dictionary used for segmenting text. This dictionary contains the necessary
    /// data structures and algorithms to perform morphological analysis and tokenization.
    ///
    /// Assigning to this field after construction is not supported: [`Segmenter::new`]
    /// derives per-dictionary state from it (the `SPACE` category lookup behind
    /// `keep_whitespace`, and the [`Segmenter::space_penalty`] table when one is set),
    /// and that state is *not* recomputed here. A replacement dictionary therefore
    /// leaves those caches addressing the previous dictionary's ids, which yields
    /// wrong results silently rather than failing. Build a new `Segmenter` instead.
    pub dictionary: Dictionary,

    /// An optional user-defined dictionary that can be used to customize the segmentation process.
    /// If provided, this dictionary will be used in addition to the default dictionary to improve
    /// the accuracy of segmentation for specific words or phrases.
    ///
    /// Assigning to this field after construction is not supported, for the same reason
    /// as [`Segmenter::dictionary`] plus one of its own: [`Segmenter::new`] is where a
    /// user dictionary's context IDs are remapped into the system dictionary's ID space
    /// (see [`UserDictionary::remap_context_ids`]), so a dictionary put here directly
    /// keeps its original IDs and addresses the wrong connection-matrix cells. Build a
    /// new `Segmenter` instead.
    pub user_dictionary: Option<UserDictionary>,

    /// Keep whitespace tokens in output.
    ///
    /// When false (default), whitespace is handled as MeCab handles it: it is
    /// skipped in the lattice, so the words on either side of it connect
    /// directly (see [`Segmenter::skip_whitespace`]), and it is not output.
    /// When true, whitespace stays in the lattice as `SPACE` unknown-word
    /// nodes and is output as tokens. "Whitespace" is the dictionary's
    /// `SPACE` character category (`char.def`).
    pub keep_whitespace: bool,

    /// Whether whitespace is skipped in the lattice when `keep_whitespace` is
    /// false (ignored when `keep_whitespace` is true).
    ///
    /// MeCab never puts whitespace in the lattice: a word after a space
    /// connects to the word before it, and dictionaries trained with MeCab
    /// carry no connection costs for the `SPACE` unknown word (all zero in
    /// ko-dic). [`Segmenter::new`] takes the default from the dictionary's
    /// metadata (`Metadata::skip_whitespace`): `true` unless the dictionary
    /// says otherwise, and `false` for SudachiDict, whose costs are tuned
    /// for Sudachi's lattice, which keeps whitespace. With `false`,
    /// whitespace stays a `SPACE` node, as in Lindera v6, while still being
    /// dropped from the output; every space then resets the connection
    /// context. Either way, a dictionary entry whose surface ends with
    /// whitespace keeps it in its token, as in MeCab (#1108).
    pub skip_whitespace: bool,

    /// Cap on unknown-word grouping, counted in characters beyond the
    /// first (MeCab's `max-grouping-size`; MeCab defaults to 24). The cap
    /// is applied at each lattice position: when the same-category run
    /// starting there is longer than the cap, the grouped candidate is not
    /// emitted at that position -- the single-character candidate (plus
    /// the `unknown_word_ladder` candidates and any dictionary words)
    /// remains, and the remaining tail is grouped again once it fits. No
    /// unknown token is therefore longer than cap + 1 characters, but an
    /// over-long run does not degrade to single characters throughout.
    /// `None` (default) leaves grouping unbounded, matching previous
    /// behavior. `Some(0)` never groups a run of two or more characters;
    /// note that the `max_grouping_len` config key and the CLI flag treat
    /// `0` as unbounded instead.
    pub max_grouping_len: Option<usize>,

    /// Whether to additionally emit MeCab/Vibrato-inspired shorter
    /// unknown-word candidates up to each category's `char.def` `LENGTH`
    /// field (#945). Lindera parsed but never read this field before this
    /// option existed. Defaults to `true` (since v6). With `false`, each
    /// category gets only its grouped or single-character candidate at a
    /// position, as before v6; the output still differs from pre-v6 output
    /// wherever later changes apply, such as unknown words that start
    /// inside a grouped run (#1105).
    pub unknown_word_ladder: bool,

    /// Left-space penalty rules (mecab-ko's `left-space-penalty-factor`,
    /// nori's `computeSpacePenalty`): a candidate that starts right after
    /// whitespace and whose first part-of-speech tag matches a rule gets the
    /// rule's cost added, so e.g. a particle or ending is not chosen across
    /// a space. [`Segmenter::new`] initializes it from the rules the
    /// dictionary ships in its metadata (`Metadata::space_penalty`; ko-dic
    /// does, the other bundled dictionaries do not), so for ko-dic the
    /// penalty is on by default; `None` adds no penalty. Together with
    /// `skip_whitespace(false)`, `None` turns off both changes since v6.0
    /// in how text is read around spaces (this penalty and whitespace
    /// skipping), but not the other output changes since then, such as the
    /// split of Korean punctuation runs like `."` (#1105). Set through
    /// [`Segmenter::space_penalty`] /
    /// [`Segmenter::set_space_penalty`], which also build the per-word-id
    /// lookup the lattice uses; the field is read-only for that reason.
    space_penalty: Option<SpacePenaltyConfig>,

    /// Per-word-id lookup derived from `space_penalty` for the current
    /// dictionary pair; `Arc` so `Clone` stays cheap.
    space_penalty_table: Option<Arc<SpacePenaltyTable>>,

    /// The dictionary's `SPACE`-category classifier, behind
    /// `keep_whitespace` and `skip_whitespace`; built once from the loaded
    /// dictionary's own `char.def`, with an ASCII/Latin-1 fast path. `None`
    /// when the dictionary defines no `SPACE` category.
    whitespace: Option<WhitespaceClassifier>,

    /// The margins between right context ids by which the 1-best
    /// segmentation drops the exits of a sentence that cannot win (see
    /// `exit_dominance`), cached per pair for the dictionary's connection
    /// matrix; `Arc` so that clones share it.
    exit_margins: Arc<MarginCache>,
}

impl Segmenter {
    /// Creates a new instance with the specified mode, dictionary, and optional user dictionary.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `Mode` in which the instance will operate. This typically defines how aggressively the text is segmented or processed.
    /// * `dictionary` - A `Dictionary` object that provides the core data and rules for processing text.
    /// * `user_dictionary` - An optional `UserDictionary` that allows for additional, user-defined tokens or rules to be used in conjunction with the main dictionary.
    ///
    /// # Returns
    ///
    /// Returns a new instance of the struct with the provided mode, dictionary, and user dictionary (if any).
    ///
    /// # Details
    ///
    /// - `mode`: This defines the behavior of the instance, such as whether to process text in normal or aggressive mode.
    /// - `dictionary`: The main dictionary containing tokenization or processing rules.
    /// - `user_dictionary`: This is optional. If provided, it allows the user to extend or override the rules of the main dictionary with custom tokens.
    /// - Left-space penalty: when the dictionary ships rules in its metadata
    ///   (`Metadata::space_penalty`, as ko-dic does), they are applied here, so
    ///   the penalty is on by default for such dictionaries. Rules that cannot
    ///   be applied (a schema without a part-of-speech field) log a warning
    ///   and leave the penalty off. Opt out with [`Segmenter::space_penalty`]
    ///   and `None`.
    /// - Whitespace skipping: taken from the dictionary's metadata
    ///   (`Metadata::skip_whitespace`), on unless the dictionary turns it off
    ///   (SudachiDict does). Override with [`Segmenter::skip_whitespace`].
    pub fn new(
        mode: Mode,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        // Classify whitespace by the dictionary's SPACE category, which MeCab
        // skips (and Lindera drops from the output by default).
        let whitespace = WhitespaceClassifier::new(&dictionary.character_definition);
        // Skip that whitespace in the lattice unless the dictionary's metadata
        // says otherwise (SudachiDict does: Sudachi keeps it in the lattice).
        let skip_whitespace = dictionary.metadata.skip_whitespace.unwrap_or(true);

        // A user dictionary is always compiled in the original context-ID space. If the
        // system dictionary was built with `connection_id_mapping`, relabel the user
        // entries into the same space; otherwise their connection costs would address
        // the wrong matrix cells. This is the single point where both dictionaries are
        // available, and `user_dictionary` is taken by value so it cannot be remapped
        // twice.
        let user_dictionary = match (user_dictionary, dictionary.metadata.context_id_map.as_ref()) {
            (Some(mut user_dictionary), Some(map)) => {
                user_dictionary.remap_context_ids(map);
                Some(user_dictionary)
            }
            (user_dictionary, _) => user_dictionary,
        };

        let mut segmenter = Self {
            mode,
            dictionary,
            user_dictionary,
            keep_whitespace: false, // Default: drop whitespace, as MeCab does
            skip_whitespace,        // Default: the dictionary's (skip, as MeCab does)
            max_grouping_len: None, // Default: unbounded grouping
            unknown_word_ladder: true, // Default (v6+): honor char.def's LENGTH field
            space_penalty: None,    // Set below from the dictionary's shipped rules, if any
            space_penalty_table: None,
            whitespace,
            exit_margins: Arc::default(),
        };

        // Default (6.1+): apply the left-space penalty rules the dictionary
        // ships in its metadata (ko-dic carries mecab-ko-dic's). `new` cannot
        // return an error, so a dictionary whose rules cannot be applied (a
        // schema without a part-of-speech field) logs a warning and runs
        // unpenalized, the state `Segmenter::space_penalty(None)` selects
        // explicitly.
        let shipped_rules = segmenter.dictionary.metadata.space_penalty.clone();
        if let Some(rules) = shipped_rules
            && let Err(err) = segmenter.set_space_penalty(Some(rules))
        {
            warn!(
                "space penalty rules shipped by dictionary '{}' were not applied: {err}",
                segmenter.dictionary.metadata.name
            );
        }

        segmenter
    }

    /// Builder method to set whether to keep whitespace tokens in output.
    ///
    /// When `keep_whitespace` is false (default), whitespace is skipped in the lattice and
    /// dropped from the output, as MeCab does (see [`Segmenter::skip_whitespace`]).
    /// When true, whitespace stays in the lattice and whitespace tokens are included in the output.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included in the output.
    ///
    /// # Example
    ///
    /// ```
    /// use lindera_segmenter::mode::Mode;
    /// use lindera_segmenter::dictionary::load_dictionary;
    /// use lindera_segmenter::segmenter::Segmenter;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "embed-ipadic")]
    /// # {
    /// let dictionary = load_dictionary("embedded://ipadic")?;
    /// let segmenter = Segmenter::new(Mode::Normal, dictionary, None)
    ///     .keep_whitespace(true);
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn keep_whitespace(mut self, keep_whitespace: bool) -> Self {
        self.keep_whitespace = keep_whitespace;
        self
    }

    /// Builder method to set whether whitespace is skipped in the lattice
    /// when whitespace tokens are not kept (see the
    /// [`skip_whitespace`](Segmenter#structfield.skip_whitespace) field),
    /// overriding the dictionary's default.
    ///
    /// Skipping is on by default for every bundled dictionary but
    /// SudachiDict. Turning it off keeps whitespace as `SPACE` unknown-word
    /// nodes, as Lindera v6 did, which resets the connection context at
    /// every space; whitespace is still dropped from the output.
    ///
    /// # Arguments
    ///
    /// * `skip_whitespace` - `false` to keep whitespace nodes in the lattice.
    ///
    /// # Returns
    ///
    /// The segmenter.
    ///
    /// # Example
    ///
    /// ```
    /// use lindera_segmenter::mode::Mode;
    /// use lindera_segmenter::dictionary::load_dictionary;
    /// use lindera_segmenter::segmenter::Segmenter;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "embed-ko-dic")]
    /// # {
    /// let dictionary = load_dictionary("embedded://ko-dic")?;
    /// let segmenter = Segmenter::new(Mode::Normal, dictionary, None)
    ///     .skip_whitespace(false);
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn skip_whitespace(mut self, skip_whitespace: bool) -> Self {
        self.skip_whitespace = skip_whitespace;
        self
    }

    /// Builder method to cap unknown-word grouping (MeCab's
    /// `max-grouping-size` semantics; see the `max_grouping_len` field).
    ///
    /// # 引数
    ///
    /// * `max_grouping_len` - Maximum grouped characters beyond the first,
    ///   or `None` for unbounded grouping (the default). `Some(0)` never
    ///   groups two or more characters, unlike the config key, where `0`
    ///   means unbounded.
    ///
    /// # 戻り値
    ///
    /// `self`, for chaining.
    pub fn max_grouping_len(mut self, max_grouping_len: Option<usize>) -> Self {
        self.max_grouping_len = max_grouping_len;
        self
    }

    /// Builder method to enable/disable the unknown-word length ladder
    /// (see the `unknown_word_ladder` field; defaults to `true`).
    ///
    /// # 引数
    ///
    /// * `unknown_word_ladder` - Whether to emit the length ladder.
    ///
    /// # 戻り値
    ///
    /// `self`, for chaining.
    pub fn unknown_word_ladder(mut self, unknown_word_ladder: bool) -> Self {
        self.unknown_word_ladder = unknown_word_ladder;
        self
    }

    /// Builder method to set the left-space penalty rules (see the
    /// `space_penalty` field). `None` disables the penalty, including the
    /// rules the dictionary ships and [`Segmenter::new`] applies by default.
    ///
    /// Building the per-word-id lookup reads every entry's part-of-speech
    /// tag once, so this costs a few tens of milliseconds on a large
    /// dictionary; call it once at construction, not per document.
    ///
    /// # 引数
    ///
    /// * `space_penalty` - The rules, or `None` to disable the penalty.
    ///
    /// # 戻り値
    ///
    /// `self`, for chaining, or an error when the dictionary schema has no
    /// part-of-speech field (`part_of_speech_tag` or `part_of_speech`).
    ///
    /// # Example
    ///
    /// ```
    /// use lindera_segmenter::mode::Mode;
    /// use lindera_segmenter::dictionary::load_dictionary;
    /// use lindera_segmenter::segmenter::Segmenter;
    /// use lindera_segmenter::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "embed-ko-dic")]
    /// # {
    /// // mecab-ko-dic's `left-space-penalty-factor`, expressed by first POS tag.
    /// let rules = SpacePenaltyConfig::new(vec![
    ///     SpacePenaltyRule::new(["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], 3000),
    ///     SpacePenaltyRule::new(["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], 6000),
    /// ]);
    /// let dictionary = load_dictionary("embedded://ko-dic")?;
    /// let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty(Some(rules))?;
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn space_penalty(
        mut self,
        space_penalty: Option<SpacePenaltyConfig>,
    ) -> LinderaResult<Self> {
        self.set_space_penalty(space_penalty)?;
        Ok(self)
    }

    /// In-place form of [`Segmenter::space_penalty`], for holders that own
    /// a `Segmenter` by `&mut` (e.g. a reusable worker).
    ///
    /// # 引数
    ///
    /// * `space_penalty` - The rules, or `None` to disable the penalty.
    ///
    /// # 戻り値
    ///
    /// `Ok(())`, or the schema error described on
    /// [`Segmenter::space_penalty`]; on error the previous setting is kept.
    pub fn set_space_penalty(
        &mut self,
        space_penalty: Option<SpacePenaltyConfig>,
    ) -> LinderaResult<()> {
        let table = match &space_penalty {
            Some(config) => Some(Arc::new(SpacePenaltyTable::build(
                config,
                &self.dictionary,
                self.user_dictionary.as_ref(),
            )?)),
            None => None,
        };
        self.space_penalty = space_penalty;
        self.space_penalty_table = table;
        Ok(())
    }

    /// Returns the configured left-space penalty rules, if any.
    ///
    /// # 戻り値
    ///
    /// The rules set through [`Segmenter::space_penalty`], or `None`.
    pub fn space_penalty_config(&self) -> Option<&SpacePenaltyConfig> {
        self.space_penalty.as_ref()
    }

    /// Builder method to enable the left-space penalty with the rules the
    /// dictionary ships in its `metadata.json` (`space_penalty`); ko-dic
    /// carries mecab-ko-dic's `left-space-penalty-factor` there.
    ///
    /// # 戻り値
    ///
    /// `self`, for chaining, or an error when the dictionary ships no rules
    /// (or its schema has no part-of-speech field).
    ///
    /// # Example
    ///
    /// ```
    /// use lindera_segmenter::mode::Mode;
    /// use lindera_segmenter::dictionary::load_dictionary;
    /// use lindera_segmenter::segmenter::Segmenter;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "embed-ko-dic")]
    /// # {
    /// let dictionary = load_dictionary("embedded://ko-dic")?;
    /// let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty_from_dictionary()?;
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn space_penalty_from_dictionary(mut self) -> LinderaResult<Self> {
        self.set_space_penalty_from_dictionary()?;
        Ok(self)
    }

    /// In-place form of [`Segmenter::space_penalty_from_dictionary`].
    ///
    /// # 戻り値
    ///
    /// `Ok(())`, or an error when the dictionary ships no rules; the
    /// previous setting is kept on error.
    pub fn set_space_penalty_from_dictionary(&mut self) -> LinderaResult<()> {
        let rules = self
            .dictionary
            .metadata
            .space_penalty
            .clone()
            .ok_or_else(|| {
                LinderaErrorKind::Dictionary.with_error(anyhow::anyhow!(
                    "dictionary '{}' ships no space_penalty rules in its metadata; pass explicit rules instead",
                    self.dictionary.metadata.name
                ))
            })?;
        self.set_space_penalty(Some(rules))
    }

    /// Bundles the per-sentence lattice options from this segmenter's
    /// settings.
    ///
    /// # 戻り値
    ///
    /// The options for `set_text_with_options` /
    /// `set_text_nbest_with_options`.
    fn lattice_options(&self) -> LatticeOptions<'_> {
        let mut options = LatticeOptions::new(&self.mode);
        options.max_grouping_len = self.max_grouping_len;
        options.unknown_word_ladder = self.unknown_word_ladder;
        options.space_penalty = self.space_penalty_table.as_deref();
        options.skip_whitespace = self.whitespace_to_skip();
        options
    }

    /// Builds the 1-best lattice of one sentence.
    ///
    /// # 引数
    ///
    /// * `lattice` - The lattice to reuse.
    /// * `sentence` - The sentence's text.
    /// * `options` - The lattice options, from [`Segmenter::lattice_options`].
    #[inline]
    fn set_lattice_text(&self, lattice: &mut Lattice, sentence: &str, options: &LatticeOptions) {
        lattice.set_text_with_options(
            &self.dictionary.prefix_dictionary,
            &self.user_dictionary.as_ref().map(|d| &d.dict),
            &self.dictionary.character_definition,
            &self.dictionary.unknown_dictionary,
            &self.dictionary.connection_cost_matrix,
            sentence,
            options,
        );
    }

    /// Builds the N-best lattice of one sentence.
    ///
    /// # 引数
    ///
    /// * `lattice` - The lattice to reuse.
    /// * `sentence` - The sentence's text.
    /// * `options` - The lattice options, from [`Segmenter::lattice_options`].
    fn set_lattice_text_nbest(
        &self,
        lattice: &mut Lattice,
        sentence: &str,
        options: &LatticeOptions,
    ) {
        lattice.set_text_nbest_with_options(
            &self.dictionary.prefix_dictionary,
            &self.user_dictionary.as_ref().map(|d| &d.dict),
            &self.dictionary.character_definition,
            &self.dictionary.unknown_dictionary,
            &self.dictionary.connection_cost_matrix,
            sentence,
            options,
        );
    }

    /// Returns the classifier of the whitespace tokens to drop from the
    /// output: the dictionary's `SPACE` classifier unless `keep_whitespace`
    /// is set.
    ///
    /// # 戻り値
    ///
    /// The classifier, or `None` to keep every token.
    fn space_filter(&self) -> Option<&WhitespaceClassifier> {
        if self.keep_whitespace {
            None
        } else {
            self.whitespace.as_ref()
        }
    }

    /// Returns the classifier of the whitespace to skip in the lattice:
    /// whitespace is skipped whenever it is dropped from the output, unless
    /// `skip_whitespace` is off or the dictionary defines no `SPACE`
    /// category.
    ///
    /// # Returns
    ///
    /// The classifier, or `None` to keep whitespace in the lattice.
    fn whitespace_to_skip(&self) -> Option<&WhitespaceClassifier> {
        if self.keep_whitespace || !self.skip_whitespace {
            None
        } else {
            self.whitespace.as_ref()
        }
    }

    /// Returns whether `token_text` consists of whitespace only, i.e. is a
    /// `SPACE` token to drop when `keep_whitespace` is false.
    ///
    /// # Arguments
    ///
    /// * `token_text` - The token's text.
    /// * `whitespace` - The dictionary's `SPACE` classifier.
    ///
    /// # Returns
    ///
    /// `true` for a whitespace-only token.
    fn is_whitespace_token(&self, token_text: &str, whitespace: &WhitespaceClassifier) -> bool {
        let char_definitions = &self.dictionary.character_definition;
        token_text
            .chars()
            .all(|c| whitespace.is_space(c, char_definitions))
    }

    /// A struct representing a segmenter for tokenizing text.
    ///
    /// The `Segmenter` struct provides methods for creating a segmenter from a configuration,
    /// creating a new segmenter, and segmenting text into tokens.
    ///
    /// # Methods
    ///
    /// - `from_config`: Creates a `Segmenter` from a given configuration.
    /// - `new`: Creates a new `Segmenter` with the specified mode, dictionary, and optional user dictionary.
    /// - `segment`: Segments the given text into tokens.
    ///
    /// # Errors
    ///
    /// Methods that return `LinderaResult` may produce errors related to dictionary loading,
    /// user dictionary loading, or tokenization process.
    pub fn from_config(config: &SegmenterConfig) -> LinderaResult<Self> {
        // Whether to route filesystem-loaded dictionaries through memory-mapped
        // reads. Ignored for `embedded://` dictionaries. Defaults to on when
        // the `mmap` feature is compiled in (#879); set `"use_mmap": false`
        // to force eager reads.
        let use_mmap = config
            .get("use_mmap")
            .and_then(Value::as_bool)
            .unwrap_or(cfg!(feature = "mmap"));

        // Load the dictionary from the config
        let dictionary = load_dictionary_with_options(
            config
                .get("dictionary")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("dictionary field is missing"))
                })?,
            use_mmap,
        )?;

        // Get metadata from the dictionary
        let metadata = &dictionary.metadata;

        // Load the user dictionary from the config
        let user_dictionary_uri = config
            .get("user_dictionary")
            .and_then(Value::as_str)
            .map(String::from);

        let user_dictionary = match user_dictionary_uri {
            Some(uri) => Some(load_user_dictionary(&uri, metadata)?),
            None => None,
        };

        // Load the mode from the config
        let mode: Mode = config.get("mode").map_or_else(
            || Ok(Mode::Normal),
            |v| {
                if let Some(s) = v.as_str() {
                    Mode::from_str(s).map_err(|e| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("mode field is invalid string: {e}"))
                    })
                } else {
                    serde_json::from_value::<Mode>(v.clone()).map_err(|e| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("mode field is invalid object: {e}"))
                    })
                }
            },
        )?;

        // Load the keep_whitespace option from the config
        // Default is false (MeCab compatible - drop whitespace)
        // Set to true explicitly to include whitespace tokens
        let keep_whitespace = config
            .get("keep_whitespace")
            .and_then(Value::as_bool)
            .unwrap_or(false); // Default: false (drop whitespace)

        // Load the skip_whitespace option from the config. Absent or `null`
        // keeps the default `Segmenter::new` chose (the dictionary's, which
        // is to skip unless it says otherwise); a bool overrides it.
        let skip_whitespace = match config.get("skip_whitespace") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(skip)) => Some(*skip),
            Some(value) => {
                return Err(LinderaErrorKind::Parse.with_error(anyhow::anyhow!(
                    "skip_whitespace field must be a bool or null, got {value}"
                )));
            }
        };

        // Ignoring whitespace requires a SPACE category to detect it by.
        if !keep_whitespace {
            dictionary
                .character_definition
                .category_id_by_name("SPACE")
                .ok_or_else(|| {
                    LinderaErrorKind::Parse.with_error(anyhow::anyhow!(
                        "SPACE category is not defined in the dictionary (char.def)"
                    ))
                })?;
        }

        // Go through `new` so the user-dictionary context-ID remap has a single
        // application point. The `SPACE` classifier is only read when `keep_whitespace`
        // is false, so letting `new` always build it is equivalent to the previous
        // conditional.
        // Load the max_grouping_len option from the config.
        // Absent or 0 means unbounded grouping (the default).
        let max_grouping_len = config
            .get("max_grouping_len")
            .and_then(Value::as_u64)
            .filter(|&n| n > 0)
            .map(|n| n as usize);

        // Load the unknown_word_ladder option from the config.
        // Absent means the default (true).
        let unknown_word_ladder = config
            .get("unknown_word_ladder")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        // Load the space_penalty option from the config. Absent or `null`
        // keeps the default `Segmenter::new` chose (the rules the dictionary
        // ships, if any); `false` turns the penalty off; `true` requires the
        // dictionary's rules; an object holds explicit rules.
        enum SpacePenaltySetting {
            Default,
            Off,
            FromDictionary,
            Rules(SpacePenaltyConfig),
        }
        let space_penalty = match config.get("space_penalty") {
            None | Some(Value::Null) => SpacePenaltySetting::Default,
            Some(Value::Bool(false)) => SpacePenaltySetting::Off,
            Some(Value::Bool(true)) => SpacePenaltySetting::FromDictionary,
            Some(value) => SpacePenaltySetting::Rules(
                serde_json::from_value::<SpacePenaltyConfig>(value.clone()).map_err(|e| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("space_penalty field is invalid: {e}"))
                })?,
            ),
        };

        let mut segmenter = Self::new(mode, dictionary, user_dictionary)
            .keep_whitespace(keep_whitespace)
            .max_grouping_len(max_grouping_len)
            .unknown_word_ladder(unknown_word_ladder);
        if let Some(skip_whitespace) = skip_whitespace {
            segmenter.skip_whitespace = skip_whitespace;
        }
        match space_penalty {
            SpacePenaltySetting::Default => Ok(segmenter),
            SpacePenaltySetting::Off => segmenter.space_penalty(None),
            SpacePenaltySetting::FromDictionary => segmenter.space_penalty_from_dictionary(),
            SpacePenaltySetting::Rules(rules) => segmenter.space_penalty(Some(rules)),
        }
    }

    /// Segments the input text into tokens based on the dictionary and user-defined rules.
    ///
    /// # Arguments
    ///
    /// * `text` - A `Cow<'a, str>` representing the input text. This can either be borrowed or owned, allowing for efficient text handling depending on the use case.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<Vec<Token<'a>>>` which contains a vector of tokens segmented from the input text. Each token represents a portion of the original text, along with metadata such as byte offsets and dictionary information.
    ///
    /// # Process
    ///
    /// 1. **Sentence Splitting**:
    ///    - The input text is split into sentences at `。`, `、`, `\n` and `\t`, and by a forced cut after 32 KiB without any of them. A cut at `\n` or `\t` (or the end of the input) ends a segment; within a segment, the cuts at `、` and `。` and the forced cuts carry the context from one sentence to the next.
    ///
    /// 2. **Lattice Processing**:
    ///    - For each sentence, a lattice structure is set up using the main dictionary and, if available, the user dictionary. The lattice helps identify possible token boundaries within the sentence.
    ///    - The cost matrix is used to calculate the best path (i.e., the optimal sequence of tokens) through the lattice based on the mode.
    ///    - A sentence that continues a segment starts from the exits of the previous sentence (the right context ids of the words that end it), and the EOS connection is paid only at the end of the segment, so the best path of a segment is the one that a single lattice over the segment gives, except that no dictionary entry or unknown-word group spans a cut and a word that starts right at a forced cut never pays the left-space penalty.
    ///
    /// 3. **Token Generation**:
    ///    - For each word of the best path, a token is generated using the byte offsets. The tokens contain the original text (in `Cow::Owned` form to ensure safe return), byte start/end positions, token positions, and dictionary references.
    ///
    /// # Notes
    ///
    /// - The function ensures that each token is safely returned by converting substrings into `Cow::Owned` strings.
    /// - Byte offsets are carefully calculated to ensure that token boundaries are correct even across multiple sentences.
    ///
    /// # Example Flow
    ///
    /// - Text is split into sentences based on punctuation.
    /// - A lattice is created and processed for each sentence, continuing the context of the previous sentence within a segment.
    /// - Tokens are extracted from the lattices and returned in a vector.
    ///
    /// # Errors
    ///
    /// - If the lattice fails to be processed or if there is an issue with the segmentation process, the function returns an error.
    pub fn segment<'a>(&'a self, text: Cow<'a, str>) -> LinderaResult<Vec<Token<'a>>> {
        let mut lattice = Lattice::default();
        self.segment_with_lattice(text, &mut lattice)
    }

    /// Segments the input text into tokens based on the dictionary and user-defined rules.
    ///
    /// # Arguments
    ///
    /// * `text` - A `Cow<'a, str>` representing the input text. This can either be borrowed or owned, allowing for efficient text handling depending on the use case.
    /// * `lattice` - A mutable reference to a `Lattice` structure. This allows reusing the lattice across multiple calls to avoid memory allocation.
    ///
    /// # Returns
    ///
    /// Returns a `LinderaResult<Vec<Token<'a>>>` which contains a vector of tokens segmented from the input text. Each token represents a portion of the original text, along with metadata such as byte offsets and dictionary information.
    ///
    /// # Process
    ///
    /// 1. **Sentence Splitting**:
    ///    - The input text is split into sentences at `。`, `、`, `\n` and `\t`, and by a forced cut after 32 KiB without any of them. A cut at `\n` or `\t` (or the end of the input) ends a segment; within a segment, the cuts at `、` and `。` and the forced cuts carry the context from one sentence to the next.
    ///
    /// 2. **Lattice Processing**:
    ///    - For each sentence, a lattice structure is set up using the main dictionary and, if available, the user dictionary. The lattice helps identify possible token boundaries within the sentence.
    ///    - The cost matrix is used to calculate the best path (i.e., the optimal sequence of tokens) through the lattice based on the mode.
    ///    - A sentence that continues a segment starts from the exits of the previous sentence (the right context ids of the words that end it), and the EOS connection is paid only at the end of the segment, so the best path of a segment is the one that a single lattice over the segment gives, except that no dictionary entry or unknown-word group spans a cut and a word that starts right at a forced cut never pays the left-space penalty.
    ///
    /// 3. **Token Generation**:
    ///    - For each word of the best path, a token is generated using the byte offsets. The tokens contain the original text (in `Cow::Owned` form to ensure safe return), byte start/end positions, token positions, and dictionary references.
    ///
    /// # Notes
    ///
    /// - The function ensures that each token is safely returned by converting substrings into `Cow::Owned` strings.
    /// - Byte offsets are carefully calculated to ensure that token boundaries are correct even across multiple sentences.
    ///
    /// # Example Flow
    ///
    /// - Text is split into sentences based on punctuation.
    /// - A lattice is created and processed for each sentence, continuing the context of the previous sentence within a segment.
    /// - Tokens are extracted from the lattices and returned in a vector.
    ///
    /// # Errors
    ///
    /// - If the lattice fails to be processed or if there is an issue with the segmentation process, the function returns an error.
    pub fn segment_with_lattice<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
    ) -> LinderaResult<Vec<Token<'a>>> {
        // The scratch buffers are allocated once per call; SegmentWorker
        // routes through segment_with_buffers to reuse them across calls too.
        let mut buffers = SegmentBuffers::default();
        self.segment_with_buffers(text, lattice, &mut buffers)
    }

    /// Segments the input text reusing both the caller's lattice and the
    /// caller's scratch buffers.
    ///
    /// This is the shared body behind [`Segmenter::segment_with_lattice`]
    /// (which passes fresh buffers) and `SegmentWorker::segment` (which
    /// keeps them alive across calls). Output is identical to
    /// `segment_with_lattice` for the same input.
    ///
    /// # 引数
    ///
    /// * `text` - The input text, borrowed or owned.
    /// * `lattice` - The Viterbi lattice to reuse across sentences/calls.
    /// * `buffers` - The scratch buffers; every call overwrites what it
    ///   uses, so no pre-clearing is required.
    ///
    /// # 戻り値
    ///
    /// The tokens segmented from `text`, in reading order.
    pub(crate) fn segment_with_buffers<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
        buffers: &mut SegmentBuffers,
    ) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();

        // Hoisted out of the per-sentence and per-token loops (#942): the
        // lattice options and the classifier of the whitespace tokens to
        // drop. Skipped whitespace never becomes a token, and the lattice
        // returns token ends without it (#1108).
        let options = self.lattice_options();
        let space_filter = self.space_filter();

        let SegmentBuffers {
            offsets,
            exits,
            contexts,
            states,
            next_states,
            tree,
            dominated,
        } = buffers;
        // A previous call that panicked may have left paths behind.
        tree.clear();
        // Whether the current sentence starts a segment, from the
        // dictionary's single BOS edge; otherwise it continues `states`.
        let mut fresh = true;

        for sentence in SentenceWalk::new(&text) {
            let sentence_text = &text[sentence.start..sentence.end];
            // The cost of the cheapest state, which the lattice's costs are
            // relative to.
            let base = if fresh {
                self.set_lattice_text(lattice, sentence_text, &options);
                if !sentence.carries {
                    // A segment of one sentence: the best path from BOS to
                    // EOS, as before the context was carried.
                    lattice.tokens_offset_into(offsets);
                    self.push_tokens(&text, sentence.start, offsets, space_filter, &mut tokens);
                    continue;
                }
                states.clear();
                states.push(CarryState::ROOT);
                0
            } else {
                let base = fill_bos_contexts(
                    states.iter().map(|state| (state.right_id, state.cost)),
                    contexts,
                );
                let mut carried = options;
                carried.bos = contexts;
                self.set_lattice_text(lattice, sentence_text, &carried);
                base
            };

            // Whether the sentence has a complete path: an exit when it
            // carries the context, EOS otherwise.
            let complete = if sentence.carries {
                // Every exit that can win becomes a state: its best path is
                // the path of the state its BOS edge continues, plus this
                // sentence's part. The exits and their paths are read now,
                // before the next sentence rebuilds the lattice.
                lattice.exits_into(exits);
                prune_dominated(
                    exits,
                    &self.exit_margins,
                    &self.dictionary.connection_cost_matrix,
                    dominated,
                );
                if let [exit] = exits.as_slice() {
                    // One state: its path is final, so it is emitted at once.
                    let bos = lattice.exit_tokens_offset_into(exit, offsets);
                    tree.for_each_part(states[bos].node, |part_start, part| {
                        self.push_tokens(&text, part_start, part, space_filter, &mut tokens);
                    });
                    self.push_tokens(&text, sentence.start, offsets, space_filter, &mut tokens);
                    tree.clear();
                    states.clear();
                    states.push(CarryState {
                        right_id: exit.right_id(),
                        cost: base.saturating_add(i64::from(exit.cost())),
                        node: NO_NODE,
                    });
                    fresh = false;
                    continue;
                }
                next_states.clear();
                for exit in exits.iter() {
                    let node = tree.add(sentence.start);
                    let bos = lattice.exit_tokens_offset_into(exit, tree.tokens_mut(node));
                    tree.set_parent(node, states[bos].node);
                    next_states.push(CarryState {
                        right_id: exit.right_id(),
                        cost: base.saturating_add(i64::from(exit.cost())),
                        node,
                    });
                }
                if next_states.is_empty() {
                    false
                } else {
                    std::mem::swap(states, next_states);
                    // The BOS edges of the next sentence follow the order of
                    // the right ids, which decides between paths of equal
                    // cost.
                    states.sort_unstable_by_key(|state| state.right_id);

                    // The parts that every state's path shares are final.
                    let cut = tree.common_ancestor(states);
                    if cut != NO_NODE {
                        tree.for_each_part(cut, |part_start, part| {
                            self.push_tokens(&text, part_start, part, space_filter, &mut tokens);
                        });
                    }
                    tree.retain(states, cut);
                    fresh = false;
                    true
                }
            } else if let Some(bos) = lattice.tokens_offset_into(offsets) {
                // The end of the segment pays the EOS connection; the best
                // path picks the state it continues.
                tree.for_each_part(states[bos].node, |part_start, part| {
                    self.push_tokens(&text, part_start, part, space_filter, &mut tokens);
                });
                self.push_tokens(&text, sentence.start, offsets, space_filter, &mut tokens);
                tree.clear();
                fresh = true;
                true
            } else {
                false
            };

            if !complete {
                // No exit or no complete path, which a dictionary that
                // covers every character with an unknown word never gives:
                // commit the path of the first of the cheapest states, then
                // segment the sentence from the dictionary's BOS edge, as
                // before the context was carried. The next sentence starts
                // a new segment.
                if !fresh {
                    if let Some(best) = states.iter().min_by_key(|state| state.cost) {
                        tree.for_each_part(best.node, |part_start, part| {
                            self.push_tokens(&text, part_start, part, space_filter, &mut tokens);
                        });
                    }
                    self.set_lattice_text(lattice, sentence_text, &options);
                }
                tree.clear();
                lattice.tokens_offset_into(offsets);
                self.push_tokens(&text, sentence.start, offsets, space_filter, &mut tokens);
                fresh = true;
            }
        }

        Ok(tokens)
    }

    /// Segments the input text and returns the top-N segmentation results.
    ///
    /// Each result is a `Vec<Token>` representing one possible segmentation.
    /// Results are ordered by cost (best first).
    /// If `unique` is true, results with the same word boundaries but different
    /// POS tags are deduplicated (only the lowest-cost variant is kept).
    ///
    /// Every result segments the whole input. The input is split into
    /// segments as [`Segmenter::segment`] splits it, and the results are the
    /// `n` cheapest combinations of one path per segment; see
    /// [`Segmenter::segment_nbest_with_lattice`].
    ///
    /// # 引数
    ///
    /// * `text` - The input text, borrowed or owned.
    /// * `n` - The maximum number of results.
    /// * `unique` - Whether to drop results whose word boundaries repeat an
    ///   earlier result's.
    /// * `cost_threshold` - If `Some(t)`, the maximum cost above the first
    ///   result's, measured over the whole input.
    ///
    /// # 戻り値
    ///
    /// The results, best first, each as its tokens and its total cost.
    pub fn segment_nbest<'a>(
        &'a self,
        text: Cow<'a, str>,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> LinderaResult<Vec<(Vec<Token<'a>>, i64)>> {
        let mut lattice = Lattice::default();
        self.segment_nbest_with_lattice(text, &mut lattice, n, unique, cost_threshold)
    }

    /// Segments the input text and returns the top-N segmentation results with costs.
    /// Each result is a (tokens, cost) pair.
    /// If `unique` is true, results with the same word boundaries but different
    /// POS tags are deduplicated (only the lowest-cost variant is kept).
    /// If `cost_threshold` is Some(t), paths whose cost exceeds best_cost + t
    /// are discarded.
    ///
    /// The input is split into sentences and segments by the same rules as
    /// [`Segmenter::segment`]. Within a segment (the sentences up to a `\n`,
    /// a `\t` or the end of the input), the context is carried across the
    /// cuts at `、`, `。` and the forced cuts: a path of a segment costs what
    /// a single lattice over the segment gives it (with the exceptions that
    /// [`Segmenter::segment`] lists), the EOS connection paid only at its
    /// end, and the segment's `n` best paths are searched exactly
    /// (with `unique` over the word boundaries of the whole segment, and the
    /// threshold from the segment's best). Every result is a segmentation of
    /// the whole input that takes one path in each segment, and its cost is
    /// the sum of those paths' costs; the results are the `n` cheapest of
    /// these combinations, in ascending order of cost. `best_cost` is the
    /// cost of the first result, i.e. of the best path of every segment, so
    /// the threshold applies to the whole input. The first result is the
    /// path of [`Segmenter::segment`].
    ///
    /// # 引数
    ///
    /// * `text` - The input text, borrowed or owned.
    /// * `lattice` - The N-best lattice to reuse across sentences and calls.
    /// * `n` - The maximum number of results.
    /// * `unique` - Whether to drop results whose word boundaries repeat an
    ///   earlier result's.
    /// * `cost_threshold` - If `Some(t)`, the maximum cost above the first
    ///   result's.
    ///
    /// # 戻り値
    ///
    /// The results, best first, each as its tokens and its total cost. Empty
    /// if `n` is zero, the input has no sentence, or `cost_threshold` is
    /// negative.
    pub fn segment_nbest_with_lattice<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> LinderaResult<Vec<(Vec<Token<'a>>, i64)>> {
        // A negative threshold excludes even the best result.
        if n == 0 || cost_threshold.is_some_and(|threshold| threshold < 0) {
            return Ok(Vec::new());
        }

        // Phase 1: the N-best paths of every segment, a run of sentences
        // that carry the context from one to the next. A path that exceeds
        // its segment's best by more than the threshold cannot be part of a
        // result, because the other segments add at least their best costs,
        // so the threshold already prunes each segment's list here.
        let options = self.lattice_options();
        let mut parts: Vec<NbestPart> = Vec::new();
        let mut search = CarriedNbest::new(n, unique, cost_threshold);
        // Whether the current sentence starts a segment, from the
        // dictionary's single BOS edge; otherwise it continues `search`.
        let mut fresh = true;
        for sentence in SentenceWalk::new(&text) {
            let sentence_text = &text[sentence.start..sentence.end];
            let first = fresh;
            if first {
                self.set_lattice_text_nbest(lattice, sentence_text, &options);
                if !sentence.carries {
                    // A segment of one sentence: its own N-best list, as
                    // before the context was carried.
                    let paths = lattice.nbest_tokens_offset(n, unique, cost_threshold);
                    // A sentence without paths adds nothing, as in the
                    // 1-best segmentation. A path without tokens (a sentence
                    // of skipped whitespace) is a path, and its cost counts.
                    if !paths.is_empty() {
                        parts.push(NbestPart {
                            start: sentence.start,
                            paths,
                        });
                    }
                    continue;
                }
                search.start(sentence.start);
            } else {
                let mut carried = options;
                carried.bos = search.bos_contexts();
                self.set_lattice_text_nbest(lattice, sentence_text, &carried);
            }

            // Whether the sentence has a complete path: an exit when it
            // carries the context, EOS otherwise.
            let complete = if sentence.carries {
                search.carry(
                    lattice,
                    sentence.start,
                    &self.exit_margins,
                    &self.dictionary.connection_cost_matrix,
                )
            } else {
                let paths = search.finish(lattice, sentence.start);
                let complete = !paths.is_empty();
                if complete {
                    parts.push(NbestPart {
                        start: search.segment_start(),
                        paths,
                    });
                }
                complete
            };
            fresh = !sentence.carries;

            if !complete {
                // No exit or no complete path, which a dictionary that
                // covers every character with an unknown word never gives:
                // as in the 1-best segmentation, end the segment before the
                // sentence, then search the sentence from the dictionary's
                // BOS edge on its own. The next sentence starts a new
                // segment.
                if !first {
                    let committed = search.commit();
                    if !committed.is_empty() {
                        parts.push(NbestPart {
                            start: search.segment_start(),
                            paths: committed,
                        });
                    }
                    self.set_lattice_text_nbest(lattice, sentence_text, &options);
                }
                let paths = lattice.nbest_tokens_offset(n, unique, cost_threshold);
                if !paths.is_empty() {
                    parts.push(NbestPart {
                        start: sentence.start,
                        paths,
                    });
                }
                fresh = true;
            }
        }

        let space_filter = self.space_filter();

        // A single part needs no merge: its list, already cut to `n` and
        // filtered by the threshold, is the result (`merge_nbest` would
        // return it unchanged).
        if let [part] = parts.as_slice() {
            return Ok(part
                .paths
                .iter()
                .map(|(offsets, cost)| {
                    let mut tokens = Vec::new();
                    self.push_tokens(&text, part.start, offsets, space_filter, &mut tokens);
                    (tokens, *cost)
                })
                .collect());
        }

        // Phase 2: the cheapest combinations of one path per part. With
        // `unique`, the paths of each part have distinct word boundaries,
        // so the combinations do too.
        let costs: Vec<Vec<i64>> = parts
            .iter()
            .map(|part| part.paths.iter().map(|&(_, cost)| cost).collect())
            .collect();
        let combinations = merge_nbest(&costs, n, cost_threshold);

        // Phase 3: the tokens of each combination, part by part.
        let mut results = Vec::with_capacity(combinations.len());
        for (ranks, cost) in combinations {
            let mut tokens = Vec::new();
            for (part, rank) in parts.iter().zip(ranks) {
                self.push_tokens(
                    &text,
                    part.start,
                    &part.paths[rank].0,
                    space_filter,
                    &mut tokens,
                );
            }
            results.push((tokens, cost));
        }

        Ok(results)
    }

    /// Appends the tokens of one path to `tokens`, numbering them on from
    /// `tokens.len()`. The shared token builder of the 1-best and the N-best
    /// segmentation. Whitespace is handled as in [`Segmenter::segment`]: the
    /// lattice's token ends leave skipped whitespace out (an entry keeps the
    /// whitespace its own surface ends with), and whitespace tokens are
    /// dropped when `space_filter` is set.
    ///
    /// # 引数
    ///
    /// * `text` - The whole input, which the surfaces borrow from or copy.
    /// * `base` - The offset in `text` that the path's offsets are relative
    ///   to, e.g. the start of its sentence.
    /// * `offsets` - The path's tokens: start and end offsets relative to
    ///   `base` and word ids, in reading order.
    /// * `space_filter` - The classifier of the whitespace tokens to drop,
    ///   from [`Segmenter::space_filter`].
    /// * `tokens` - The tokens of the result so far.
    // `text` stays a `Cow` (not `&str`, as `ptr_arg` suggests) because its
    // variant decides whether a surface borrows the input or copies it.
    #[allow(clippy::ptr_arg)]
    #[inline]
    fn push_tokens<'a>(
        &'a self,
        text: &Cow<'a, str>,
        base: usize,
        offsets: &[TokenOffset],
        space_filter: Option<&WhitespaceClassifier>,
        tokens: &mut Vec<Token<'a>>,
    ) {
        let whole: &str = text;
        tokens.reserve(offsets.len());
        for &(byte_start, byte_end, word_id) in offsets {
            let absolute_start = base + byte_start;
            let absolute_end = base + byte_end;

            // Skip whitespace tokens if keep_whitespace is false (default
            // MeCab behavior).
            if let Some(whitespace) = space_filter
                && self.is_whitespace_token(&whole[absolute_start..absolute_end], whitespace)
            {
                continue;
            }

            // A borrowed input lends the surface; an owned one is copied so
            // the token does not borrow a temporary.
            let surface_cow = match text {
                Cow::Borrowed(s) => Cow::Borrowed(&s[absolute_start..absolute_end]),
                Cow::Owned(s) => Cow::Owned(s[absolute_start..absolute_end].to_owned()),
            };

            tokens.push(Token::new(
                surface_cow,
                absolute_start,
                absolute_end,
                tokens.len(),
                word_id,
                &self.dictionary,
                self.user_dictionary.as_ref(),
            ));
        }
    }
}

/// The N-best paths of one part of the input (a segment: a run of sentences
/// that carry the context from one to the next), kept until the parts are
/// combined (see [`Segmenter::segment_nbest_with_lattice`]).
struct NbestPart {
    /// The part's start in the input, in bytes.
    start: usize,
    /// The paths in ascending order of cost, as `nbest_tokens_offset`
    /// returns them: each its tokens' start and end offsets relative to
    /// `start` and word ids, and its cost.
    paths: Vec<NBestPath>,
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    use crate::segmenter::{MAX_SENTENCE_BYTES, Sentence, SentenceWalk, find_sentence_end};
    #[cfg(feature = "embed-ipadic")]
    use crate::segmenter::{Segmenter, SegmenterConfig};

    #[test]
    fn test_find_sentence_end_no_delimiter_short_text() {
        let text = "hello world";
        let (end, forced) = find_sentence_end(text, 0);
        assert_eq!(end, text.len());
        assert!(!forced);
    }

    #[test]
    fn test_find_sentence_end_newline_delimiter() {
        let text = "hello\nworld";
        let (end, forced) = find_sentence_end(text, 0);
        assert_eq!(end, 6); // includes the '\n'
        assert!(!forced);
    }

    #[test]
    fn test_find_sentence_end_japanese_period_delimiter() {
        let text = "これはテストです。続き";
        let expected_end = text.find('。').unwrap() + '。'.len_utf8();
        let (end, forced) = find_sentence_end(text, 0);
        assert_eq!(end, expected_end);
        assert!(!forced);
    }

    #[test]
    fn test_find_sentence_end_touten_delimiter() {
        let text = "これは、テストです";
        let expected_end = text.find('、').unwrap() + '、'.len_utf8();
        let (end, forced) = find_sentence_end(text, 0);
        assert_eq!(end, expected_end);
        assert!(!forced);
    }

    #[test]
    fn test_find_sentence_end_forces_cut_when_no_delimiter() {
        // Every byte is a char boundary, so the forced cut lands exactly at
        // MAX_SENTENCE_BYTES.
        let text = "a".repeat(MAX_SENTENCE_BYTES * 2);
        let (end, forced) = find_sentence_end(&text, 0);
        assert_eq!(end, MAX_SENTENCE_BYTES);
        assert!(forced);
    }

    #[test]
    fn test_find_sentence_end_forced_cut_respects_char_boundary() {
        // Place a multi-byte character straddling MAX_SENTENCE_BYTES so a
        // naive byte-count cut would slice through it.
        let mut text = "a".repeat(MAX_SENTENCE_BYTES - 1);
        text.push('あ');
        text.push_str(&"a".repeat(100));

        let (end, forced) = find_sentence_end(&text, 0);
        assert!(forced);
        // Panics if `end` is not a valid char boundary.
        let _ = &text[..end];
        assert!(end >= MAX_SENTENCE_BYTES);
        assert!(end <= MAX_SENTENCE_BYTES - 1 + 'あ'.len_utf8());
    }

    /// The walk cuts where `find_sentence_end` cuts; a cut after `、` or `。`
    /// carries the context unless it ends the input, a cut after `\n` or
    /// `\t` never does.
    #[test]
    fn test_sentence_walk_cuts_and_carries() {
        let text = "東京、です。行く\nもも\t京都、\n。";
        let sentences: Vec<(&str, bool)> = SentenceWalk::new(text)
            .map(|sentence| (&text[sentence.start..sentence.end], sentence.carries))
            .collect();
        assert_eq!(
            sentences,
            vec![
                ("東京、", true),
                ("です。", true),
                ("行く\n", false),
                ("もも\t", false),
                ("京都、", true),
                ("\n", false),
                ("。", false),
            ]
        );
        assert_eq!(SentenceWalk::new("").next(), None);
        assert_eq!(
            SentenceWalk::new("東京、").collect::<Vec<_>>(),
            vec![Sentence {
                start: 0,
                end: "東京、".len(),
                carries: false,
            }]
        );
    }

    /// A forced cut carries the context unless it ends the input.
    #[test]
    fn test_sentence_walk_forced_cut_carries() {
        let text = "a".repeat(MAX_SENTENCE_BYTES + 10);
        assert_eq!(
            SentenceWalk::new(&text).collect::<Vec<_>>(),
            vec![
                Sentence {
                    start: 0,
                    end: MAX_SENTENCE_BYTES,
                    carries: true,
                },
                Sentence {
                    start: MAX_SENTENCE_BYTES,
                    end: MAX_SENTENCE_BYTES + 10,
                    carries: false,
                },
            ]
        );

        let text = "a".repeat(MAX_SENTENCE_BYTES);
        assert_eq!(
            SentenceWalk::new(&text).collect::<Vec<_>>(),
            vec![Sentence {
                start: 0,
                end: MAX_SENTENCE_BYTES,
                carries: false,
            }]
        );
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segmenter_config_ipadic_normal() {
        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;

        let result: Result<SegmenterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segmenter_config_ipadic_decompose() {
        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
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

        let result: Result<SegmenterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_ipadic() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "日本語の形態素解析を行うことができます。テスト。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "日本語");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "の");
            assert_eq!(token.byte_start, 9);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "形態素");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 21);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "解析");
            assert_eq!(token.byte_start, 21);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "を");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 30);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "を", "ヲ", "ヲ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "行う");
            assert_eq!(token.byte_start, 30);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "こと");
            assert_eq!(token.byte_start, 36);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "が");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 45);
            assert_eq!(token.position, 7);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "格助詞", "一般", "*", "*", "*", "が", "ガ", "ガ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "でき");
            assert_eq!(token.byte_start, 45);
            assert_eq!(token.byte_end, 51);
            assert_eq!(token.position, 8);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "ます");
            assert_eq!(token.byte_start, 51);
            assert_eq!(token.byte_end, 57);
            assert_eq!(token.position, 9);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 57);
            assert_eq!(token.byte_end, 60);
            assert_eq!(token.position, 10);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "テスト");
            assert_eq!(token.byte_start, 60);
            assert_eq!(token.byte_end, 69);
            assert_eq!(token.position, 11);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 69);
            assert_eq!(token.byte_end, 72);
            assert_eq!(token.position, 12);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_simple_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_simple_userdic_bin_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.bin");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリー",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "カスタム名詞",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "*",
                    "トウキョウスカイツリーエキ",
                    "*"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_detailed_userdic_ipadic() {
        use std::borrow::Cow;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_detailed_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です。",
            ))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "東京スカイツリー");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 24);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "の");
            assert_eq!(token.byte_start, 24);
            assert_eq!(token.byte_end, 27);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "連体化", "*", "*", "*", "*", "の", "ノ", "ノ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "最寄り駅");
            assert_eq!(token.byte_start, 27);
            assert_eq!(token.byte_end, 39);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "は");
            assert_eq!(token.byte_start, 39);
            assert_eq!(token.byte_end, 42);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "とうきょうスカイツリー駅");
            assert_eq!(token.byte_start, 42);
            assert_eq!(token.byte_end, 78);
            assert_eq!(token.position, 4);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "カスタム名詞",
                    "*",
                    "*",
                    "とうきょうスカイツリー駅",
                    "トウキョウスカイツリーエキ",
                    "トウキョウスカイツリーエキ"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "です");
            assert_eq!(token.byte_start, 78);
            assert_eq!(token.byte_end, 84);
            assert_eq!(token.position, 5);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "。");
            assert_eq!(token.byte_start, 84);
            assert_eq!(token.byte_end, 87);
            assert_eq!(token.position, 6);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec!["記号", "句点", "*", "*", "*", "*", "。", "。", "。"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    #[should_panic(expected = "failed to parse word cost")]
    fn test_user_dict_invalid_word_cost() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_userdic_invalid_word_cost.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    #[should_panic(expected = "user dictionary should be a CSV with 3 or 13+ fields")]
    fn test_user_dict_number_of_fields_is_11() {
        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_userdic_insufficient_number_of_fields.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        Segmenter::from_config(&config).unwrap();
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_nomal_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田空港");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
                vec![
                    "名詞",
                    "固有名詞",
                    "一般",
                    "*",
                    "*",
                    "*",
                    "羽田空港",
                    "ハネダクウコウ",
                    "ハネダクーコー"
                ]
            );
        }
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert!(token.word_id.is_unknown());
            assert_eq!(
                token.details(),
                vec!["名詞", "一般", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_decompose_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        });

        let segmenter = Segmenter::from_config(&config).unwrap();
        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "空港");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert!(token.word_id.is_unknown());
            assert_eq!(
                token.details(),
                vec!["名詞", "一般", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_decompose_mode_default_penalty() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "decompose"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();

        let mut tokens = segmenter
            .segment(Cow::Borrowed("羽田空港限定トートバッグ"))
            .unwrap();
        let mut tokens_iter = tokens.iter_mut();
        {
            let token = tokens_iter.next().unwrap();
            assert_eq!(token.surface, "羽田");
            assert_eq!(token.byte_start, 0);
            assert_eq!(token.byte_end, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "空港");
            assert_eq!(token.byte_start, 6);
            assert_eq!(token.byte_end, 12);
            assert_eq!(token.position, 1);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "限定");
            assert_eq!(token.byte_start, 12);
            assert_eq!(token.byte_end, 18);
            assert_eq!(token.position, 2);
            assert_eq!(token.position_length, 1);
            assert_eq!(
                token.details(),
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
            assert_eq!(token.surface, "トートバッグ");
            assert_eq!(token.byte_start, 18);
            assert_eq!(token.byte_end, 36);
            assert_eq!(token.position, 3);
            assert_eq!(token.position_length, 1);
            assert!(token.word_id.is_unknown());
            assert_eq!(
                token.details(),
                vec!["名詞", "一般", "*", "*", "*", "*", "*", "*", "*"]
            );
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_ignores_space() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // Default behavior: should have 2 tokens, space is ignored (MeCab compatible)
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_keep_whitespace() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal",
            "keep_whitespace": true
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // With keep_whitespace=true: should have 3 tokens including space
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, " ");
        assert_eq!(tokens[2].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_with_builder_keep_whitespace() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
        let tokens = segmenter.segment(Cow::Borrowed("東京 都")).unwrap();

        // With keep_whitespace=true: should have 3 tokens including space
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, " ");
        assert_eq!(tokens[2].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_unknown_word_starts_inside_grouped_run() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;

        // #1105: an unknown word may start inside a run of one character
        // category that an earlier position grouped, as in MeCab. The
        // examples of the Issue (a dictionary symbol in front of unknown
        // symbols) and a katakana name joined by `・`. The expected tokens
        // are MeCab's with the same IPADIC, for the 1-best path and for the
        // first N-best path alike.
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let cases: [(&str, &[&str]); 4] = [
            (
                "それが「⁂⁂第一だ",
                &["それ", "が", "「", "⁂⁂", "第", "一", "だ"],
            ),
            (
                "それが…⁂⁂第一だ",
                &["それ", "が", "…", "⁂⁂", "第", "一", "だ"],
            ),
            (
                "それが＝⁂⁂第一だ",
                &["それ", "が", "＝", "⁂⁂", "第", "一", "だ"],
            ),
            (
                "ジョン・レノンが歌う",
                &["ジョン", "・", "レノン", "が", "歌う"],
            ),
        ];
        for (text, expected) in cases {
            let tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
            let surfaces: Vec<&str> = tokens.iter().map(|token| token.surface.as_ref()).collect();
            assert_eq!(surfaces, expected, "{text:?}");

            let results = segmenter
                .segment_nbest(Cow::Borrowed(text), 3, false, None)
                .unwrap();
            let rank1: Vec<&str> = results[0]
                .0
                .iter()
                .map(|token| token.surface.as_ref())
                .collect();
            assert_eq!(rank1, expected, "{text:?} (N-best rank 1)");
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_keeps_latin_capital_eth() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;

        // IPADIC's char.def maps U+00D0 to SPACE and then to ALPHA. The later
        // line wins, as in MeCab, so 'Ð' is a letter and is not dropped as
        // whitespace (#1095).
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

        let tokens = segmenter.segment(Cow::Borrowed("GUÐMUNDUR さん")).unwrap();
        let surfaces: Vec<&str> = tokens.iter().map(|token| token.surface.as_ref()).collect();
        assert_eq!(surfaces, vec!["GUÐMUNDUR", "さん"]);

        let tokens = segmenter.segment(Cow::Borrowed("Ð")).unwrap();
        let surfaces: Vec<&str> = tokens.iter().map(|token| token.surface.as_ref()).collect();
        assert_eq!(surfaces, vec!["Ð"]);
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_dictionary_clone_shares_heavy_fields_via_arc() {
        use std::sync::Arc;

        use crate::dictionary::load_dictionary;

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        assert_eq!(Arc::strong_count(&dictionary.prefix_dictionary), 1);
        assert_eq!(Arc::strong_count(&dictionary.connection_cost_matrix), 1);

        let cloned = dictionary.clone();

        // A cheap (Arc-based) clone bumps the refcount rather than
        // allocating a new copy of the trie/cost matrix.
        assert_eq!(Arc::strong_count(&dictionary.prefix_dictionary), 2);
        assert_eq!(Arc::strong_count(&dictionary.connection_cost_matrix), 2);
        assert!(Arc::ptr_eq(
            &dictionary.prefix_dictionary,
            &cloned.prefix_dictionary
        ));
        assert!(Arc::ptr_eq(
            &dictionary.connection_cost_matrix,
            &cloned.connection_cost_matrix
        ));

        drop(cloned);
        assert_eq!(Arc::strong_count(&dictionary.prefix_dictionary), 1);
        assert_eq!(Arc::strong_count(&dictionary.connection_cost_matrix), 1);
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_from_config_use_mmap_is_ignored_for_embedded_dictionary() {
        // `use_mmap: true` has no effect on an `embedded://` dictionary
        // (the data is already a static, zero-copy byte slice) but must not
        // be treated as an error either.
        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal",
            "use_mmap": true
        }
        "#;

        let config: SegmenterConfig = serde_json::from_str(config_str).unwrap();
        let result = Segmenter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_multiple_spaces() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();
        let tokens = segmenter.segment(Cow::Borrowed("東京   都")).unwrap();

        // Should have 2 tokens: "東京" and "都", multiple spaces are ignored
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_default_leading_trailing() {
        use std::borrow::Cow;

        let config_str = r#"
        {
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        }
        "#;
        let config = serde_json::from_str::<SegmenterConfig>(config_str).unwrap();

        let segmenter = Segmenter::from_config(&config).unwrap();

        // Leading spaces - "   東京都" is segmented as "東京" and "都" (not "東京都")
        let tokens = segmenter.segment(Cow::Borrowed("   東京都")).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");

        // Trailing spaces - "東京都   " is also segmented as "東京" and "都"
        let tokens = segmenter.segment(Cow::Borrowed("東京都   ")).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].surface, "東京");
        assert_eq!(tokens[1].surface, "都");
    }

    /// Whitespace is skipped in the lattice, so a word after a space
    /// connects to the word before it as in MeCab. The expected readings are
    /// MeCab 0.996's with the same IPADIC source; with the whitespace node
    /// (`skip_whitespace(false)`) `が` and `で` read as conjunctions and `都`
    /// as a common noun instead.
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_skips_whitespace_like_mecab_ipadic() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;

        fn render(segmenter: &Segmenter, text: &str) -> Vec<String> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .iter_mut()
                .map(|t| {
                    let pos = t.details()[..2].join("-");
                    format!("{}/{pos}", t.surface)
                })
                .collect()
        }

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let cases = [
            (
                "東京 都に行く",
                "東京/名詞-固有名詞 都/名詞-接尾 に/助詞-格助詞 行く/動詞-自立",
            ),
            (
                "Google が 新しい サービス を 発表した",
                "Google/名詞-固有名詞 が/助詞-格助詞 新しい/形容詞-自立 サービス/名詞-サ変接続 \
                 を/助詞-格助詞 発表/名詞-サ変接続 し/動詞-自立 た/助動詞-*",
            ),
            (
                "私は Python で プログラム を書く",
                "私/名詞-代名詞 は/助詞-係助詞 Python/名詞-一般 で/助詞-格助詞 \
                 プログラム/名詞-サ変接続 を/助詞-格助詞 書く/動詞-自立",
            ),
        ];
        for (text, expected) in cases {
            let expected: Vec<String> = expected.split_whitespace().map(String::from).collect();
            assert_eq!(render(&segmenter, text), expected, "{text}");
        }

        let legacy = segmenter.clone().skip_whitespace(false);
        assert_eq!(
            render(&legacy, "Google が 新しい")[1],
            "が/接続詞-*",
            "the whitespace node reads が as a conjunction"
        );
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_long_text() {
        use std::borrow::Cow;

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

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });

        let segmenter = Segmenter::from_config(&config).unwrap();

        let tokens = segmenter
            .segment(Cow::Borrowed(large_text.as_str()))
            .unwrap();
        assert!(!tokens.is_empty());
    }

    /// Regression test for https://github.com/lindera/lindera/issues/871:
    /// delimiter-free (no `\n`, `\t`, `。`, `、`) mixed-script text used to
    /// build one ever-growing Viterbi lattice whose accumulated path cost
    /// could saturate `i32::MAX`, silently collapsing the remainder of the
    /// input into a single giant token. This asserts that text well past
    /// `MAX_SENTENCE_BYTES` still segments into many small tokens instead of
    /// one oversized one.
    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_bounds_lattice_for_delimiter_free_text() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;
        use crate::segmenter::MAX_SENTENCE_BYTES;

        // Space-joined, mixed Japanese/English words with no real sentence
        // delimiter, comfortably longer than MAX_SENTENCE_BYTES.
        let words = ["test", "検証", "buffer", "実装", "measure", "性能"];
        let mut text = String::new();
        while text.len() < MAX_SENTENCE_BYTES * 2 {
            for w in &words {
                text.push_str(w);
                text.push(' ');
            }
        }

        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokens = segmenter.segment(Cow::Borrowed(text.as_str())).unwrap();

        assert!(!tokens.is_empty());
        let biggest = tokens
            .iter()
            .map(|t| t.byte_end - t.byte_start)
            .max()
            .unwrap();
        assert!(
            biggest < 100,
            "expected only small word tokens, got a {biggest}-byte token; \
             the lattice was not bounded for delimiter-free input"
        );
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_nbest_1best_matches_segment() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });
        let segmenter = Segmenter::from_config(&config).unwrap();

        let text = "すもももももももものうち";

        // 1-best result should match normal segment
        let normal_tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
        let nbest_results = segmenter
            .segment_nbest(Cow::Borrowed(text), 1, false, None)
            .unwrap();

        assert_eq!(nbest_results.len(), 1);
        let (nbest_tokens, _cost) = &nbest_results[0];
        assert_eq!(normal_tokens.len(), nbest_tokens.len());
        for (normal, nbest) in normal_tokens.iter().zip(nbest_tokens.iter()) {
            assert_eq!(normal.surface.as_ref(), nbest.surface.as_ref());
            assert_eq!(normal.byte_start, nbest.byte_start);
            assert_eq!(normal.byte_end, nbest.byte_end);
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_nbest_multiple_results() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });
        let segmenter = Segmenter::from_config(&config).unwrap();

        let text = "すもももももももものうち";
        let results = segmenter
            .segment_nbest(Cow::Borrowed(text), 3, false, None)
            .unwrap();

        // Should return at least 2 different results for this ambiguous text
        assert!(results.len() >= 2);

        // All results should cover the full text
        for (tokens, _cost) in &results {
            assert!(!tokens.is_empty());
            // First token starts at 0
            assert_eq!(tokens[0].byte_start, 0);
            // Last token ends at the end of the text
            let last = tokens.last().unwrap();
            assert_eq!(last.byte_end, text.len());
        }
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_nbest_empty_input() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });
        let segmenter = Segmenter::from_config(&config).unwrap();

        let results = segmenter
            .segment_nbest(Cow::Borrowed(""), 3, false, None)
            .unwrap();
        assert!(results.is_empty() || results.iter().all(|(r, _)| r.is_empty()));
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_nbest_zero_n() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": "normal"
        });
        let segmenter = Segmenter::from_config(&config).unwrap();

        let results = segmenter
            .segment_nbest(Cow::Borrowed("テスト"), 0, false, None)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_segment_nbest_decompose_mode() {
        use std::borrow::Cow;

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "mode": {
                "decompose": {
                    "kanji_penalty_length_threshold": 2,
                    "kanji_penalty_length_penalty": 3000,
                    "other_penalty_length_threshold": 7,
                    "other_penalty_length_penalty": 1700
                }
            }
        });
        let segmenter = Segmenter::from_config(&config).unwrap();

        let text = "関西国際空港限定トートバッグ";
        let results = segmenter
            .segment_nbest(Cow::Borrowed(text), 2, false, None)
            .unwrap();

        // Should get at least 1 result
        assert!(!results.is_empty());
        // All tokens should cover the full text
        for (tokens, _cost) in &results {
            assert!(!tokens.is_empty());
            assert_eq!(tokens[0].byte_start, 0);
            assert_eq!(tokens.last().unwrap().byte_end, text.len());
        }
    }

    /// Left-space penalty (mecab-ko `left-space-penalty-factor`) on ko-dic.
    ///
    /// These tests keep whitespace in the lattice (`skip_whitespace(false)`):
    /// there every space resets the connection context, so the penalty alone
    /// decides readings such as `시/EP` vs `시/NNG` and its effect shows in
    /// isolation. With whitespace skipped (the default) the connection costs
    /// already pick `시/NNG`; `skip_whitespace::test_space_penalty_applies_
    /// when_skipping_whitespace` covers the penalty in that lattice.
    #[cfg(feature = "embed-ko-dic")]
    mod space_penalty {
        use std::borrow::Cow;
        use std::io::Write;

        use lindera_dictionary::dictionary::schema::Schema;
        use lindera_dictionary::viterbi::{LexType, WordId};

        use crate::dictionary::{load_dictionary, load_user_dictionary};
        use crate::mode::{Mode, Penalty};
        use crate::segmenter::{Segmenter, SegmenterConfig};
        use crate::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};

        /// mecab-ko-dic's `left-space-penalty-factor`, expressed by first
        /// POS tag (pos-id.def: 100/200 = E*, 172/230 = VCP, 183-185/220-222
        /// = XS* -> 3000; 120/210 = J* -> 6000).
        fn ko_dic_rules() -> SpacePenaltyConfig {
            SpacePenaltyConfig::new(vec![
                SpacePenaltyRule::new(
                    ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"],
                    3000,
                ),
                SpacePenaltyRule::new(
                    ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"],
                    6000,
                ),
            ])
        }

        fn segmenter(mode: Mode, penalty: bool) -> Segmenter {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            Segmenter::new(mode, dictionary, None)
                .skip_whitespace(false)
                .space_penalty(penalty.then(ko_dic_rules))
                .unwrap()
        }

        /// Renders tokens as `surface/POS` (the first detail field).
        fn render(segmenter: &Segmenter, text: &str) -> Vec<String> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .iter_mut()
                .map(|t| {
                    let pos = t.details()[0].to_string();
                    format!("{}/{}", t.surface, pos)
                })
                .collect()
        }

        /// N-best results rendered like [`render`], paired with their costs.
        fn render_nbest(segmenter: &Segmenter, text: &str, n: usize) -> Vec<(Vec<String>, i64)> {
            segmenter
                .segment_nbest(Cow::Borrowed(text), n, false, None)
                .unwrap()
                .into_iter()
                .map(|(mut tokens, cost)| {
                    let rendered = tokens
                        .iter_mut()
                        .map(|t| {
                            let pos = t.details()[0].to_string();
                            format!("{}/{}", t.surface, pos)
                        })
                        .collect();
                    (rendered, cost)
                })
                .collect()
        }

        fn words(s: &str) -> Vec<String> {
            s.split_whitespace().map(str::to_string).collect()
        }

        /// The segmenter's `SPACE` classifier.
        fn segmenter_whitespace(
            segmenter: &Segmenter,
        ) -> &lindera_dictionary::whitespace::WhitespaceClassifier {
            segmenter.whitespace.as_ref().unwrap()
        }

        /// The penalty flips the particle/ending reading of a token that
        /// follows a space (`시/EP` -> `시/NNG`, `이/VCP` -> a non-VCP tag),
        /// which is what mecab-ko does for these inputs; with the option off
        /// they keep their v6.0.0 output.
        #[test]
        fn test_space_penalty_flips_spaced_particles_and_endings() {
            let off = segmenter(Mode::Normal, false);
            let on = segmenter(Mode::Normal, true);

            assert_eq!(
                render(&off, "서울 시 에서 출발"),
                words("서울/NNP 시/EP 에서/JKB 출발/NNG")
            );
            let penalized = render(&on, "서울 시 에서 출발");
            assert_eq!(penalized[1], "시/NNG", "{penalized:?}");

            assert_eq!(
                render(&off, "검색 이 잘 된다"),
                words("검색/NNG 이/VCP 잘/MAG 된다/VV+EC")
            );
            let penalized = render(&on, "검색 이 잘 된다");
            assert_ne!(penalized[1], "이/VCP", "{penalized:?}");
            assert_eq!(&penalized[2..], &words("잘/MAG 된다/VV+EC")[..]);
        }

        /// Sentences without a space before a particle/ending must not
        /// change, whether or not they contain other spaces.
        #[test]
        fn test_space_penalty_leaves_unspaced_sentences_unchanged() {
            let off = segmenter(Mode::Normal, false);
            let on = segmenter(Mode::Normal, true);

            let cases = [
                (
                    "무궁화꽃이 피었습니다.",
                    "무궁화/NNG 꽃/NNG 이/JKS 피/VV 었/EP 습니다/EF ./SF",
                ),
                (
                    "아버지가방에들어가신다",
                    "아버지/NNG 가/JKS 방/NNG 에/JKB 들어가/VV 신다/EP+EC",
                ),
                (
                    "기계학습을활용한이미지인식",
                    "기계/NNG 학습/NNG 을/JKO 활용/NNG 한/XSV+ETM 이미지/NNG 인식/NNG",
                ),
                (
                    "삼천2백2십삼원",
                    "삼/NR 천/NR 2/SN 백/NR 2/SN 십/NR 삼/NR 원/NNBC",
                ),
                ("서울시에서 출발", "서울시/NNP 에서/JKB 출발/NNG"),
                ("검색이 잘 된다", "검색/NNG 이/JKS 잘/MAG 된다/VV+EC"),
                // A particle at the sentence start has no preceding space.
                ("에서", "에서/JKB"),
            ];
            for (text, expected) in cases {
                assert_eq!(render(&off, text), words(expected), "off: {text}");
                assert_eq!(render(&on, text), words(expected), "on: {text}");
            }
        }

        /// The penalty is applied in Decompose mode too.
        #[test]
        fn test_space_penalty_applies_in_decompose_mode() {
            let mode = Mode::Decompose(Penalty::default());
            let off = render(&segmenter(mode.clone(), false), "서울 시 에서 출발");
            let on = render(&segmenter(mode, true), "서울 시 에서 출발");
            assert_eq!(off[1], "시/EP");
            assert_eq!(on[1], "시/NNG");
        }

        /// N-best costs account for the penalty exactly: a segmentation's
        /// cost rises by the sum of the rule costs of its tokens that follow
        /// a space, and the 1-best path agrees with `segment`.
        #[test]
        fn test_space_penalty_is_reflected_in_nbest_costs() {
            let text = "서울 시 에서 출발";
            let off = segmenter(Mode::Normal, false);
            let on = segmenter(Mode::Normal, true);

            let off_results = render_nbest(&off, text, 20);
            let on_results = render_nbest(&on, text, 20);

            // 1-best agrees with segment().
            assert_eq!(on_results[0].0, render(&on, text));
            assert_eq!(off_results[0].0, render(&off, text));

            // 시/EP (3000, after a space) and 에서/JKB (6000, after a space):
            // +9000 relative to the unpenalized lattice.
            let ep_path = words("서울/NNP 시/EP 에서/JKB 출발/NNG");
            let cost = |results: &[(Vec<String>, i64)], path: &[String]| {
                results
                    .iter()
                    .find(|(p, _)| p == path)
                    .map(|(_, c)| *c)
                    .unwrap_or_else(|| panic!("path {path:?} not in {results:?}"))
            };
            assert_eq!(
                cost(&on_results, &ep_path),
                cost(&off_results, &ep_path) + 9000
            );

            // 시/NNG carries no penalty; 에서/JKB carries 6000.
            let nng_path = words("서울/NNP 시/NNG 에서/JKB 출발/NNG");
            assert_eq!(
                cost(&on_results, &nng_path),
                cost(&off_results, &nng_path) + 6000
            );

            // The N-best list stays sorted by cost.
            assert!(on_results.windows(2).all(|w| w[0].1 <= w[1].1));
        }

        /// The precomputed table covers the system, user and unknown
        /// lexicons and resolves rules by first POS tag.
        #[test]
        fn test_space_penalty_table_covers_all_lexicons() {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();

            let mut csv = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
            writeln!(csv, "테스트조사,JKB,테스트조사").unwrap();
            writeln!(csv, "테스트명사,NNG,테스트명사").unwrap();
            writeln!(csv, "테스트어미,EP,테스트어미").unwrap();
            csv.flush().unwrap();
            let user_dictionary =
                load_user_dictionary(csv.path().to_str().unwrap(), &dictionary.metadata).unwrap();

            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary))
                .space_penalty(Some(ko_dic_rules()))
                .unwrap();
            let table = segmenter.space_penalty_table.as_deref().unwrap();

            // Resolve word ids through segmentation (user rows are re-numbered
            // in sorted order by the builder, so ids are not the CSV order).
            let by_surface = |tokens: &mut Vec<crate::token::Token>, surface: &str| {
                tokens
                    .iter_mut()
                    .find(|t| t.surface == surface)
                    .map(|t| (t.word_id, t.details()[0].to_string()))
                    .unwrap()
            };

            // User entries (the simple-format default cost makes them win).
            for (surface, pos, expected) in [
                ("테스트조사", "JKB", 6000),
                ("테스트명사", "NNG", 0),
                ("테스트어미", "EP", 3000),
            ] {
                let mut tokens = segmenter.segment(Cow::Borrowed(surface)).unwrap();
                let (id, found_pos) = by_surface(&mut tokens, surface);
                assert_eq!(id.lex_type(), LexType::User, "{surface}");
                assert_eq!(found_pos, pos, "{surface}");
                assert_eq!(table.cost(id), expected, "{surface}");
            }

            // System entries.
            let mut tokens = segmenter.segment(Cow::Borrowed("서울시에서")).unwrap();
            let (id, pos) = by_surface(&mut tokens, "에서");
            assert_eq!(id.lex_type(), LexType::System);
            assert_eq!(pos, "JKB");
            assert_eq!(table.cost(id), 6000);
            let (id, pos) = by_surface(&mut tokens, "서울시");
            assert_eq!(pos, "NNP");
            assert_eq!(table.cost(id), 0);

            // Unknown words (ko-dic unk.def has no penalized tag).
            let tokens = segmenter.segment(Cow::Borrowed("쀓쀓")).unwrap();
            assert!(tokens[0].word_id.is_unknown());
            assert_eq!(table.cost(tokens[0].word_id), 0);

            // Out-of-range ids never panic.
            assert_eq!(table.cost(WordId::new(LexType::User, 999)), 0);
            assert_eq!(table.cost(WordId::default()), 0);
        }

        /// `from_config` accepts an object of explicit rules, `true` for the
        /// rules the dictionary ships in its metadata, `false` for off,
        /// leaves the `Segmenter::new` default (the shipped rules) for
        /// `null`/absent, and rejects malformed objects.
        #[test]
        fn test_from_config_space_penalty() {
            let base = serde_json::json!({
                "dictionary": "embedded://ko-dic",
                "mode": "normal",
                "skip_whitespace": false,
            });

            let mut config: SegmenterConfig = base.clone();
            config["space_penalty"] = serde_json::to_value(ko_dic_rules()).unwrap();
            let segmenter = Segmenter::from_config(&config).unwrap();
            assert_eq!(segmenter.space_penalty_config(), Some(&ko_dic_rules()));
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");

            // `false` turns the penalty off: `시` is read as in v6.0 again.
            let mut config = base.clone();
            config["space_penalty"] = serde_json::json!(false);
            let segmenter = Segmenter::from_config(&config).unwrap();
            assert!(segmenter.space_penalty_config().is_none());
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/EP");

            // Absent or `null` keeps the default `Segmenter::new` chose: the
            // rules ko-dic ships in its metadata.json.
            let mut null_config = base.clone();
            null_config["space_penalty"] = serde_json::Value::Null;
            for config in [&base, &null_config] {
                let segmenter = Segmenter::from_config(config).unwrap();
                assert_eq!(segmenter.space_penalty_config(), Some(&ko_dic_rules()));
                assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");
            }

            // `true` takes the rules ko-dic ships in its metadata.json, which
            // are mecab-ko-dic's `left-space-penalty-factor`.
            let mut config = base.clone();
            config["space_penalty"] = serde_json::json!(true);
            let segmenter = Segmenter::from_config(&config).unwrap();
            assert_eq!(segmenter.space_penalty_config(), Some(&ko_dic_rules()));
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");

            let mut config = base.clone();
            config["space_penalty"] = serde_json::json!({"rules": [{"pos": "JKB", "cost": 1}]});
            assert!(Segmenter::from_config(&config).is_err());
        }

        /// `SpacePenaltyTable::is_space` classifies by the dictionary's
        /// `SPACE` category, not by Unicode `White_Space`. The two differ on
        /// ko-dic: U+3000 IDEOGRAPHIC SPACE is `SYMBOL` in its `char.def`, so
        /// it must not trigger the penalty even though `char::is_whitespace`
        /// accepts it. The ASCII fast path must agree with the category
        /// lookup it replaces.
        #[test]
        fn test_space_penalty_whitespace_is_the_space_category() {
            let on = segmenter(Mode::Normal, true);
            let table = on.space_penalty_table.as_deref().unwrap();
            let char_definitions = &on.dictionary.character_definition;

            // ko-dic char.def SPACE: 0x20, 0x09, 0x0A, 0x0B, 0x0D.
            for c in [' ', '\t', '\n', '\u{0B}', '\r'] {
                assert!(table.is_space(c, char_definitions), "{c:?}");
            }
            for c in ['가', 'a', '0', '.', '\u{3000}'] {
                assert!(!table.is_space(c, char_definitions), "{c:?}");
            }
            // U+3000 is Unicode whitespace but not a ko-dic SPACE character.
            assert!('\u{3000}'.is_whitespace());
            // No ko-dic SPACE character lies above U+00FF, so the classifier
            // answers those codepoints without a category lookup.
            let classifier = segmenter_whitespace(&on);
            for c in ['가', '\u{3000}', '\u{2028}', '漢'] {
                assert!(!classifier.is_space(c, char_definitions), "{c:?}");
            }

            // The ASCII fast path must return what the category lookup would.
            let space_id = char_definitions.category_id_by_name("SPACE").unwrap();
            for codepoint in 0..256u32 {
                let c = char::from_u32(codepoint).unwrap();
                assert_eq!(
                    table.is_space(c, char_definitions),
                    char_definitions.lookup_categories(c).contains(&space_id),
                    "U+{codepoint:04X}"
                );
            }

            // And the classification is what actually gates the penalty: an
            // input whose only "whitespace" is U+3000 is analyzed identically
            // with the penalty on and off. (U+3000 surfaces as its own `SY`
            // token rather than being dropped, which is the other half of not
            // being a `SPACE` character.)
            let off = segmenter(Mode::Normal, false);
            assert_eq!(
                render(&on, "서울\u{3000}시"),
                render(&off, "서울\u{3000}시")
            );
        }

        /// `Segmenter::new` applies the rules the dictionary ships (ko-dic),
        /// leaves a dictionary without rules unpenalized, and
        /// `space_penalty(None)` opts out, which reads `시` as in v6.0 again.
        #[test]
        fn test_new_applies_shipped_rules_by_default() {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
            assert_eq!(segmenter.space_penalty_config(), Some(&ko_dic_rules()));
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");

            let off = segmenter.space_penalty(None).unwrap();
            assert!(off.space_penalty_config().is_none());
            assert_eq!(render(&off, "서울 시 에서 출발")[1], "시/EP");

            // A dictionary that ships no rules stays unpenalized.
            let mut dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let mut metadata = (*dictionary.metadata).clone();
            metadata.space_penalty = None;
            dictionary.metadata = std::sync::Arc::new(metadata);
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
            assert!(segmenter.space_penalty_config().is_none());
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/EP");
        }

        /// Shipped rules that cannot be applied (a schema without a
        /// part-of-speech field) must not make `new` fail: it warns and runs
        /// unpenalized.
        #[test]
        fn test_new_tolerates_shipped_rules_it_cannot_apply() {
            let mut dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let mut metadata = (*dictionary.metadata).clone();
            // Rename the part-of-speech column so the lookup table cannot be
            // built; the rules themselves stay in place.
            let fields = metadata
                .dictionary_schema
                .fields
                .iter()
                .map(|field| {
                    if field == "part_of_speech_tag" {
                        "pos".to_string()
                    } else {
                        field.clone()
                    }
                })
                .collect();
            metadata.dictionary_schema = Schema::new(fields);
            dictionary.metadata = std::sync::Arc::new(metadata);

            let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
            assert!(segmenter.space_penalty_config().is_none());
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/EP");
        }

        /// The dictionary-shipped rules are also reachable from the builder,
        /// and a dictionary without rules reports an error rather than
        /// silently running unpenalized.
        #[test]
        fn test_space_penalty_from_dictionary() {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            assert_eq!(
                dictionary.metadata.space_penalty.as_ref(),
                Some(&ko_dic_rules())
            );
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None)
                .space_penalty_from_dictionary()
                .unwrap();
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");

            let mut dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let mut metadata = (*dictionary.metadata).clone();
            metadata.space_penalty = None;
            dictionary.metadata = std::sync::Arc::new(metadata);
            let mut segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            assert!(segmenter.set_space_penalty_from_dictionary().is_err());
            assert!(segmenter.space_penalty_config().is_none());
        }
    }

    /// Jieba and CC-CEDICT carry no connection costs (a 1x1 matrix), so
    /// whitespace skipping cannot change their 1-best output on spaced text.
    #[cfg(any(feature = "embed-jieba", feature = "embed-cc-cedict"))]
    fn assert_skip_whitespace_keeps_output(uri: &str) {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;
        use crate::segmenter::Segmenter;

        let render = |segmenter: &Segmenter, text: &str| -> Vec<(String, usize, usize)> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .into_iter()
                .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                .collect()
        };
        let on = Segmenter::new(Mode::Normal, load_dictionary(uri).unwrap(), None);
        let off = on.clone().skip_whitespace(false);
        for text in [
            "我 喜欢 吃 苹果和香蕉。",
            "北京 是 中华人民共和国 的 首都",
            "  可以 进行   中文 形态学 分析  ",
            "Hello World 你好",
        ] {
            assert_eq!(render(&on, text), render(&off, text), "{uri}: {text}");
        }
    }

    #[test]
    #[cfg(feature = "embed-jieba")]
    fn test_skip_whitespace_keeps_jieba_output() {
        assert_skip_whitespace_keeps_output("embedded://jieba");
    }

    #[test]
    #[cfg(feature = "embed-cc-cedict")]
    fn test_skip_whitespace_keeps_cc_cedict_output() {
        assert_skip_whitespace_keeps_output("embedded://cc-cedict");
    }

    /// SudachiDict keeps whitespace in the lattice by default, as Sudachi
    /// does and as its costs assume: its metadata sets `skip_whitespace` to
    /// `false`. A multi-word entry such as `caramel man` then stays one
    /// token, whereas skipping would split it (the split path no longer pays
    /// for the whitespace node). The config can still turn skipping on.
    #[test]
    #[cfg(feature = "embed-sudachidict")]
    fn test_sudachidict_keeps_whitespace_nodes_by_default() {
        use std::borrow::Cow;

        use crate::dictionary::load_dictionary;
        use crate::mode::Mode;
        use crate::segmenter::{Segmenter, SegmenterConfig};

        let surfaces = |segmenter: &Segmenter, text: &str| -> Vec<String> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .into_iter()
                .map(|t| t.surface.to_string())
                .collect()
        };
        let dictionary = load_dictionary("embedded://sudachidict").unwrap();
        assert_eq!(dictionary.metadata.skip_whitespace, Some(false));
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        assert!(!segmenter.skip_whitespace);

        let text = "昨日caramel manを見た";
        let kept = surfaces(&segmenter, text);
        assert_eq!(kept[1], "caramel man", "{kept:?}");
        let skipping = segmenter.clone().skip_whitespace(true);
        assert_ne!(surfaces(&skipping, text), kept);

        let config: SegmenterConfig = serde_json::json!({
            "dictionary": "embedded://sudachidict",
            "skip_whitespace": true,
        });
        let from_config = Segmenter::from_config(&config).unwrap();
        assert!(from_config.skip_whitespace);
        assert_eq!(surfaces(&from_config, text), surfaces(&skipping, text));
    }

    /// MeCab-compatible whitespace handling on ko-dic: with
    /// `keep_whitespace` false, whitespace is skipped in the lattice and the
    /// words on either side of it connect directly.
    #[cfg(feature = "embed-ko-dic")]
    mod skip_whitespace {
        use std::borrow::Cow;
        use std::collections::HashSet;
        use std::io::Write;

        use lindera_dictionary::viterbi::WordId;

        use crate::dictionary::{load_dictionary, load_user_dictionary};
        use crate::mode::{Mode, Penalty};
        use crate::segmenter::{Segmenter, SegmenterConfig};
        use crate::token::Token;

        /// mecab-ko 0.996-ko-0.9.2 with mecab-ko-dic 2.1.1-20180720, the
        /// source ko-dic is built from: `%m/%f[0]` per morpheme and the path
        /// cost (`%pc` at EOS).
        const MECAB_KO: [(&str, &str, i64); 14] = [
            ("2년 전 대회", "2/SN 년/NNBC 전/NNG 대회/NNG", 6257),
            (
                "전 대통령이 방문했다",
                "전/MM 대통령/NNG 이/JKS 방문/NNG 했/XSV+EP 다/EC",
                547,
            ),
            (
                "전 세계에서 가장 큰",
                "전/MM 세계/NNG 에서/JKB 가장/MAG 큰/VA+ETM",
                -5078,
            ),
            (
                "공부를 하고 있다",
                "공부/NNG 를/JKO 하/VV 고/EC 있/VX 다/EC",
                -7222,
            ),
            ("갈 수 있다", "갈/VV+ETM 수/NNB 있/VV 다/EC", 682),
            ("메일 제목에", "메일/NNG 제목/NNG 에/JKB", 2656),
            (
                "오늘 날씨가 참 좋네요",
                "오늘/MAG 날씨/NNG 가/JKS 참/MAG 좋/VA 네요/EC",
                -155,
            ),
            (
                "부산광역시 해운대구에",
                "부산광역시/NNP 해운대구/NNP 에/JKB",
                5672,
            ),
            ("서울 시 에서", "서울/NNP 시/NNG 에서/JKB", 4166),
            ("검색 이 잘 된다", "검색/NNG 이/IC 잘/MAG 된다/VV+EC", 6820),
            (
                "10년 전에 지어진 건물",
                "10/SN 년/NNBC 전/NNG 에/JKB 지/VV 어/EC 진/VX+ETM 건물/NNG",
                9317,
            ),
            (
                "무궁화꽃이 피었습니다.",
                "무궁화/NNG 꽃/NNG 이/JKS 피/VV 었/EP 습니다/EF ./SF",
                1003,
            ),
            (
                "아버지가방에들어가신다",
                "아버지/NNG 가/JKS 방/NNG 에/JKB 들어가/VV 신다/EP+EC",
                413,
            ),
            (
                "기계학습을활용한이미지인식",
                "기계/NNG 학습/NNG 을/JKO 활용/NNG 한/XSV+ETM 이미지/NNG 인식/NNG",
                -971,
            ),
        ];

        /// Sentences without whitespace.
        const UNSPACED: [&str; 4] = [
            "무궁화꽃이피었습니다.",
            "아버지가방에들어가신다",
            "기계학습을활용한이미지인식",
            "삼천2백2십삼원",
        ];

        /// Inputs exercising whitespace at the sentence edges, runs of it,
        /// the `\t`/`\n` sentence delimiters, whitespace-only text, and
        /// ko-dic entries whose surface ends with a space (`내셔날 `).
        const WHITESPACE_EDGE_CASES: [&str; 18] = [
            "  2년 전 대회",
            "2년 전 대회  ",
            "2년   전   대회",
            "2년 전\t대회",
            "\n2년 전 대회\n",
            "첫 줄\n  둘째 줄 \n\n셋째",
            " \t\n ",
            " ",
            "",
            "내셔날 지오그래픽",
            "내셔날   지오그래픽 ",
            "\u{000B}서울 시\r 에서",
            // Entries that end with a space (#1108).
            "에듀 센터",
            "내셔날  지오그래픽",
            "에듀 ",
            "에듀 \t센터",
            "에듀 \n센터",
            "콜라비 샐러드",
        ];

        /// A ko-dic segmenter with the defaults `Segmenter::new` applies.
        fn segmenter(mode: Mode) -> Segmenter {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            Segmenter::new(mode, dictionary, None)
        }

        /// Renders tokens as `surface/POS` (the first detail field).
        fn render_tokens(tokens: &mut [Token]) -> Vec<String> {
            tokens
                .iter_mut()
                .map(|t| {
                    let pos = t.details()[0].to_string();
                    format!("{}/{}", t.surface, pos)
                })
                .collect()
        }

        /// Segments `text` and renders the tokens.
        fn render(segmenter: &Segmenter, text: &str) -> Vec<String> {
            render_tokens(&mut segmenter.segment(Cow::Borrowed(text)).unwrap())
        }

        /// N-best results rendered like [`render`], paired with their costs.
        fn render_nbest(segmenter: &Segmenter, text: &str, n: usize) -> Vec<(Vec<String>, i64)> {
            segmenter
                .segment_nbest(Cow::Borrowed(text), n, false, None)
                .unwrap()
                .into_iter()
                .map(|(mut tokens, cost)| (render_tokens(&mut tokens), cost))
                .collect()
        }

        fn words(s: &str) -> Vec<String> {
            s.split_whitespace().map(str::to_string).collect()
        }

        /// Whether `c` is in ko-dic's `SPACE` category (`char.def`).
        fn is_ko_dic_space(c: char) -> bool {
            matches!(c, ' ' | '\t' | '\n' | '\u{000B}' | '\r')
        }

        /// Whether `token` is a system dictionary entry with exactly its
        /// surface and word id, or one of `user_entries`, so that whitespace
        /// at its end is the entry's own.
        fn is_dictionary_entry(
            segmenter: &Segmenter,
            token: &Token,
            user_entries: &[&str],
        ) -> bool {
            let surface = token.surface.as_ref();
            user_entries.contains(&surface)
                || segmenter
                    .dictionary
                    .prefix_dictionary
                    .find_surface(surface)
                    .iter()
                    .any(|entry| entry.word_id() == token.word_id)
        }

        /// Every token's offsets address its surface in `text`, tokens are in
        /// order and do not overlap, only whitespace lies between and after
        /// them, no surface starts with whitespace, and a surface ends with
        /// whitespace only when it is a dictionary entry that ends with it
        /// (#1108; `user_entries` lists the user dictionary's surfaces).
        fn assert_offsets(
            segmenter: &Segmenter,
            text: &str,
            tokens: &[Token],
            user_entries: &[&str],
        ) {
            let mut previous_end = 0;
            for token in tokens {
                assert_eq!(
                    &text[token.byte_start..token.byte_end],
                    token.surface,
                    "offsets of {:?} in {text:?}",
                    token.surface
                );
                assert!(!token.surface.is_empty(), "empty token in {text:?}");
                assert!(
                    !token.surface.starts_with(is_ko_dic_space),
                    "whitespace at the start of {:?} in {text:?}",
                    token.surface
                );
                assert!(
                    !token.surface.ends_with(is_ko_dic_space)
                        || is_dictionary_entry(segmenter, token, user_entries),
                    "whitespace at the end of {:?} in {text:?}",
                    token.surface
                );
                assert!(token.byte_start >= previous_end, "overlap in {text:?}");
                assert!(
                    text[previous_end..token.byte_start]
                        .chars()
                        .all(is_ko_dic_space),
                    "text other than whitespace left out before {:?} in {text:?}",
                    token.surface
                );
                previous_end = token.byte_end;
            }
            assert!(
                text[previous_end..].chars().all(is_ko_dic_space),
                "text other than whitespace left out at the end of {text:?}"
            );
        }

        /// The 1-best path is mecab-ko's, whitespace or not.
        #[test]
        fn test_skip_whitespace_matches_mecab_ko() {
            let segmenter = segmenter(Mode::Normal);
            for (text, expected, _) in MECAB_KO {
                assert_eq!(render(&segmenter, text), words(expected), "{text}");
            }
        }

        /// The N-best lattice skips whitespace too: its best path agrees with
        /// `segment` and costs exactly what mecab-ko reports for it.
        #[test]
        fn test_skip_whitespace_nbest_matches_1best_and_mecab_ko_cost() {
            let segmenter = segmenter(Mode::Normal);
            for (text, expected, cost) in MECAB_KO {
                let results = render_nbest(&segmenter, text, 3);
                assert_eq!(results[0].0, render(&segmenter, text), "{text}");
                assert_eq!(results[0].0, words(expected), "{text}");
                assert_eq!(results[0].1, cost, "{text}");
                assert!(results.windows(2).all(|w| w[0].1 <= w[1].1), "{text}");
            }
        }

        /// Text without whitespace comes out the same whether or not
        /// whitespace is skipped.
        #[test]
        fn test_skip_whitespace_leaves_unspaced_sentences_unchanged() {
            let mode = Mode::Normal;
            let on = segmenter(mode.clone());
            let off = segmenter(mode).skip_whitespace(false);
            for text in UNSPACED {
                assert_eq!(render(&on, text), render(&off, text), "{text}");
                assert_eq!(
                    render_nbest(&on, text, 5),
                    render_nbest(&off, text, 5),
                    "{text}"
                );
            }
        }

        /// Turning skipping off restores the whitespace nodes and with them
        /// the pre-7.1 output, through the builder, the config and the
        /// worker.
        #[test]
        fn test_skip_whitespace_can_be_turned_off() {
            let text = "2년 전 대회";
            let legacy = words("2/SN 년/NNBC 전/NP+JX 대회/NNG");

            let builder = segmenter(Mode::Normal).skip_whitespace(false);
            assert!(!builder.skip_whitespace);
            assert_eq!(render(&builder, text), legacy);

            let config: SegmenterConfig = serde_json::from_str(
                r#"{ "dictionary": "embedded://ko-dic", "skip_whitespace": false }"#,
            )
            .unwrap();
            let from_config = Segmenter::from_config(&config).unwrap();
            assert_eq!(render(&from_config, text), legacy);

            let mut worker = segmenter(Mode::Normal).into_worker();
            worker.set_skip_whitespace(false);
            let tokens = render_tokens(&mut worker.segment(text).unwrap());
            assert_eq!(tokens, legacy);
            worker.set_skip_whitespace(true);
            let tokens = render_tokens(&mut worker.segment(text).unwrap());
            assert_eq!(tokens, words("2/SN 년/NNBC 전/NNG 대회/NNG"));
        }

        /// `Segmenter::new` takes the default from the dictionary's metadata:
        /// absent or `true` skips, `false` keeps whitespace nodes, and the
        /// builder overrides it either way.
        #[test]
        fn test_skip_whitespace_default_comes_from_metadata() {
            let text = "2년 전 대회";
            let skipped = words("2/SN 년/NNBC 전/NNG 대회/NNG");
            let legacy = words("2/SN 년/NNBC 전/NP+JX 대회/NNG");
            let with_metadata = |value: Option<bool>| {
                let mut dictionary = load_dictionary("embedded://ko-dic").unwrap();
                let mut metadata = (*dictionary.metadata).clone();
                metadata.skip_whitespace = value;
                dictionary.metadata = std::sync::Arc::new(metadata);
                Segmenter::new(Mode::Normal, dictionary, None)
            };

            assert_eq!(
                load_dictionary("embedded://ko-dic")
                    .unwrap()
                    .metadata
                    .skip_whitespace,
                None
            );
            for (value, expected) in [
                (None, &skipped),
                (Some(true), &skipped),
                (Some(false), &legacy),
            ] {
                let segmenter = with_metadata(value);
                assert_eq!(segmenter.skip_whitespace, value.unwrap_or(true));
                assert_eq!(&render(&segmenter, text), expected, "{value:?}");
            }
            let overridden = with_metadata(Some(false)).skip_whitespace(true);
            assert_eq!(render(&overridden, text), skipped);
        }

        /// In the config, `skip_whitespace` absent or `null` keeps the
        /// dictionary's default, a bool overrides it, and anything else is
        /// rejected.
        #[test]
        fn test_from_config_skip_whitespace() {
            let text = "2년 전 대회";
            let build = |value: Option<serde_json::Value>| {
                let mut config = serde_json::json!({ "dictionary": "embedded://ko-dic" });
                if let Some(value) = value {
                    config["skip_whitespace"] = value;
                }
                Segmenter::from_config(&config)
            };
            for value in [
                None,
                Some(serde_json::Value::Null),
                Some(serde_json::json!(true)),
            ] {
                let segmenter = build(value.clone()).unwrap();
                assert!(segmenter.skip_whitespace, "{value:?}");
                assert_eq!(render(&segmenter, text)[2], "전/NNG");
            }
            let off = build(Some(serde_json::json!(false))).unwrap();
            assert!(!off.skip_whitespace);
            assert_eq!(render(&off, text)[2], "전/NP+JX");
            assert!(build(Some(serde_json::json!("false"))).is_err());
        }

        /// `keep_whitespace` keeps whitespace in the lattice, so whitespace
        /// tokens still come out and the tokens still tile the input.
        #[test]
        fn test_keep_whitespace_keeps_whitespace_nodes() {
            let segmenter = segmenter(Mode::Normal).keep_whitespace(true);
            assert_eq!(
                render(&segmenter, "2년 전 대회"),
                ["2/SN", "년/NNBC", " /SP", "전/NP+JX", " /SP", "대회/NNG"]
            );
            for text in WHITESPACE_EDGE_CASES {
                let tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
                let rebuilt: String = tokens.iter().map(|t| t.surface.as_ref()).collect();
                assert_eq!(rebuilt, text);
                let mut expected_start = 0;
                for token in &tokens {
                    assert_eq!(token.byte_start, expected_start, "{text:?}");
                    assert_eq!(&text[token.byte_start..token.byte_end], token.surface);
                    expected_start = token.byte_end;
                }
            }
        }

        /// Whitespace at the sentence edges or in runs neither changes the
        /// analysis nor leaks into token surfaces and offsets, in the 1-best
        /// and N-best paths and in both modes.
        #[test]
        fn test_skip_whitespace_offsets_and_edges() {
            for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
                let segmenter = segmenter(mode);
                let reference = render(&segmenter, "2년 전 대회");
                for text in WHITESPACE_EDGE_CASES {
                    let tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
                    assert_offsets(&segmenter, text, &tokens, &[]);
                    for (position, token) in tokens.iter().enumerate() {
                        assert_eq!(token.position, position, "{text:?}");
                    }
                    let owned = segmenter.segment(Cow::Owned(text.to_string())).unwrap();
                    assert_offsets(&segmenter, text, &owned, &[]);
                    assert_eq!(owned.len(), tokens.len());

                    for (tokens, _) in segmenter
                        .segment_nbest(Cow::Borrowed(text), 3, false, None)
                        .unwrap()
                    {
                        assert_offsets(&segmenter, text, &tokens, &[]);
                    }
                }
                for text in [
                    "  2년 전 대회",
                    "2년 전 대회  ",
                    "2년   전   대회",
                    "\n2년 전 대회\n",
                ] {
                    assert_eq!(render(&segmenter, text), reference, "{text:?}");
                }
                let tokens = segmenter.segment(Cow::Borrowed("  2년 전 대회")).unwrap();
                assert_eq!((tokens[0].byte_start, tokens[0].byte_end), (2, 3));
                for text in [" \t\n ", " ", ""] {
                    assert!(segmenter.segment(Cow::Borrowed(text)).unwrap().is_empty());
                }
            }
        }

        /// The length of a whitespace run costs nothing: neither as a node
        /// nor through the Decompose length penalty, which measures a word
        /// without the skipped whitespace after it.
        #[test]
        fn test_whitespace_run_length_does_not_change_costs() {
            let one = "부산광역시 해운대구에";
            let many = "부산광역시     해운대구에";
            for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
                let segmenter = segmenter(mode);
                let one_results = render_nbest(&segmenter, one, 5);
                let many_results = render_nbest(&segmenter, many, 5);
                assert_eq!(one_results, many_results);
            }
            assert_eq!(render_nbest(&segmenter(Mode::Normal), many, 1)[0].1, 5672);
        }

        /// The left-space penalty still keys on the character before a
        /// candidate, so it applies when whitespace is skipped: `이` after a
        /// space is not read as a particle (mecab-ko: `이/IC`).
        #[test]
        fn test_space_penalty_applies_when_skipping_whitespace() {
            let text = "검색 이 잘 된다";
            let on = segmenter(Mode::Normal);
            let off = segmenter(Mode::Normal).space_penalty(None).unwrap();
            assert_eq!(render(&on, text), words("검색/NNG 이/IC 잘/MAG 된다/VV+EC"));
            assert_eq!(
                render(&off, text),
                words("검색/NNG 이/JKS 잘/MAG 된다/VV+EC")
            );

            // In the N-best lattice the particle path costs JX/JKS's 6000 more.
            let jks = words("검색/NNG 이/JKS 잘/MAG 된다/VV+EC");
            let cost = |segmenter: &Segmenter| {
                render_nbest(segmenter, text, 20)
                    .into_iter()
                    .find(|(path, _)| *path == jks)
                    .map(|(_, cost)| cost)
                    .unwrap()
            };
            assert_eq!(cost(&on), cost(&off) + 6000);
        }

        /// A user-dictionary entry may contain a space: it is matched across
        /// the whitespace and keeps it in its surface.
        #[test]
        fn test_skip_whitespace_user_entry_with_inner_space() {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let mut csv = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
            writeln!(csv, "해운대 해수욕장,NNP,해운대해수욕장").unwrap();
            csv.flush().unwrap();
            let user_dictionary =
                load_user_dictionary(csv.path().to_str().unwrap(), &dictionary.metadata).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

            let text = "해운대 해수욕장에 갔다";
            let tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
            assert_eq!(tokens[0].surface, "해운대 해수욕장");
            assert_offsets(&segmenter, text, &tokens, &["해운대 해수욕장"]);
        }

        /// The surface, offsets and word id of each token.
        fn spans(tokens: &[Token]) -> Vec<(String, usize, usize, WordId)> {
            tokens
                .iter()
                .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end, t.word_id))
                .collect()
        }

        /// The surface and offsets of each token of `text`.
        fn segment_spans(segmenter: &Segmenter, text: &str) -> Vec<(String, usize, usize)> {
            spans(&segmenter.segment(Cow::Borrowed(text)).unwrap())
                .into_iter()
                .map(|(surface, start, end, _)| (surface, start, end))
                .collect()
        }

        /// A ko-dic segmenter with a user dictionary of `NNP` entries in the
        /// simple CSV format.
        fn segmenter_with_user_entries(mode: Mode, surfaces: &[&str]) -> Segmenter {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let mut csv = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
            for surface in surfaces {
                writeln!(csv, "{surface},NNP,{}", surface.trim()).unwrap();
            }
            csv.flush().unwrap();
            let user_dictionary =
                load_user_dictionary(csv.path().to_str().unwrap(), &dictionary.metadata).unwrap();
            Segmenter::new(mode, dictionary, Some(user_dictionary))
        }

        /// An entry whose surface ends with a space keeps it in its token, as
        /// in mecab-ko, and only the whitespace after it is skipped (#1108).
        #[test]
        fn test_entry_keeps_its_trailing_whitespace() {
            let segmenter = segmenter(Mode::Normal);
            let span = |surface: &str, start, end| (surface.to_string(), start, end);

            let tokens = segment_spans(&segmenter, "에듀 센터");
            assert_eq!(tokens, [span("에듀 ", 0, 7), span("센터", 7, 13)]);
            assert_eq!(render(&segmenter, "에듀 센터")[0], "에듀 /NNG");

            let tokens = segment_spans(&segmenter, "내셔날  지오그래픽");
            assert_eq!(tokens[0], span("내셔날 ", 0, 10));
            assert_eq!(tokens[1].1, 11);

            // With the default left-space penalty as well.
            assert_eq!(
                segment_spans(&segmenter, "콜라비 샐러드")[0],
                span("콜라비 ", 0, 10)
            );

            // At the end of the text and before a sentence delimiter.
            for text in ["에듀 ", "에듀   ", "에듀 \t센터", "에듀 \n센터"] {
                assert_eq!(
                    segment_spans(&segmenter, text)[0],
                    span("에듀 ", 0, 7),
                    "{text:?}"
                );
            }
        }

        /// N-best results keep an entry's whitespace too, and `unique` tells
        /// apart results that differ only in it: `에듀` and a space is NNG,
        /// `에듀` alone NNP.
        #[test]
        fn test_entry_trailing_whitespace_in_nbest() {
            let segmenter = segmenter(Mode::Normal);
            let text = "에듀 센터";

            let results = segmenter
                .segment_nbest(Cow::Borrowed(text), 3, false, None)
                .unwrap();
            assert!(results.windows(2).all(|pair| pair[0].1 <= pair[1].1));
            let best = segmenter.segment(Cow::Borrowed(text)).unwrap();
            assert_eq!(spans(&results[0].0), spans(&best));
            for (tokens, _) in &results {
                assert_offsets(&segmenter, text, tokens, &[]);
            }

            let unique = segmenter
                .segment_nbest(Cow::Borrowed(text), 3, true, None)
                .unwrap();
            let boundaries: HashSet<Vec<(usize, usize)>> = unique
                .iter()
                .map(|(tokens, _)| tokens.iter().map(|t| (t.byte_start, t.byte_end)).collect())
                .collect();
            assert_eq!(boundaries.len(), unique.len());
            let first_words: Vec<String> = unique
                .into_iter()
                .map(|(mut tokens, _)| render_tokens(&mut tokens).swap_remove(0))
                .collect();
            assert!(
                first_words.contains(&"에듀 /NNG".to_string()),
                "{first_words:?}"
            );
            assert!(
                first_words.contains(&"에듀/NNP".to_string()),
                "{first_words:?}"
            );
        }

        /// A user entry that ends with whitespace keeps all of it, however
        /// much whitespace follows, and matches only where the text has it,
        /// with whitespace skipped or kept in the lattice.
        #[test]
        fn test_user_entry_keeps_its_trailing_whitespace() {
            let entry = "Foo  ";
            let segmenter = segmenter_with_user_entries(Mode::Normal, &[entry]);
            let span = |surface: &str, start, end| (surface.to_string(), start, end);

            for (text, expected) in [
                ("Foo   Bar", vec![span(entry, 0, 5), span("Bar", 6, 9)]),
                ("Foo  Bar", vec![span(entry, 0, 5), span("Bar", 5, 8)]),
                ("Foo  ", vec![span(entry, 0, 5)]),
                ("Foo   ", vec![span(entry, 0, 5)]),
                ("Foo  \tBar", vec![span(entry, 0, 5), span("Bar", 6, 9)]),
                ("Foo  \nBar", vec![span(entry, 0, 5), span("Bar", 6, 9)]),
            ] {
                assert_eq!(segment_spans(&segmenter, text), expected, "{text:?}");
                let tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
                assert_offsets(&segmenter, text, &tokens, &[entry]);
            }
            assert_eq!(segment_spans(&segmenter, "Foo Bar")[0], span("Foo", 0, 3));

            // With whitespace kept in the lattice the entry's token is the
            // same (a longer run would add a `SPACE` node after it, whose
            // costs decide the path instead).
            for segmenter in [
                segmenter_with_user_entries(Mode::Normal, &[entry]).skip_whitespace(false),
                segmenter_with_user_entries(Mode::Normal, &[entry]).keep_whitespace(true),
            ] {
                assert_eq!(segment_spans(&segmenter, "Foo  Bar")[0], span(entry, 0, 5));
            }
        }

        /// The Decompose length penalty counts the whitespace an entry ends
        /// with, as its token shows it, but not the whitespace skipped after
        /// it: a 10-character user entry (`Foo` and 7 spaces) costs
        /// (10 - 7) * 1700 = 5100 more in Decompose mode on the same path,
        /// however many spaces follow it.
        #[test]
        fn test_decompose_penalty_counts_the_entry_whitespace() {
            let entry = "Foo       ";
            assert_eq!(entry.chars().count(), 10);
            let normal = segmenter_with_user_entries(Mode::Normal, &[entry]);
            let decompose =
                segmenter_with_user_entries(Mode::Decompose(Penalty::default()), &[entry]);
            let results = |segmenter: &Segmenter, text: &str| {
                segmenter
                    .segment_nbest(Cow::Borrowed(text), 10, false, None)
                    .unwrap()
                    .iter()
                    .map(|(tokens, cost)| (spans(tokens), *cost))
                    .collect::<Vec<_>>()
            };
            for spaces in [8, 11] {
                let text = format!("Foo{}Bar", " ".repeat(spaces));
                let normal_results = results(&normal, &text);
                let (path, normal_cost) = &normal_results[0];
                assert_eq!(path[0].0, entry, "{text:?}");
                let decompose_results = results(&decompose, &text);
                let Some((_, decompose_cost)) =
                    decompose_results.iter().find(|(tokens, _)| tokens == path)
                else {
                    panic!(
                        "the path through the entry is not in the Decompose results of {text:?}"
                    );
                };
                assert_eq!(decompose_cost - normal_cost, 5100, "{text:?}");
            }
        }

        /// With whitespace kept in the lattice, turning skipping off gives
        /// the tokens of `keep_whitespace(true)` without its whitespace
        /// tokens: both read the same lattice.
        #[test]
        fn test_whitespace_in_lattice_settings_agree() {
            let no_skip = segmenter(Mode::Normal).skip_whitespace(false);
            let keep = segmenter(Mode::Normal).keep_whitespace(true);
            for text in WHITESPACE_EDGE_CASES {
                let without = spans(&no_skip.segment(Cow::Borrowed(text)).unwrap());
                let kept: Vec<_> = spans(&keep.segment(Cow::Borrowed(text)).unwrap())
                    .into_iter()
                    .filter(|(surface, ..)| !surface.chars().all(is_ko_dic_space))
                    .collect();
                assert_eq!(without, kept, "{text:?}");
            }
        }
    }

    /// A sentence without a complete path (a character no word or unknown
    /// word covers) inside a segment: the segment ends before it, and the
    /// sentence is segmented on its own, as before the context was carried.
    mod carried_context_fallback {
        use std::borrow::Cow;
        use std::fs;

        use crate::dictionary::{DictionaryBuilder, Metadata, load_fs_dictionary};
        use crate::mode::Mode;
        use crate::segmenter::{Segmenter, SentenceWalk};

        /// `あ` is HIRAGANA, which has no unknown-word entry.
        const CHAR_DEF: &str = "\
DEFAULT 0 1 0
HIRAGANA 0 1 0
KANJI 0 0 2
0x3041..0x309F HIRAGANA
0x4E00..0x9FFF KANJI
";
        const UNK_DEF: &str = "\
DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*
KANJI,0,0,10000,名詞,一般,*,*,*,*,*,*,*
";
        const LEX_CSV: &str = "\
東京,0,0,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
";
        const MATRIX_DEF: &str = "\
1 1
0 0 0
";

        fn segmenter() -> (tempfile::TempDir, Segmenter) {
            let source = tempfile::tempdir().unwrap();
            fs::write(source.path().join("char.def"), CHAR_DEF).unwrap();
            fs::write(source.path().join("unk.def"), UNK_DEF).unwrap();
            fs::write(source.path().join("lex.csv"), LEX_CSV).unwrap();
            fs::write(source.path().join("matrix.def"), MATRIX_DEF).unwrap();
            let output = tempfile::tempdir().unwrap();
            DictionaryBuilder::new(Metadata::default())
                .build_dictionary(source.path(), output.path())
                .unwrap();
            let dictionary = load_fs_dictionary(output.path()).unwrap();
            (output, Segmenter::new(Mode::Normal, dictionary, None))
        }

        fn spans(segmenter: &Segmenter, text: &str) -> Vec<(String, usize, usize)> {
            segmenter
                .segment(Cow::Borrowed(text))
                .unwrap()
                .iter()
                .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                .collect()
        }

        /// Every sentence on its own, as before the context was carried.
        fn per_sentence(segmenter: &Segmenter, text: &str) -> Vec<(String, usize, usize)> {
            let mut out = Vec::new();
            for sentence in SentenceWalk::new(text) {
                for (surface, start, end) in spans(segmenter, &text[sentence.start..sentence.end]) {
                    out.push((surface, start + sentence.start, end + sentence.start));
                }
            }
            out
        }

        #[test]
        fn test_sentence_without_path_ends_the_segment() {
            let (_dir, segmenter) = segmenter();
            // The connection costs are all 0, so carrying the context
            // changes nothing but the costs of EOS, which are 0 as well.
            for text in [
                "東京、あ、東京",
                "東京、東京、あ",
                "あ、東京",
                "東京。あ。東京、東京",
            ] {
                let tokens = spans(&segmenter, text);
                assert_eq!(tokens, per_sentence(&segmenter, text), "{text:?}");
                assert!(!tokens.iter().any(|(surface, ..)| surface.contains('あ')));

                // The N-best search drops the sentence, as it drops a
                // sentence without paths on its own.
                let results = segmenter
                    .segment_nbest(Cow::Borrowed(text), 3, false, None)
                    .unwrap();
                assert!(!results.is_empty(), "{text:?}");
                for (result, _) in &results {
                    let result: Vec<(String, usize, usize)> = result
                        .iter()
                        .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                        .collect();
                    assert_eq!(result, tokens, "{text:?}");
                }
            }
        }
    }

    /// The context carried across `、`, `。` and forced cuts (#1096). A
    /// segment (the sentences up to `\n`, `\t` or the end of the input) costs
    /// what one lattice over it gives: the EOS connection is paid only at the
    /// end of the segment, and a sentence continues the exits of the previous
    /// one. Segments are independent, and the N-best results are the
    /// cheapest combinations of one path per segment (#1097).
    #[cfg(feature = "embed-ipadic")]
    mod carried_context {
        use std::borrow::Cow;
        use std::collections::HashMap;
        use std::path::PathBuf;

        use lindera_dictionary::nbest::NBestGenerator;
        use lindera_dictionary::viterbi::{BosContext, Lattice, TokenOffset, WordId};

        use crate::dictionary::load_dictionary;
        use crate::mode::{Mode, Penalty};
        use crate::segmenter::{
            MAX_SENTENCE_BYTES, SegmentBuffers, Segmenter, Sentence, SentenceWalk,
        };
        use crate::token::Token;

        /// The fields of a token that the tests compare: surface, byte
        /// offsets, position and word id.
        type Flat = (String, usize, usize, usize, WordId);

        /// N-best results with their tokens flattened to [`Flat`].
        type Results = Vec<(Vec<Flat>, i64)>;

        /// The expected results: per result (per word-boundary class with
        /// `unique`), its cost and the token lists that may stand for it.
        type Expected = Vec<(i64, Vec<Vec<Flat>>)>;

        fn ipadic(mode: Mode) -> Segmenter {
            Segmenter::new(mode, load_dictionary("embedded://ipadic").unwrap(), None)
        }

        fn flatten(tokens: &[Token]) -> Vec<Flat> {
            tokens
                .iter()
                .map(|t| {
                    (
                        t.surface.to_string(),
                        t.byte_start,
                        t.byte_end,
                        t.position,
                        t.word_id,
                    )
                })
                .collect()
        }

        fn segment(segmenter: &Segmenter, text: &str) -> Vec<Flat> {
            flatten(&segmenter.segment(Cow::Borrowed(text)).unwrap())
        }

        fn nbest(
            segmenter: &Segmenter,
            text: &str,
            n: usize,
            unique: bool,
            threshold: Option<i64>,
        ) -> Results {
            segmenter
                .segment_nbest(Cow::Borrowed(text), n, unique, threshold)
                .unwrap()
                .iter()
                .map(|(tokens, cost)| (flatten(tokens), *cost))
                .collect()
        }

        /// The tokens of a path whose offsets are relative to `base`, built
        /// as the segmenter builds them.
        fn build(
            segmenter: &Segmenter,
            text: &str,
            base: usize,
            offsets: &[TokenOffset],
        ) -> Vec<Flat> {
            let text = Cow::Borrowed(text);
            let mut tokens = Vec::new();
            segmenter.push_tokens(&text, base, offsets, segmenter.space_filter(), &mut tokens);
            flatten(&tokens)
        }

        fn boundaries(tokens: &[Flat]) -> Vec<(usize, usize)> {
            tokens.iter().map(|t| (t.1, t.2)).collect()
        }

        /// The best path of `text` as one lattice, without any cut.
        fn unsplit_segment(segmenter: &Segmenter, text: &str) -> Vec<Flat> {
            let mut lattice = Lattice::default();
            segmenter.set_lattice_text(&mut lattice, text, &segmenter.lattice_options());
            build(segmenter, text, 0, &lattice.tokens_offset())
        }

        /// The N-best paths of `text` as one lattice, without any cut, as
        /// expected results (one token list each).
        fn unsplit_nbest(
            segmenter: &Segmenter,
            text: &str,
            n: usize,
            unique: bool,
            threshold: Option<i64>,
        ) -> Expected {
            let mut lattice = Lattice::default();
            segmenter.set_lattice_text_nbest(&mut lattice, text, &segmenter.lattice_options());
            lattice
                .nbest_tokens_offset(n, unique, threshold)
                .iter()
                .map(|(offsets, cost)| (*cost, vec![build(segmenter, text, 0, offsets)]))
                .collect()
        }

        /// Asserts that `actual` is the top `n` of `expected`: the same
        /// costs as its first `n` entries, every result one of the token
        /// lists of a distinct entry of its cost (so ties may come in any
        /// order), and, with `unique`, distinct word boundaries. With
        /// `boundaries_only`, results are matched by their word boundaries
        /// (for the unsplit lattice, which keeps one variant per boundary
        /// class and may break ties between variants differently).
        fn assert_top_n(
            actual: &Results,
            expected: &Expected,
            n: usize,
            unique: bool,
            boundaries_only: bool,
            context: &str,
        ) {
            let costs: Vec<i64> = actual.iter().map(|(_, cost)| *cost).collect();
            let expected_costs: Vec<i64> = expected.iter().take(n).map(|(cost, _)| *cost).collect();
            assert_eq!(costs, expected_costs, "costs of {context}");
            let mut used = vec![false; expected.len()];
            for (tokens, cost) in actual {
                let found = expected.iter().enumerate().position(|(i, (c, lists))| {
                    !used[i]
                        && c == cost
                        && lists.iter().any(|list| {
                            if boundaries_only {
                                boundaries(list) == boundaries(tokens)
                            } else {
                                list == tokens
                            }
                        })
                });
                let Some(i) = found else {
                    panic!("{context}: unexpected result {tokens:?} at cost {cost}");
                };
                used[i] = true;
            }
            if unique {
                let mut seen = std::collections::HashSet::new();
                for (tokens, _) in actual {
                    assert!(
                        seen.insert(boundaries(tokens)),
                        "{context}: repeated boundaries"
                    );
                }
            }
        }

        /// Every result covers the whole input except the dropped
        /// whitespace, with consistent offsets and positions, and the costs
        /// do not decrease.
        fn assert_whole_input(segmenter: &Segmenter, text: &str, results: &Results) {
            let keep_whitespace = segmenter.space_filter().is_none();
            let expected: String = text
                .chars()
                .filter(|c| keep_whitespace || !matches!(c, ' ' | '\t' | '\n' | '\u{0B}'))
                .collect();
            for (tokens, _) in results {
                let joined: String = tokens.iter().map(|t| t.0.as_str()).collect();
                assert_eq!(joined, expected);
                for (i, (surface, start, end, position, _)) in tokens.iter().enumerate() {
                    assert_eq!(&text[*start..*end], surface);
                    assert_eq!(*position, i);
                }
            }
            assert!(results.windows(2).all(|pair| pair[0].1 <= pair[1].1));
        }

        /// One path of a sentence from one BOS context: its tokens (offsets
        /// in the input), its cost (from a BOS edge of cost 0) and the right
        /// id of its exit (0 after EOS).
        type SentencePath = (Vec<TokenOffset>, i64, u16);

        /// Every path of `sentence` from the BOS context `bos` (`None`: the
        /// dictionary's BOS edge): to every exit without EOS when the cut
        /// after it carries the context, through EOS otherwise.
        fn sentence_paths(
            segmenter: &Segmenter,
            text: &str,
            sentence: Sentence,
            bos: Option<u16>,
        ) -> Vec<SentencePath> {
            let contexts: Vec<BosContext> = bos
                .map(|right_id| BosContext { right_id, cost: 0 })
                .into_iter()
                .collect();
            let mut options = segmenter.lattice_options();
            options.bos = &contexts;
            let mut lattice = Lattice::default();
            segmenter.set_lattice_text_nbest(
                &mut lattice,
                &text[sentence.start..sentence.end],
                &options,
            );
            let shift = |offsets: Vec<TokenOffset>| -> Vec<TokenOffset> {
                offsets
                    .into_iter()
                    .map(|(start, end, word_id)| {
                        (start + sentence.start, end + sentence.start, word_id)
                    })
                    .collect()
            };
            let mut paths = Vec::new();
            if sentence.carries {
                let mut exits = Vec::new();
                lattice.exits_into(&mut exits);
                for exit in &exits {
                    let mut generator = NBestGenerator::from_exit(&lattice, exit);
                    while let Some(((offsets, cost), _)) = generator.next_with_bos() {
                        paths.push((shift(offsets), cost, exit.right_id()));
                    }
                }
            } else {
                let mut generator = NBestGenerator::new(&lattice);
                while let Some(((offsets, cost), _)) = generator.next_with_bos() {
                    paths.push((shift(offsets), cost, 0));
                }
            }
            paths
        }

        /// Every path of a segment by definition: the first sentence starts
        /// from the dictionary's BOS edge, every other one from the exit of
        /// the previous sentence's path, and only the last pays EOS.
        fn segment_paths(
            segmenter: &Segmenter,
            text: &str,
            sentences: &[Sentence],
        ) -> Vec<(Vec<TokenOffset>, i64)> {
            let mut partial: Vec<(Vec<TokenOffset>, i64, Option<u16>)> =
                vec![(Vec::new(), 0, None)];
            for &sentence in sentences {
                let mut memo: HashMap<Option<u16>, Vec<SentencePath>> = HashMap::new();
                let mut next = Vec::new();
                for (offsets, cost, state) in partial {
                    let paths = memo
                        .entry(state)
                        .or_insert_with(|| sentence_paths(segmenter, text, sentence, state));
                    for (path, path_cost, exit) in paths.iter() {
                        let mut combined = offsets.clone();
                        combined.extend_from_slice(path);
                        next.push((combined, cost + path_cost, Some(*exit)));
                    }
                }
                assert!(next.len() <= 200_000, "the oracle's input is too long");
                partial = next;
            }
            partial
                .into_iter()
                .map(|(offsets, cost, _)| (offsets, cost))
                .collect()
        }

        /// Every path of `text` by definition, built and sorted by cost:
        /// every combination of one path per segment.
        fn all_paths(segmenter: &Segmenter, text: &str) -> Vec<(i64, Vec<Flat>)> {
            let mut segments: Vec<Vec<Sentence>> = vec![Vec::new()];
            for sentence in SentenceWalk::new(text) {
                segments.last_mut().unwrap().push(sentence);
                if !sentence.carries {
                    segments.push(Vec::new());
                }
            }
            segments.retain(|segment| !segment.is_empty());

            let mut all: Vec<(Vec<TokenOffset>, i64)> = vec![(Vec::new(), 0)];
            for segment in &segments {
                let paths = segment_paths(segmenter, text, segment);
                let mut next = Vec::new();
                for (offsets, cost) in &all {
                    for (path, path_cost) in &paths {
                        let mut combined = offsets.clone();
                        combined.extend_from_slice(path);
                        next.push((combined, cost + path_cost));
                    }
                }
                assert!(next.len() <= 200_000, "the oracle's input is too long");
                all = next;
            }

            let mut built: Vec<(i64, Vec<Flat>)> = all
                .into_iter()
                .map(|(offsets, cost)| (cost, build(segmenter, text, 0, &offsets)))
                .collect();
            built.sort_by_key(|(cost, _)| *cost);
            built
        }

        /// The expected results by definition from [`all_paths`]: with
        /// `unique`, one entry per word-boundary class, at its cheapest cost
        /// and with every variant of that cost; cut at the threshold above
        /// the best.
        fn oracle(all: &[(i64, Vec<Flat>)], unique: bool, threshold: Option<i64>) -> Expected {
            let mut expected: Expected = Vec::new();
            if unique {
                let mut classes: HashMap<Vec<(usize, usize)>, usize> = HashMap::new();
                for (cost, tokens) in all {
                    match classes.get(&boundaries(tokens)) {
                        Some(&i) => {
                            if expected[i].0 == *cost {
                                expected[i].1.push(tokens.clone());
                            }
                        }
                        None => {
                            classes.insert(boundaries(tokens), expected.len());
                            expected.push((*cost, vec![tokens.clone()]));
                        }
                    }
                }
            } else {
                expected = all
                    .iter()
                    .map(|(cost, tokens)| (*cost, vec![tokens.clone()]))
                    .collect();
            }
            if let (Some(threshold), Some(&(best, _))) = (threshold, expected.first()) {
                expected.retain(|(cost, _)| cost - best <= threshold);
            }
            expected
        }

        /// Checks the N-best results of `text` against the oracle for
        /// several `n`, with and without `unique`, and with thresholds, and
        /// the 1-best path against the best of the oracle.
        fn assert_oracle(segmenter: &Segmenter, text: &str) {
            let all = all_paths(segmenter, text);
            for unique in [false, true] {
                for threshold in [None, Some(0), Some(1000), Some(5000)] {
                    let expected = oracle(&all, unique, threshold);
                    assert!(!expected.is_empty());
                    for n in [1, 2, 3, 7, 50] {
                        let context = format!("{text:?}, n {n}, unique {unique}, {threshold:?}");
                        let results = nbest(segmenter, text, n, unique, threshold);
                        assert_top_n(&results, &expected, n, unique, false, &context);
                        assert_whole_input(segmenter, text, &results);
                    }
                }
            }
            let best = oracle(&all, false, Some(0));
            let tokens = segment(segmenter, text);
            assert!(
                best.iter().any(|(_, lists)| lists.contains(&tokens)),
                "{text:?}: segment is not a best path"
            );
        }

        /// Lines of the bocchan text whose `、` and `。` are followed by a
        /// letter (kana or kanji), so no unknown symbol group spans a cut,
        /// without ruby annotations and short enough for an N-best search
        /// over the whole line.
        fn bocchan_lines(count: usize) -> Vec<String> {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/bocchan.txt");
            let text = std::fs::read_to_string(path).unwrap();
            let is_letter = |c: char| matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}' | '\u{4E00}'..='\u{9FAF}');
            text.lines()
                .map(|line| line.trim_start_matches('\u{3000}'))
                .filter(|line| {
                    let chars: Vec<char> = line.chars().collect();
                    let cuts = chars.iter().filter(|&&c| c == '、' || c == '。').count();
                    (2..=6).contains(&cuts)
                        && chars.len() <= 60
                        && !chars.iter().any(|c| "《》｜［］＃".contains(*c))
                        && chars
                            .windows(2)
                            .all(|pair| !(pair[0] == '、' || pair[0] == '。') || is_letter(pair[1]))
                })
                .take(count)
                .map(str::to_string)
                .collect()
        }

        /// Lines that one lattice and the carried context solve alike: no
        /// entry contains `、` or `。`, and no unknown word groups across a
        /// cut.
        fn unsplit_lines() -> Vec<String> {
            let mut lines: Vec<String> = [
                "東京、です。関西国際空港へ行く",
                "やがて、やがて",
                "つまらない、だから",
                "狸、狸。狸",
                "すもももももももものうち、すもも。もものうち",
                "東京、 です",
                // The best path continues the exit that is not the cheapest
                // one of `一、`, also after skipped whitespace.
                "一、人",
                "三、人です。二、人",
                "一、 人",
                "1、 人。 １、 人",
            ]
            .iter()
            .map(|line| line.to_string())
            .collect();
            let bocchan = bocchan_lines(10);
            assert_eq!(bocchan.len(), 10);
            lines.extend(bocchan);
            lines
        }

        /// The 1-best path and its cost are those of one lattice over the
        /// line.
        #[test]
        fn test_segment_matches_one_lattice_over_the_line() {
            for mode in [Mode::Normal, Mode::Decompose(Penalty::default())] {
                let segmenter = ipadic(mode);
                for line in unsplit_lines() {
                    assert_eq!(
                        segment(&segmenter, &line),
                        unsplit_segment(&segmenter, &line),
                        "{line:?}"
                    );
                    let best = &unsplit_nbest(&segmenter, &line, 1, false, None)[0];
                    assert_eq!(nbest(&segmenter, &line, 1, false, None)[0].1, best.0);
                }
            }
        }

        /// The N-best results are those of one lattice over the line.
        #[test]
        fn test_nbest_matches_one_lattice_over_the_line() {
            let segmenter = ipadic(Mode::Normal);
            for line in unsplit_lines() {
                for unique in [false, true] {
                    for threshold in [None, Some(0), Some(2000)] {
                        // A margin beyond `n` so that ties at the cut are
                        // among the expected results.
                        let expected = unsplit_nbest(&segmenter, &line, 20, unique, threshold);
                        for n in [1, 3, 10] {
                            let context =
                                format!("{line:?}, n {n}, unique {unique}, {threshold:?}");
                            let results = nbest(&segmenter, &line, n, unique, threshold);
                            assert_top_n(&results, &expected, n, unique, unique, &context);
                            assert_whole_input(&segmenter, &line, &results);
                        }
                    }
                }
            }
        }

        /// The model's own definition: EOS only at the ends of segments,
        /// every sentence continuing the exit of the previous one's path,
        /// `unique` and the threshold over the whole input.
        #[test]
        fn test_nbest_matches_the_definition() {
            let segmenter = ipadic(Mode::Normal);
            for text in [
                "東京、です",
                "一、人",
                "一、 人",
                "もも、もも",
                "狸、狸。狸",
                "犬、犬。犬",
                // A sentence that starts with skipped whitespace.
                "東京、 です",
                // A segment that ends with a sentence of whitespace only.
                "東京、 \nです",
                // Two segments, each with a carried cut.
                "犬、犬\tもも、もも",
                // The input ends with `、`, whose cut pays EOS.
                "東京、です、",
                // Cuts without a carried context only.
                "東京\nです\tもも",
            ] {
                assert_oracle(&segmenter, text);
            }
        }

        #[test]
        fn test_nbest_matches_the_definition_with_whitespace_settings() {
            let text = "犬 犬、 犬\tへ、 もも";
            assert_oracle(&ipadic(Mode::Normal).skip_whitespace(false), text);
            assert_oracle(&ipadic(Mode::Normal).keep_whitespace(true), text);
        }

        #[test]
        fn test_nbest_matches_the_definition_in_decompose_mode() {
            let segmenter = ipadic(Mode::Decompose(Penalty::default()));
            for text in ["東京、です", "関西、です", "犬、犬。犬"] {
                assert_oracle(&segmenter, text);
            }
        }

        /// A sentence of skipped whitespace between two carried cuts (a
        /// forced cut inside a long run of spaces) passes its BOS edges on
        /// as its exits.
        #[test]
        fn test_whitespace_only_sentence_passes_the_context_on() {
            let segmenter = ipadic(Mode::Normal);
            let text = format!("東京、{}、です", " ".repeat(MAX_SENTENCE_BYTES + 100));
            let sentences: Vec<Sentence> = SentenceWalk::new(&text).collect();
            assert_eq!(sentences.len(), 4);
            assert!(sentences[1].carries);
            assert!(text[sentences[1].start..sentences[1].end].trim().is_empty());

            let expected = oracle(&all_paths(&segmenter, &text), false, None);
            for n in [1, 3, 10] {
                let results = nbest(&segmenter, &text, n, false, None);
                assert_top_n(&results, &expected, n, false, false, &format!("n {n}"));
                assert_whole_input(&segmenter, &text, &results);
            }
            let tokens = segment(&segmenter, &text);
            assert!(
                expected
                    .iter()
                    .any(|(cost, lists)| *cost == expected[0].0 && lists.contains(&tokens))
            );
            // The same costs as without the whitespace: it adds nothing.
            let compact = nbest(&segmenter, "東京、、です", 10, false, None);
            let costs: Vec<i64> = nbest(&segmenter, &text, 10, false, None)
                .iter()
                .map(|(_, cost)| *cost)
                .collect();
            let compact_costs: Vec<i64> = compact.iter().map(|(_, cost)| *cost).collect();
            assert_eq!(costs, compact_costs);
        }

        /// Only `\n` and `\t` cut the input: every sentence is a segment, as
        /// before the context was carried, so the results are the cheapest
        /// combinations of the sentences' own N-best lists, and `segment`
        /// is the sentences' own best paths.
        #[test]
        fn test_cuts_without_carried_context_are_unchanged() {
            let segmenter = ipadic(Mode::Normal);
            for text in [
                "東京\nです\tもも",
                "すもも\n\nもものうち\t",
                " 東京 \n です",
            ] {
                let mut joined = Vec::new();
                for sentence in SentenceWalk::new(text) {
                    assert!(!sentence.carries);
                    let part = &text[sentence.start..sentence.end];
                    for (surface, start, end, _, word_id) in segment(&segmenter, part) {
                        let position = joined.len();
                        joined.push((
                            surface,
                            start + sentence.start,
                            end + sentence.start,
                            position,
                            word_id,
                        ));
                    }
                }
                assert_eq!(segment(&segmenter, text), joined, "{text:?}");
                assert_oracle(&segmenter, text);
            }
        }

        /// The first N-best result is the 1-best path, also over a long line
        /// with forced cuts.
        #[test]
        fn test_nbest_first_result_matches_segment() {
            let segmenter = ipadic(Mode::Normal);
            let long = "すもももももももものうち".repeat(70 * 1024 / 36 + 1);
            assert!(long.len() > 2 * MAX_SENTENCE_BYTES);
            let texts = [
                "東京、です。すもももももももものうち\nもも".to_string(),
                "やがて、やがて。つまらない、だから".to_string(),
                long,
            ];
            for text in &texts {
                let tokens = segment(&segmenter, text);
                for n in [1, 3] {
                    let results = nbest(&segmenter, text, n, false, None);
                    assert_eq!(results[0].0, tokens, "n {n}");
                    assert_whole_input(&segmenter, text, &results);
                }
            }
        }

        /// The 1-best search emits a path once every state shares it and
        /// leaves no open path behind, so the buffers stay small over a long
        /// line with forced cuts and many `、`.
        #[test]
        fn test_segment_leaves_no_open_path() {
            let segmenter = ipadic(Mode::Normal);
            let long = "すもももももももものうち".repeat(70 * 1024 / 36 + 1);
            let commas = "やがて、つまらない。".repeat(2000);
            let mut lattice = Lattice::default();
            let mut buffers = SegmentBuffers::default();
            for text in [long.as_str(), commas.as_str()] {
                let tokens = segmenter
                    .segment_with_buffers(Cow::Borrowed(text), &mut lattice, &mut buffers)
                    .unwrap();
                assert_eq!(flatten(&tokens), segment(&segmenter, text));
                assert_eq!(buffers.tree.len(), 0);
            }
        }

        /// A huge `n` presizes nothing, a threshold of 0 keeps the best
        /// results only, and a negative one keeps nothing.
        #[test]
        fn test_nbest_huge_n_and_zero_threshold() {
            let segmenter = ipadic(Mode::Normal);
            let text = "東京、です";
            let results = nbest(&segmenter, text, usize::MAX, false, Some(0));
            assert!(!results.is_empty());
            assert_eq!(results[0], nbest(&segmenter, text, 1, false, None)[0]);
            assert!(results.iter().all(|(_, cost)| *cost == results[0].1));
            assert!(nbest(&segmenter, text, 5, false, Some(-1)).is_empty());
            assert!(nbest(&segmenter, text, 0, false, None).is_empty());

            let all = nbest(&segmenter, "狸、狸", usize::MAX, true, None);
            assert_eq!(
                all.len(),
                oracle(&all_paths(&segmenter, "狸、狸"), true, None).len()
            );
        }

        /// The context carried across `、` changes the output where the
        /// unsplit line differs from the sentences solved on their own: the
        /// cost of the 1-best path is that of the line, not the sum of the
        /// sentences'.
        #[test]
        fn test_carried_cost_is_not_the_sum_of_sentences() {
            let segmenter = ipadic(Mode::Normal);
            let text = "東京、です";
            let sum: i64 = SentenceWalk::new(text)
                .map(|sentence| {
                    nbest(
                        &segmenter,
                        &text[sentence.start..sentence.end],
                        1,
                        false,
                        None,
                    )[0]
                    .1
                })
                .sum();
            let carried = nbest(&segmenter, text, 1, false, None)[0].1;
            assert_eq!(
                carried,
                unsplit_nbest(&segmenter, text, 1, false, None)[0].0
            );
            assert_ne!(carried, sum);
        }

        #[test]
        fn test_nbest_owned_matches_borrowed() {
            let segmenter = ipadic(Mode::Normal);
            let text = "東京、です。すもももももももものうち";
            let owned: Results = segmenter
                .segment_nbest(Cow::Owned(text.to_string()), 5, false, None)
                .unwrap()
                .iter()
                .map(|(tokens, cost)| (flatten(tokens), *cost))
                .collect();
            assert_eq!(owned, nbest(&segmenter, text, 5, false, None));
            let owned_tokens = flatten(&segmenter.segment(Cow::Owned(text.to_string())).unwrap());
            assert_eq!(owned_tokens, segment(&segmenter, text));
        }
    }
}
