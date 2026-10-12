//! CRF-based dictionary training for Lindera.
//!
//! This crate implements the training pipeline (`lindera train`), producing
//! a serialized model that `lindera export` turns into MeCab-format
//! dictionary source files, which `lindera build` then compiles into a
//! binary dictionary. It builds on the runtime types of
//! [`lindera_dictionary`] and the CRF core of [`lindera_crf`].

pub mod config;
pub mod corpus;
pub mod feature_extractor;
pub mod feature_rewriter;
pub mod model;

use std::num::NonZeroU32;

use anyhow::Result;

use self::feature_extractor::TemplateContext;

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
/// - Unknown word categorization for 6 character types
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

    /// Maximum length (in characters) for tokens to be included in training.
    /// Tokens longer than this value will be skipped during lattice construction
    /// to improve training efficiency and avoid memory issues with extremely long tokens.
    /// Default: Some(10) - tokens with more than 10 characters are skipped.
    /// Set to None to include all tokens regardless of length.
    max_grouping_len: Option<usize>,

    provider: lindera_crf::FeatureProvider,

    /// Pre-allocated label IDs for unknown word categories.
    /// Index corresponds to character type categories:
    /// - 0: DEFAULT (fallback for unclassified characters)
    /// - 1: HIRAGANA (Japanese hiragana characters)
    /// - 2: KATAKANA (Japanese katakana characters)
    /// - 3: KANJI (Chinese/Japanese ideographic characters)
    /// - 4: ALPHA (ASCII alphabetic characters)
    /// - 5: NUMERIC (ASCII numeric characters)
    label_id_map_unk: Vec<NonZeroU32>,

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
    /// Gold tokens labelled with an unknown-word label.
    gold_unknown: usize,
    /// Gold tokens labelled with a label created during lattice building.
    gold_empty: usize,
    /// Negative edges from the seed lexicon.
    negative_lexicon: usize,
    /// Negative edges from unknown-word generation.
    negative_unknown: usize,
}

impl Trainer {
    /// Creates a new [`Trainer`] using the specified configuration.
    pub fn new(mut config: TrainerConfig) -> Result<Self> {
        let mut provider = lindera_crf::FeatureProvider::default();

        // Build label mapping from surfaces and add feature sets to provider
        // Generate default features based on dictionary schema
        let default_features = if let Some(first_feature) = config.features.first() {
            let field_count = first_feature.split(',').count();
            vec!["*"; field_count].join(",")
        } else {
            "*".to_string()
        };

        for (i, surface) in config.surfaces.iter().enumerate() {
            // Get feature string from config.features (parallel to surfaces)
            let feature_str: &str = if i < config.features.len() {
                &config.features[i]
            } else {
                &default_features
            };

            // Apply dictionary rewriter to get ufeature, lfeature, rfeature.
            // Uncached on purpose: real feature strings are unique per row,
            // so a cache keyed on them never hits (#975).
            let (ufeature, lfeature, rfeature) = config.dictionary_rewriter.rewrite(feature_str);
            let u_vec: Vec<&str> = ufeature.split(',').collect();
            let l_vec: Vec<&str> = lfeature.split(',').collect();
            let r_vec: Vec<&str> = rfeature.split(',').collect();

            // The %t feature: the default category (MeCab's default_type) of
            // the first character of the surface, or 0 (DEFAULT) if none
            let cate_id = surface
                .chars()
                .next()
                .and_then(|first_char| config.default_category(first_char))
                .unwrap_or(0);

            // Create feature set for this vocabulary entry
            let feature_extractor = &mut config.feature_extractor;
            let ctx = TemplateContext {
                surface: Some(surface),
                ufeature: Some(&ufeature),
                lfeature: Some(&lfeature),
                rfeature: Some(&rfeature),
            };

            let unigram_ids =
                feature_extractor.extract_unigram_feature_ids_with_ctx(&u_vec, cate_id, &ctx);
            let left_ids = feature_extractor.extract_left_feature_ids_with_ctx(&l_vec, &ctx);
            let right_ids = feature_extractor.extract_right_feature_ids_with_ctx(&r_vec, &ctx);

            let feature_set = lindera_crf::FeatureSet::new(&unigram_ids, &right_ids, &left_ids);

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

        // Initialize unknown word labels from character definition categories
        let mut label_id_map_unk = Vec::new();
        let char_def = &config.dict.character_definition;
        let unk_category_names = char_def.categories();

        for (i, category) in unk_category_names.iter().enumerate() {
            // Get unknown word feature string from unk_categories
            let unk_feature: &str = config
                .unk_categories
                .get(category)
                .map(|s| s.as_str())
                .unwrap_or(&default_features);

            // Apply dictionary rewriter to get ufeature, lfeature, rfeature.
            // Uncached on purpose: see the seed loop above (#975).
            let (ufeature, lfeature, rfeature) = config.dictionary_rewriter.rewrite(unk_feature);
            let u_vec: Vec<&str> = ufeature.split(',').collect();
            let l_vec: Vec<&str> = lfeature.split(',').collect();
            let r_vec: Vec<&str> = rfeature.split(',').collect();

            // Create feature set for unknown word category
            let feature_extractor = &mut config.feature_extractor;
            let ctx = TemplateContext {
                surface: None,
                ufeature: Some(&ufeature),
                lfeature: Some(&lfeature),
                rfeature: Some(&rfeature),
            };

            let unigram_ids =
                feature_extractor.extract_unigram_feature_ids_with_ctx(&u_vec, i as u32, &ctx);
            let left_ids = feature_extractor.extract_left_feature_ids_with_ctx(&l_vec, &ctx);
            let right_ids = feature_extractor.extract_right_feature_ids_with_ctx(&r_vec, &ctx);

            let feature_set = lindera_crf::FeatureSet::new(&unigram_ids, &right_ids, &left_ids);

            // Add to provider
            let unk_label_id = provider.add_feature_set(feature_set)?;
            label_id_map_unk.push(unk_label_id);
        }

        Ok(Self {
            config,
            max_grouping_len: None, // Default: no length limit for feature grouping
            provider,
            label_id_map_unk,
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

    /// Sets the maximum grouping length for token sequences.
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
            "Lattice statistics: gold_lexicon={} gold_unknown={} gold_empty={} negative_lexicon={} negative_unknown={}",
            stats.gold_lexicon,
            stats.gold_unknown,
            stats.gold_empty,
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

    /// Extracts labels from the configuration
    fn extract_labels(&self) -> Vec<String> {
        let mut labels = self.config.surfaces.clone();
        // Add unknown word category labels from character definition
        // This makes it work for any dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT, etc.)
        let char_def = &self.config.dict.character_definition;
        for category_name in char_def.categories() {
            labels.push(category_name.to_string());
        }
        labels
    }

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

        // Add positive edges from training data
        let mut edges = vec![];
        let mut pos = 0;
        for token in &example.tokens {
            let token_len = token.surface().chars().count();

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
                            .is_some_and(|feature| feature == token.feature())
                })
            });
            let label_id = if let Some(m) = lexicon_match {
                self.stats.gold_lexicon += 1;
                lexicon_label(m.word_idx.word_id as usize)?
            } else {
                // Check for compatible unknown word first
                match self.config.dict().unknown_dictionary.compatible_unk_index(
                    &example.sentence,
                    pos,
                    pos + token_len,
                    token.feature(),
                ) {
                    Some(unk_index) => {
                        self.stats.gold_unknown += 1;
                        self.label_id_map_unk[unk_index.word_id as usize]
                    }
                    None => {
                        self.stats.gold_empty += 1;
                        self.provider.add_feature_set(lindera_crf::FeatureSet::new(
                            &[],
                            &[],
                            &[],
                        ))?
                    }
                }
            };

            edges.push((pos, Edge::new(pos + token_len, label_id)));
            pos += token_len;
        }
        assert_eq!(pos, input_len);

        let mut lattice = Lattice::new(input_len)?;

        // Add positive edges to lattice
        for (pos, edge) in edges {
            lattice.add_edge(pos, edge)?;
        }

        // Add negative edges: the seed words, then the unknown words
        for (start_word, matches) in lexicon.iter().enumerate() {
            let has_matched = !matches.is_empty();

            for m in matches {
                let label_id = lexicon_label(m.word_idx.word_id as usize)?;
                let pos = start_word;
                let target = pos + m.end_char;
                let edge = Edge::new(target, label_id);

                // Skip adding if the edge is already added as a positive edge
                if let Some(first_edge) = lattice.nodes()[pos].edges().first()
                    && edge == *first_edge
                {
                    continue;
                }
                lattice.add_edge(pos, edge)?;
                self.stats.negative_lexicon += 1;
            }

            // Generate unknown words using callback system
            let sentence: String = input_chars.iter().collect();
            let mut negative_unknown = 0;
            self.config.unk_handler().gen_unk_words(
                &sentence,
                start_word,
                has_matched,
                self.max_grouping_len,
                |w| {
                    let id_offset = self.config.surfaces.len() as u32;
                    let label_id = NonZeroU32::new(id_offset + w.word_idx().word_id + 1).unwrap(); // Offset for unknown words
                    let pos = start_word;
                    let target = w.end_char();
                    let edge = lindera_crf::Edge::new(target, label_id);

                    // Skip adding if the edge is already added as a positive edge
                    if let Some(first_edge) = lattice.nodes()[pos].edges().first()
                        && edge == *first_edge
                    {
                        return;
                    }
                    lattice.add_edge(pos, edge).unwrap();
                    negative_unknown += 1;
                },
            );
            self.stats.negative_unknown += negative_unknown;
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

        // Verify unknown word categories are preserved
        assert!(
            !serializable.unk_categories.is_empty(),
            "Unknown word categories should not be empty"
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
