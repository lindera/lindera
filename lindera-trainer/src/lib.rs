//! CRF-based dictionary training for Lindera.
//!
//! This crate implements the training pipeline (`lindera train`), producing
//! a serialized model that `lindera export` turns into MeCab-format
//! dictionary source files, which `lindera build` then compiles into a
//! binary dictionary. It builds on the runtime types of
//! [`lindera_dictionary`] and the CRF core of [`lindera_crf`].

pub mod config;
pub mod corpus;
mod feature_csv;
pub mod feature_extractor;
mod feature_id_map;
pub mod feature_rewriter;
mod lexicon_csv;
pub mod model;
mod regularization;

use std::collections::HashMap;
use std::num::NonZeroU32;

use anyhow::Result;

use lindera_dictionary::dictionary::prefix_dictionary::Match;
use lindera_dictionary::viterbi::UnknownWordCandidate;

use self::feature_csv::{split_feature, wildcard_feature};
use self::feature_extractor::{FeatureExtractor, TemplateContext};
use self::feature_rewriter::{DictionaryRewriter, RewriteBuffers};

/// Logging macros for training process
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!($($arg)*)
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!("DEBUG: {}", format!($($arg)*))
        }
    };
}

macro_rules! log_progress {
    ($($arg:tt)*) => {
        println!($($arg)*)
    };
}

/// Writes a warning on stderr, apart from the progress on stdout. It does
/// not go through the `log` crate, for which the bindings install no logger,
/// so that their users see it too (#1259).
macro_rules! log_warn {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}

pub use self::config::TrainerConfig;
pub use self::corpus::{Corpus, Example, Word};
pub use self::model::{Model, SerializableModel};
pub use self::regularization::{
    DEFAULT_ELASTIC_NET_L1_RATIO, ParseRegularizationTypeError, RegularizationType,
};

/// The default tolerance of the convergence test, set with
/// [`Trainer::eta`]: `5e-5`, the default `eta` of MeCab's learner.
pub use lindera_crf::DEFAULT_ETA;

/// The default regularization coefficient, set with
/// [`Trainer::regularization_cost`]: `1.0`. With L2 regularization, the
/// default type, it gives the objective of MeCab's learner with its default
/// `-c 1.0`; in general the coefficient is `1 / C`.
pub use lindera_crf::DEFAULT_LAMBDA;

/// CRF-based morphological analysis trainer for Japanese text
///
/// This trainer implements standard CRF-based morphological analysis training,
/// adapted for Lindera's architecture. It supports:
/// - Feature extraction from vocabulary and corpus
/// - L-BFGS optimization for weight learning
/// - Unknown words from every `unk.def` entry, created as the runtime
///   creates them (#1110)
/// - Connection cost matrix generation
///
/// # Training Process
/// 1. Initialize feature provider with vocabulary entries
/// 2. Build lattices from training corpus
/// 3. Execute CRF training with L-BFGS optimization
/// 4. Extract learned weights and create final model
///
/// # Example
/// ```
/// use lindera_trainer::{Trainer, TrainerConfig};
/// use std::io::Cursor;
///
/// // Create minimal training data for demonstration
/// let seed_csv = "これ,0,0,1000,連体詞,*,*,*,*,*,これ,コレ,コレ\n";
/// let char_def = "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n0x3042..0x3096 HIRAGANA\n";
/// // One line for each char.def category, as MeCab requires.
/// let unk_def = "DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n\
///                HIRAGANA,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n";
/// let feature_def = "UNIGRAM:%F[0]\nUNIGRAM:%F[1]\n";
/// let rewrite_def = "*\tUNK\n";
///
/// let config = TrainerConfig::from_readers(
///     Cursor::new(seed_csv),
///     Cursor::new(char_def),
///     Cursor::new(unk_def),
///     Cursor::new(feature_def),
///     Cursor::new(rewrite_def)
/// ).unwrap();
///
/// let trainer = Trainer::new(config).unwrap()
///     .max_iter(10); // Reduced for doc test
///
/// // Note: In practice, you would load an actual corpus file
/// // let corpus = Corpus::from_reader(corpus_reader).unwrap();
/// // let model = trainer.train(corpus).unwrap();
/// ```
pub struct Trainer {
    config: TrainerConfig,

    /// Unknown-word grouping cap, with the meaning of the runtime's
    /// `max_grouping_len` (MeCab's `max-grouping-size`): a grouped candidate
    /// with more than this many characters beyond the first is not created
    /// at its start position, where the one-character candidate and the
    /// length ladder stand in. `None` (the default) leaves grouping
    /// unbounded, as the runtime's default does.
    max_grouping_len: Option<usize>,

    provider: lindera_crf::FeatureProvider,

    /// The CRF label of every `unk.def` entry, indexed by unknown-word entry
    /// id (the entry's index in `TrainerConfig::unk_entries`).
    unk_entry_labels: Vec<NonZeroU32>,

    /// Labels created for gold tokens that no candidate matches, as MeCab's
    /// learner adds a virtual node for them, keyed by the gold feature
    /// string so that a repeated token reuses its label. They are trained
    /// but never exported.
    virtual_labels: HashMap<String, NonZeroU32>,

    /// The feature string of every virtual label, in label order. The
    /// virtual labels follow the seed and `unk.def` labels; their feature
    /// strings name the context ids only they use in `left-id.def` and
    /// `right-id.def`.
    virtual_features: Vec<String>,

    /// Reusable lattice that lists the unknown-word candidates of a sentence
    /// (`Lattice::unknown_word_candidates`).
    candidate_lattice: lindera_dictionary::viterbi::Lattice,

    /// Reusable buffer for the unknown-word candidates of a sentence.
    unknown_candidates: Vec<UnknownWordCandidate>,

    /// Reusable buffers for the rewritten features of a label
    /// (`label_feature_set`).
    rewrite_buffers: RewriteBuffers,

    regularization_cost: f64,
    use_l2: bool,
    elastic_net_l1_ratio: Option<f64>,
    max_iter: u64,
    eta: f64,
    num_threads: usize,

    /// Counts gathered while the training lattices are built, logged once
    /// they are all built.
    stats: LatticeStats,
}

/// Returns the CRF label of a seed lexicon row: its index plus one.
///
/// `Trainer::new` creates the seed rows' labels first, in row order, and
/// the training lexicon uses the row index as the word id, so a lexicon
/// match turns into its label through this function.
///
/// # Arguments
///
/// * `index` - The row index in `TrainerConfig::surfaces`, the word id of
///   the training lexicon.
///
/// # Returns
///
/// The label, or an error when the index does not fit a label.
fn lexicon_label(index: usize) -> Result<NonZeroU32> {
    index
        .checked_add(1)
        .and_then(|label| u32::try_from(label).ok())
        .and_then(NonZeroU32::new)
        .ok_or_else(|| anyhow::anyhow!("seed row {index} has no label"))
}

/// Counts of the gold labels and negative edges of the training lattices.
#[derive(Debug, Default)]
struct LatticeStats {
    /// Gold tokens labelled with a seed row's label.
    gold_lexicon: usize,
    /// Gold tokens labelled with an `unk.def` entry's label.
    gold_unknown: usize,
    /// Gold tokens labelled with a virtual label.
    gold_virtual: usize,
    /// Negative edges from the seed lexicon.
    negative_lexicon: usize,
    /// Negative edges from the unknown-word candidates.
    negative_unknown: usize,
    /// Gold tokens that are not seed words although seed rows with their
    /// span start at their position: seed rows have their surface, but none
    /// has their whole feature string. They are also counted in
    /// `gold_unknown` or `gold_virtual`.
    gold_feature_mismatch: usize,
    /// The first of the `gold_feature_mismatch` tokens, in corpus order.
    first_feature_mismatch: Option<FeatureMismatch>,
}

/// The most seed rows [`LatticeStats::seed_word_warning`] shows next to a
/// gold token.
const MAX_SHOWN_SEED_ROWS: usize = 5;

/// A gold token whose surface has seed rows, none of which has its whole
/// feature string.
#[derive(Debug)]
struct FeatureMismatch {
    /// The gold token's surface.
    surface: String,
    /// The gold token's feature string.
    feature: String,
    /// The feature strings of the first seed rows with the token's surface,
    /// at most [`MAX_SHOWN_SEED_ROWS`], in seed lexicon order.
    seed_features: Vec<String>,
    /// The number of seed rows with the token's surface.
    seed_rows: usize,
}

impl LatticeStats {
    /// Returns the warning that few gold tokens are seed words, as when the
    /// corpus does not give its gold tokens the feature strings of the seed
    /// rows (#1259).
    ///
    /// There is one when there are gold tokens and either none is a seed
    /// word, or the gold tokens whose surface has seed rows but none of
    /// their feature strings (`gold_feature_mismatch`) outnumber the seed
    /// words: fewer than half of the gold tokens whose surface is a seed
    /// word are seed words. The share of all the gold tokens is not the
    /// test, since it is also small when the seed lexicon covers little of
    /// the corpus.
    ///
    /// # Returns
    ///
    /// The warning, in lines without a trailing line break, or `None`.
    fn seed_word_warning(&self) -> Option<String> {
        let gold = self.gold_lexicon + self.gold_unknown + self.gold_virtual;
        if gold == 0 || (self.gold_lexicon > 0 && self.gold_feature_mismatch <= self.gold_lexicon) {
            return None;
        }
        // `first_feature_mismatch` is set exactly when
        // `gold_feature_mismatch` is not 0.
        let Some(mismatch) = &self.first_feature_mismatch else {
            return Some(format!(
                "Warning: none of the {gold} gold tokens is a seed word, and no seed row has the \
                 surface of one. The seed rows are only wrong candidates in training."
            ));
        };
        let mut lines = vec![
            format!(
                "Warning: {} of the {gold} gold tokens are seed words, and {} are not although \
                 seed rows have their surface. A gold token is a seed word only when a seed row \
                 has its surface and its whole feature string, as the seed lexicon writes it; \
                 otherwise the seed rows with its surface are only wrong candidates at its \
                 position. The first such gold token, and the seed rows with its surface:",
                self.gold_lexicon, self.gold_feature_mismatch
            ),
            format!("  gold token: {}\t{}", mismatch.surface, mismatch.feature),
        ];
        lines.extend(
            mismatch
                .seed_features
                .iter()
                .map(|feature| format!("  seed row:   {}\t{feature}", mismatch.surface)),
        );
        let others = mismatch.seed_rows - mismatch.seed_features.len();
        if others > 0 {
            lines.push(format!("  and {others} more"));
        }
        Some(lines.join("\n"))
    }
}

/// The `%t` value of a virtual label: MeCab allocates its virtual node
/// zeroed, so the node's `char_type` is 0.
const VIRTUAL_CHAR_TYPE: u32 = 0;

/// Returns whether an `unk.def` entry accepts a gold feature string: every
/// column of the entry's feature string that is not `*` equals the gold
/// column at the same index.
///
/// Both feature strings are split into columns as MeCab splits a feature,
/// so a quoted column with `,` in it is one column (#1184).
///
/// # Arguments
///
/// * `entry_feature` - The entry's feature string.
/// * `gold_feature` - The gold token's feature string.
///
/// # Returns
///
/// `true` when every column of the entry other than `*` matches.
fn unk_entry_accepts(entry_feature: &str, gold_feature: &str) -> bool {
    let gold_columns = split_feature(gold_feature);
    split_feature(entry_feature)
        .iter()
        .enumerate()
        .all(|(index, column)| column == "*" || gold_columns.get(index) == Some(column))
}

/// Returns the candidates that start at a position.
///
/// # Arguments
///
/// * `candidates` - Unknown-word candidates in increasing start order, as
///   `Lattice::unknown_word_candidates` lists them.
/// * `pos` - The start position, in characters.
///
/// # Returns
///
/// The candidates starting at `pos`, empty when there are none.
fn candidates_at(candidates: &[UnknownWordCandidate], pos: usize) -> &[UnknownWordCandidate] {
    let from = candidates.partition_point(|candidate| candidate.start_char < pos);
    let to = candidates.partition_point(|candidate| candidate.start_char <= pos);
    &candidates[from..to]
}

/// Builds the CRF feature set of a label from its feature string.
///
/// # Arguments
///
/// * `rewriter` - Rewrites the feature string into its unigram, left and
///   right forms.
/// * `buffers` - Where the rewritten features go; reused from one label to
///   the next, so that the rewrite allocates only when a buffer grows
///   (#1286).
/// * `extractor` - Expands the feature templates; it assigns ids to the
///   features it has not seen.
/// * `feature` - The label's feature string.
/// * `char_type` - The `%t` value.
/// * `surface` - The `%w` value, if any.
///
/// # Returns
///
/// The feature set. Its left and right IDs are shared with the other labels
/// that have the same rewritten left and right features (see
/// `FeatureExtractor::bigram_feature_ids`, #1263).
///
/// # Errors
///
/// Returns an error when a feature id map of `extractor` has given out all
/// the ids of `u32` and had none for a new feature (#1286).
fn label_feature_set(
    rewriter: &DictionaryRewriter,
    buffers: &mut RewriteBuffers,
    extractor: &mut FeatureExtractor,
    feature: &str,
    char_type: u32,
    surface: Option<&str>,
) -> Result<lindera_crf::FeatureSet> {
    // The rewrite is uncached on purpose: real feature strings are unique per
    // seed row, so a cache keyed on them never hits (#975). The rewritten
    // left and right features are not unique, and the IDs made from them are
    // cached by `bigram_feature_ids`.
    rewriter.rewrite_into(feature, buffers);
    let RewriteBuffers {
        ufeature,
        lfeature,
        rfeature,
    } = &*buffers;
    // Split as MeCab splits the rewritten features for its templates, so a
    // quoted column with `,` in it is one column (#1184).
    let u_columns = split_feature(ufeature);
    let u_vec: Vec<&str> = u_columns.iter().map(|column| column.as_ref()).collect();
    let ctx = TemplateContext {
        surface,
        ufeature: Some(ufeature.as_str()),
        lfeature: Some(lfeature.as_str()),
        rfeature: Some(rfeature.as_str()),
    };
    let unigram_ids = extractor.extract_unigram_feature_ids_with_ctx(&u_vec, char_type, &ctx);
    let (left_ids, right_ids) = extractor.bigram_feature_ids(lfeature, rfeature);
    if extractor.ids_exhausted() {
        return Err(anyhow::anyhow!(
            "too many distinct features: a feature id map has given out all {} ids",
            u32::MAX
        ));
    }
    Ok(lindera_crf::FeatureSet::with_shared_bigrams(
        &unigram_ids,
        right_ids,
        left_ids,
    ))
}

/// Returns the warning that training writes on stderr after the optimizer
/// stopped for `stop_reason`, if any (#1268).
///
/// # Arguments
///
/// * `stop_reason` - Why the optimizer stopped.
///
/// # Returns
///
/// The warning when an iteration left the objective unchanged or the line
/// search failed far from the optimum, and `None` when the optimizer
/// converged, near the optimum included (#1300), or reached the iteration
/// cap.
fn optimizer_warning(stop_reason: lindera_crf::StopReason) -> Option<&'static str> {
    match stop_reason {
        lindera_crf::StopReason::NoProgress => Some(
            "Warning: the optimizer could not make progress; after few iterations the model is barely trained",
        ),
        lindera_crf::StopReason::LineSearchFailed => Some(
            "Warning: the line search failed; the model has the best weights the optimizer reached",
        ),
        _ => None,
    }
}

impl Trainer {
    /// Creates a new [`Trainer`] using the specified configuration.
    ///
    /// Creates one CRF label per seed row, in row order (label `row + 1`),
    /// then one per `unk.def` entry, in file order, whose `%t` is the id of
    /// the entry's category (#1110).
    ///
    /// # Arguments
    ///
    /// * `config` - The training configuration.
    ///
    /// # Returns
    ///
    /// The trainer, or an error when a label cannot be created.
    pub fn new(mut config: TrainerConfig) -> Result<Self> {
        let mut provider = lindera_crf::FeatureProvider::default();
        let mut rewrite_buffers = RewriteBuffers::default();

        // Generate default features based on dictionary schema
        let default_features = config.features.first().map_or_else(
            || "*".to_string(),
            |first_feature| wildcard_feature(first_feature),
        );

        for (i, surface) in config.surfaces.iter().enumerate() {
            // Get feature string from config.features (parallel to surfaces)
            let feature_str = config
                .features
                .get(i)
                .map_or(default_features.as_str(), String::as_str);

            // The %t feature: the default category (MeCab's default_type) of
            // the first character of the surface, or 0 (DEFAULT) if none
            let cate_id = config.surface_category(surface);

            let feature_set = label_feature_set(
                &config.dictionary_rewriter,
                &mut rewrite_buffers,
                &mut config.feature_extractor,
                feature_str,
                cate_id,
                Some(surface),
            )?;

            // Add feature set to provider and get label ID. `build_lattice`
            // finds a seed row by its word id in the training lexicon, the
            // row index, and labels it `index + 1` (see `lexicon_label`).
            let label_id = provider.add_feature_set(feature_set)?;
            if lexicon_label(i)? != label_id {
                return Err(anyhow::anyhow!(
                    "seed row {i} got label {label_id}, not its index plus one"
                ));
            }
        }

        // One label per unk.def entry, in file order (#1110). %t is the id of
        // the entry's category, the default category of the characters whose
        // unknown words the entry creates. `TrainerConfig::from_readers`
        // rejects an entry whose category char.def does not define (#1195);
        // such an entry would never become a candidate, and gets 0.
        let mut unk_entry_labels = Vec::with_capacity(config.unk_entries.len());
        for (category, feature) in &config.unk_entries {
            let char_type = config
                .dict
                .character_definition
                .category_id_by_name(category)
                .map_or(0, |category_id| category_id.0 as u32);
            let feature_set = label_feature_set(
                &config.dictionary_rewriter,
                &mut rewrite_buffers,
                &mut config.feature_extractor,
                feature,
                char_type,
                None,
            )?;
            unk_entry_labels.push(provider.add_feature_set(feature_set)?);
        }

        Ok(Self {
            config,
            max_grouping_len: None,
            provider,
            unk_entry_labels,
            virtual_labels: HashMap::new(),
            virtual_features: Vec::new(),
            candidate_lattice: lindera_dictionary::viterbi::Lattice::default(),
            unknown_candidates: Vec::new(),
            rewrite_buffers,
            regularization_cost: DEFAULT_LAMBDA,
            use_l2: true,
            elastic_net_l1_ratio: None,
            max_iter: 100,
            eta: DEFAULT_ETA,
            num_threads: 8,
            stats: LatticeStats::default(),
        })
    }

    /// Sets the regularization cost coefficient (>= 0). The default is
    /// [`DEFAULT_LAMBDA`], whatever the regularization type.
    pub fn regularization_cost(mut self, cost: f64) -> Self {
        self.regularization_cost = cost;
        self
    }

    /// Sets whether to use L2 regularization (default: `true`); `false`
    /// selects L1.
    pub fn use_l2(mut self, l2: bool) -> Self {
        self.use_l2 = l2;
        self
    }

    /// Sets the Elastic Net L1 ratio (0.0 to 1.0). Unset by default.
    /// When set, uses Elastic Net regularization instead of pure L1 or L2,
    /// whatever [`Self::use_l2`] says.
    /// l1_ratio=1.0 is pure L1, l1_ratio=0.0 is pure L2.
    pub fn elastic_net_l1_ratio(mut self, ratio: f64) -> Self {
        self.elastic_net_l1_ratio = Some(ratio);
        self
    }

    /// Sets the regularization type, as `lindera train --regularization`
    /// and the bindings do: [`RegularizationType::L1`] and
    /// [`RegularizationType::L2`] set [`Self::use_l2`] and clear an Elastic
    /// Net ratio set before, and [`RegularizationType::ElasticNet`] sets
    /// [`Self::elastic_net_l1_ratio`]. The ratio is checked when training
    /// starts: it must be between 0.0 and 1.0.
    ///
    /// # Arguments
    ///
    /// * `kind` - The type.
    /// * `elastic_net_l1_ratio` - The L1 ratio, used only with
    ///   [`RegularizationType::ElasticNet`]; [`DEFAULT_ELASTIC_NET_L1_RATIO`]
    ///   is the default of the CLI and the bindings.
    ///
    /// # Returns
    ///
    /// The trainer.
    pub fn regularization_type(
        mut self,
        kind: RegularizationType,
        elastic_net_l1_ratio: f64,
    ) -> Self {
        match kind {
            RegularizationType::L1 | RegularizationType::L2 => {
                self.use_l2 = kind == RegularizationType::L2;
                self.elastic_net_l1_ratio = None;
            }
            RegularizationType::ElasticNet => {
                self.elastic_net_l1_ratio = Some(elastic_net_l1_ratio);
            }
        }
        self
    }

    /// Sets the maximum number of iterations.
    pub fn max_iter(mut self, iter: u64) -> Self {
        self.max_iter = iter;
        self
    }

    /// Sets the tolerance of the convergence test: training stops when the
    /// relative change of the objective stays below `eta` for 3 iterations
    /// in a row, before the cap set with [`Self::max_iter`]. The default is
    /// [`DEFAULT_ETA`]; `0.0` turns the test off. The value is checked when
    /// training starts: it must be finite and >= 0.
    ///
    /// # Arguments
    ///
    /// * `eta` - The tolerance.
    ///
    /// # Returns
    ///
    /// The trainer.
    pub fn eta(mut self, eta: f64) -> Self {
        self.eta = eta;
        self
    }

    /// Sets the number of threads for training.
    pub fn num_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Sets the unknown-word grouping cap, with the meaning of the runtime's
    /// `max_grouping_len`: a grouped candidate with more than `len`
    /// characters beyond the first is not created at its start position,
    /// where the one-character candidate and the length ladder stand in.
    /// Train with the value the dictionary will be used with.
    ///
    /// # Arguments
    ///
    /// * `len` - The cap, or `None` (the default) for unbounded grouping.
    ///
    /// # Returns
    ///
    /// The trainer.
    pub fn max_grouping_len(mut self, len: Option<usize>) -> Self {
        self.max_grouping_len = len;
        self
    }

    /// Get the regularization cost (lambda)
    pub fn get_regularization_cost(&self) -> f64 {
        self.regularization_cost
    }

    /// Get the maximum number of iterations
    pub fn get_max_iter(&self) -> u64 {
        self.max_iter
    }

    /// Get the tolerance of the convergence test
    pub fn get_eta(&self) -> f64 {
        self.eta
    }

    /// Get the number of threads
    pub fn get_num_threads(&self) -> usize {
        self.num_threads
    }

    /// Trains a model from the given corpus.
    ///
    /// Once the training lattices are built, it prints how their gold tokens
    /// were labelled, and warns on stderr when none of them, or fewer than
    /// half of those whose surface has seed rows, are seed words: a gold
    /// token is a seed word only when a seed row has its surface and its
    /// whole feature string (#1259). Once the optimizer stops, it prints why,
    /// and warns on stderr when an iteration left the objective unchanged or
    /// the line search failed far from the optimum (#1268, #1300).
    pub fn train(mut self, corpus: Corpus) -> Result<Model> {
        let lattices = self.build_lattices_from_corpus(&corpus)?;
        let labels = self.extract_labels();
        let (crf_model, iterations) = self.train_crf_model(lattices)?;

        self.create_final_model(crf_model, labels, corpus, iterations)
    }

    /// Build feature lattices from the training corpus
    fn build_lattices_from_corpus(&mut self, corpus: &Corpus) -> Result<Vec<lindera_crf::Lattice>> {
        log_info!("Building feature lattices...");

        let mut lattices = Vec::new();
        for (i, example) in corpus.examples.iter().enumerate() {
            log_progress!("Processing example {}/{}", i + 1, corpus.examples.len());

            // NOTE: Character-level processing is performed here during sentence compilation
            // In Lindera, character property processing should be handled differently
            // For now, we proceed with the existing approach

            let lattice = self.build_lattice(example)?;
            lattices.push(lattice);
        }

        let stats = &self.stats;
        log_info!(
            "Lattice statistics: gold_lexicon={} gold_unknown={} gold_virtual={} virtual_labels={} negative_lexicon={} negative_unknown={}",
            stats.gold_lexicon,
            stats.gold_unknown,
            stats.gold_virtual,
            self.virtual_features.len(),
            stats.negative_lexicon,
            stats.negative_unknown
        );
        if let Some(warning) = stats.seed_word_warning() {
            log_warn!("{warning}");
        }

        Ok(lattices)
    }

    /// Returns the regularization training uses, as lindera-crf takes it:
    /// Elastic Net when a ratio was set with [`Self::elastic_net_l1_ratio`]
    /// or [`Self::regularization_type`], whatever [`Self::use_l2`] says, and
    /// otherwise L2 or L1. The trained model records it (#1272).
    ///
    /// # Returns
    ///
    /// The regularization, with the Elastic Net L1 ratio.
    fn regularization(&self) -> lindera_crf::Regularization {
        match self.elastic_net_l1_ratio {
            Some(l1_ratio) => lindera_crf::Regularization::ElasticNet { l1_ratio },
            None if self.use_l2 => lindera_crf::Regularization::L2,
            None => lindera_crf::Regularization::L1,
        }
    }

    /// Configure and execute CRF training
    ///
    /// # Arguments
    ///
    /// * `lattices` - The training lattices.
    ///
    /// # Returns
    ///
    /// The trained CRF model and the number of L-BFGS iterations that ran.
    fn train_crf_model(
        &mut self,
        lattices: Vec<lindera_crf::Lattice>,
    ) -> Result<(lindera_crf::RawModel, u64)> {
        log_info!("Starting CRF training with {} lattices...", lattices.len());

        let reg_type = self.regularization();
        let described = match reg_type {
            lindera_crf::Regularization::L1 => "L1".to_string(),
            lindera_crf::Regularization::L2 => "L2".to_string(),
            lindera_crf::Regularization::ElasticNet { l1_ratio } => {
                format!("ElasticNet, l1_ratio={l1_ratio}")
            }
        };
        log_info!(
            "Training parameters: regularization={} ({described}), max_iter={}, eta={}, threads={}",
            self.regularization_cost,
            self.max_iter,
            self.eta,
            self.num_threads
        );

        // Configure the CRF trainer
        let trainer = lindera_crf::Trainer::new()
            .regularization(reg_type, self.regularization_cost)?
            .max_iter(self.max_iter)?
            .eta(self.eta)?
            .n_threads(self.num_threads)?;

        self.execute_training(trainer, lattices)
    }

    /// Execute the actual CRF training with detailed logging
    ///
    /// # Arguments
    ///
    /// * `trainer` - The configured CRF trainer.
    /// * `lattices` - The training lattices.
    ///
    /// # Returns
    ///
    /// The trained CRF model and the number of L-BFGS iterations that ran.
    fn execute_training(
        &mut self,
        trainer: lindera_crf::Trainer,
        lattices: Vec<lindera_crf::Lattice>,
    ) -> Result<(lindera_crf::RawModel, u64)> {
        println!("L-BFGS optimization starting...");
        println!(
            "Note: This may take several minutes for large datasets. Progress will be shown by L-BFGS iterations above."
        );
        println!("Each 'iter:' line indicates training progress. Please wait...");

        let start_time = std::time::Instant::now();

        // Training with provider (consumes provider as per CRF training requirements)
        let provider = std::mem::take(&mut self.provider);
        let (model, stats) = trainer.train_with_stats(&lattices, provider);
        let training_duration = start_time.elapsed();

        self.log_training_results(&model);
        println!(
            "L-BFGS stopped after {} iterations and {} evaluations of the loss and its gradient: {}",
            stats.iterations, stats.evaluations, stats.stop_reason
        );
        if let Some(warning) = optimizer_warning(stats.stop_reason) {
            log_warn!("{warning}");
        }
        println!(
            "Training completed successfully in {:.2}s!",
            training_duration.as_secs_f64()
        );

        Ok((model, stats.iterations))
    }

    /// Log detailed training results for debugging
    fn log_training_results(&self, model: &lindera_crf::RawModel) {
        log_debug!("Training completed, checking raw model...");
        log_debug!("Raw model weights count: {}", model.weights().len());
        log_debug!(
            "Raw model first 5 weights: {:?}",
            &model.weights()[..std::cmp::min(5, model.weights().len())]
        );

        // Analyze weights for debugging
        let weights = model.weights();
        let nan_count = weights.iter().filter(|&&w| w.is_nan()).count();
        let inf_count = weights.iter().filter(|&&w| w.is_infinite()).count();
        let zero_count = weights.iter().filter(|&&w| w == 0.0).count();
        log_debug!(
            "Weight analysis - NaN: {}, Inf: {}, Zero: {}, Total: {}",
            nan_count,
            inf_count,
            zero_count,
            weights.len()
        );

        let weight_sum: f64 = weights.iter().sum();
        log_debug!("Sum of all weights: {:.16}", weight_sum);

        log_debug!(
            "Model unigram_weight_indices len: {}",
            model.unigram_weight_indices().len()
        );
        log_debug!(
            "Model bigram_weight_indices len: {}",
            model.bigram_weight_indices().len()
        );
    }

    /// Create the final trained model from CRF results
    ///
    /// # Arguments
    ///
    /// * `crf_model` - The trained CRF model.
    /// * `labels` - The exported labels.
    /// * `_corpus` - The training corpus.
    /// * `iterations` - The number of L-BFGS iterations that ran.
    ///
    /// # Returns
    ///
    /// The trained model.
    fn create_final_model(
        mut self,
        crf_model: lindera_crf::RawModel,
        labels: Vec<String>,
        _corpus: Corpus,
        iterations: u64,
    ) -> Result<Model> {
        // Remove unused features from feature extractor to optimize model size
        self.remove_unused_features(&crf_model);

        // Extract feature weights from the trained model
        let feature_weights = self.extract_feature_weights(&crf_model);

        // Create final model with metadata
        let regularization = self.regularization();
        Ok(Model::new_with_metadata(
            crf_model,
            self.config,
            feature_weights,
            labels,
            self.virtual_features,
            regularization,
            self.regularization_cost,
            iterations,
        ))
    }

    /// Extract feature weights from the trained CRF model
    fn extract_feature_weights(&self, crf_model: &lindera_crf::RawModel) -> Vec<f64> {
        log_info!("Extracting feature weights from trained model...");

        let mut feature_weights = Vec::new();
        match crf_model.merge() {
            Ok(merged_model) => {
                // Extract weights in the order of surfaces
                for (i, _surface) in self.config.surfaces.iter().enumerate() {
                    if i < merged_model.feature_sets.len() {
                        feature_weights.push(merged_model.feature_sets[i].weight);
                    } else {
                        feature_weights.push(0.0);
                    }
                }

                let non_zero_count = feature_weights.iter().filter(|&&w| w != 0.0).count();
                log_info!(
                    "Extracted {} weights ({} non-zero) from merged model",
                    feature_weights.len(),
                    non_zero_count
                );
            }
            Err(e) => {
                log_info!("Model merge failed: {e}, falling back to raw weights");
                self.use_raw_weights(crf_model, &mut feature_weights);
            }
        }

        feature_weights
    }

    /// Use raw CRF model weights as fallback
    fn use_raw_weights(&self, crf_model: &lindera_crf::RawModel, feature_weights: &mut Vec<f64>) {
        let raw_weights = crf_model.weights();

        for i in 0..self.config.surfaces.len() {
            if i < raw_weights.len() {
                feature_weights.push(raw_weights[i]);
            } else {
                feature_weights.push(0.0);
            }
        }
        log_info!("Used {} raw model weights", feature_weights.len());
    }

    /// Extracts the exported labels: the seed surfaces, then the category
    /// name of every `unk.def` entry in file order. The virtual labels follow
    /// them in the CRF model but are not exported, so they have no name here.
    ///
    /// # Returns
    ///
    /// The label names, in label order.
    fn extract_labels(&self) -> Vec<String> {
        let mut labels = self.config.surfaces.clone();
        labels.extend(
            self.config
                .unk_entries
                .iter()
                .map(|(category, _)| category.clone()),
        );
        labels
    }

    /// Returns the CRF label of an `unk.def` entry.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The unknown-word entry id.
    ///
    /// # Returns
    ///
    /// The label, or an error when the entry has none.
    fn unk_entry_label(&self, word_id: u32) -> Result<NonZeroU32> {
        self.unk_entry_labels
            .get(word_id as usize)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("unk.def entry {word_id} has no label"))
    }

    /// Returns the virtual label of a gold feature string, creating it on
    /// first use. Its feature set is built from the gold feature string as a
    /// seed row's is, but with `%t` = 0 and no `%w`, as MeCab builds the
    /// features of its virtual node; without the surface, the feature string
    /// alone decides the features, so it can key the cache.
    ///
    /// # Arguments
    ///
    /// * `feature` - The gold token's feature string.
    ///
    /// # Returns
    ///
    /// The label, or an error when it cannot be created.
    fn virtual_label(&mut self, feature: &str) -> Result<NonZeroU32> {
        if let Some(&label) = self.virtual_labels.get(feature) {
            return Ok(label);
        }
        let feature_set = label_feature_set(
            &self.config.dictionary_rewriter,
            &mut self.rewrite_buffers,
            &mut self.config.feature_extractor,
            feature,
            VIRTUAL_CHAR_TYPE,
            None,
        )?;
        let label = self.provider.add_feature_set(feature_set)?;
        self.virtual_labels.insert(feature.to_string(), label);
        self.virtual_features.push(feature.to_string());
        Ok(label)
    }

    /// Counts a gold token that no seed row labels, in
    /// `LatticeStats::gold_feature_mismatch`, when seed rows with its span
    /// start at its position, and keeps the first such token with the
    /// feature strings of those rows (#1259).
    ///
    /// # Arguments
    ///
    /// * `matches` - The seed words that start at the token's position.
    /// * `token` - The gold token.
    /// * `token_len` - The token's length, in characters.
    fn count_feature_mismatch(&mut self, matches: &[Match], token: &Word, token_len: usize) {
        // The training lexicon lists the rows of a surface in seed order.
        let same_span = || matches.iter().filter(move |m| m.end_char == token_len);
        let seed_rows = same_span().count();
        if seed_rows == 0 {
            return;
        }
        self.stats.gold_feature_mismatch += 1;
        if self.stats.first_feature_mismatch.is_some() {
            return;
        }
        let seed_features = same_span()
            .take(MAX_SHOWN_SEED_ROWS)
            .filter_map(|m| {
                self.config
                    .features
                    .get(m.word_idx.word_id as usize)
                    .cloned()
            })
            .collect();
        self.stats.first_feature_mismatch = Some(FeatureMismatch {
            surface: token.surface().to_string(),
            feature: token.feature().to_string(),
            seed_features,
            seed_rows,
        });
    }

    /// Builds the CRF lattice of one example, as MeCab's learner builds its
    /// lattice (#1110).
    ///
    /// The candidates are the seed words at every position and the unknown
    /// words the runtime creates there (`Lattice::unknown_word_candidates`:
    /// every `unk.def` entry of the default category). Each gold token is
    /// labelled, in this order, with the first seed row of the same span and
    /// the same feature string, else with an unknown-word candidate of the
    /// same span whose entry accepts the gold feature string
    /// ([`unk_entry_accepts`]; the first such entry in `unk.def` order, the
    /// node MeCab's learner takes), else with a virtual label built from the
    /// gold feature string. Every other candidate is a negative edge. A gold
    /// token that no seed row labels although seed rows with its span start
    /// at its position is counted for [`LatticeStats::seed_word_warning`].
    ///
    /// # Arguments
    ///
    /// * `example` - The example.
    ///
    /// # Returns
    ///
    /// The lattice, with the gold edge first at each of its positions.
    fn build_lattice(&mut self, example: &Example) -> Result<lindera_crf::Lattice> {
        use lindera_crf::{Edge, Lattice};

        let input_chars: Vec<char> = example.sentence.chars().collect();
        let input_len = input_chars.len();

        // The seed-lexicon words that start at each position (#1164): the
        // gold tokens are looked up among them, and the others become
        // negative edges.
        let lexicon: Vec<_> = (0..input_len)
            .map(|pos| {
                self.config
                    .system_lexicon()
                    .common_prefix_iterator(&input_chars[pos..])
            })
            .collect();

        // The unknown words the runtime creates in this sentence, in
        // increasing start order (#1110).
        let dict = &self.config.dict;
        self.candidate_lattice.unknown_word_candidates(
            &dict.prefix_dictionary,
            &dict.character_definition,
            &dict.unknown_dictionary,
            &example.sentence,
            self.max_grouping_len,
            &mut self.unknown_candidates,
        );

        // Add positive edges from training data
        let mut edges = Vec::with_capacity(example.tokens.len());
        let mut pos = 0;
        for token in &example.tokens {
            let token_len = token.surface().chars().count();
            let end = pos + token_len;
            let feature = token.feature();

            // A gold token is a seed word when a seed row with its span and
            // its whole feature starts here. The first such row in seed order
            // gives the label, as MeCab's learner takes the first matching
            // dictionary node; matching the feature alone could pick another
            // word's row.
            let lexicon_match = lexicon.get(pos).and_then(|matches| {
                matches.iter().find(|m| {
                    m.end_char == token_len
                        && self
                            .config
                            .features
                            .get(m.word_idx.word_id as usize)
                            .is_some_and(|seed_feature| seed_feature == feature)
                })
            });
            let label_id = if let Some(m) = lexicon_match {
                self.stats.gold_lexicon += 1;
                lexicon_label(m.word_idx.word_id as usize)?
            } else {
                if let Some(matches) = lexicon.get(pos) {
                    self.count_feature_mismatch(matches, token, token_len);
                }
                // Otherwise an unknown-word candidate of the same span whose
                // entry accepts the gold feature string: the first such
                // entry in unk.def order. MeCab's learner takes the matching
                // node it created first, and it creates them entry by entry.
                let unknown_match = candidates_at(&self.unknown_candidates, pos)
                    .iter()
                    .filter(|candidate| candidate.end_char == end)
                    .map(|candidate| candidate.word_id)
                    .filter(|&word_id| {
                        self.config
                            .unk_entries
                            .get(word_id as usize)
                            .is_some_and(|(_, entry)| unk_entry_accepts(entry, feature))
                    })
                    .min();
                if let Some(word_id) = unknown_match {
                    self.stats.gold_unknown += 1;
                    self.unk_entry_label(word_id)?
                } else {
                    // No candidate: a virtual label, as MeCab's learner adds
                    // a virtual node.
                    self.stats.gold_virtual += 1;
                    self.virtual_label(feature)?
                }
            };

            edges.push((pos, Edge::new(end, label_id)));
            pos = end;
        }
        if pos != input_len {
            return Err(anyhow::anyhow!(
                "the gold tokens cover {pos} of the {input_len} characters of the sentence"
            ));
        }

        let mut lattice = Lattice::new(input_len)?;

        // Add positive edges to lattice
        for (pos, edge) in edges {
            lattice.add_edge(pos, edge)?;
        }

        // Add negative edges: the seed words, then the unknown words, except
        // the gold edge
        for (pos, matches) in lexicon.iter().enumerate() {
            let gold = lattice.nodes()[pos].edges().first().copied();

            for m in matches {
                let edge = Edge::new(
                    pos + m.end_char,
                    lexicon_label(m.word_idx.word_id as usize)?,
                );
                if Some(edge) == gold {
                    continue;
                }
                lattice.add_edge(pos, edge)?;
                self.stats.negative_lexicon += 1;
            }

            for candidate in candidates_at(&self.unknown_candidates, pos) {
                let edge = Edge::new(candidate.end_char, self.unk_entry_label(candidate.word_id)?);
                if Some(edge) == gold {
                    continue;
                }
                lattice.add_edge(pos, edge)?;
                self.stats.negative_unknown += 1;
            }
        }

        Ok(lattice)
    }

    /// Remove unused features from the feature extractor to optimize the model size and performance
    fn remove_unused_features(&mut self, model: &lindera_crf::RawModel) {
        println!("Removing unused features...");

        use std::collections::HashSet;

        // Collect used right features from bigram weights
        let mut used_right_features = HashSet::new();
        for feature_ids in model.bigram_weight_indices() {
            for (feature_id, _) in feature_ids {
                used_right_features.insert(*feature_id);
            }
        }

        // A unigram feature is unused when it has no weight. A left feature
        // is unused when its row of bigram weights is empty; one past the
        // rows is kept. A right feature is unused when no row names it. The
        // cached left and right IDs, which may name the features removed, are
        // forgotten in the same call.
        self.config.feature_extractor.retain_features(
            |id| {
                model
                    .unigram_weight_indices()
                    .get((id.get() - 1) as usize)
                    .copied()
                    .flatten()
                    .is_some()
            },
            |id| {
                !model
                    .bigram_weight_indices()
                    .get(id.get() as usize)
                    .is_some_and(|row| row.is_empty())
            },
            |id| used_right_features.contains(&id.get()),
        );

        println!("Feature cleanup completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // `regularization_type` maps each type to the settings that
    // `train_crf_model` reads: L1 and L2 clear a ratio set before, so the
    // last call decides the type, as the CLI's one flag does (#1262).
    #[test]
    fn test_regularization_type_sets_the_type_and_the_ratio() {
        let trainer = || {
            let config = TrainerConfig::from_readers(
                Cursor::new("これ,0,0,1000,連体詞,*,*,*,*,*,これ,コレ,コレ\n"),
                Cursor::new("DEFAULT 0 1 0\nHIRAGANA 1 1 0\n0x3042..0x3096 HIRAGANA\n"),
                Cursor::new(
                    "DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n\
                     HIRAGANA,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n",
                ),
                Cursor::new("UNIGRAM:%F[0]\n"),
                Cursor::new("*\tUNK\n"),
            )
            .unwrap();
            Trainer::new(config).unwrap()
        };

        let l1 = trainer().regularization_type(RegularizationType::L1, 0.3);
        assert!(!l1.use_l2);
        assert_eq!(l1.elastic_net_l1_ratio, None);
        assert!(matches!(
            l1.regularization(),
            lindera_crf::Regularization::L1
        ));

        let l2 = trainer()
            .use_l2(false)
            .regularization_type(RegularizationType::L2, 0.3);
        assert!(l2.use_l2);
        assert_eq!(l2.elastic_net_l1_ratio, None);
        assert!(matches!(
            l2.regularization(),
            lindera_crf::Regularization::L2
        ));

        let elastic_net = trainer().regularization_type(RegularizationType::ElasticNet, 0.3);
        assert_eq!(elastic_net.elastic_net_l1_ratio, Some(0.3));
        // Elastic Net wins over `use_l2`, which a new trainer sets.
        assert!(matches!(
            elastic_net.regularization(),
            lindera_crf::Regularization::ElasticNet { l1_ratio } if l1_ratio == 0.3
        ));

        let back_to_l1 = trainer()
            .regularization_type(RegularizationType::ElasticNet, 0.3)
            .regularization_type(RegularizationType::L1, 0.3);
        assert!(!back_to_l1.use_l2);
        assert_eq!(back_to_l1.elastic_net_l1_ratio, None);
        assert!(matches!(
            back_to_l1.regularization(),
            lindera_crf::Regularization::L1
        ));
    }

    #[test]
    fn test_train_with_bigram_template() {
        let seed_csv = "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\nは,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\nテスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n";
        let char_def = "DEFAULT 0 1 0\nHIRAGANA 1 1 0\nKATAKANA 1 1 0\nKANJI 0 0 2\nALPHA 1 1 0\nNUMERIC 1 1 0\n\n0x3041..0x3096 HIRAGANA\n0x30A1..0x30F6 KATAKANA\n0x4E00..0x9FAF KANJI\n0x0030..0x0039 NUMERIC\n0x0041..0x005A ALPHA\n0x0061..0x007A ALPHA\n";
        let unk_def = "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\nHIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\nKATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\nKANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*\nALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*\nNUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*\n";
        let feature_def = "UNIGRAM U00:%F[0]\nUNIGRAM U01:%F[0],%F?[1]\nBIGRAM B00:%L[0]/%R[0]\n";
        let rewrite_def = "名詞,一般\tNOUN,GENERAL\n";
        let corpus_text = "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\nは\t助詞,係助詞,*,*,*,*,は,ハ,ワ\nテスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\nEOS\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        let trainer = Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(false)
            .max_iter(5)
            .num_threads(1);

        let corpus = Corpus::from_reader(Cursor::new(corpus_text)).unwrap();

        let model = trainer.train(corpus).unwrap();

        // Verify model can be serialized and deserialized
        let mut buf = Vec::new();
        model.write_model(&mut buf).unwrap();
        assert!(!buf.is_empty(), "Serialized model should not be empty");

        let serializable: model::SerializableModel =
            model::Model::read_model(Cursor::new(&buf)).unwrap();

        // Verify labels match seed data (surfaces are encoded as labels)
        assert!(
            !serializable.labels.is_empty(),
            "Labels should not be empty"
        );

        // Verify connection costs are generated (dense matrix)
        assert!(
            !serializable.connection_matrix.is_empty(),
            "Connection matrix should not be empty"
        );

        // Verify left/right context ID mappings exist
        assert!(
            !serializable.left_id_map.is_empty(),
            "Left ID map should not be empty"
        );
        assert!(
            !serializable.right_id_map.is_empty(),
            "Right ID map should not be empty"
        );

        // Verify unknown word entries are preserved
        assert!(
            !serializable.unk_entries.is_empty(),
            "Unknown word entries should not be empty"
        );

        // Verify cost factor is set
        assert!(
            serializable.cost_factor > 0,
            "Cost factor should be positive"
        );

        // === Verify all exported dictionary files ===

        // 1. lex.csv: should contain all seed entries with costs
        let mut lex_buf = Vec::new();
        serializable.write_lexicon(&mut lex_buf).unwrap();
        let lex_content = String::from_utf8(lex_buf).unwrap();
        assert!(!lex_content.is_empty(), "lex.csv should not be empty");
        for surface in &["これ", "は", "テスト"] {
            assert!(
                lex_content.contains(surface),
                "lex.csv should contain {surface}"
            );
        }
        // Each line should have CSV format: surface,left_id,right_id,cost,features...
        for line in lex_content.lines() {
            let fields: Vec<&str> = line.split(',').collect();
            assert!(
                fields.len() >= 4,
                "lex.csv line should have at least 4 fields: {line}"
            );
            // left_id and right_id should be valid integers
            fields[1]
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid left_id in lex.csv: {line}"));
            fields[2]
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid right_id in lex.csv: {line}"));
            // cost should be a valid integer
            fields[3]
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid cost in lex.csv: {line}"));
        }

        // 2. matrix.def: dense connection cost matrix
        let mut matrix_buf = Vec::new();
        serializable
            .write_connection_costs(&mut matrix_buf)
            .unwrap();
        let matrix_content = String::from_utf8(matrix_buf).unwrap();
        let mut lines = matrix_content.lines();
        let header = lines.next().unwrap();
        let dims: Vec<usize> = header
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(dims.len(), 2, "matrix.def header should have 2 dimensions");
        let (rows, cols) = (dims[0], dims[1]);
        assert!(rows > 0 && cols > 0, "Matrix dimensions should be positive");
        // Each entry: right_id left_id cost
        let mut entry_count = 0;
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                parts.len(),
                3,
                "matrix.def entry should have 3 fields: {line}"
            );
            parts[0]
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("Invalid right_id: {line}"));
            parts[1]
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("Invalid left_id: {line}"));
            parts[2]
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid cost: {line}"));
            entry_count += 1;
        }
        assert_eq!(
            entry_count,
            rows * cols,
            "Dense matrix should have rows*cols entries"
        );

        // 3. unk.def: unknown word definitions
        let mut unk_buf = Vec::new();
        serializable.write_unknown_dictionary(&mut unk_buf).unwrap();
        let unk_content = String::from_utf8(unk_buf).unwrap();
        assert!(!unk_content.is_empty(), "unk.def should not be empty");
        assert!(
            unk_content.contains("DEFAULT"),
            "unk.def should contain DEFAULT category"
        );

        // 4. char.def: character definition
        let mut char_buf = Vec::new();
        serializable.write_char_def(&mut char_buf).unwrap();
        let char_content = String::from_utf8(char_buf).unwrap();
        assert!(!char_content.is_empty(), "char.def should not be empty");

        // 5. feature.def: feature template definition
        let mut feat_buf = Vec::new();
        serializable.write_feature_def(&mut feat_buf).unwrap();
        let feat_content = String::from_utf8(feat_buf).unwrap();
        assert!(!feat_content.is_empty(), "feature.def should not be empty");
        assert!(
            feat_content.contains("UNIGRAM"),
            "feature.def should contain UNIGRAM"
        );
        assert!(
            feat_content.contains("BIGRAM"),
            "feature.def should contain BIGRAM"
        );

        // 6. rewrite.def: rewrite rule definition
        let mut rewrite_buf = Vec::new();
        serializable.write_rewrite_def(&mut rewrite_buf).unwrap();
        let rewrite_content = String::from_utf8(rewrite_buf).unwrap();
        assert!(
            !rewrite_content.is_empty(),
            "rewrite.def should not be empty"
        );

        // 7. left-id.def: left context ID mapping
        let mut left_id_buf = Vec::new();
        serializable.write_left_id_def(&mut left_id_buf).unwrap();
        let left_id_content = String::from_utf8(left_id_buf).unwrap();
        assert!(
            !left_id_content.is_empty(),
            "left-id.def should not be empty"
        );
        // Each line: id feature_string
        for line in left_id_content.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            assert_eq!(
                parts.len(),
                2,
                "left-id.def line should have id and feature: {line}"
            );
            parts[0]
                .parse::<u32>()
                .unwrap_or_else(|_| panic!("Invalid id in left-id.def: {line}"));
        }

        // 8. right-id.def: right context ID mapping
        let mut right_id_buf = Vec::new();
        serializable.write_right_id_def(&mut right_id_buf).unwrap();
        let right_id_content = String::from_utf8(right_id_buf).unwrap();
        assert!(
            !right_id_content.is_empty(),
            "right-id.def should not be empty"
        );
        for line in right_id_content.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            assert_eq!(
                parts.len(),
                2,
                "right-id.def line should have id and feature: {line}"
            );
            parts[0]
                .parse::<u32>()
                .unwrap_or_else(|_| panic!("Invalid id in right-id.def: {line}"));
        }

        // Cross-check: left-id.def count should match matrix columns
        let left_id_count = left_id_content.lines().count();
        assert_eq!(
            left_id_count, cols,
            "left-id.def entries ({left_id_count}) should match matrix columns ({cols})"
        );

        // Cross-check: right-id.def count should match matrix rows
        let right_id_count = right_id_content.lines().count();
        assert_eq!(
            right_id_count, rows,
            "right-id.def entries ({right_id_count}) should match matrix rows ({rows})"
        );
    }

    fn train_fixture_with_feature_def(feature_def: &str) -> model::Model {
        let (seed_csv, char_def, unk_def, _, rewrite_def, corpus_text) = test_fixtures::fixtures();
        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        let trainer = Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(false)
            .max_iter(5)
            .num_threads(1);
        let corpus = Corpus::from_reader(Cursor::new(corpus_text)).unwrap();
        trainer.train(corpus).unwrap()
    }

    fn written(write: impl FnOnce(&mut Vec<u8>) -> anyhow::Result<()>) -> String {
        let mut buf = Vec::new();
        write(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn matrix_header(matrix_def: &str) -> (usize, usize) {
        let header = matrix_def.lines().next().unwrap();
        let dims: Vec<usize> = header
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(dims.len(), 2, "matrix.def header: {header}");
        (dims[0], dims[1])
    }

    /// Asserts that every context id of the exported lexicon and unknown
    /// entries lies inside the exported matrix, as the runtime requires.
    fn assert_entries_inside_matrix(serializable: &model::SerializableModel) {
        let matrix = written(|w| serializable.write_connection_costs(w));
        let (rows, cols) = matrix_header(&matrix);
        assert_eq!(matrix.lines().count() - 1, rows * cols, "dense matrix");

        let left_ids = written(|w| serializable.write_left_id_def(w));
        let right_ids = written(|w| serializable.write_right_id_def(w));
        assert_eq!(left_ids.lines().count(), cols, "left-id.def vs columns");
        assert_eq!(right_ids.lines().count(), rows, "right-id.def vs rows");

        let lexicon = written(|w| serializable.write_lexicon(w));
        let unknown = written(|w| serializable.write_unknown_dictionary(w));
        for line in lexicon.lines().chain(unknown.lines()) {
            let fields: Vec<&str> = line.split(',').collect();
            let left_id: usize = fields[1].parse().unwrap();
            let right_id: usize = fields[2].parse().unwrap();
            assert!(left_id < cols, "left id outside the matrix: {line}");
            assert!(right_id < rows, "right id outside the matrix: {line}");
        }
    }

    #[test]
    fn test_exported_matrix_covers_every_context_id_without_bigram_weights() {
        // Without a BIGRAM template no bigram weight exists: the state that
        // L1 with lambda 0.01, v6's default, reaches on `resources/training/`
        // once every bigram weight is zeroed. The entries still carry context
        // ids, so the matrix must cover them; it used to be written as `0 0`.
        let model = train_fixture_with_feature_def("UNIGRAM U00:%F[0]\n");
        let mut buf = Vec::new();
        model.write_model(&mut buf).unwrap();
        let serializable = model::Model::read_model(Cursor::new(&buf)).unwrap();
        assert!(
            serializable.connection_matrix.is_empty(),
            "precondition: no bigram weight"
        );

        let matrix = written(|w| serializable.write_connection_costs(w));
        let (rows, cols) = matrix_header(&matrix);
        assert!(rows > 1 && cols > 1, "matrix {rows}x{cols}");
        assert_entries_inside_matrix(&serializable);

        // Without a bigram weight every connection costs 0, as in training
        // and in MeCab; it used to be i16::MAX (#1168).
        for line in matrix.lines().skip(1) {
            assert_eq!(line.split(' ').nth(2), Some("0"), "matrix.def line: {line}");
        }

        // The model the trainer holds writes the same matrix as the one read
        // back from model.dat.
        assert_eq!(
            written(|w| model.write_connection_costs(w)),
            written(|w| serializable.write_connection_costs(w))
        );
    }

    #[test]
    fn test_exported_matrix_matches_the_trainer_matrix() {
        // With bigram weights, too, both writers give the same matrix, and
        // every exported entry lies inside it.
        let model = test_fixtures::train_wide_model(1);
        let mut buf = Vec::new();
        model.write_model(&mut buf).unwrap();
        let serializable = model::Model::read_model(Cursor::new(&buf)).unwrap();

        assert_entries_inside_matrix(&serializable);
        assert_eq!(
            written(|w| model.write_connection_costs(w)),
            written(|w| serializable.write_connection_costs(w))
        );
    }

    fn lattice_of(seed_csv: &str, corpus_text: &str) -> lindera_crf::Lattice {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = test_fixtures::fixtures();
        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        let mut trainer = Trainer::new(config).unwrap();
        let corpus = Corpus::from_reader(Cursor::new(corpus_text)).unwrap();
        trainer.build_lattice(&corpus.examples[0]).unwrap()
    }

    /// The `(target, label)` pairs of the edges starting at `pos`, gold edge
    /// first.
    fn edges_at(lattice: &lindera_crf::Lattice, pos: usize) -> Vec<(usize, u32)> {
        lattice.nodes()[pos]
            .edges()
            .iter()
            .map(|edge| (edge.target(), edge.label().get()))
            .collect()
    }

    #[test]
    fn test_training_lattice_has_the_other_seed_words() {
        // #1164: the gold `都` (名詞,接尾,地域) must compete with the seed's
        // other `都` row; the training lexicon used to be empty. A seed row's
        // label is its index plus one.
        let seed = "東京,0,0,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー\n\
                    都,0,0,0,名詞,一般,*,*,*,*,都,ミヤコ,ミヤコ\n\
                    都,0,0,0,名詞,接尾,地域,*,*,*,都,ト,ト\n";
        let corpus = "東京\t名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー\n\
                      都\t名詞,接尾,地域,*,*,*,都,ト,ト\n\
                      EOS\n";
        let lattice = lattice_of(seed, corpus);

        assert_eq!(edges_at(&lattice, 0).first(), Some(&(2, 1)), "gold 東京");
        let at_2 = edges_at(&lattice, 2);
        assert_eq!(at_2.first(), Some(&(3, 3)), "gold 都 is row 2");
        assert!(
            at_2.contains(&(3, 2)),
            "the other 都 (row 1) is a negative edge: {at_2:?}"
        );
    }

    #[test]
    fn test_gold_seed_word_keeps_its_own_label() {
        // Two seed words with the same feature and first character: the gold
        // `東京` gets its own row's label, not `東北`'s, and is not added again
        // as a negative edge.
        let seed = "東京,0,0,0,名詞,固有名詞,地域,一般,*,*,*,*,*\n\
                    東北,0,0,0,名詞,固有名詞,地域,一般,*,*,*,*,*\n\
                    へ,0,0,0,助詞,格助詞,一般,*,*,*,へ,ヘ,エ\n";
        let corpus = "東京\t名詞,固有名詞,地域,一般,*,*,*,*,*\n\
                      へ\t助詞,格助詞,一般,*,*,*,へ,ヘ,エ\n\
                      EOS\n";
        let lattice = lattice_of(seed, corpus);

        let at_0 = edges_at(&lattice, 0);
        assert_eq!(at_0.first(), Some(&(2, 1)), "gold 東京 is row 0");
        assert_eq!(
            at_0.iter().filter(|&&(_, label)| label == 1).count(),
            1,
            "東京 is not also a negative edge: {at_0:?}"
        );
    }
}

// Regression tests for #974: `lindera train` must produce a byte-reproducible
// `model.dat`.
//
// These train for real, unlike the pure-serialization tests in `model.rs`, so
// they cover the path the CLI actually takes -- including the fact that
// `write_model` rebuilds the connection matrix from scratch on every call.
#[cfg(test)]
pub(crate) mod test_fixtures {
    use crate::{Corpus, Trainer, TrainerConfig};
    use std::io::Cursor;

    /// Seed lexicon and corpus wide enough that the connection matrix holds
    /// many entries.
    ///
    /// This matters for the determinism assertions built on it. With only a
    /// handful of entries every key probes the same 16-slot window in rkyv's
    /// archived `HashMap`, so two runs can land on the same layout by luck and
    /// a reproducibility assertion passes for the wrong reason. Verified by
    /// mutation: with three seed words such a test still passed after
    /// reverting the #974 fix; with this fixture it fails.
    pub(crate) fn wide_seed_and_corpus() -> (String, String) {
        // Distinct part-of-speech pairs mint distinct connection ids, which is
        // what widens the matrix.
        let pos = [
            ("名詞", "一般"),
            ("名詞", "固有名詞"),
            ("名詞", "サ変接続"),
            ("名詞", "代名詞"),
            ("動詞", "自立"),
            ("動詞", "非自立"),
            ("形容詞", "自立"),
            ("副詞", "一般"),
            ("助詞", "係助詞"),
            ("助詞", "格助詞"),
            ("助動詞", "*"),
            ("接続詞", "*"),
        ];
        let mut seed = String::new();
        let mut corpus = String::new();
        for (i, (major, sub)) in pos.iter().enumerate() {
            for variant in 0..4 {
                let surface = format!("語{i}{variant}");
                let feature = format!("{major},{sub},*,*,*,*,{surface},カナ,カナ");
                seed.push_str(&format!("{surface},0,0,0,{feature}\n"));
                corpus.push_str(&format!("{surface}\t{feature}\n"));
            }
            corpus.push_str("EOS\n");
        }
        (seed, corpus)
    }

    /// The tiny in-line fixtures shared by these tests, mirroring
    /// `test_train_with_bigram_template`.
    pub(crate) fn fixtures() -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        (
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\nは,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\nテスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n",
            "DEFAULT 0 1 0\nHIRAGANA 1 1 0\nKATAKANA 1 1 0\nKANJI 0 0 2\nALPHA 1 1 0\nNUMERIC 1 1 0\n\n0x3041..0x3096 HIRAGANA\n0x30A1..0x30F6 KATAKANA\n0x4E00..0x9FAF KANJI\n0x0030..0x0039 NUMERIC\n0x0041..0x005A ALPHA\n0x0061..0x007A ALPHA\n",
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\nHIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\nKATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\nKANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*\nALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*\nNUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*\n",
            "UNIGRAM U00:%F[0]\nUNIGRAM U01:%F[0],%F?[1]\nBIGRAM B00:%L[0]/%R[0]\n",
            "名詞,一般\tNOUN,GENERAL\n",
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\nは\t助詞,係助詞,*,*,*,*,は,ハ,ワ\nテスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\nEOS\n",
        )
    }

    /// Trains the wide fixture with the given thread count and returns the
    /// model, with L1 regularization and lambda 0.01, the settings the tests
    /// that use it were written for.
    pub(crate) fn train_wide_model(threads: usize) -> crate::model::Model {
        train_wide_model_with(threads, |trainer| {
            trainer.use_l2(false).regularization_cost(0.01)
        })
    }

    /// Trains the wide fixture for 5 iterations with the given thread count,
    /// after `configure` has set up the trainer, and returns the model.
    pub(crate) fn train_wide_model_with(
        threads: usize,
        configure: impl FnOnce(Trainer) -> Trainer,
    ) -> crate::model::Model {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = fixtures();
        let (seed, corpus_text) = wide_seed_and_corpus();

        let config = match TrainerConfig::from_readers(
            Cursor::new(seed.as_bytes()),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        ) {
            Ok(config) => config,
            Err(err) => panic!("failed to build config: {err}"),
        };

        let trainer = match Trainer::new(config) {
            Ok(trainer) => configure(trainer),
            Err(err) => panic!("failed to build trainer: {err}"),
        }
        .max_iter(5)
        .num_threads(threads);

        let corpus = match Corpus::from_reader(Cursor::new(corpus_text.as_bytes())) {
            Ok(corpus) => corpus,
            Err(err) => panic!("failed to read corpus: {err}"),
        };

        match trainer.train(corpus) {
            Ok(model) => model,
            Err(err) => panic!("training failed: {err}"),
        }
    }
}

#[cfg(test)]
mod export_determinism_tests {
    use crate::test_fixtures::train_wide_model;
    use std::io::Cursor;

    /// `read_user_lexicon` -> `write_dictionary` must emit the user entries,
    /// in input order, byte-identically across runs.
    ///
    /// Before #981 this wrote an empty file: the writer read
    /// `config.user_lexicon`, which nothing populates, instead of the
    /// `user_entries` that `read_user_lexicon` fills.
    #[test]
    fn user_lexicon_roundtrips_in_input_order() {
        // One `0,0,0` entry (parameters must be inferred) and one entry with
        // explicit parameters (must be re-emitted verbatim, per the contract
        // documented on `read_user_lexicon`).
        let user_csv = "外食,0,0,0,名詞,一般,*,*,*,*,外食,ガイショク,ガイショク\n\
                        ラー油,3,3,912,名詞,一般,*,*,*,*,ラー油,ラーユ,ラーユ\n";

        let write_all = |model: &crate::model::Model| -> Vec<u8> {
            let (mut lex, mut conn, mut unk, mut user) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            if let Err(err) = model.write_dictionary(&mut lex, &mut conn, &mut unk, &mut user) {
                panic!("write_dictionary failed: {err}");
            }
            user
        };

        let mut model = train_wide_model(1);
        if let Err(err) = model.read_user_lexicon(Cursor::new(user_csv.as_bytes())) {
            panic!("read_user_lexicon failed: {err}");
        }

        let first = write_all(&model);
        assert!(
            !first.is_empty(),
            "user lexicon output is empty: write_user_lexicon is not reading user_entries"
        );

        let text = match String::from_utf8(first.clone()) {
            Ok(text) => text,
            Err(err) => panic!("user lexicon output is not UTF-8: {err}"),
        };
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "expected both user entries: {text}");

        // Input order, not map order.
        assert!(
            lines[0].starts_with("外食,"),
            "first line must be the first input entry: {}",
            lines[0]
        );
        // The parameterized entry is re-emitted verbatim.
        assert_eq!(
            lines[1], "ラー油,3,3,912,名詞,一般,*,*,*,*,ラー油,ラーユ,ラーユ",
            "non-zero parameters must be preserved as-is"
        );
        // The inferred entry keeps its surface and feature string.
        assert!(
            lines[0].ends_with(",名詞,一般,*,*,*,*,外食,ガイショク,ガイショク"),
            "inferred line must keep the input features: {}",
            lines[0]
        );

        // Byte-reproducible across repeated writes.
        let second = write_all(&model);
        assert_eq!(first, second, "user lexicon output must be reproducible");
    }

    /// The three bigram-details files must be byte-identical across repeated
    /// writes, and must carry real feature names.
    ///
    /// Before #981 the cost file iterated an unsorted `hashbrown::HashMap`
    /// rebuilt per call (different seed each time), and its labels were
    /// fabricated `L{i}`/`R{j}` strings in the wrong id space, degenerating
    /// to `*`.
    #[test]
    fn bigram_details_are_reproducible_and_named() {
        let model = train_wide_model(1);

        let dump = || -> (Vec<u8>, Vec<u8>, Vec<u8>) {
            let (mut left, mut right, mut cost) = (Vec::new(), Vec::new(), Vec::new());
            if let Err(err) = model.write_bigram_details(&mut left, &mut right, &mut cost) {
                panic!("write_bigram_details failed: {err}");
            }
            (left, right, cost)
        };

        let (left_a, right_a, cost_a) = dump();
        let (left_b, right_b, cost_b) = dump();
        assert_eq!(left_a, left_b, "left file must be reproducible");
        assert_eq!(right_a, right_b, "right file must be reproducible");
        assert_eq!(cost_a, cost_b, "cost file must be reproducible");

        let cost_text = match String::from_utf8(cost_a) {
            Ok(text) => text,
            Err(err) => panic!("cost file is not UTF-8: {err}"),
        };
        assert!(
            cost_text.lines().any(|line| line.starts_with("BOS/")),
            "cost file must label bigram feature id 0 as BOS: {cost_text}"
        );
        assert!(
            cost_text.lines().any(|line| line.contains("/EOS\t")),
            "cost file must label right feature id 0 as EOS: {cost_text}"
        );
        assert!(
            cost_text.contains("名詞"),
            "cost file must carry real feature names, not fabricated ids: {cost_text}"
        );

        let left_text = match String::from_utf8(left_a) {
            Ok(text) => text,
            Err(err) => panic!("left file is not UTF-8: {err}"),
        };
        assert!(
            left_text.contains("名詞"),
            "left file must carry real feature names: {left_text}"
        );
    }
}

// The feature ids that the extractor hands out, pinned at each step of a run
// (#1286). The SHA-256 of `model.dat` cannot catch a change of these ids:
// lindera-crf gives the weights their slots in the order the lattices meet
// the features, so renumbered ids can still give the same file. Neither the
// CLI nor the bindings call `read_user_lexicon` or `write_bigram_details`, so
// only these tests cover the ids they see.
#[cfg(test)]
mod feature_id_tests {
    use crate::feature_extractor::FeatureExtractor;
    use crate::feature_rewriter::{DictionaryRewriter, RewriteBuffers};
    use crate::test_fixtures::{fixtures, wide_seed_and_corpus};
    use crate::{Corpus, Trainer, TrainerConfig};
    use std::io::Cursor;

    /// The (id, feature) pairs of the unigram, left and right maps, each in
    /// ascending id order.
    type IdTables = [Vec<(u32, String)>; 3];

    /// Returns the (id, feature) pairs of the extractor's three maps.
    ///
    /// # Arguments
    ///
    /// * `extractor` - The feature extractor.
    ///
    /// # Returns
    ///
    /// The unigram, left and right pairs, each in the order the map lists
    /// them, which is ascending id order.
    fn id_tables(extractor: &FeatureExtractor) -> IdTables {
        [
            extractor.unigram_feature_ids(),
            extractor.left_feature_ids(),
            extractor.right_feature_ids(),
        ]
        .map(|map| {
            map.iter()
                .map(|(s, id)| (id.get(), s.to_string()))
                .collect()
        })
    }

    /// Turns pairs written as literals into [`IdTables`].
    ///
    /// # Arguments
    ///
    /// * `tables` - The unigram, left and right pairs.
    ///
    /// # Returns
    ///
    /// The same pairs with owned strings.
    fn owned(tables: [&[(u32, &str)]; 3]) -> IdTables {
        tables.map(|pairs| pairs.iter().map(|&(id, s)| (id, s.to_string())).collect())
    }

    /// Returns a trainer for the wide fixture, as `train_wide_model` builds
    /// it, before any training.
    ///
    /// # Returns
    ///
    /// The trainer.
    fn wide_trainer() -> Trainer {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = fixtures();
        let (seed, _) = wide_seed_and_corpus();
        let config = match TrainerConfig::from_readers(
            Cursor::new(seed.as_bytes()),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        ) {
            Ok(config) => config,
            Err(err) => panic!("failed to build config: {err}"),
        };
        match Trainer::new(config) {
            Ok(trainer) => trainer,
            Err(err) => panic!("failed to build trainer: {err}"),
        }
    }

    /// The ids after `Trainer::new`: the seed rows, then the `unk.def`
    /// entries, give each map's strings consecutive ids from 1, in the order
    /// the labels first meet them.
    #[test]
    fn ids_follow_first_use_after_trainer_new() {
        let trainer = wide_trainer();

        assert_eq!(
            id_tables(&trainer.config.feature_extractor),
            owned([
                &[
                    (1, "名詞"),
                    (2, "名詞,一般"),
                    (3, "名詞,固有名詞"),
                    (4, "名詞,サ変接続"),
                    (5, "名詞,代名詞"),
                    (6, "動詞"),
                    (7, "動詞,自立"),
                    (8, "動詞,非自立"),
                    (9, "形容詞"),
                    (10, "形容詞,自立"),
                    (11, "副詞"),
                    (12, "副詞,一般"),
                    (13, "助詞"),
                    (14, "助詞,係助詞"),
                    (15, "助詞,格助詞"),
                    (16, "助動詞"),
                    (17, "接続詞"),
                    (18, "名詞,数"),
                ],
                &[
                    (1, "名詞"),
                    (2, "動詞"),
                    (3, "形容詞"),
                    (4, "副詞"),
                    (5, "助詞"),
                    (6, "助動詞"),
                    (7, "接続詞"),
                ],
                &[
                    (1, "NOUN"),
                    (2, "名詞"),
                    (3, "動詞"),
                    (4, "形容詞"),
                    (5, "副詞"),
                    (6, "助詞"),
                    (7, "助動詞"),
                    (8, "接続詞"),
                ],
            ])
        );
    }

    /// Training removes the unused features and keeps the ids of the others;
    /// `write_bigram_details` names the kept left and right features; and a
    /// user lexicon entry interns a removed feature again under a new id,
    /// after the last id the map ever gave out, not after its live count.
    ///
    /// Which features are unused depends on the weights that the 5 L1
    /// iterations reach; the expected values were taken on x86_64 Linux,
    /// where the CI runs these tests.
    #[test]
    fn removal_keeps_ids_and_reinterning_gets_new_ids() {
        let (_, corpus_text) = wide_seed_and_corpus();
        let corpus = match Corpus::from_reader(Cursor::new(corpus_text.as_bytes())) {
            Ok(corpus) => corpus,
            Err(err) => panic!("failed to read corpus: {err}"),
        };
        let trainer = wide_trainer()
            .use_l2(false)
            .regularization_cost(0.01)
            .max_iter(5)
            .num_threads(1);
        let mut model = match trainer.train(corpus) {
            Ok(model) => model,
            Err(err) => panic!("training failed: {err}"),
        };

        assert_eq!(
            id_tables(&model.config.feature_extractor),
            owned([
                &[
                    (1, "名詞"),
                    (3, "名詞,固有名詞"),
                    (16, "助動詞"),
                    (17, "接続詞"),
                    (18, "名詞,数"),
                ],
                &[(1, "名詞"), (6, "助動詞"), (7, "接続詞")],
                &[(2, "名詞"), (7, "助動詞"), (8, "接続詞")],
            ]),
            "after training"
        );

        let (mut left, mut right, mut cost) = (Vec::new(), Vec::new(), Vec::new());
        if let Err(err) = model.write_bigram_details(&mut left, &mut right, &mut cost) {
            panic!("write_bigram_details failed: {err}");
        }
        assert_eq!(
            String::from_utf8_lossy(&left),
            "1\t*\n2\t\"名詞\"\n3\t\"助動詞\"\n4\t\"接続詞\"\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&right),
            "1\t\"名詞\"\n2\t*\n3\t\"助動詞\"\n4\t\"接続詞\"\n"
        );

        // `名詞,一般` (unigram id 2) and `NOUN` (right id 1) were removed;
        // `感動詞` is new to every map.
        let user_csv = "外食,0,0,0,名詞,一般,*,*,*,*,外食,ガイショク,ガイショク\n\
                        感嘆,0,0,0,感動詞,*,*,*,*,*,感嘆,カンタン,カンタン\n";
        if let Err(err) = model.read_user_lexicon(Cursor::new(user_csv.as_bytes())) {
            panic!("read_user_lexicon failed: {err}");
        }
        assert_eq!(
            id_tables(&model.config.feature_extractor),
            owned([
                &[
                    (1, "名詞"),
                    (3, "名詞,固有名詞"),
                    (16, "助動詞"),
                    (17, "接続詞"),
                    (18, "名詞,数"),
                    (19, "名詞,一般"),
                    (20, "感動詞"),
                ],
                &[(1, "名詞"), (6, "助動詞"), (7, "接続詞"), (8, "感動詞")],
                &[
                    (2, "名詞"),
                    (7, "助動詞"),
                    (8, "接続詞"),
                    (9, "NOUN"),
                    (10, "感動詞"),
                ],
            ]),
            "after the user lexicon"
        );
    }

    /// A label that needs a new feature after a map has given out its
    /// largest id is an error, not a label with the feature skipped: the
    /// error reaches `Trainer::new`, `virtual_label` and `read_user_lexicon`.
    #[test]
    fn running_out_of_feature_ids_is_an_error() {
        let rewriter = match DictionaryRewriter::from_reader(Cursor::new("")) {
            Ok(rewriter) => rewriter,
            Err(err) => panic!("rewrite.def: {err}"),
        };
        let mut extractor =
            FeatureExtractor::from_templates(&["U0:%F[0]"], &[("L0:%L[0]", "R0:%R[0]")])
                .with_max_feature_id(1);

        if let Err(err) = crate::label_feature_set(
            &rewriter,
            &mut RewriteBuffers::default(),
            &mut extractor,
            "a",
            0,
            None,
        ) {
            panic!("the first label fits: {err}");
        }
        // `U0:b` would need unigram id 2.
        match crate::label_feature_set(
            &rewriter,
            &mut RewriteBuffers::default(),
            &mut extractor,
            "b",
            0,
            None,
        ) {
            Ok(_) => panic!("a label with no id left for its feature was built"),
            Err(err) => assert!(
                err.to_string().contains("too many distinct features"),
                "{err}"
            ),
        }
    }
}

#[cfg(test)]
mod train_determinism_tests {
    use crate::test_fixtures::fixtures;
    use crate::{Corpus, Trainer, TrainerConfig};
    use std::io::Cursor;

    /// Trains once with the given thread count and serializes the resulting
    /// model, returning the bytes.
    ///
    /// The thread count must not affect the bytes: the CRF loss and gradient
    /// sum a fixed partition of the lattices in a fixed order regardless of
    /// how many workers run (`lindera-crf`'s `LatticesLoss`), which
    /// `training_is_identical_across_thread_counts` pins end to end.
    fn train_and_serialize(threads: usize) -> Vec<u8> {
        serialize(&crate::test_fixtures::train_wide_model(threads))
    }

    /// Serializes `model` and returns the bytes.
    fn serialize(model: &crate::model::Model) -> Vec<u8> {
        let mut buf = Vec::new();
        if let Err(err) = model.write_model(&mut buf) {
            panic!("failed to serialize model: {err}");
        }
        buf
    }

    /// The default trainer regularizes with L2 and `DEFAULT_LAMBDA` (1.0),
    /// MeCab's default `-c 1.0` (#1261): it trains the bytes that the
    /// explicit settings train. The other tests here train with L1, so this
    /// one also pins the L2 path, with its own line search, across thread
    /// counts.
    #[test]
    fn the_default_trains_with_l2_and_lambda_1() {
        use crate::test_fixtures::train_wide_model_with;

        let default = serialize(&train_wide_model_with(1, |trainer| trainer));
        let explicit = serialize(&train_wide_model_with(1, |trainer| {
            trainer.use_l2(true).regularization_cost(1.0)
        }));
        let parallel = serialize(&train_wide_model_with(4, |trainer| trainer));

        assert!(!default.is_empty(), "serialized model must not be empty");
        assert_eq!(
            default, explicit,
            "the default must train as L2 with lambda 1.0 does"
        );
        assert_eq!(
            default, parallel,
            "1-thread and 4-thread L2 training must produce identical bytes"
        );
    }

    /// Two independent training runs over identical inputs must serialize to
    /// identical bytes. This is the issue's own repro, in-process.
    #[test]
    fn training_twice_produces_identical_bytes() {
        let first = train_and_serialize(1);
        let second = train_and_serialize(1);

        assert!(!first.is_empty(), "serialized model must not be empty");
        assert_eq!(
            first.len(),
            second.len(),
            "two training runs produced different model sizes"
        );
        assert_eq!(
            first, second,
            "two training runs over identical inputs must produce identical bytes"
        );
    }

    /// Training with one thread and with four threads must serialize to
    /// identical bytes: the CRF reduction is a fixed partition summed in a
    /// fixed order, so the thread count is a pure performance knob.
    #[test]
    fn training_is_identical_across_thread_counts() {
        let single = train_and_serialize(1);
        let parallel = train_and_serialize(4);

        assert!(!single.is_empty(), "serialized model must not be empty");
        assert_eq!(
            single, parallel,
            "1-thread and 4-thread training must produce identical bytes"
        );
    }

    /// Serializing one trained model twice must also be stable. `write_model`
    /// rebuilds the connection matrix on every call, so this exercises that
    /// rebuild rather than a cached buffer.
    #[test]
    fn writing_one_model_twice_produces_identical_bytes() {
        let (seed, char_def, unk_def, feature_def, rewrite_def, corpus_text) = fixtures();

        let config = match TrainerConfig::from_readers(
            Cursor::new(seed),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        ) {
            Ok(config) => config,
            Err(err) => panic!("failed to build config: {err}"),
        };
        let trainer = match Trainer::new(config) {
            Ok(trainer) => trainer,
            Err(err) => panic!("failed to build trainer: {err}"),
        }
        .regularization_cost(0.01)
        .use_l2(false)
        .max_iter(5)
        .num_threads(1);
        let corpus = match Corpus::from_reader(Cursor::new(corpus_text)) {
            Ok(corpus) => corpus,
            Err(err) => panic!("failed to read corpus: {err}"),
        };
        let model = match trainer.train(corpus) {
            Ok(model) => model,
            Err(err) => panic!("training failed: {err}"),
        };

        let mut first = Vec::new();
        let mut second = Vec::new();
        if let Err(err) = model.write_model(&mut first) {
            panic!("first serialization failed: {err}");
        }
        if let Err(err) = model.write_model(&mut second) {
            panic!("second serialization failed: {err}");
        }

        assert_eq!(first, second, "two writes of one model must match");
    }
}

/// #1110: unknown words in training follow `unk.def` and `char.def` as the
/// runtime does. The fixture's `unk.def` lists the categories in another
/// order than `char.def` (ko-dic lists them alphabetically) and gives
/// `ALPHA`, `KANJI` and `KATAKANA` several entries (as IPADIC does).
#[cfg(test)]
mod unk_def_entry_tests {
    use std::collections::BTreeSet;
    use std::io::Cursor;
    use std::num::NonZeroU32;

    use crate::{Corpus, Example, Trainer, TrainerConfig, Word};

    const SEED: &str = "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n\
                        東,0,0,0,名詞,固有名詞,地域,一般,*,*,東,ヒガシ,ヒガシ\n";
    const CHAR_DEF: &str = "DEFAULT 0 1 0\nSPACE 0 1 0\nKANJI 0 0 2\nALPHA 1 1 0\n\
                            HIRAGANA 0 1 2\nKATAKANA 1 1 2\n\
                            0x0020 SPACE\n0x4E00..0x9FFF KANJI\n0x0041..0x005A ALPHA\n\
                            0x0061..0x007A ALPHA\n0x3041..0x3096 HIRAGANA\n\
                            0x30A1..0x30F6 KATAKANA\n";
    /// Entry ids: DEFAULT 0, SPACE 1, ALPHA 2-3, HIRAGANA 4, KANJI 5-7,
    /// KATAKANA 8-9 (9 accepts every noun).
    const UNK_DEF: &str = "DEFAULT,5,5,4000,記号,一般,*,*,*,*,*\n\
                           SPACE,6,6,4000,記号,空白,*,*,*,*,*\n\
                           ALPHA,7,7,4000,名詞,固有名詞,組織,*,*,*,*\n\
                           ALPHA,8,8,4000,名詞,一般,*,*,*,*,*\n\
                           HIRAGANA,9,9,4000,名詞,一般,*,*,*,*,*\n\
                           KANJI,10,10,4000,名詞,一般,*,*,*,*,*\n\
                           KANJI,11,11,4000,名詞,固有名詞,人名,姓,*,*,*\n\
                           KANJI,12,12,4000,名詞,固有名詞,組織,*,*,*,*\n\
                           KATAKANA,13,13,4000,名詞,一般,*,*,*,*,*\n\
                           KATAKANA,14,14,4000,名詞,*,*,*,*,*,*\n";
    const FEATURE_DEF: &str = "UNIGRAM U00:%F[0]\nUNIGRAM U01:%F[0],%F?[1]\n\
                               UNIGRAM U02:%t\nBIGRAM B00:%L[0]/%R[0]\n";
    const REWRITE_DEF: &str = "[unigram rewrite]\n*,*,*\t$1,$2,$3\n\
                               [left rewrite]\n*,*\t$1,$2\n\
                               [right rewrite]\n*,*\t$1,$2\n";
    /// `山田`: a surname no seed row has; `ABC`: an organization; `ほげ`: an
    /// interjection no `HIRAGANA` entry accepts.
    const CORPUS: &str = "山田\t名詞,固有名詞,人名,姓,*,*,山田,ヤマダ,ヤマダ\n\
                          は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n\
                          ABC\t名詞,固有名詞,組織,*,*,*,*\n\
                          ほげ\t感動詞,*,*,*,*,*,ほげ,ホゲ,ホゲ\n\
                          EOS\n\
                          ほげ\t感動詞,*,*,*,*,*,ほげ,ホゲ,ホゲ\n\
                          東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,ヒガシ\n\
                          EOS\n";
    /// The feature string of `ほげ`, which only a virtual label carries.
    const HOGE: &str = "感動詞,*,*,*,*,*,ほげ,ホゲ,ホゲ";

    fn trainer() -> Trainer {
        trainer_with_seed(SEED)
    }

    /// The fixture's trainer with another seed lexicon.
    fn trainer_with_seed(seed: &str) -> Trainer {
        let config = match TrainerConfig::from_readers(
            Cursor::new(seed),
            Cursor::new(CHAR_DEF),
            Cursor::new(UNK_DEF),
            Cursor::new(FEATURE_DEF),
            Cursor::new(REWRITE_DEF),
        ) {
            Ok(config) => config,
            Err(err) => panic!("failed to build config: {err}"),
        };
        match Trainer::new(config) {
            Ok(trainer) => trainer.max_iter(5).num_threads(1),
            Err(err) => panic!("failed to build trainer: {err}"),
        }
    }

    fn example(tokens: &[(&str, &str)]) -> Example {
        Example::new(
            tokens
                .iter()
                .map(|(surface, feature)| Word::new(surface, feature))
                .collect(),
        )
    }

    fn build(trainer: &mut Trainer, example: &Example) -> lindera_crf::Lattice {
        match trainer.build_lattice(example) {
            Ok(lattice) => lattice,
            Err(err) => panic!("build_lattice failed: {err}"),
        }
    }

    /// The label of `unk.def` entry `id`: the entries follow the two seed
    /// rows.
    fn unk(id: u32) -> NonZeroU32 {
        match NonZeroU32::new(2 + id + 1) {
            Some(label) => label,
            None => unreachable!(),
        }
    }

    /// The `(target, label)` pair of the gold edge at `pos`.
    fn gold(lattice: &lindera_crf::Lattice, pos: usize) -> (usize, u32) {
        let edge = lattice.nodes()[pos].edges()[0];
        (edge.target(), edge.label().get())
    }

    /// The `(target, label)` pairs of the edges at `pos` after the gold one.
    fn negatives(lattice: &lindera_crf::Lattice, pos: usize) -> BTreeSet<(usize, u32)> {
        lattice.nodes()[pos].edges()[1..]
            .iter()
            .map(|edge| (edge.target(), edge.label().get()))
            .collect()
    }

    /// `(target, unk(id))` pairs.
    fn unks(pairs: &[(usize, u32)]) -> BTreeSet<(usize, u32)> {
        pairs
            .iter()
            .map(|&(target, id)| (target, unk(id).get()))
            .collect()
    }

    /// One label per `unk.def` line, named by its category, in file order.
    #[test]
    fn labels_follow_unk_def_lines() {
        let trainer = trainer();
        assert_eq!(
            trainer.extract_labels(),
            [
                "は", "東", "DEFAULT", "SPACE", "ALPHA", "ALPHA", "HIRAGANA", "KANJI", "KANJI",
                "KANJI", "KATAKANA", "KATAKANA"
            ]
        );
        assert_eq!(
            trainer.unk_entry_labels,
            (0..10).map(unk).collect::<Vec<_>>()
        );
        assert_eq!(trainer.provider.len(), 2 + 10);
    }

    /// The candidates come from the default category's entries with the
    /// spans `char.def` gives, and each gold token takes the first entry
    /// that accepts its features, or a virtual label shared by repeats.
    #[test]
    fn lattice_uses_unk_def_entries_and_virtual_labels() {
        let mut trainer = trainer();
        let first = example(&[
            ("山田", "名詞,固有名詞,人名,姓,*,*,山田,ヤマダ,ヤマダ"),
            ("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
            ("ABC", "名詞,固有名詞,組織,*,*,*,*"),
            ("ほげ", HOGE),
        ]);
        let lattice = build(&mut trainer, &first);

        // 山田 (KANJI: no grouping, lengths 1 and 2): the surname entry (6),
        // not the first KANJI entry; the KANJI entries are the negatives.
        assert_eq!(gold(&lattice, 0), (2, unk(6).get()));
        assert_eq!(
            negatives(&lattice, 0),
            unks(&[(1, 5), (2, 5), (1, 6), (1, 7), (2, 7)])
        );

        // は: the seed row, label 1, and no unknown word: HIRAGANA has
        // invoke off and a seed word starts here.
        assert_eq!(gold(&lattice, 2), (3, 1));
        assert!(negatives(&lattice, 2).is_empty());

        // ABC (ALPHA groups): the organization entry (2), and the other
        // ALPHA entry over the same span as a negative.
        assert_eq!(gold(&lattice, 3), (6, unk(2).get()));
        assert_eq!(negatives(&lattice, 3), unks(&[(6, 3)]));

        // ほげ: no HIRAGANA entry accepts an interjection, so a virtual
        // label, after the 2 seed and 10 unk.def labels. HIRAGANA groups ほげ
        // and its length ladder adds ほ.
        let virtual_label = 2 + 10 + 1;
        assert_eq!(gold(&lattice, 6), (8, virtual_label));
        assert_eq!(negatives(&lattice, 6), unks(&[(7, 4), (8, 4)]));

        // The repeat reuses the virtual label. 東 is a seed word, so the
        // invoke-off KANJI creates no unknown word there.
        let second = example(&[
            ("ほげ", HOGE),
            ("東", "名詞,固有名詞,地域,一般,*,*,東,ヒガシ,ヒガシ"),
        ]);
        let lattice2 = build(&mut trainer, &second);
        assert_eq!(gold(&lattice2, 0), (2, virtual_label));
        assert_eq!(gold(&lattice2, 2), (3, 2));
        assert!(negatives(&lattice2, 2).is_empty());
        assert_eq!(trainer.provider.len(), 2 + 10 + 1);
        assert_eq!(trainer.virtual_features, [HOGE]);

        // テスト (KATAKANA groups, ladder up to 2): both KATAKANA entries
        // accept a common noun; the first in unk.def order (8) is the gold
        // label, as the node MeCab's learner takes.
        let third = example(&[("テスト", "名詞,一般,*,*,*,*,テスト,テスト,テスト")]);
        let lattice3 = build(&mut trainer, &third);
        assert_eq!(gold(&lattice3, 0), (3, unk(8).get()));
        assert_eq!(
            negatives(&lattice3, 0),
            unks(&[(1, 8), (2, 8), (3, 9), (1, 9), (2, 9)])
        );

        let stats = &trainer.stats;
        assert_eq!(
            (stats.gold_lexicon, stats.gold_unknown, stats.gold_virtual),
            (2, 3, 2)
        );
        assert_eq!(stats.gold_feature_mismatch, 0);
        assert!(stats.first_feature_mismatch.is_none());
    }

    /// A gold token whose surface has seed rows but none with its feature
    /// string is counted, and the first one in corpus order is kept with
    /// the feature strings of the first five of those rows, in seed order,
    /// and their number (#1259). A token without a seed row of its surface
    /// is not counted.
    #[test]
    fn lattice_counts_gold_tokens_whose_surface_has_seed_rows_of_other_features() {
        // は, then six rows of 東.
        let east = [
            "名詞,固有名詞,地域,一般,*,*,東,ヒガシ,ヒガシ",
            "名詞,固有名詞,人名,姓,*,*,東,アズマ,アズマ",
            "名詞,固有名詞,人名,名,*,*,東,アズマ,アズマ",
            "名詞,一般,*,*,*,*,東,ヒガシ,ヒガシ",
            "名詞,固有名詞,地域,一般,*,*,東,トウ,トウ",
            "名詞,固有名詞,組織,*,*,*,東,トウ,トウ",
        ];
        let seed: String = std::iter::once("は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n".to_string())
            .chain(east.iter().map(|feature| format!("東,0,0,0,{feature}\n")))
            .collect();
        let mut trainer = trainer_with_seed(&seed);

        // 東 without its base form and readings: no 東 row has the feature.
        build(
            &mut trainer,
            &example(&[
                ("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
                ("東", "名詞,一般,*,*,*,*"),
            ]),
        );
        // は without them, a seed word 東, and 山田, which no seed row has.
        build(
            &mut trainer,
            &example(&[
                ("は", "助詞,係助詞,*,*,*,*"),
                ("東", east[3]),
                ("山田", "名詞,固有名詞,人名,姓,*,*,山田,ヤマダ,ヤマダ"),
            ]),
        );

        let stats = &trainer.stats;
        assert_eq!((stats.gold_lexicon, stats.gold_feature_mismatch), (2, 2));
        let Some(mismatch) = &stats.first_feature_mismatch else {
            panic!("no gold token was kept");
        };
        assert_eq!(
            (
                mismatch.surface.as_str(),
                mismatch.feature.as_str(),
                mismatch.seed_rows
            ),
            ("東", "名詞,一般,*,*,*,*", 6)
        );
        assert_eq!(mismatch.seed_features, east[..5]);
    }

    /// `Trainer::max_grouping_len` caps grouping as the runtime option does:
    /// `ABC` has 2 characters beyond the first, so a cap of 2 keeps the
    /// grouped candidate and a cap of 1 leaves only one character, which no
    /// ALPHA length ladder extends.
    #[test]
    fn max_grouping_len_caps_grouping_as_at_runtime() {
        let abc = example(&[("ABC", "名詞,固有名詞,組織,*,*,*,*")]);

        let mut capped_at_2 = trainer().max_grouping_len(Some(2));
        let lattice2 = build(&mut capped_at_2, &abc);
        assert_eq!(gold(&lattice2, 0), (3, unk(2).get()));
        assert_eq!(negatives(&lattice2, 0), unks(&[(3, 3)]));

        let mut capped_at_1 = trainer().max_grouping_len(Some(1));
        let lattice1 = build(&mut capped_at_1, &abc);
        assert_eq!(gold(&lattice1, 0), (3, 2 + 10 + 1), "a virtual label");
        assert_eq!(negatives(&lattice1, 0), unks(&[(1, 2), (1, 3)]));
    }

    /// The exported `unk.def` has one line per input line, in input order,
    /// with the category and feature string of that line -- from the model
    /// and from `model.dat` alike -- and `lex.csv` has the seed rows only.
    #[test]
    fn exported_unk_def_keeps_every_line_in_order() {
        let corpus = match Corpus::from_reader(Cursor::new(CORPUS)) {
            Ok(corpus) => corpus,
            Err(err) => panic!("failed to read corpus: {err}"),
        };
        let model = match trainer().train(corpus) {
            Ok(model) => model,
            Err(err) => panic!("training failed: {err}"),
        };
        assert_eq!(model.virtual_features, [HOGE]);

        let expected: Vec<(String, String)> = UNK_DEF
            .lines()
            .map(|line| {
                let columns: Vec<&str> = line.split(',').collect();
                (columns[0].to_string(), columns[4..].join(","))
            })
            .collect();
        let entries = |bytes: Vec<u8>| -> Vec<(String, String)> {
            let text = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(err) => panic!("unk.def is not UTF-8: {err}"),
            };
            text.lines()
                .map(|line| {
                    let columns: Vec<&str> = line.split(',').collect();
                    for column in &columns[1..4] {
                        if column.parse::<i32>().is_err() {
                            panic!("non-numeric id or cost in {line}");
                        }
                    }
                    (columns[0].to_string(), columns[4..].join(","))
                })
                .collect()
        };

        let mut from_model = Vec::new();
        if let Err(err) = model.write_unknown_dictionary(&mut from_model) {
            panic!("write_unknown_dictionary failed: {err}");
        }
        assert_eq!(entries(from_model), expected, "Model");

        let mut bytes = Vec::new();
        if let Err(err) = model.write_model(&mut bytes) {
            panic!("write_model failed: {err}");
        }
        let serializable = match crate::Model::read_model(Cursor::new(bytes)) {
            Ok(model) => model,
            Err(err) => panic!("read_model failed: {err}"),
        };
        assert_eq!(serializable.unk_entries, expected);
        let mut from_file = Vec::new();
        if let Err(err) = serializable.write_unknown_dictionary(&mut from_file) {
            panic!("write_unknown_dictionary failed: {err}");
        }
        assert_eq!(entries(from_file), expected, "SerializableModel");

        // The lexicon keeps the seed rows only: neither the unk.def entries
        // nor the virtual label.
        let mut lexicon = Vec::new();
        if let Err(err) = serializable.write_lexicon(&mut lexicon) {
            panic!("write_lexicon failed: {err}");
        }
        let lexicon = String::from_utf8_lossy(&lexicon).to_string();
        let surfaces: Vec<&str> = lexicon
            .lines()
            .filter_map(|line| line.split(',').next())
            .collect();
        assert_eq!(surfaces, ["は", "東"]);
    }
}

/// The warning that few gold tokens are seed words (#1259).
#[cfg(test)]
mod seed_word_warning_tests {
    use crate::{FeatureMismatch, LatticeStats};

    /// Statistics with `lexicon` seed words, `virtual_` virtual labels and
    /// `mismatch` of them whose surface has seed rows of other feature
    /// strings; the first of those is `東`, with `seed_rows` rows.
    fn stats(lexicon: usize, virtual_: usize, mismatch: usize, seed_rows: usize) -> LatticeStats {
        let seed_features = (0..seed_rows.min(5))
            .map(|row| format!("名詞,固有名詞,地域,一般,*,*,東,ヒガシ,{row}"))
            .collect();
        LatticeStats {
            gold_lexicon: lexicon,
            gold_virtual: virtual_,
            gold_feature_mismatch: mismatch,
            first_feature_mismatch: (mismatch > 0).then(|| FeatureMismatch {
                surface: "東".to_string(),
                feature: "名詞,一般,*,*,*,*".to_string(),
                seed_features,
                seed_rows,
            }),
            ..LatticeStats::default()
        }
    }

    #[test]
    fn no_warning_without_gold_tokens() {
        assert_eq!(stats(0, 0, 0, 0).seed_word_warning(), None);
    }

    /// No gold token is a seed word, and none has the surface of a seed row.
    #[test]
    fn warns_when_no_gold_token_has_the_surface_of_a_seed_row() {
        let stats = LatticeStats {
            gold_unknown: 1,
            ..stats(0, 2, 0, 0)
        };
        assert_eq!(
            stats.seed_word_warning().as_deref(),
            Some(
                "Warning: none of the 3 gold tokens is a seed word, and no seed row has the \
                 surface of one. The seed rows are only wrong candidates in training."
            )
        );
    }

    /// No gold token is a seed word: the warning shows the first one whose
    /// surface has seed rows, next to the first five of them.
    #[test]
    fn warns_with_the_first_mismatch_when_no_gold_token_is_a_seed_word() {
        assert_eq!(
            stats(0, 4, 3, 6).seed_word_warning().as_deref(),
            Some(
                "Warning: 0 of the 4 gold tokens are seed words, and 3 are not although seed \
                 rows have their surface. A gold token is a seed word only when a seed row has \
                 its surface and its whole feature string, as the seed lexicon writes it; \
                 otherwise the seed rows with its surface are only wrong candidates at its \
                 position. The first such gold token, and the seed rows with its surface:\n  \
                 gold token: 東\t名詞,一般,*,*,*,*\n  \
                 seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,0\n  \
                 seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,1\n  \
                 seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,2\n  \
                 seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,3\n  \
                 seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,4\n  \
                 and 1 more"
            )
        );
    }

    /// Some gold tokens are seed words, but fewer than those whose surface
    /// has seed rows of other feature strings. Every seed row is shown.
    #[test]
    fn warns_when_mismatches_outnumber_seed_words() {
        let Some(warning) = stats(2, 3, 3, 2).seed_word_warning() else {
            panic!("no warning");
        };
        assert!(
            warning.starts_with(
                "Warning: 2 of the 5 gold tokens are seed words, and 3 are not although seed \
                 rows have their surface."
            ),
            "got: {warning}"
        );
        assert!(
            warning.ends_with("\n  seed row:   東\t名詞,固有名詞,地域,一般,*,*,東,ヒガシ,1"),
            "got: {warning}"
        );
    }

    /// Half or more of the gold tokens whose surface has seed rows are seed
    /// words.
    #[test]
    fn no_warning_when_half_or_more_are_seed_words() {
        assert_eq!(stats(3, 3, 3, 1).seed_word_warning(), None);
        assert_eq!(stats(3, 0, 0, 0).seed_word_warning(), None);
    }
}

/// The warnings after the optimizer stopped (#1268).
#[cfg(test)]
mod optimizer_warning_tests {
    use lindera_crf::StopReason;

    use crate::optimizer_warning;

    #[test]
    fn warns_when_an_iteration_left_the_objective_unchanged() {
        assert_eq!(
            optimizer_warning(StopReason::NoProgress),
            Some(
                "Warning: the optimizer could not make progress; after few iterations the model \
                 is barely trained"
            )
        );
    }

    #[test]
    fn warns_when_the_line_search_failed() {
        assert_eq!(
            optimizer_warning(StopReason::LineSearchFailed),
            Some(
                "Warning: the line search failed; the model has the best weights the optimizer \
                 reached"
            )
        );
    }

    #[test]
    fn no_warning_when_it_converged_or_reached_the_cap() {
        assert_eq!(optimizer_warning(StopReason::Converged), None);
        assert_eq!(optimizer_warning(StopReason::MaxIterations), None);
    }
}

#[cfg(test)]
mod lexicon_csv_export_tests {
    use std::io::Cursor;

    use crate::corpus::Word;
    use crate::test_fixtures::{fixtures, train_wide_model, wide_seed_and_corpus};
    use crate::{Corpus, Trainer, TrainerConfig};

    /// Returns the surfaces of `content` as `lindera build` reads `lex.csv`.
    fn surfaces_read_by_dictionary_builder(content: &[u8]) -> Vec<String> {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content)
            .records()
            .map(|record| record.unwrap()[0].to_string())
            .collect()
    }

    /// A seed with mecab-ko-dic's quoted `","` row, a doubled quote and
    /// UniDic's `#` row trains with a label for every row, in seed order, and
    /// both lexicon writers write every row, so that `lindera build` reads the
    /// seed surfaces back (#1170).
    #[test]
    fn seed_with_quoted_and_hash_surfaces_exports_every_row() {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = fixtures();
        let (wide_seed, corpus_text) = wide_seed_and_corpus();
        let seed = format!(
            "\",\",0,0,0,記号,読点,*,*,*,*,*,*,*\n\
             \"\"\"\",0,0,0,記号,括弧開,*,*,*,*,*,*,*\n\
             #,0,0,0,記号,一般,*,*,*,*,#,#,#\n\
             {wide_seed}"
        );
        let config = TrainerConfig::from_readers(
            Cursor::new(seed.as_bytes()),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        let surfaces = config.surfaces().to_vec();
        assert_eq!(surfaces.len(), seed.lines().count());
        assert_eq!(surfaces[..4], [",", "\"", "#", "語00"]);

        let trainer = Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(false)
            .max_iter(5)
            .num_threads(1);
        let corpus = Corpus::from_reader(Cursor::new(corpus_text.as_bytes())).unwrap();
        let model = trainer.train(corpus).unwrap();

        let mut lexicon = Vec::new();
        model.write_lexicon(&mut lexicon).unwrap();
        assert_eq!(
            surfaces_read_by_dictionary_builder(&lexicon),
            surfaces,
            "Model::write_lexicon:\n{}",
            String::from_utf8_lossy(&lexicon)
        );

        let mut bytes = Vec::new();
        model.write_model(&mut bytes).unwrap();
        let serializable = crate::Model::read_model(Cursor::new(bytes)).unwrap();
        assert_eq!(serializable.labels[..surfaces.len()], surfaces[..]);
        let mut lexicon = Vec::new();
        serializable.write_lexicon(&mut lexicon).unwrap();
        assert_eq!(
            surfaces_read_by_dictionary_builder(&lexicon),
            surfaces,
            "SerializableModel::write_lexicon:\n{}",
            String::from_utf8_lossy(&lexicon)
        );
    }

    /// A user lexicon with a quoted `","` row and a `#` row round-trips: the
    /// rows are read as CSV records, and a row with parameters is written
    /// back as it was read (#1170).
    #[test]
    fn user_lexicon_with_quoted_and_hash_surfaces_round_trips() {
        let user_csv = "\",\",3,3,912,記号,読点,*,*,*,*,*,*,*\n\
                        #,0,0,0,記号,一般,*,*,*,*,#,#,#\n";
        let mut model = train_wide_model(1);
        model
            .read_user_lexicon(Cursor::new(user_csv.as_bytes()))
            .unwrap();

        let (mut lex, mut conn, mut unk, mut user) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        model
            .write_dictionary(&mut lex, &mut conn, &mut unk, &mut user)
            .unwrap();

        assert_eq!(surfaces_read_by_dictionary_builder(&user), [",", "#"]);
        let text = String::from_utf8(user).unwrap();
        assert_eq!(text.lines().next(), user_csv.lines().next());
        assert!(
            text.lines()
                .nth(1)
                .is_some_and(|line| line.ends_with(",記号,一般,*,*,*,*,#,#,#")),
            "{text}"
        );
    }

    /// Every lexicon writer quotes a surface with `,` or `"` in it, so that
    /// `lindera build` reads the same surfaces, in the same order (#1170).
    /// A label of `"` is what a model trained before #1170 has for
    /// mecab-ko-dic's `","` row; written unquoted, it opened a quoted field
    /// that swallowed the rows after it.
    #[test]
    fn lexicon_writers_quote_surfaces() {
        let mut model = train_wide_model(1);
        for (index, surface) in [(0, ","), (1, "\""), (2, "a\"b,c"), (3, "#")] {
            model.config.surfaces[index] = surface.to_string();
            model.labels[index] = surface.to_string();
        }
        let surfaces = model.config.surfaces.clone();

        let mut lexicon = Vec::new();
        model.write_lexicon(&mut lexicon).unwrap();
        assert_eq!(
            surfaces_read_by_dictionary_builder(&lexicon),
            surfaces,
            "Model::write_lexicon:\n{}",
            String::from_utf8_lossy(&lexicon)
        );

        let mut bytes = Vec::new();
        model.write_model(&mut bytes).unwrap();
        let serializable = crate::Model::read_model(Cursor::new(bytes)).unwrap();
        let mut lexicon = Vec::new();
        serializable.write_lexicon(&mut lexicon).unwrap();
        assert_eq!(
            surfaces_read_by_dictionary_builder(&lexicon),
            surfaces,
            "SerializableModel::write_lexicon:\n{}",
            String::from_utf8_lossy(&lexicon)
        );
    }

    /// The user lexicon writer quotes a surface with `,` or `"` in it, for
    /// both an entry whose parameters it infers and one it re-emits (#1170).
    #[test]
    fn user_lexicon_writer_quotes_surfaces() {
        let user_csv = "外食,0,0,0,名詞,一般,*,*,*,*,外食,ガイショク,ガイショク\n\
                        ラー油,3,3,912,名詞,一般,*,*,*,*,ラー油,ラーユ,ラーユ\n";
        let mut model = train_wide_model(1);
        model
            .read_user_lexicon(Cursor::new(user_csv.as_bytes()))
            .unwrap();
        for (index, surface) in [(0, ","), (1, "\"")] {
            let feature = model.user_entries[index].word.feature().to_string();
            model.user_entries[index].word = Word::new(surface, &feature);
        }

        let (mut lex, mut conn, mut unk, mut user) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        model
            .write_dictionary(&mut lex, &mut conn, &mut unk, &mut user)
            .unwrap();

        assert_eq!(
            surfaces_read_by_dictionary_builder(&user),
            [",", "\""],
            "{}",
            String::from_utf8_lossy(&user)
        );
        let text = String::from_utf8(user).unwrap();
        assert_eq!(
            text.lines().nth(1),
            Some("\"\"\"\",3,3,912,名詞,一般,*,*,*,*,ラー油,ラーユ,ラーユ")
        );
    }
}

/// A quoted feature column with `,` in it is one column for the feature
/// templates and for unknown-word matching (#1184).
#[cfg(test)]
mod feature_column_tests {
    use std::io::Cursor;

    use crate::feature_extractor::{FeatureExtractor, FeatureIdMap};
    use crate::feature_rewriter::{DictionaryRewriter, RewriteBuffers};
    use crate::{label_feature_set, unk_entry_accepts};

    /// The feature of the issue's reproduction: Jieba-like columns whose
    /// fifth, a definition, is quoted and has a comma in it.
    const FEATURE: &str = "n,dian4,店,店,\"shop, store\",1";

    /// Returns the feature strings an extractor has interned, sorted.
    fn interned(ids: &FeatureIdMap) -> Vec<&str> {
        let mut features: Vec<&str> = ids.iter().map(|(feature, _)| feature).collect();
        features.sort_unstable();
        features
    }

    #[test]
    fn templates_read_a_quoted_column_as_one_column() {
        let rewrite_def = "\
[unigram rewrite]
* $1,$2,$3,$4,$5,$6

[left rewrite]
* $1,$5

[right rewrite]
* $5,$6
";
        let rewriter = match DictionaryRewriter::from_reader(Cursor::new(rewrite_def)) {
            Ok(rewriter) => rewriter,
            Err(err) => panic!("rewrite.def: {err}"),
        };
        let mut extractor = FeatureExtractor::from_templates(
            &["U04:%F[4]", "U05:%F[5]"],
            &[("L1:%L[1]", "R1:%R[1]")],
        );

        if let Err(err) = label_feature_set(
            &rewriter,
            &mut RewriteBuffers::default(),
            &mut extractor,
            FEATURE,
            0,
            None,
        ) {
            panic!("label_feature_set: {err}");
        }

        // MeCab's learner gives `U04:shop, store` and `U05:1` for the same
        // feature and unigram rule.
        assert_eq!(
            interned(extractor.unigram_feature_ids()),
            ["U04:shop, store", "U05:1"]
        );
        assert_eq!(interned(extractor.left_feature_ids()), ["L1:shop, store"]);
        assert_eq!(interned(extractor.right_feature_ids()), ["R1:1"]);
    }

    #[test]
    fn user_lexicon_features_read_a_quoted_column_as_one_column() {
        // The bigram templates of the fixtures are `%L[0]` and `%R[0]`, and
        // no rule of their rewrite.def matches the entry, so its left and
        // right features are its first column, which reading the entry
        // registers with the feature extractor.
        let mut model = crate::test_fixtures::train_wide_model(1);
        if let Err(err) = model.read_user_lexicon(Cursor::new("X,0,0,0,\"名詞,固有\",*\n")) {
            panic!("user lexicon: {err}");
        }

        let extractor = &model.config.feature_extractor;
        for ids in [extractor.left_feature_ids(), extractor.right_feature_ids()] {
            let features = interned(ids);
            assert!(features.contains(&"名詞,固有"), "{features:?}");
            assert!(features.iter().all(|f| !f.contains('"')), "{features:?}");
        }
    }

    #[test]
    fn unknown_word_matching_reads_a_quoted_column_as_one_column() {
        // The entry's sixth column is compared with the gold token's sixth
        // column, `1`, which a split on `,` would make ` store"`.
        assert!(unk_entry_accepts("n,*,*,*,*,1", FEATURE));
        assert!(!unk_entry_accepts("n,*,*,*,*,2", FEATURE));
        // A quoted column of the entry is compared whole.
        assert!(unk_entry_accepts("n,*,*,*,\"shop, store\",*", FEATURE));
        assert!(!unk_entry_accepts("n,*,*,*,shop,*", FEATURE));
    }
}

/// The left and right feature IDs of a label are computed once per pair of
/// rewritten left and right features, and shared by the labels with that
/// pair (#1263).
#[cfg(test)]
mod bigram_feature_id_sharing_tests {
    use std::io::Cursor;
    use std::num::NonZeroU32;

    use crate::feature_csv::split_feature;
    use crate::feature_extractor::{FeatureExtractor, FeatureIdMap, TemplateContext};
    use crate::feature_rewriter::{DictionaryRewriter, RewriteBuffers};
    use crate::label_feature_set;

    /// The left feature is the first column, and the right feature the first
    /// two, so `n,a,x` and `n,a,y` have the same pair, and `n,b,z` the same
    /// left feature with another right one.
    const REWRITE_DEF: &str = "\
[unigram rewrite]
* $1,$2,$3

[left rewrite]
* $1

[right rewrite]
* $1,$2
";

    fn rewriter() -> DictionaryRewriter {
        match DictionaryRewriter::from_reader(Cursor::new(REWRITE_DEF)) {
            Ok(rewriter) => rewriter,
            Err(err) => panic!("rewrite.def: {err}"),
        }
    }

    /// Each side reads its own columns, its whole feature, and the whole
    /// feature of the other side.
    fn extractor() -> FeatureExtractor {
        FeatureExtractor::from_templates(
            &["U0:%F[0]", "U2:%F[2]"],
            &[
                ("L0:%L[0]", "R1:%R[1]"),
                ("LL:%l", "RR:%r"),
                ("LR:%r", "RL:%l"),
            ],
        )
    }

    /// Returns the feature strings of `ids`, in order, from `map`.
    fn names(ids: &[Option<NonZeroU32>], map: &FeatureIdMap) -> Vec<String> {
        ids.iter()
            .map(|id| {
                id.and_then(|id| map.resolve(id))
                    .map_or_else(|| format!("{id:?}"), str::to_string)
            })
            .collect()
    }

    /// Returns the feature set of a label, as `Trainer::new` builds it.
    fn feature_set(
        rewriter: &DictionaryRewriter,
        extractor: &mut FeatureExtractor,
        feature: &str,
        surface: Option<&str>,
    ) -> lindera_crf::FeatureSet {
        match label_feature_set(
            rewriter,
            &mut RewriteBuffers::default(),
            extractor,
            feature,
            0,
            surface,
        ) {
            Ok(feature_set) => feature_set,
            Err(err) => panic!("label_feature_set({feature}): {err}"),
        }
    }

    #[test]
    fn labels_with_the_same_left_and_right_features_share_their_ids() {
        let rewriter = rewriter();
        let mut extractor = extractor();

        let first = feature_set(&rewriter, &mut extractor, "n,a,x", None);
        let second = feature_set(&rewriter, &mut extractor, "n,a,y", None);
        let other_right = feature_set(&rewriter, &mut extractor, "n,b,z", None);

        assert!(std::ptr::eq(
            first.bigram_left().as_ptr(),
            second.bigram_left().as_ptr()
        ));
        assert!(std::ptr::eq(
            first.bigram_right().as_ptr(),
            second.bigram_right().as_ptr()
        ));
        let (left_ids, right_ids) = (extractor.left_feature_ids(), extractor.right_feature_ids());
        assert_eq!(
            names(first.bigram_left(), left_ids),
            ["L0:n", "LL:n", "LR:n,a"]
        );
        assert_eq!(
            names(first.bigram_right(), right_ids),
            ["R1:a", "RR:n,a", "RL:n"]
        );
        // The same left feature with another right one: the left template
        // `%r` reads the right feature, so the pair, not the left feature
        // alone, decides both lists.
        assert_eq!(
            names(other_right.bigram_left(), left_ids),
            ["L0:n", "LL:n", "LR:n,b"]
        );
        assert_eq!(
            names(other_right.bigram_right(), right_ids),
            ["R1:b", "RR:n,b", "RL:n"]
        );
    }

    #[test]
    fn cached_ids_are_the_ids_of_the_uncached_extraction() {
        let rewriter = rewriter();
        let rows = ["n,a,x", "v,c,y", "n,a,y", "n,b,z", "v,c,w", "n,a,x"];

        let mut cached = extractor();
        let sets: Vec<_> = rows
            .iter()
            .map(|row| feature_set(&rewriter, &mut cached, row, Some("s")))
            .collect();

        // What `label_feature_set` did before the IDs were cached: every
        // template of every row, in the order unigram, left, right.
        let mut uncached = extractor();
        for (row, set) in rows.iter().zip(&sets) {
            let (ufeature, lfeature, rfeature) = rewriter.rewrite(row);
            let columns = |feature: &str| -> Vec<String> {
                split_feature(feature)
                    .into_iter()
                    .map(|column| column.into_owned())
                    .collect()
            };
            let (u, l, r) = (columns(&ufeature), columns(&lfeature), columns(&rfeature));
            fn as_strs(columns: &[String]) -> Vec<&str> {
                columns.iter().map(String::as_str).collect()
            }
            let ctx = TemplateContext {
                surface: Some("s"),
                ufeature: Some(&ufeature),
                lfeature: Some(&lfeature),
                rfeature: Some(&rfeature),
            };
            let unigram = uncached.extract_unigram_feature_ids_with_ctx(&as_strs(&u), 0, &ctx);
            let left = uncached.extract_left_feature_ids_with_ctx(&as_strs(&l), &ctx);
            let right = uncached.extract_right_feature_ids_with_ctx(&as_strs(&r), &ctx);
            assert_eq!(set.unigram(), unigram.as_slice(), "{row}");
            assert_eq!(set.bigram_left(), left.as_slice(), "{row}");
            assert_eq!(set.bigram_right(), right.as_slice(), "{row}");
        }
        assert_eq!(cached.unigram_feature_ids(), uncached.unigram_feature_ids());
        assert_eq!(cached.left_feature_ids(), uncached.left_feature_ids());
        assert_eq!(cached.right_feature_ids(), uncached.right_feature_ids());
    }

    // Training removes the unused features from the ID maps, and then
    // forgets the cached IDs, which may name them.
    #[test]
    fn training_forgets_the_cached_ids() {
        let model = crate::test_fixtures::train_wide_model_with(1, |trainer| {
            assert!(
                trainer
                    .config
                    .feature_extractor
                    .cached_bigram_feature_id_pairs()
                    > 0
            );
            trainer
        });

        assert_eq!(
            model
                .config
                .feature_extractor
                .cached_bigram_feature_id_pairs(),
            0
        );
    }
}

/// A feature template that reads past the last column of a label's feature
/// gives the label no feature, as in MeCab's learner (#1189).
#[cfg(test)]
mod template_past_last_column_tests {
    use std::io::Cursor;

    use crate::feature_extractor::FeatureExtractor;
    use crate::feature_rewriter::{DictionaryRewriter, RewriteBuffers};
    use crate::label_feature_set;

    #[test]
    fn a_short_unk_def_feature_gets_no_feature_from_templates_past_its_last_column() {
        // mecab-jieba's feature.def reads up to the tenth column, which every
        // seed row has, but its unk.def features, such as `n,*,*,*,*`, have
        // five. Its rewrite.def has no rules, so the rewritten features are
        // the feature.
        let rewriter = match DictionaryRewriter::from_reader(Cursor::new("# No rules\n")) {
            Ok(rewriter) => rewriter,
            Err(err) => panic!("rewrite.def: {err}"),
        };
        let mut extractor = FeatureExtractor::from_templates(
            &["U00:%F[0]", "U06:%F[6]"],
            &[("%L[0]", "%R[0]"), ("%L[6]", "%R[6]")],
        );

        let feature_set = match label_feature_set(
            &rewriter,
            &mut RewriteBuffers::default(),
            &mut extractor,
            "n,*,*,*,*",
            0,
            None,
        ) {
            Ok(feature_set) => feature_set,
            Err(err) => panic!("label_feature_set: {err}"),
        };

        // MeCab's learner gives this label `U00:n` and the first bigram
        // template only; the CRF drops a bigram slot that is `None`.
        assert_eq!(feature_set.unigram().len(), 1, "U06 reads column 7 of 5");
        assert_eq!(feature_set.bigram_left().len(), 2);
        assert!(feature_set.bigram_left()[0].is_some());
        assert!(feature_set.bigram_left()[1].is_none());
        assert_eq!(feature_set.bigram_right().len(), 2);
        assert!(feature_set.bigram_right()[0].is_some());
        assert!(feature_set.bigram_right()[1].is_none());
    }
}

/// A user lexicon entry with `0,0,0` gets the context ids that `lex.csv`
/// gives the seed row whose feature is the most similar to its own, and one
/// whose feature has no column in common with any seed row is an error
/// (#1190).
#[cfg(test)]
mod user_lexicon_context_id_tests {
    use std::collections::HashMap;
    use std::io::Cursor;

    use crate::test_fixtures::train_wide_model;

    /// The result of `write_dictionary` and what it wrote: `lex.csv`,
    /// `matrix.def`, `unk.def` and the user lexicon.
    type Written = (anyhow::Result<()>, String, String, String, String);

    fn write_dictionary(user_csv: &str) -> Written {
        let mut model = train_wide_model(1);
        model
            .read_user_lexicon(Cursor::new(user_csv.as_bytes()))
            .unwrap();
        let (mut lex, mut matrix, mut unk, mut user) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let result = model.write_dictionary(&mut lex, &mut matrix, &mut unk, &mut user);
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
        (result, text(lex), text(matrix), text(unk), text(user))
    }

    /// Maps the surface of each row of a lexicon CSV whose surfaces are not
    /// quoted to its left and right context ids.
    fn context_ids(csv: &str) -> HashMap<&str, (&str, &str)> {
        csv.lines()
            .map(|line| {
                let fields: Vec<&str> = line.splitn(4, ',').collect();
                (fields[0], (fields[1], fields[2]))
            })
            .collect()
    }

    #[test]
    fn an_entry_gets_the_context_ids_of_its_most_similar_seed_row() {
        // `語01` is seed row 1 and `語40` seed row 16 of `train_wide_model`.
        // Before #1190 `Q` got the ids of `R`, the user entry at index 1, and
        // `S` and `T` those of `Q`, the first user entry. `T`'s seventh column
        // is that of no seed row; the `動詞,自立` rows are the most similar,
        // and `語40` is the first of them.
        let user_csv = "Q,0,0,0,名詞,一般,*,*,*,*,語01,カナ,カナ\n\
                        R,7,8,912,動詞,自立,*,*,*,*,R,カナ,カナ\n\
                        S,0,0,0,動詞,自立,*,*,*,*,語40,カナ,カナ\n\
                        T,0,0,0,動詞,自立,*,*,*,*,T,カナ,カナ\n";
        let (result, lex, _, _, user) = write_dictionary(user_csv);
        result.unwrap();
        let seed = context_ids(&lex);
        let entries = context_ids(&user);

        // The ids the old lookup gave differ from the expected ones.
        assert_ne!(seed["語01"], ("7", "8"), "{lex}");
        assert_ne!(seed["語40"], seed["語01"], "{lex}");

        assert_eq!(entries["Q"], seed["語01"], "{user}");
        assert_eq!(entries["S"], seed["語40"], "{user}");
        assert_eq!(entries["T"], seed["語40"], "{user}");
        assert_eq!(
            user.lines().nth(1),
            Some("R,7,8,912,動詞,自立,*,*,*,*,R,カナ,カナ")
        );
    }

    #[test]
    fn an_entry_with_no_column_in_common_with_any_seed_row_is_an_error() {
        // Before #1190 `Z` got `4,4`, the index of ALPHA in char.def.
        let user_csv = "Q,0,0,0,名詞,一般,*,*,*,*,語01,カナ,カナ\nZ,0,0,0,X\n";
        let (result, lex, matrix, unk, user) = write_dictionary(user_csv);
        let err = match result {
            Ok(()) => panic!("an error was expected, got the user lexicon:\n{user}"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("\"Z\"") && err.contains("\"X\""), "{err}");
        // Nothing is written, not even the user entry before `Z`.
        assert_eq!([lex, matrix, unk, user], ["", "", "", ""]);

        // An entry whose parameters are given needs no seed row.
        let (result, _, _, _, user) = write_dictionary("W,3,3,100,X\n");
        result.unwrap();
        assert_eq!(user, "W,3,3,100,X\n");
    }
}

/// A user lexicon entry without a cost gets the one `lex.csv` gives a seed
/// row with its unigram features, and the cost and the context ids are
/// filled in separately, as `mecab-dict-index` fills them in (#1198).
#[cfg(test)]
mod user_lexicon_cost_tests {
    use std::collections::HashMap;
    use std::io::Cursor;

    use crate::model::Model;
    use crate::test_fixtures::{fixtures, wide_seed_and_corpus};
    use crate::{Corpus, Trainer, TrainerConfig};

    /// Trains the wide fixture with a `%t` and a `%u` unigram template
    /// added, with L2 regularization, under which the seed rows of
    /// `名詞,一般`, `名詞,固有名詞` and `助動詞,*` get different non-zero
    /// costs (the L1 of `train_wide_model` gives most rows 0).
    fn train_cost_model() -> Model {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = fixtures();
        let (seed, corpus) = wide_seed_and_corpus();
        let feature_def = format!("{feature_def}UNIGRAM U02:%t\nUNIGRAM U03:%u\n");
        let config = TrainerConfig::from_readers(
            Cursor::new(seed.as_bytes()),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def.as_bytes()),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(true)
            .max_iter(20)
            .num_threads(1)
            .train(Corpus::from_reader(Cursor::new(corpus.as_bytes())).unwrap())
            .unwrap()
    }

    /// The result of `write_dictionary` and what it wrote: `lex.csv`,
    /// `unk.def` and the user lexicon.
    type Written = (anyhow::Result<()>, String, String, String);

    /// Reads a user lexicon into a model trained by `train_cost_model` and
    /// returns what `write_dictionary` wrote.
    fn write_dictionary(user_csv: &str) -> Written {
        let mut model = train_cost_model();
        model
            .read_user_lexicon(Cursor::new(user_csv.as_bytes()))
            .unwrap();
        let (mut lex, mut matrix, mut unk, mut user) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let result = model.write_dictionary(&mut lex, &mut matrix, &mut unk, &mut user);
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
        (result, text(lex), text(unk), text(user))
    }

    /// Maps the surface of each row of a lexicon CSV whose surfaces are not
    /// quoted to its left id, right id and cost.
    fn parameters(csv: &str) -> HashMap<&str, (&str, &str, &str)> {
        csv.lines()
            .map(|line| {
                let fields: Vec<&str> = line.splitn(5, ',').collect();
                (fields[0], (fields[1], fields[2], fields[3]))
            })
            .collect()
    }

    /// Returns the feature of a row of a lexicon CSV whose surfaces are not
    /// quoted.
    fn feature<'a>(csv: &'a str, surface: &str) -> &'a str {
        csv.lines()
            .find_map(|line| line.strip_prefix(&format!("{surface},")))
            .map(|rest| rest.splitn(4, ',').nth(3).unwrap())
            .unwrap()
    }

    #[test]
    fn an_entry_gets_the_cost_lex_csv_gives_a_seed_row_with_its_feature() {
        // Before #1198 every entry got `tocost(0.8, cost_factor)`. Each entry
        // has the feature of a seed row, all its columns included so that its
        // `%u` feature is the row's, and a surface that starts with `語`, so
        // that its `%t` feature is the row's too.
        let (_, lex, _, _) = write_dictionary("");
        let seeds = ["語00", "語10", "語100", "語40"];
        let user_csv: String = seeds
            .iter()
            .enumerate()
            .map(|(i, seed)| format!("語X{i},0,0,0,{}\n", feature(&lex, seed)))
            .collect();

        let (result, lex, _, user) = write_dictionary(&user_csv);
        result.unwrap();
        let seed_rows = parameters(&lex);
        let entries = parameters(&user);

        // The first three rows' costs differ from each other and from 0, so
        // that an entry cannot get its row's cost by chance.
        let costs: Vec<&str> = seeds[..3].iter().map(|seed| seed_rows[seed].2).collect();
        assert!(
            costs[0] != costs[1] && costs[1] != costs[2] && costs[0] != costs[2],
            "{lex}"
        );
        assert!(!costs.contains(&"0"), "{lex}");

        for (i, seed) in seeds.iter().enumerate() {
            assert_eq!(
                entries[format!("語X{i}").as_str()],
                seed_rows[seed],
                "{user}"
            );
        }
    }

    #[test]
    fn an_entry_gets_the_t_feature_of_its_first_characters_category() {
        // Every seed surface starts with `語`, so the `%t` feature of KANJI
        // gets no weight, and the test above passes whatever `%t` an entry
        // gets. The unknown words of NUMERIC are only on wrong paths, so its
        // `%t` feature does get one. An `unk.def` label's features are made
        // as a seed row's are, with its category as `%t`, so an entry with
        // the feature of the NUMERIC line whose surface starts with a digit
        // gets the cost the exported `unk.def` gives that line.
        let numeric = "名詞,数,*,*,*,*,*,*,*";
        let user_csv = format!("1語,0,0,0,{numeric}\n語1,0,0,0,{numeric}\n");

        let (result, _, unk, user) = write_dictionary(&user_csv);
        result.unwrap();
        let numeric_cost = parameters(&unk)["NUMERIC"].2;
        let entries = parameters(&user);

        assert_eq!(entries["1語"].2, numeric_cost, "{unk}{user}");
        // The same feature with a KANJI first character gets another cost,
        // so the cost above does not come from the feature alone.
        assert_ne!(entries["語1"].2, numeric_cost, "{unk}{user}");
    }

    #[test]
    fn missing_parameters_are_filled_in_separately() {
        let (_, lex, _, _) = write_dictionary("");
        let row = feature(&lex, "語00").to_string();
        let (left_id, right_id, cost) = parameters(&lex)["語00"];
        // The given parameters below differ from the filled-in ones.
        assert!((left_id, right_id) != ("5", "6") && (left_id, right_id) != ("0", "0"));
        assert_ne!(cost, "0");

        let user_csv = format!(
            "語A,5,6,,{row}\n\
             語B,,,500,{row}\n\
             語C,-1,-1,500,{row}\n\
             語D,5,,500,{row}\n\
             語E,0,0,500,{row}\n\
             語F,,,,{row}\n\
             語G,0,0,0,{row}\n\
             語H,5,6,0,{row}\n"
        );
        let (result, _, _, user) = write_dictionary(&user_csv);
        result.unwrap();
        let entries = parameters(&user);

        // An empty cost is computed, and the given ids are kept.
        assert_eq!(entries["語A"], ("5", "6", cost), "{user}");
        // An empty or negative id makes both ids inferred, and the given
        // cost is kept.
        assert_eq!(entries["語B"], (left_id, right_id, "500"), "{user}");
        assert_eq!(entries["語C"], (left_id, right_id, "500"), "{user}");
        assert_eq!(entries["語D"], (left_id, right_id, "500"), "{user}");
        // `0` ids with a non-zero cost are kept, as in MeCab.
        assert_eq!(entries["語E"], ("0", "0", "500"), "{user}");
        // Empty fields, and `0,0,0`, fill in all three.
        assert_eq!(entries["語F"], (left_id, right_id, cost), "{user}");
        assert_eq!(entries["語G"], (left_id, right_id, cost), "{user}");
        // A cost of `0` with given ids is kept.
        assert_eq!(entries["語H"], ("5", "6", "0"), "{user}");
    }

    #[test]
    fn a_parameter_that_is_neither_empty_nor_a_number_in_range_is_an_error() {
        for (fields, expected) in [
            ("abc,1,0", "the left id \"abc\""),
            ("1,65536,0", "the right id \"65536\""),
            ("1,1,abc", "the cost \"abc\""),
            ("1,1,40000", "the cost \"40000\""),
        ] {
            let mut model = train_cost_model();
            let user_csv = format!("語A,0,0,0,名詞,一般\n語B,{fields},名詞,一般\n");
            let err = match model.read_user_lexicon(Cursor::new(user_csv.as_bytes())) {
                Ok(()) => panic!("{fields}: an error was expected"),
                Err(err) => err.to_string(),
            };
            assert!(
                err.contains("\"語B\"") && err.contains(expected),
                "{fields}: {err}"
            );
            // Not even the entry before `語B` is added.
            assert!(model.user_entries.is_empty(), "{fields}");
        }
    }
}

/// What `lindera export` and the bindings' export functions write agrees
/// with the model (#1169).
#[cfg(test)]
mod export_consistency_tests {
    use std::io::Cursor;

    use crate::model::{Model, SerializableModel};
    use crate::test_fixtures::{fixtures, wide_seed_and_corpus};
    use crate::{Corpus, Trainer, TrainerConfig};

    /// Trains the tiny fixture with at most `max_iter` iterations.
    fn train_fixture(max_iter: u64) -> Model {
        let (seed, char_def, unk_def, feature_def, rewrite_def, corpus) = fixtures();
        let config = TrainerConfig::from_readers(
            Cursor::new(seed),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(false)
            .max_iter(max_iter)
            .num_threads(1)
            .train(Corpus::from_reader(Cursor::new(corpus)).unwrap())
            .unwrap()
    }

    /// Writes the model and reads it back, as `lindera export` reads
    /// `model.dat`.
    fn read_back(model: &Model) -> SerializableModel {
        let mut dat = Vec::new();
        model.write_model(&mut dat).unwrap();
        Model::read_model(dat.as_slice()).unwrap()
    }

    #[test]
    fn the_model_records_the_iterations_that_ran() {
        // The fixture converges long before a high cap...
        let converged = read_back(&train_fixture(1000));
        let iterations = converged.metadata.iterations;
        assert!((1..1000).contains(&iterations), "{iterations} iterations");
        // ...and stops at a low one.
        assert_eq!(read_back(&train_fixture(2)).metadata.iterations, 2);

        // `metadata.json` reports the same number.
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("metadata.json");
        std::fs::write(&base, "{}").unwrap();
        let mut json = Vec::new();
        converged.update_metadata_json(&base, &mut json).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(json["model_info"]["training_iterations"], iterations);
    }

    // The model records the regularization type, and the Elastic Net ratio,
    // next to the coefficient: in `model.dat`, and in the `model_info` of
    // `metadata.json`, which lindera-dictionary reads back (#1272).
    #[test]
    fn the_model_records_the_regularization_type_and_the_ratio() {
        let train = |kind, ratio| {
            let (seed, char_def, unk_def, feature_def, rewrite_def, corpus) = fixtures();
            let config = TrainerConfig::from_readers(
                Cursor::new(seed),
                Cursor::new(char_def),
                Cursor::new(unk_def),
                Cursor::new(feature_def),
                Cursor::new(rewrite_def),
            )
            .unwrap();
            Trainer::new(config)
                .unwrap()
                .regularization_cost(0.01)
                .regularization_type(kind, ratio)
                .max_iter(3)
                .num_threads(1)
                .train(Corpus::from_reader(Cursor::new(corpus)).unwrap())
                .unwrap()
        };
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("metadata.json");
        std::fs::write(&base, "{}").unwrap();

        for (kind, recorded, name) in [
            (crate::RegularizationType::L1, None, "l1"),
            (crate::RegularizationType::L2, None, "l2"),
            (
                crate::RegularizationType::ElasticNet,
                Some(0.3),
                "elasticnet",
            ),
        ] {
            let model = read_back(&train(kind, 0.3));
            assert_eq!(model.metadata.regularization, 0.01, "{name}");
            assert_eq!(model.metadata.regularization_type, kind, "{name}");
            assert_eq!(model.metadata.elastic_net_l1_ratio, recorded, "{name}");

            let mut json = Vec::new();
            model.update_metadata_json(&base, &mut json).unwrap();
            let json: serde_json::Value = serde_json::from_slice(&json).unwrap();
            let info = &json["model_info"];
            assert_eq!(info["regularization"], 0.01, "{name}");
            assert_eq!(info["regularization_type"], name, "{name}");
            assert_eq!(
                info.get("elastic_net_l1_ratio")
                    .and_then(serde_json::Value::as_f64),
                recorded,
                "{name}"
            );
            let parsed: lindera_dictionary::dictionary::metadata::ModelInfo =
                serde_json::from_value(info.clone()).unwrap();
            assert_eq!(parsed.regularization_type.as_deref(), Some(name));
            assert_eq!(parsed.elastic_net_l1_ratio, recorded, "{name}");
        }
    }

    #[test]
    fn eta_reaches_the_optimizer() {
        let train = |eta: f64| {
            let (seed, char_def, unk_def, feature_def, rewrite_def, corpus) = fixtures();
            let config = TrainerConfig::from_readers(
                Cursor::new(seed),
                Cursor::new(char_def),
                Cursor::new(unk_def),
                Cursor::new(feature_def),
                Cursor::new(rewrite_def),
            )
            .unwrap();
            Trainer::new(config)
                .unwrap()
                .regularization_cost(0.01)
                .use_l2(false)
                .max_iter(1000)
                .eta(eta)
                .num_threads(1)
                .train(Corpus::from_reader(Cursor::new(corpus)).unwrap())
        };

        // A loose tolerance passes from the first iteration on, so the
        // convergence test stops training at its third check...
        let loose = train(1.0).unwrap();
        assert_eq!(read_back(&loose).metadata.iterations, 3);
        // ...and an invalid one is rejected when training starts.
        assert!(train(-1.0).is_err());
    }

    /// The names of the files in a directory, sorted.
    fn file_names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn export_writes_what_each_writer_writes() {
        let model = read_back(&train_fixture(5));
        let written = |write: &dyn Fn(&mut Vec<u8>) -> anyhow::Result<()>| {
            let mut buf = Vec::new();
            write(&mut buf).unwrap();
            buf
        };
        let mut expected = vec![
            ("lex.csv", written(&|w| model.write_lexicon(w))),
            ("matrix.def", written(&|w| model.write_connection_costs(w))),
            ("unk.def", written(&|w| model.write_unknown_dictionary(w))),
            ("char.def", written(&|w| model.write_char_def(w))),
            ("feature.def", written(&|w| model.write_feature_def(w))),
            ("rewrite.def", written(&|w| model.write_rewrite_def(w))),
            ("left-id.def", written(&|w| model.write_left_id_def(w))),
            ("right-id.def", written(&|w| model.write_right_id_def(w))),
        ];
        // The definition files are those the model was trained with.
        let (_, char_def, _, feature_def, rewrite_def, _) = fixtures();
        assert_eq!(expected[3].1, char_def.as_bytes());
        assert_eq!(expected[4].1, feature_def.as_bytes());
        assert_eq!(expected[5].1, rewrite_def.as_bytes());

        let dir = tempfile::tempdir().unwrap();
        let assert_exported = |out: &std::path::Path,
                               paths: Vec<std::path::PathBuf>,
                               expected: &[(&str, Vec<u8>)]| {
            let names: Vec<&str> = expected.iter().map(|(name, _)| *name).collect();
            let expected_paths: Vec<_> = names.iter().map(|name| out.join(name)).collect();
            assert_eq!(paths, expected_paths);
            let mut sorted: Vec<String> = names.iter().map(|name| name.to_string()).collect();
            sorted.sort();
            assert_eq!(file_names(out), sorted);
            for (name, bytes) in expected {
                assert_eq!(&std::fs::read(out.join(name)).unwrap(), bytes, "{name}");
            }
        };

        // Without a base `metadata.json`: the eight dictionary source files.
        let out = dir.path().join("without_metadata");
        let paths = model.export(&out, None).unwrap();
        assert_exported(&out, paths, &expected);

        // With one: `metadata.json` too.
        let base = dir.path().join("base.json");
        std::fs::write(&base, r#"{"name": "test"}"#).unwrap();
        let mut metadata = Vec::new();
        model.update_metadata_json(&base, &mut metadata).unwrap();
        expected.push(("metadata.json", metadata));
        let out = dir.path().join("with_metadata");
        let paths = model.export(&out, Some(&base)).unwrap();
        assert_exported(&out, paths, &expected);
    }

    #[test]
    fn export_writes_nothing_when_the_base_metadata_cannot_be_read() {
        let model = read_back(&train_fixture(5));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let missing = dir.path().join("missing.json");

        let err = model.export(&out, Some(&missing)).unwrap_err();
        assert!(err.to_string().contains("missing.json"), "{err}");
        assert!(!out.exists(), "{:?}", file_names(&out));
    }

    /// Trains the wide fixture with L2 regularization, under which its
    /// labels keep several context ids (the L1 of `train_wide_model` merges
    /// most of them).
    fn train_wide_l2_model() -> Model {
        let (_, char_def, unk_def, feature_def, rewrite_def, _) = fixtures();
        let (seed, corpus) = wide_seed_and_corpus();
        let config = TrainerConfig::from_readers(
            Cursor::new(seed.as_bytes()),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();
        Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .use_l2(true)
            .max_iter(20)
            .num_threads(1)
            .train(Corpus::from_reader(Cursor::new(corpus.as_bytes())).unwrap())
            .unwrap()
    }

    #[test]
    fn model_id_defs_are_those_of_the_exported_model() {
        let model = train_wide_l2_model();
        let serializable = read_back(&model);
        let written = |write: &dyn Fn(&mut Vec<u8>) -> anyhow::Result<()>| {
            let mut buf = Vec::new();
            write(&mut buf).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let left = written(&|w| model.write_left_id_def(w));
        let right = written(&|w| model.write_right_id_def(w));
        assert_eq!(left, written(&|w| serializable.write_left_id_def(w)));
        assert_eq!(right, written(&|w| serializable.write_right_id_def(w)));
        // `0 BOS/EOS` and at least two context ids, so that the files tell
        // the ids apart.
        assert!(left.lines().count() > 2, "{left}");
        assert!(right.lines().count() > 2, "{right}");
    }
}
