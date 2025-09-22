pub mod config;
pub mod corpus;
pub mod feature_extractor;
pub mod feature_rewriter;
pub mod model;

use std::collections::HashMap;
use std::num::NonZeroU32;

use anyhow::Result;

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

/// Trainer for morphological analyzer.
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

        // Build label mapping from surfaces and add feature sets to provider (Vibrato-style)
        for (i, surface) in config.surfaces.iter().enumerate() {
            // Get feature string for this surface
            let feature_str = config.get_features(surface)
                .unwrap_or_else(|| "名詞,一般,*,*,*,*,*,*,*".to_string());

            // Create feature set for this vocabulary entry
            let feature_extractor = &mut config.feature_extractor;
            let features_vec: Vec<String> = feature_str.split(',').map(|s| s.to_string()).collect();

            let unigram_ids = feature_extractor.extract_unigram_feature_ids(&features_vec, i as u32);
            let left_ids = feature_extractor.extract_left_feature_ids(&features_vec);
            let right_ids = feature_extractor.extract_right_feature_ids(&features_vec);

            let feature_set = rucrf::FeatureSet::new(
                &unigram_ids,
                &right_ids,
                &left_ids,
            );

            // Add feature set to provider and get label ID
            let label_id = provider.add_feature_set(feature_set)?;

            // Map feature string to label ID by first character (Vibrato-style)
            label_id_map
                .entry(feature_str)
                .or_insert_with(HashMap::new);
            if let Some(first_char) = surface.chars().next() {
                label_id_map
                    .get_mut(&config.get_features(surface).unwrap_or_else(|| "名詞,一般,*,*,*,*,*,*,*".to_string()))
                    .unwrap()
                    .insert(first_char, label_id);
            }
        }

        // Initialize unknown word labels for 6 character type categories (Vibrato-style)
        let mut label_id_map_unk = Vec::new();
        let unk_categories = ["DEFAULT", "HIRAGANA", "KATAKANA", "KANJI", "ALPHA", "NUMERIC"];

        for (i, category) in unk_categories.iter().enumerate() {
            // Get unknown word feature string - simplified for now
            let unk_feature = "名詞,一般,*,*,*,*,*,*,*".to_string();

            // Create feature set for unknown word category
            let feature_extractor = &mut config.feature_extractor;
            let features_vec: Vec<String> = unk_feature.split(',').map(|s| s.to_string()).collect();

            let unigram_ids = feature_extractor.extract_unigram_feature_ids(&features_vec, i as u32);
            let left_ids = feature_extractor.extract_left_feature_ids(&features_vec);
            let right_ids = feature_extractor.extract_right_feature_ids(&features_vec);

            let feature_set = rucrf::FeatureSet::new(
                &unigram_ids,
                &right_ids,
                &left_ids,
            );

            // Add to provider
            let unk_label_id = provider.add_feature_set(feature_set)?;
            label_id_map_unk.push(unk_label_id);
        }

        Ok(Self {
            config,
            max_grouping_len: None, // Vibrato default: infinite length
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
        println!("Building feature lattices...");

        // Build lattices from corpus (Vibrato-style)
        let mut lattices = Vec::new();
        for (i, example) in corpus.examples.iter().enumerate() {
            println!("Processing example {}/{}", i + 1, corpus.examples.len());

            // NOTE: Vibrato performs sentence.compile() here for character processing
            // In Lindera, character property processing should be handled differently
            // For now, we proceed with the existing approach

            let lattice = self.build_lattice(example)?;
            lattices.push(lattice);
        }

        println!("Starting CRF training with {} lattices...", lattices.len());

        // Pre-extract necessary information before consuming the provider
        let labels = self.extract_labels();

        // Store training parameters for metadata
        let regularization_cost = self.regularization_cost;
        let max_iter = self.max_iter;

        println!("Training parameters: regularization={regularization_cost}, max_iter={max_iter}, threads={}", self.num_threads);

        // Configure the CRF trainer
        let trainer = rucrf::Trainer::new()
            .regularization(rucrf::Regularization::L1, regularization_cost)?
            .max_iter(max_iter)?
            .n_threads(self.num_threads)?;

        // Take ownership of provider and train the model (Vibrato-style)
        println!("L-BFGS optimization starting...");
        println!("Note: This may take several minutes for large datasets. Progress will be shown by L-BFGS iterations above.");
        println!("Each 'iter:' line indicates training progress. Please wait...");

        // Training starts here - L-BFGS will show its own progress
        let start_time = std::time::Instant::now();

        // DEBUG: Check provider state before training
        println!("DEBUG: Provider state before training - ready for CRF");
        println!("DEBUG: Provider has {} feature sets", self.provider.len());

        // DEBUG: Check lattices state
        println!("DEBUG: Training with {} lattices", lattices.len());
        for i in 0..std::cmp::min(3, lattices.len()) {
            println!("DEBUG: Lattice {} processed", i);
        }

        // DEBUG: Floating-point environment check
        println!("DEBUG: f64::EPSILON = {}", f64::EPSILON);
        println!("DEBUG: regularization_cost = {:.16}", regularization_cost);

        // DEBUG: Memory alignment check
        let provider_ptr = &self.provider as *const _ as usize;
        println!("DEBUG: Provider memory alignment: 0x{:x}", provider_ptr);

        // Train with provider (consumes provider like Vibrato)
        let provider = std::mem::take(&mut self.provider);
        println!("DEBUG: Moved provider to trainer, training starting...");

        // DEBUG: Check for potential NaN/infinity values before training
        println!("DEBUG: Training parameters - reg: {:.16}, iter: {}, threads: {}",
                regularization_cost, max_iter, self.num_threads);

        let model = trainer.train(&lattices, provider);
        let training_duration = start_time.elapsed();

        println!("DEBUG: Training completed, checking raw model...");
        println!("DEBUG: Raw model weights count: {}", model.weights().len());
        println!("DEBUG: Raw model first 5 weights: {:?}", &model.weights()[..std::cmp::min(5, model.weights().len())]);

        // DEBUG: Check for NaN/infinity in weights
        let weights = model.weights();
        let nan_count = weights.iter().filter(|&&w| w.is_nan()).count();
        let inf_count = weights.iter().filter(|&&w| w.is_infinite()).count();
        let zero_count = weights.iter().filter(|&&w| w == 0.0).count();
        println!("DEBUG: Weight analysis - NaN: {}, Inf: {}, Zero: {}, Total: {}",
                 nan_count, inf_count, zero_count, weights.len());

        // DEBUG: Sum of weights for verification
        let weight_sum: f64 = weights.iter().sum();
        println!("DEBUG: Sum of all weights: {:.16}", weight_sum);

        // DEBUG: Check model internal structure
        println!("DEBUG: Model unigram_weight_indices len: {}", model.unigram_weight_indices().len());
        println!("DEBUG: Model bigram_weight_indices len: {}", model.bigram_weight_indices().len());

        println!("Training completed successfully in {:.2}s!", training_duration.as_secs_f64());

        // Remove unused features from feature extractor (Vibrato-style)
        self.remove_unused_features(&model);

        // Extract feature weights from the trained model (Vibrato-style)
        // First try merged model approach
        let mut feature_weights = Vec::new();
        match model.merge() {
            Ok(merged_model) => {
                println!("DEBUG: merged_model.feature_sets.len() = {}", merged_model.feature_sets.len());

                // Check if merged model has valid weights
                let has_valid_weights = merged_model.feature_sets.iter()
                    .take(self.config.surfaces.len())  // Only check relevant weights
                    .any(|fs| fs.weight != 0.0);

                println!("DEBUG: has_valid_weights = {}", has_valid_weights);
                if let Some(first_weight) = merged_model.feature_sets.first() {
                    println!("DEBUG: First merged weight = {}", first_weight.weight);
                }

                if has_valid_weights {
                    // Use merged model weights
                    for (i, _surface) in self.config.surfaces.iter().enumerate() {
                        if i < merged_model.feature_sets.len() {
                            feature_weights.push(merged_model.feature_sets[i].weight);
                        } else {
                            feature_weights.push(0.0);
                        }
                    }
                    println!("DEBUG: Used merged model weights");
                } else {
                    // Fallback: use raw model weights directly
                    let raw_weights = model.weights();
                    println!("DEBUG: Fallback to raw weights, count: {}", raw_weights.len());

                    // Take first N weights corresponding to our surfaces
                    for i in 0..self.config.surfaces.len() {
                        if i < raw_weights.len() {
                            feature_weights.push(raw_weights[i]);
                        } else {
                            feature_weights.push(0.0);
                        }
                    }
                    println!("DEBUG: Used raw model weights");
                }
            }
            Err(e) => {
                println!("DEBUG: merge() failed: {}, using raw weights", e);
                // Fallback: use raw model weights directly
                let raw_weights = model.weights();
                for i in 0..self.config.surfaces.len() {
                    if i < raw_weights.len() {
                        feature_weights.push(raw_weights[i]);
                    } else {
                        feature_weights.push(0.0);
                    }
                }
            }
        }

        println!("Extracted {} feature weights from trained model", feature_weights.len());
        println!("DEBUG: First 10 feature weights: {:?}", &feature_weights[..std::cmp::min(10, feature_weights.len())]);

        // Debug: Print what we're passing to Model::new_with_metadata
        println!("DEBUG: Passing feature_weights to Model: {:?}", &feature_weights[..std::cmp::min(5, feature_weights.len())]);

        // Return the model with actual feature weights
        Ok(Model::new_with_metadata(
            model,
            self.config,
            feature_weights,
            labels,
            regularization_cost,
            max_iter,
        ))
    }

    /// Extracts labels from the configuration
    fn extract_labels(&self) -> Vec<String> {
        let mut labels = self.config.surfaces.clone();
        labels.extend(vec![
            "DEFAULT".to_string(),
            "HIRAGANA".to_string(),
            "KATAKANA".to_string(),
            "KANJI".to_string(),
            "ALPHA".to_string(),
            "NUMERIC".to_string(),
        ]);
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
                        .compatible_unk_index(&example.sentence, pos, pos + token_len, token.feature())
                        .map_or_else(
                            || {
                                self.provider
                                    .add_feature_set(rucrf::FeatureSet::new(&[], &[], &[]))
                            },
                            |unk_index| {
                                Ok(self.label_id_map_unk[unk_index.word_id as usize])
                            },
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
                if let Some(first_edge) = lattice.nodes()[pos].edges().first() {
                    if edge == *first_edge {
                        continue;
                    }
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
                    if let Some(first_edge) = lattice.nodes()[pos].edges().first() {
                        if edge == *first_edge {
                            return;
                        }
                    }
                    lattice.add_edge(pos, edge).unwrap();
                },
            );
        }

        Ok(lattice)
    }

    /// Remove unused features from the feature extractor to optimize the model (Vibrato implementation)
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

        // Remove unused unigram features (exact Vibrato logic)
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
            if let Some(x) = model.bigram_weight_indices().get(id.get() as usize) {
                if x.is_empty() {
                    self.config.feature_extractor.left_feature_ids.remove(k);
                }
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
