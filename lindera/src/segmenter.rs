use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;

use lindera_dictionary::mode::Mode;
use log::warn;

use lindera_dictionary::dictionary::character_definition::CategoryId;
use lindera_dictionary::dictionary::{Dictionary, UserDictionary};
use lindera_dictionary::space_penalty::{SpacePenaltyConfig, SpacePenaltyTable};
use lindera_dictionary::viterbi::{Lattice, LatticeOptions, WordId};
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
        // here), so this cheap pre-check skips the 3-byte slice
        // comparison for ~254/256 possible byte values.
        if (ch == 0x81 || ch == 0x82) && sentence_end >= 3 {
            let last_3 = &text_bytes[sentence_end - 3..sentence_end];
            if last_3 == "。".as_bytes() || last_3 == "、".as_bytes() {
                return (sentence_end, false);
            }
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
    /// When false (default), whitespace is ignored for MeCab compatibility.
    /// When true, whitespace tokens are included in the output.
    pub keep_whitespace: bool,

    /// Maximum number of characters beyond the first that an unknown-word
    /// grouping may span (MeCab's `max-grouping-size`; MeCab defaults to
    /// 24). A grouped candidate longer than this is not emitted -- the
    /// single-char unknown word is emitted instead. `None` (default)
    /// leaves grouping unbounded, matching previous behavior.
    pub max_grouping_len: Option<usize>,

    /// Whether to additionally emit MeCab/Vibrato-inspired shorter
    /// unknown-word candidates up to each category's `char.def` `LENGTH`
    /// field (#945). Lindera parsed but never read this field before this
    /// option existed. Defaults to `true` starting with this release --
    /// set to `false` to match pre-v6 output exactly.
    pub unknown_word_ladder: bool,

    /// Left-space penalty rules (mecab-ko's `left-space-penalty-factor`,
    /// nori's `computeSpacePenalty`): a candidate that starts right after
    /// whitespace and whose first part-of-speech tag matches a rule gets the
    /// rule's cost added, so e.g. a particle or ending is not chosen across
    /// a space. `None` (default) adds no penalty and keeps the previous
    /// output. Set through [`Segmenter::space_penalty`] /
    /// [`Segmenter::set_space_penalty`], which also build the per-word-id
    /// lookup the lattice uses; the field is read-only for that reason.
    space_penalty: Option<SpacePenaltyConfig>,

    /// Per-word-id lookup derived from `space_penalty` for the current
    /// dictionary pair; `Arc` so `Clone` stays cheap.
    space_penalty_table: Option<Arc<SpacePenaltyTable>>,

    /// The category ID for space characters, used when keep_whitespace is false.
    space_category_id: Option<CategoryId>,

    /// Precomputed, per-codepoint (0..256) SPACE-category membership for the
    /// ASCII/Latin-1 range, used as a fast path for the keep_whitespace=false
    /// skip check. Built once from the loaded dictionary's actual character
    /// definitions (not a hardcoded byte pattern), so it stays correct for
    /// any dictionary regardless of which characters its char.def classifies
    /// as SPACE. Codepoints >= 256 always fall back to
    /// `character_definition.lookup_categories`.
    space_ascii_table: Option<[bool; 256]>,
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
    pub fn new(
        mode: Mode,
        dictionary: Dictionary,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        // Get SPACE category ID for MeCab compatibility (ignore whitespace by default)
        let space_category_id = dictionary.character_definition.category_id_by_name("SPACE");

        // Precompute ASCII/Latin-1 SPACE-category membership once, from the
        // dictionary's actual character definitions.
        let space_ascii_table = space_category_id.map(|space_id| {
            let mut table = [false; 256];
            for (codepoint, is_space) in table.iter_mut().enumerate() {
                if let Some(c) = char::from_u32(codepoint as u32) {
                    *is_space = dictionary
                        .character_definition
                        .lookup_categories(c)
                        .contains(&space_id);
                }
            }
            table
        });

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

        Self {
            mode,
            dictionary,
            user_dictionary,
            keep_whitespace: false, // Default: ignore whitespace for MeCab compatibility
            max_grouping_len: None, // Default: unbounded grouping
            unknown_word_ladder: true, // Default (v6+): honor char.def's LENGTH field
            space_penalty: None,    // Default: no left-space penalty
            space_penalty_table: None,
            space_category_id,
            space_ascii_table,
        }
    }

    /// Builder method to set whether to keep whitespace tokens in output.
    ///
    /// When `keep_whitespace` is false (default), whitespace is ignored for MeCab compatibility.
    /// When true, whitespace tokens are included in the output.
    ///
    /// # Arguments
    ///
    /// * `keep_whitespace` - If true, whitespace tokens will be included in the output.
    ///
    /// # Example
    ///
    /// ```
    /// use lindera::mode::Mode;
    /// use lindera::dictionary::load_dictionary;
    /// use lindera::segmenter::Segmenter;
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

    /// Builder method to cap unknown-word grouping (MeCab's
    /// `max-grouping-size` semantics; see the `max_grouping_len` field).
    ///
    /// # 引数
    ///
    /// * `max_grouping_len` - Maximum grouped characters beyond the first,
    ///   or `None` for unbounded grouping (the default).
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

    /// Builder method to enable the left-space penalty (see the
    /// `space_penalty` field; `None`, the default, disables it).
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
    /// use lindera::mode::Mode;
    /// use lindera::dictionary::load_dictionary;
    /// use lindera::segmenter::Segmenter;
    /// use lindera::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};
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
    /// use lindera::mode::Mode;
    /// use lindera::dictionary::load_dictionary;
    /// use lindera::segmenter::Segmenter;
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
        options
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
        // Default is false (MeCab compatible - ignore whitespace)
        // Set to true explicitly to include whitespace tokens
        let keep_whitespace = config
            .get("keep_whitespace")
            .and_then(Value::as_bool)
            .unwrap_or(false); // Default: false (ignore whitespace)

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
        // application point. `space_category_id` is only read when `keep_whitespace` is
        // false, so letting `new` always resolve it is equivalent to the previous
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

        // Load the space_penalty option from the config. Absent, `null` or
        // `false` means off (the default); `true` uses the rules the
        // dictionary ships in its metadata; an object holds explicit rules.
        enum SpacePenaltySetting {
            Off,
            FromDictionary,
            Rules(SpacePenaltyConfig),
        }
        let space_penalty = match config.get("space_penalty") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => SpacePenaltySetting::Off,
            Some(Value::Bool(true)) => SpacePenaltySetting::FromDictionary,
            Some(value) => SpacePenaltySetting::Rules(
                serde_json::from_value::<SpacePenaltyConfig>(value.clone()).map_err(|e| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("space_penalty field is invalid: {e}"))
                })?,
            ),
        };

        let segmenter = Self::new(mode, dictionary, user_dictionary)
            .keep_whitespace(keep_whitespace)
            .max_grouping_len(max_grouping_len)
            .unknown_word_ladder(unknown_word_ladder);
        match space_penalty {
            SpacePenaltySetting::Off => Ok(segmenter),
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
    ///    - The input text is split into sentences using Japanese punctuation (`。`, `、`, `\n`, `\t`). Each sentence is processed individually.
    ///
    /// 2. **Lattice Processing**:
    ///    - For each sentence, a lattice structure is set up using the main dictionary and, if available, the user dictionary. The lattice helps identify possible token boundaries within the sentence.
    ///    - The cost matrix is used to calculate the best path (i.e., the optimal sequence of tokens) through the lattice based on the mode.
    ///
    /// 3. **Token Generation**:
    ///    - For each segment (determined by the lattice), a token is generated using the byte offsets. The tokens contain the original text (in `Cow::Owned` form to ensure safe return), byte start/end positions, token positions, and dictionary references.
    ///
    /// # Notes
    ///
    /// - The function ensures that each token is safely returned by converting substrings into `Cow::Owned` strings.
    /// - Byte offsets are carefully calculated to ensure that token boundaries are correct even across multiple sentences.
    ///
    /// # Example Flow
    ///
    /// - Text is split into sentences based on punctuation.
    /// - A lattice is created and processed for each sentence.
    /// - Tokens are extracted from the lattice and returned in a vector.
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
    ///    - The input text is split into sentences using Japanese punctuation (`。`, `、`, `\n`, `\t`). Each sentence is processed individually.
    ///
    /// 2. **Lattice Processing**:
    ///    - For each sentence, a lattice structure is set up using the main dictionary and, if available, the user dictionary. The lattice helps identify possible token boundaries within the sentence.
    ///    - The cost matrix is used to calculate the best path (i.e., the optimal sequence of tokens) through the lattice based on the mode.
    ///
    /// 3. **Token Generation**:
    ///    - For each segment (determined by the lattice), a token is generated using the byte offsets. The tokens contain the original text (in `Cow::Owned` form to ensure safe return), byte start/end positions, token positions, and dictionary references.
    ///
    /// # Notes
    ///
    /// - The function ensures that each token is safely returned by converting substrings into `Cow::Owned` strings.
    /// - Byte offsets are carefully calculated to ensure that token boundaries are correct even across multiple sentences.
    ///
    /// # Example Flow
    ///
    /// - Text is split into sentences based on punctuation.
    /// - A lattice is created and processed for each sentence.
    /// - Tokens are extracted from the lattice and returned in a vector.
    ///
    /// # Errors
    ///
    /// - If the lattice fails to be processed or if there is an issue with the segmentation process, the function returns an error.
    pub fn segment_with_lattice<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
    ) -> LinderaResult<Vec<Token<'a>>> {
        // The backtrace buffer is allocated once per call; SegmentWorker
        // routes through segment_with_buffers to reuse it across calls too.
        let mut offsets: Vec<(usize, WordId)> = Vec::new();
        self.segment_with_buffers(text, lattice, &mut offsets)
    }

    /// Segments the input text reusing both the caller's lattice and the
    /// caller's backtrace scratch buffer.
    ///
    /// This is the shared body behind [`Segmenter::segment_with_lattice`]
    /// (which passes a fresh buffer) and `SegmentWorker::segment` (which
    /// keeps one buffer alive across calls). Output is identical to
    /// `segment_with_lattice` for the same input.
    ///
    /// # 引数
    ///
    /// * `text` - The input text, borrowed or owned.
    /// * `lattice` - The Viterbi lattice to reuse across sentences/calls.
    /// * `offsets` - Backtrace scratch buffer; overwritten per sentence
    ///   (`tokens_offset_into` clears it), so no pre-clearing is required.
    ///
    /// # 戻り値
    ///
    /// The tokens segmented from `text`, in reading order.
    pub(crate) fn segment_with_buffers<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
        offsets: &mut Vec<(usize, WordId)>,
    ) -> LinderaResult<Vec<Token<'a>>> {
        let mut tokens: Vec<Token> = Vec::new();

        let mut position = 0_usize;
        let mut byte_position = 0_usize;

        // Whitespace-filter configuration, hoisted out of the per-token
        // loop: the per-char closure previously re-unwrapped the Option'd
        // ASCII table (a by-value array copy at source level) on every
        // character it examined (#942). `space_category_id` and
        // `space_ascii_table` are built together, so `zip` preserves the
        // old `keep_whitespace`/`Some` gating exactly.
        let space_filter = if self.keep_whitespace {
            None
        } else {
            self.space_category_id.zip(self.space_ascii_table.as_ref())
        };

        // Process whole text without splitting first for better performance with borrowed text
        let text_len = text.len();
        let mut sentence_start = 0;

        while sentence_start < text_len {
            // Find the end of the current sentence
            let (sentence_end, forced_cut) = find_sentence_end(&text, sentence_start);
            if forced_cut {
                warn!(
                    "no sentence delimiter (\\n, \\t, 。, 、) found within {MAX_SENTENCE_BYTES} bytes from offset {sentence_start}; forcing a sentence boundary to bound lattice size (see https://github.com/lindera/lindera/issues/871)"
                );
            }

            let sentence = &text[sentence_start..sentence_end];
            if sentence.is_empty() {
                sentence_start = sentence_end;
                continue;
            }

            // Process the sentence through lattice
            lattice.set_text_with_options(
                &self.dictionary.prefix_dictionary,
                &self.user_dictionary.as_ref().map(|d| &d.dict),
                &self.dictionary.character_definition,
                &self.dictionary.unknown_dictionary,
                &self.dictionary.connection_cost_matrix,
                sentence,
                &self.lattice_options(),
            );
            // Forward Viterbi implementation handles cost calculation within `set_text`.

            lattice.tokens_offset_into(offsets);
            tokens.reserve(offsets.len());

            for i in 0..offsets.len() {
                let (byte_start, word_id) = offsets[i];
                let byte_end = if i == offsets.len() - 1 {
                    sentence.len()
                } else {
                    let (next_start, _word_id) = offsets[i + 1];
                    next_start
                };

                // Calculate absolute position in the original text
                let absolute_start = sentence_start + byte_start;
                let absolute_end = sentence_start + byte_end;

                // Skip whitespace tokens if keep_whitespace is false (default MeCab behavior)
                if let Some((space_category_id, space_ascii_table)) = space_filter {
                    // Check if this token consists only of whitespace characters.
                    // ASCII/Latin-1 codepoints use the precomputed table (O(1));
                    // anything else falls back to the dictionary lookup.
                    let token_text = &sentence[byte_start..byte_end];
                    let is_space = token_text.chars().all(|c| {
                        if (c as u32) < 256 {
                            space_ascii_table[c as usize]
                        } else {
                            self.dictionary
                                .character_definition
                                .lookup_categories(c)
                                .contains(&space_category_id)
                        }
                    });

                    if is_space {
                        // Update byte_position to maintain correct offsets
                        byte_position += byte_end - byte_start;
                        continue;
                    }
                }

                // Create surface Cow efficiently - avoid unnecessary string allocation for owned strings
                let surface_cow = match &text {
                    Cow::Borrowed(s) => Cow::Borrowed(&s[absolute_start..absolute_end]),
                    Cow::Owned(s) => {
                        // Use slice from owned string instead of creating new string
                        Cow::Owned(s[absolute_start..absolute_end].to_owned())
                    }
                };

                // compute the token's absolute byte positions
                let token_start = byte_position;
                byte_position += byte_end - byte_start;
                let token_end = byte_position;

                tokens.push(Token::new(
                    surface_cow,
                    token_start,
                    token_end,
                    position,
                    word_id,
                    &self.dictionary,
                    self.user_dictionary.as_ref(),
                ));

                position += 1;
            }

            sentence_start = sentence_end;
        }

        Ok(tokens)
    }

    /// Segments the input text and returns the top-N segmentation results.
    ///
    /// Each result is a `Vec<Token>` representing one possible segmentation.
    /// Results are ordered by cost (best first).
    /// If `unique` is true, results with the same word boundaries but different
    /// POS tags are deduplicated (only the lowest-cost variant is kept).
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
    pub fn segment_nbest_with_lattice<'a>(
        &'a self,
        text: Cow<'a, str>,
        lattice: &mut Lattice,
        n: usize,
        unique: bool,
        cost_threshold: Option<i64>,
    ) -> LinderaResult<Vec<(Vec<Token<'a>>, i64)>> {
        let mut all_results: Vec<(Vec<Token>, i64)> = Vec::with_capacity(n);

        // Hoisted whitespace-filter configuration; see segment_with_buffers.
        let space_filter = if self.keep_whitespace {
            None
        } else {
            self.space_category_id.zip(self.space_ascii_table.as_ref())
        };

        let text_len = text.len();
        let mut sentence_start = 0;

        while sentence_start < text_len {
            // Find the end of the current sentence
            let (sentence_end, forced_cut) = find_sentence_end(&text, sentence_start);
            if forced_cut {
                warn!(
                    "no sentence delimiter (\\n, \\t, 。, 、) found within {MAX_SENTENCE_BYTES} bytes from offset {sentence_start}; forcing a sentence boundary to bound lattice size (see https://github.com/lindera/lindera/issues/871)"
                );
            }

            let sentence = &text[sentence_start..sentence_end];
            if sentence.is_empty() {
                sentence_start = sentence_end;
                continue;
            }

            // Process the sentence through N-Best lattice
            lattice.set_text_nbest_with_options(
                &self.dictionary.prefix_dictionary,
                &self.user_dictionary.as_ref().map(|d| &d.dict),
                &self.dictionary.character_definition,
                &self.dictionary.unknown_dictionary,
                &self.dictionary.connection_cost_matrix,
                sentence,
                &self.lattice_options(),
            );

            let nbest_offsets = lattice.nbest_tokens_offset(n, unique, cost_threshold);

            for (rank, (offsets, cost)) in nbest_offsets.into_iter().enumerate() {
                if rank >= all_results.len() {
                    all_results.resize_with(rank + 1, || (Vec::new(), 0));
                }

                // Accumulate cost across sentences
                all_results[rank].1 += cost;

                let mut position = all_results[rank].0.len();
                let mut byte_position: usize = if all_results[rank].0.is_empty() {
                    0
                } else {
                    all_results[rank].0.last().map_or(0, |t| t.byte_end)
                };

                for i in 0..offsets.len() {
                    let (byte_start, word_id) = offsets[i];
                    let byte_end = if i == offsets.len() - 1 {
                        sentence.len()
                    } else {
                        offsets[i + 1].0
                    };

                    let absolute_start = sentence_start + byte_start;
                    let absolute_end = sentence_start + byte_end;

                    // Skip whitespace tokens if keep_whitespace is false
                    if let Some((space_category_id, space_ascii_table)) = space_filter {
                        // ASCII/Latin-1 codepoints use the precomputed table
                        // (O(1)); anything else falls back to the dictionary
                        // lookup.
                        let token_text = &sentence[byte_start..byte_end];
                        let is_space = token_text.chars().all(|c| {
                            if (c as u32) < 256 {
                                space_ascii_table[c as usize]
                            } else {
                                self.dictionary
                                    .character_definition
                                    .lookup_categories(c)
                                    .contains(&space_category_id)
                            }
                        });

                        if is_space {
                            byte_position += byte_end - byte_start;
                            continue;
                        }
                    }

                    let surface_cow = match &text {
                        Cow::Borrowed(s) => Cow::Borrowed(&s[absolute_start..absolute_end]),
                        Cow::Owned(s) => Cow::Owned(s[absolute_start..absolute_end].to_owned()),
                    };

                    let token_start = byte_position;
                    byte_position += byte_end - byte_start;
                    let token_end = byte_position;

                    all_results[rank].0.push(Token::new(
                        surface_cow,
                        token_start,
                        token_end,
                        position,
                        word_id,
                        &self.dictionary,
                        self.user_dictionary.as_ref(),
                    ));

                    position += 1;
                }
            }

            sentence_start = sentence_end;
        }

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    use crate::segmenter::{MAX_SENTENCE_BYTES, find_sentence_end};
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
    #[cfg(feature = "embed-ko-dic")]
    mod space_penalty {
        use std::borrow::Cow;
        use std::io::Write;

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

        /// The penalty flips the particle/ending reading of a token that
        /// follows a space (`시/EP` -> `시/NNG`, `이/VCP` -> a non-VCP tag),
        /// which is what mecab-ko does for these inputs; with the option off
        /// the v6.0.0 output is unchanged.
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
        /// rules the dictionary ships in its metadata, `false`/`null`/absent
        /// for off, and rejects malformed objects.
        #[test]
        fn test_from_config_space_penalty() {
            let base = serde_json::json!({
                "dictionary": "embedded://ko-dic",
                "mode": "normal",
            });

            let mut config: SegmenterConfig = base.clone();
            config["space_penalty"] = serde_json::to_value(ko_dic_rules()).unwrap();
            let segmenter = Segmenter::from_config(&config).unwrap();
            assert_eq!(segmenter.space_penalty_config(), Some(&ko_dic_rules()));
            assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/NNG");

            for off in [serde_json::Value::Null, serde_json::json!(false)] {
                let mut config = base.clone();
                config["space_penalty"] = off;
                let segmenter = Segmenter::from_config(&config).unwrap();
                assert!(segmenter.space_penalty_config().is_none());
                assert_eq!(render(&segmenter, "서울 시 에서 출발")[1], "시/EP");
            }
            assert!(
                Segmenter::from_config(&base)
                    .unwrap()
                    .space_penalty_config()
                    .is_none()
            );

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
}
