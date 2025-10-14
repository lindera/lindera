pub mod config;
pub mod corpus;
pub mod feature_extractor;
pub mod feature_rewriter;
pub mod model;

use std::collections::HashMap;
use std::num::NonZeroU32;

use anyhow::Result;

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

/// Match structure for common prefix iterator
#[derive(Debug, Clone)]
pub struct Match {
    pub word_idx: WordIdx,
    pub end_char: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct WordIdx {
    pub word_id: u32,
}

impl WordIdx {
    pub fn new(word_id: u32) -> Self {
        Self { word_id }
    }
}

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
/// use lindera_dictionary::trainer::{Trainer, TrainerConfig};
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

    provider: rucrf::FeatureProvider,

    // Maps feature strings to label IDs
    label_id_map: std::collections::HashMap<String, std::collections::HashMap<char, NonZeroU32>>,

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
    max_iter: u64,
    num_threads: usize,
}

impl Trainer {
    /// Creates a new [`Trainer`] using the specified configuration.
    pub fn new(mut config: TrainerConfig) -> Result<Self> {
        let mut provider = rucrf::FeatureProvider::default();
        let mut label_id_map = HashMap::new();

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
            let feature_str = if i < config.features.len() {
                &config.features[i]
            } else {
                &default_features
            };

            // Create feature set for this vocabulary entry
            let feature_extractor = &mut config.feature_extractor;
            let features_vec: Vec<String> = feature_str.split(',').map(|s| s.to_string()).collect();

            let unigram_ids =
                feature_extractor.extract_unigram_feature_ids(&features_vec, i as u32);
            let left_ids = feature_extractor.extract_left_feature_ids(&features_vec);
            let right_ids = feature_extractor.extract_right_feature_ids(&features_vec);

            let feature_set = rucrf::FeatureSet::new(&unigram_ids, &right_ids, &left_ids);

            // Add feature set to provider and get label ID
            let label_id = provider.add_feature_set(feature_set)?;

            // Map feature string to label ID using first character classification
            label_id_map
                .entry(feature_str.to_string())
                .or_insert_with(HashMap::new);
            if let Some(first_char) = surface.chars().next() {
                label_id_map
                    .get_mut(feature_str)
                    .unwrap()
                    .insert(first_char, label_id);
            }
        }

        // Initialize unknown word labels from character definition categories
        let mut label_id_map_unk = Vec::new();
        let char_def = &config.dict.character_definition;
        let unk_category_names = char_def.categories();

        for (i, category) in unk_category_names.iter().enumerate() {
            // Get unknown word feature string from unk_categories
            let unk_feature = config
                .unk_categories
                .get(category)
                .cloned()
                .unwrap_or_else(|| default_features.clone());

            // Create feature set for unknown word category
            let feature_extractor = &mut config.feature_extractor;
            let features_vec: Vec<String> = unk_feature.split(',').map(|s| s.to_string()).collect();

            let unigram_ids =
                feature_extractor.extract_unigram_feature_ids(&features_vec, i as u32);
            let left_ids = feature_extractor.extract_left_feature_ids(&features_vec);
            let right_ids = feature_extractor.extract_right_feature_ids(&features_vec);

            let feature_set = rucrf::FeatureSet::new(&unigram_ids, &right_ids, &left_ids);

            // Add to provider
            let unk_label_id = provider.add_feature_set(feature_set)?;
            label_id_map_unk.push(unk_label_id);
        }

        Ok(Self {
            config,
            max_grouping_len: None, // Default: no length limit for feature grouping
            provider,
            label_id_map,
            label_id_map_unk,
            regularization_cost: 0.01,
            max_iter: 100,
            num_threads: 8,
        })
    }

    /// Sets the regularization cost (L1 regularization coefficient).
    pub fn regularization_cost(mut self, cost: f64) -> Self {
        self.regularization_cost = cost;
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
    fn build_lattices_from_corpus(&mut self, corpus: &Corpus) -> Result<Vec<rucrf::Lattice>> {
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

        Ok(lattices)
    }

    /// Configure and execute CRF training
    fn train_crf_model(&mut self, lattices: Vec<rucrf::Lattice>) -> Result<rucrf::RawModel> {
        log_info!("Starting CRF training with {} lattices...", lattices.len());
        log_info!(
            "Training parameters: regularization={}, max_iter={}, threads={}",
            self.regularization_cost,
            self.max_iter,
            self.num_threads
        );

        // Configure the CRF trainer
        let trainer = rucrf::Trainer::new()
            .regularization(rucrf::Regularization::L1, self.regularization_cost)?
            .max_iter(self.max_iter)?
            .n_threads(self.num_threads)?;

        self.execute_training(trainer, lattices)
    }

    /// Execute the actual CRF training with detailed logging
    fn execute_training(
        &mut self,
        trainer: rucrf::Trainer,
        lattices: Vec<rucrf::Lattice>,
    ) -> Result<rucrf::RawModel> {
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
    fn log_training_results(&self, model: &rucrf::RawModel) {
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
        crf_model: rucrf::RawModel,
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
    fn extract_feature_weights(&self, crf_model: &rucrf::RawModel) -> Vec<f64> {
        println!("Extracted feature weights from trained model");

        let mut feature_weights = Vec::new();
        match crf_model.merge() {
            Ok(merged_model) => {
                println!(
                    "DEBUG: merged_model.feature_sets.len() = {}",
                    merged_model.feature_sets.len()
                );

                // feature_sets is indexed by label ID (0-based index corresponds to label ID)
                // Extract weights in the order of surfaces
                for (i, _surface) in self.config.surfaces.iter().enumerate() {
                    // The feature_sets vector is indexed by label ID
                    // Since label IDs are 1-based in the CRF model, but 0-based in our surfaces
                    // we directly use the index
                    if i < merged_model.feature_sets.len() {
                        feature_weights.push(merged_model.feature_sets[i].weight);
                    } else {
                        // No weight found for this label, use 0.0
                        feature_weights.push(0.0);
                    }
                }

                println!(
                    "DEBUG: Extracted {} weights from merged model",
                    feature_weights.len()
                );
                // Count non-zero weights
                let non_zero_count = feature_weights.iter().filter(|&&w| w != 0.0).count();
                println!(
                    "DEBUG: Non-zero weights: {}/{}",
                    non_zero_count,
                    feature_weights.len()
                );
            }
            Err(e) => {
                println!("DEBUG: merge() failed: {e}, using raw weights");
                self.use_raw_weights(crf_model, &mut feature_weights);
            }
        }

        println!(
            "DEBUG: First 10 feature weights: {:?}",
            &feature_weights[..std::cmp::min(10, feature_weights.len())]
        );
        println!(
            "DEBUG: Passing feature_weights to Model: {:?}",
            &feature_weights[..std::cmp::min(5, feature_weights.len())]
        );

        feature_weights
    }

    /// Use raw CRF model weights as fallback
    fn use_raw_weights(&self, crf_model: &rucrf::RawModel, feature_weights: &mut Vec<f64>) {
        let raw_weights = crf_model.weights();
        println!(
            "DEBUG: Fallback to raw weights, count: {}",
            raw_weights.len()
        );

        for i in 0..self.config.surfaces.len() {
            if i < raw_weights.len() {
                feature_weights.push(raw_weights[i]);
            } else {
                feature_weights.push(0.0);
            }
        }
        println!("DEBUG: Used raw model weights");
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

    fn build_lattice(&mut self, example: &Example) -> Result<rucrf::Lattice> {
        use rucrf::{Edge, Lattice};

        let input_chars: Vec<char> = example.sentence.chars().collect();
        let input_len = input_chars.len();

        // Add positive edges from training data
        let mut edges = vec![];
        let mut pos = 0;
        for token in &example.tokens {
            let token_len = token.surface().chars().count();
            let first_char = input_chars[pos];

            // Try to find existing label ID, or create one
            let label_id = self
                .label_id_map
                .get(token.feature())
                .and_then(|hm| hm.get(&first_char))
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| {
                    // Check for compatible unknown word first
                    self.config
                        .dict()
                        .unknown_dictionary
                        .compatible_unk_index(
                            &example.sentence,
                            pos,
                            pos + token_len,
                            token.feature(),
                        )
                        .map_or_else(
                            || {
                                self.provider
                                    .add_feature_set(rucrf::FeatureSet::new(&[], &[], &[]))
                            },
                            |unk_index| Ok(self.label_id_map_unk[unk_index.word_id as usize]),
                        )
                })?;

            edges.push((pos, Edge::new(pos + token_len, label_id)));
            pos += token_len;
        }
        assert_eq!(pos, input_len);

        let mut lattice = Lattice::new(input_len)?;

        // Add positive edges to lattice
        for (pos, edge) in edges {
            lattice.add_edge(pos, edge)?;
        }

        // Add negative edges using optimized unknown word generation
        for start_word in 0..input_len {
            let mut has_matched = false;

            let suffix = &input_chars[start_word..];

            // System lexicon matching with common_prefix_iterator
            for m in self.config.system_lexicon().common_prefix_iterator(suffix) {
                has_matched = true;
                let label_id = NonZeroU32::new(m.word_idx.word_id + 1).unwrap(); // word_id is 0-based, NonZeroU32 needs +1
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
            }

            // Generate unknown words using callback system
            let sentence: String = input_chars.iter().collect();
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
                    let edge = rucrf::Edge::new(target, label_id);

                    // Skip adding if the edge is already added as a positive edge
                    if let Some(first_edge) = lattice.nodes()[pos].edges().first()
                        && edge == *first_edge
                    {
                        return;
                    }
                    lattice.add_edge(pos, edge).unwrap();
                },
            );
        }

        Ok(lattice)
    }

    /// Remove unused features from the feature extractor to optimize the model size and performance
    fn remove_unused_features(&mut self, model: &rucrf::RawModel) {
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
