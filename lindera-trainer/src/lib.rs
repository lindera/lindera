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
pub mod feature_rewriter;
mod lexicon_csv;
pub mod model;

use std::collections::HashMap;
use std::num::NonZeroU32;

use anyhow::Result;

use lindera_dictionary::viterbi::UnknownWordCandidate;

use self::feature_csv::{split_feature, wildcard_feature};
use self::feature_extractor::{FeatureExtractor, TemplateContext};
use self::feature_rewriter::DictionaryRewriter;

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

pub use self::config::TrainerConfig;
pub use self::corpus::{Corpus, Example, Word};
pub use self::model::{Model, SerializableModel};

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
/// let unk_def = "DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n";
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
///     .regularization_cost(0.01)
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

    regularization_cost: f64,
    use_l2: bool,
    elastic_net_l1_ratio: Option<f64>,
    max_iter: u64,
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
/// * `extractor` - Expands the feature templates; it assigns ids to the
///   features it has not seen.
/// * `feature` - The label's feature string.
/// * `char_type` - The `%t` value.
/// * `surface` - The `%w` value, if any.
///
/// # Returns
///
/// The feature set.
fn label_feature_set(
    rewriter: &DictionaryRewriter,
    extractor: &mut FeatureExtractor,
    feature: &str,
    char_type: u32,
    surface: Option<&str>,
) -> lindera_crf::FeatureSet {
    // Uncached on purpose: real feature strings are unique per seed row, so
    // a cache keyed on them never hits (#975).
    let (ufeature, lfeature, rfeature) = rewriter.rewrite(feature);
    // Split as MeCab splits the rewritten features for its templates, so a
    // quoted column with `,` in it is one column (#1184).
    let (u_columns, l_columns, r_columns) = (
        split_feature(&ufeature),
        split_feature(&lfeature),
        split_feature(&rfeature),
    );
    let u_vec: Vec<&str> = u_columns.iter().map(|column| column.as_ref()).collect();
    let l_vec: Vec<&str> = l_columns.iter().map(|column| column.as_ref()).collect();
    let r_vec: Vec<&str> = r_columns.iter().map(|column| column.as_ref()).collect();
    let ctx = TemplateContext {
        surface,
        ufeature: Some(&ufeature),
        lfeature: Some(&lfeature),
        rfeature: Some(&rfeature),
    };
    let unigram_ids = extractor.extract_unigram_feature_ids_with_ctx(&u_vec, char_type, &ctx);
    let left_ids = extractor.extract_left_feature_ids_with_ctx(&l_vec, &ctx);
    let right_ids = extractor.extract_right_feature_ids_with_ctx(&r_vec, &ctx);
    lindera_crf::FeatureSet::new(&unigram_ids, &right_ids, &left_ids)
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
                &mut config.feature_extractor,
                feature_str,
                cate_id,
                Some(surface),
            );

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
        // unknown words the entry creates; an entry whose category char.def
        // does not define never becomes a candidate, and gets 0.
        let mut unk_entry_labels = Vec::with_capacity(config.unk_entries.len());
        for (category, feature) in &config.unk_entries {
            let char_type = config
                .dict
                .character_definition
                .category_id_by_name(category)
                .map_or(0, |category_id| category_id.0 as u32);
            let feature_set = label_feature_set(
                &config.dictionary_rewriter,
                &mut config.feature_extractor,
                feature,
                char_type,
                None,
            );
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
            regularization_cost: 0.01,
            use_l2: false,
            elastic_net_l1_ratio: None,
            max_iter: 100,
            num_threads: 8,
            stats: LatticeStats::default(),
        })
    }

    /// Sets the regularization cost coefficient.
    pub fn regularization_cost(mut self, cost: f64) -> Self {
        self.regularization_cost = cost;
        self
    }

    /// Sets whether to use L2 regularization (default: L1).
    pub fn use_l2(mut self, l2: bool) -> Self {
        self.use_l2 = l2;
        self
    }

    /// Sets the Elastic Net L1 ratio (0.0 to 1.0).
    /// When set, uses Elastic Net regularization instead of pure L1 or L2.
    /// l1_ratio=1.0 is pure L1, l1_ratio=0.0 is pure L2.
    pub fn elastic_net_l1_ratio(mut self, ratio: f64) -> Self {
        self.elastic_net_l1_ratio = Some(ratio);
        self
    }

    /// Sets the maximum number of iterations.
    pub fn max_iter(mut self, iter: u64) -> Self {
        self.max_iter = iter;
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

    /// Get the number of threads
    pub fn get_num_threads(&self) -> usize {
        self.num_threads
    }

    /// Trains a model from the given corpus.
    pub fn train(mut self, corpus: Corpus) -> Result<Model> {
        let lattices = self.build_lattices_from_corpus(&corpus)?;
        let labels = self.extract_labels();
        let crf_model = self.train_crf_model(lattices)?;

        self.create_final_model(crf_model, labels, corpus)
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

        Ok(lattices)
    }

    /// Configure and execute CRF training
    fn train_crf_model(
        &mut self,
        lattices: Vec<lindera_crf::Lattice>,
    ) -> Result<lindera_crf::RawModel> {
        log_info!("Starting CRF training with {} lattices...", lattices.len());

        // Determine regularization type
        let reg_type = if let Some(l1_ratio) = self.elastic_net_l1_ratio {
            log_info!(
                "Training parameters: regularization={} (ElasticNet, l1_ratio={}), max_iter={}, threads={}",
                self.regularization_cost,
                l1_ratio,
                self.max_iter,
                self.num_threads
            );
            lindera_crf::Regularization::ElasticNet { l1_ratio }
        } else if self.use_l2 {
            log_info!(
                "Training parameters: regularization={} (L2), max_iter={}, threads={}",
                self.regularization_cost,
                self.max_iter,
                self.num_threads
            );
            lindera_crf::Regularization::L2
        } else {
            log_info!(
                "Training parameters: regularization={} (L1), max_iter={}, threads={}",
                self.regularization_cost,
                self.max_iter,
                self.num_threads
            );
            lindera_crf::Regularization::L1
        };

        // Configure the CRF trainer
        let trainer = lindera_crf::Trainer::new()
            .regularization(reg_type, self.regularization_cost)?
            .max_iter(self.max_iter)?
            .n_threads(self.num_threads)?;

        self.execute_training(trainer, lattices)
    }

    /// Execute the actual CRF training with detailed logging
    fn execute_training(
        &mut self,
        trainer: lindera_crf::Trainer,
        lattices: Vec<lindera_crf::Lattice>,
    ) -> Result<lindera_crf::RawModel> {
        println!("L-BFGS optimization starting...");
        println!(
            "Note: This may take several minutes for large datasets. Progress will be shown by L-BFGS iterations above."
        );
        println!("Each 'iter:' line indicates training progress. Please wait...");

        let start_time = std::time::Instant::now();

        // Training with provider (consumes provider as per CRF training requirements)
        let provider = std::mem::take(&mut self.provider);
        let model = trainer.train(&lattices, provider);
        let training_duration = start_time.elapsed();

        self.log_training_results(&model);
        println!(
            "Training completed successfully in {:.2}s!",
            training_duration.as_secs_f64()
        );

        Ok(model)
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
    fn create_final_model(
        mut self,
        crf_model: lindera_crf::RawModel,
        labels: Vec<String>,
        _corpus: Corpus,
    ) -> Result<Model> {
        // Remove unused features from feature extractor to optimize model size
        self.remove_unused_features(&crf_model);

        // Extract feature weights from the trained model
        let feature_weights = self.extract_feature_weights(&crf_model);

        // Create final model with metadata
        Ok(Model::new_with_metadata(
            crf_model,
            self.config,
            feature_weights,
            labels,
            self.virtual_features,
            self.regularization_cost,
            self.max_iter,
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
            &mut self.config.feature_extractor,
            feature,
            VIRTUAL_CHAR_TYPE,
            None,
        );
        let label = self.provider.add_feature_set(feature_set)?;
        self.virtual_labels.insert(feature.to_string(), label);
        self.virtual_features.push(feature.to_string());
        Ok(label)
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
    /// gold feature string. Every other candidate is a negative edge.
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

        let mut used_right_features = HashSet::new();

        // Collect all feature keys to check
        let unigram_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .unigram_feature_ids
            .keys()
            .cloned()
            .collect();
        let left_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .left_feature_ids
            .keys()
            .cloned()
            .collect();
        let right_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .right_feature_ids
            .keys()
            .cloned()
            .collect();

        // Remove unused unigram features to reduce model complexity
        for k in &unigram_feature_keys {
            let id = self
                .config
                .feature_extractor
                .unigram_feature_ids
                .get(k)
                .unwrap();
            if model
                .unigram_weight_indices()
                .get((id.get() - 1) as usize)
                .cloned()
                .flatten()
                .is_none()
            {
                self.config.feature_extractor.unigram_feature_ids.remove(k);
            }
        }

        // Collect used right features from bigram weights
        for feature_ids in model.bigram_weight_indices() {
            for (feature_id, _) in feature_ids {
                used_right_features.insert(*feature_id);
            }
        }

        // Remove unused left features
        for k in &left_feature_keys {
            let id = self
                .config
                .feature_extractor
                .left_feature_ids
                .get(k)
                .unwrap();
            if let Some(x) = model.bigram_weight_indices().get(id.get() as usize)
                && x.is_empty()
            {
                self.config.feature_extractor.left_feature_ids.remove(k);
            }
        }

        // Remove unused right features
        for k in &right_feature_keys {
            let id = self
                .config
                .feature_extractor
                .right_feature_ids
                .get(k)
                .unwrap();
            if !used_right_features.contains(&id.get()) {
                self.config.feature_extractor.right_feature_ids.remove(k);
            }
        }

        println!("Feature cleanup completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
        // Without a BIGRAM template no bigram weight exists: the state L1
        // reaches on `resources/training/` once every bigram weight is
        // zeroed. The entries still carry context ids, so the matrix must
        // cover them; it used to be written as `0 0`.
        let model = train_fixture_with_feature_def("UNIGRAM U00:%F[0]\n");
        let mut buf = Vec::new();
        model.write_model(&mut buf).unwrap();
        let serializable = model::Model::read_model(Cursor::new(&buf)).unwrap();
        assert!(
            serializable.connection_matrix.is_empty(),
            "precondition: no bigram weight"
        );

        let (rows, cols) = matrix_header(&written(|w| serializable.write_connection_costs(w)));
        assert!(rows > 1 && cols > 1, "matrix {rows}x{cols}");
        assert_entries_inside_matrix(&serializable);

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
    /// model.
    pub(crate) fn train_wide_model(threads: usize) -> crate::model::Model {
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
            Ok(trainer) => trainer,
            Err(err) => panic!("failed to build trainer: {err}"),
        }
        .regularization_cost(0.01)
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
        let model = crate::test_fixtures::train_wide_model(threads);
        let mut buf = Vec::new();
        if let Err(err) = model.write_model(&mut buf) {
            panic!("failed to serialize model: {err}");
        }
        buf
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
        let config = match TrainerConfig::from_readers(
            Cursor::new(SEED),
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
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::num::NonZeroU32;

    use crate::feature_extractor::FeatureExtractor;
    use crate::feature_rewriter::DictionaryRewriter;
    use crate::{label_feature_set, unk_entry_accepts};

    /// The feature of the issue's reproduction: Jieba-like columns whose
    /// fifth, a definition, is quoted and has a comma in it.
    const FEATURE: &str = "n,dian4,店,店,\"shop, store\",1";

    /// Returns the feature strings an extractor has interned, sorted.
    fn interned(ids: &HashMap<String, NonZeroU32>) -> Vec<&str> {
        let mut features: Vec<&str> = ids.keys().map(String::as_str).collect();
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

        label_feature_set(&rewriter, &mut extractor, FEATURE, 0, None);

        // MeCab's learner gives `U04:shop, store` and `U05:1` for the same
        // feature and unigram rule.
        assert_eq!(
            interned(&extractor.unigram_feature_ids),
            ["U04:shop, store", "U05:1"]
        );
        assert_eq!(interned(&extractor.left_feature_ids), ["L1:shop, store"]);
        assert_eq!(interned(&extractor.right_feature_ids), ["R1:1"]);
    }

    #[test]
    fn user_lexicon_features_read_a_quoted_column_as_one_column() {
        // The bigram templates of the fixtures are `%L[0]` and `%R[0]`, and
        // no rule of their rewrite.def matches the entry, so its left and
        // right features are its feature, and `write_left_id_def` and
        // `write_right_id_def` list its first column.
        let mut model = crate::test_fixtures::train_wide_model(1);
        if let Err(err) = model.read_user_lexicon(Cursor::new("X,0,0,0,\"名詞,固有\",*\n")) {
            panic!("user lexicon: {err}");
        }
        let mut left_ids = Vec::new();
        let mut right_ids = Vec::new();
        if let Err(err) = model.write_left_id_def(&mut left_ids) {
            panic!("left-id.def: {err}");
        }
        if let Err(err) = model.write_right_id_def(&mut right_ids) {
            panic!("right-id.def: {err}");
        }

        for ids in [left_ids, right_ids] {
            let ids = String::from_utf8_lossy(&ids);
            assert!(
                ids.lines().any(|line| line.ends_with(" 名詞,固有")),
                "{ids}"
            );
            assert!(!ids.contains('"'), "{ids}");
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

/// A feature template that reads past the last column of a label's feature
/// gives the label no feature, as in MeCab's learner (#1189).
#[cfg(test)]
mod template_past_last_column_tests {
    use std::io::Cursor;

    use crate::feature_extractor::FeatureExtractor;
    use crate::feature_rewriter::DictionaryRewriter;
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

        let feature_set = label_feature_set(&rewriter, &mut extractor, "n,*,*,*,*", 0, None);

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
    /// costs (the L1 default of `train_wide_model` gives most rows 0).
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
