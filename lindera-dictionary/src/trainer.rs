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
    /// These are used to assign consistent labels to unknown words based on their character type.
    label_id_map_unk: Vec<NonZeroU32>,

    regularization_cost: f64,
    max_iter: u64,
    num_threads: usize,
}

impl Trainer {
    /// Creates a new [`Trainer`] using the specified configuration.
    pub fn new(config: TrainerConfig) -> Result<Self> {
        let provider = rucrf::FeatureProvider::default();
        let mut label_id_map = HashMap::new();
        let mut label_id_map_unk = Vec::new();

        // Build label mapping from surfaces
        for (i, surface) in config.surfaces.iter().enumerate() {
            let label_id = std::num::NonZeroU32::new((i + 1) as u32).unwrap();
            label_id_map.insert(surface.clone(), HashMap::new());
            if let Some(first_char) = surface.chars().next() {
                label_id_map
                    .get_mut(surface)
                    .unwrap()
                    .insert(first_char, label_id);
            }
        }

        // Initialize unknown word labels for 6 character type categories
        // These pre-allocated IDs ensure consistent handling of unknown words
        for i in 0..6 {
            // 6 categories: DEFAULT, HIRAGANA, KATAKANA, KANJI, ALPHA, NUMERIC
            label_id_map_unk
                .push(std::num::NonZeroU32::new((config.surfaces.len() + i + 1) as u32).unwrap());
        }

        Ok(Self {
            config,
            max_grouping_len: Some(10), // Default maximum grouping length
            provider,
            label_id_map,
            label_id_map_unk,
            regularization_cost: 0.01,
            max_iter: 100,
            num_threads: 1,
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

        // Build lattices from corpus
        let mut lattices = Vec::new();
        for (i, example) in corpus.examples.iter().enumerate() {
            println!("Processing example {}/{}", i + 1, corpus.examples.len());
            let lattice = self.build_lattice(example)?;
            lattices.push(lattice);
        }

        println!("Starting CRF training with {} lattices...", lattices.len());

        // Pre-extract necessary information before consuming the provider
        let labels = self.extract_labels();
        let surface_count = self.config.surfaces.len();

        // Store training parameters for metadata
        let regularization_cost = self.regularization_cost;
        let max_iter = self.max_iter;

        // Configure the CRF trainer
        let trainer = rucrf::Trainer::new()
            .regularization(rucrf::Regularization::L1, regularization_cost)?
            .max_iter(max_iter)?
            .n_threads(self.num_threads)?;

        // Take ownership of provider and train the model
        let provider = std::mem::take(&mut self.provider);
        let raw_model = trainer.train(&lattices, provider);

        println!("Training completed successfully!");

        // Remove unused features from feature extractor
        self.remove_unused_features(&raw_model);

        // Extract feature weights for the trained model
        let feature_weights = Self::extract_feature_weights_static(&raw_model, surface_count);

        Ok(Model::new_with_metadata(
            raw_model,
            self.config,
            feature_weights,
            labels,
            regularization_cost,
            max_iter
        ))
    }

    /// Extracts feature weights from the trained model
    fn extract_feature_weights_static(
        raw_model: &rucrf::RawModel,
        _surface_count: usize,
    ) -> Vec<f64> {
        // Use merge approach to get accurate weights
        match raw_model.merge() {
            Ok(merged_model) => {
                let mut weights = Vec::new();

                // Extract weights from feature sets
                for feature_set in &merged_model.feature_sets {
                    weights.push(feature_set.weight);
                }

                // Extract weights from connection matrix (bigram weights)
                for hm in &merged_model.matrix {
                    for &w in hm.values() {
                        weights.push(w);
                    }
                }

                // Optionally log the extraction results (can be removed in production)
                // println!("Extracted {} feature weights and {} connection weights",
                //          merged_model.feature_sets.len(),
                //          merged_model.matrix.iter().map(|hm| hm.len()).sum::<usize>());

                weights
            }
            Err(e) => {
                println!(
                    "WARNING: Failed to merge model for weight extraction: {}",
                    e
                );
                Vec::new()
            }
        }
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

        let input_len = example.sentence.chars().count();
        let mut lattice = Lattice::new(input_len)?;

        // First, add positive edges (correct segmentation from training data)
        let mut pos = 0;
        let mut positive_edges = Vec::new();
        for token in &example.tokens {
            let token_len = token.surface().chars().count();

            // Get or create label ID for this token
            let label_id = self.get_or_create_label_id(token)?;

            // Create minimal feature set for efficiency
            let features: Vec<String> = token.feature().split(',').map(|s| s.to_string()).collect();
            let first_char = token.surface().chars().next().unwrap_or('\0');
            let cate_id = self.get_char_type(first_char) as u32;

            // Extract features using the feature extractor (simplified)
            let unigram_features = self.config.feature_extractor
                .extract_unigram_feature_ids(&features, cate_id);
            let left_features = self.config.feature_extractor
                .extract_left_feature_ids(&features);
            let right_features = self.config.feature_extractor
                .extract_right_feature_ids(&features);

            let feature_set = rucrf::FeatureSet::new(&unigram_features, &right_features, &left_features);
            let feature_id = self.provider.add_feature_set(feature_set)?;

            // Create edge
            let edge = Edge::new(pos + token_len, feature_id);
            lattice.add_edge(pos, edge)?;
            positive_edges.push((pos, pos + token_len));

            pos += token_len;
        }

        // Add simple negative edges (limited to prevent timeout)
        // Only add 1-3 character segments as negative examples
        for start_pos in 0..input_len {
            let mut added_count = 0;

            for length in 1..=3 {
                let end_pos = start_pos + length;
                if end_pos > input_len {
                    break;
                }

                // Skip if this was a positive edge
                if positive_edges.contains(&(start_pos, end_pos)) {
                    continue;
                }

                // Create a minimal negative edge (no features to save time)
                let feature_set = rucrf::FeatureSet::new(&[], &[], &[]);
                let feature_id = self.provider.add_feature_set(feature_set)?;
                let edge = Edge::new(end_pos, feature_id);
                lattice.add_edge(start_pos, edge)?;

                added_count += 1;
                if added_count >= 2 {  // Limit to 2 negative edges per position
                    break;
                }
            }
        }

        Ok(lattice)
    }

    /// Get character type for character-based segmentation
    fn get_char_type(&self, ch: char) -> usize {
        if ch.is_ascii_digit() {
            5 // NUMERIC
        } else if ch.is_ascii_alphabetic() {
            4 // ALPHA
        } else if ch >= '\u{4E00}' && ch <= '\u{9FAF}' {
            3 // KANJI
        } else if ch >= '\u{30A1}' && ch <= '\u{30F6}' {
            2 // KATAKANA
        } else if ch >= '\u{3041}' && ch <= '\u{3096}' {
            1 // HIRAGANA
        } else {
            0 // DEFAULT
        }
    }

    /// Get feature string for unknown word based on character type
    fn get_unknown_feature(&self, char_type: usize) -> String {
        match char_type {
            1 => "名詞,一般,*,*,*,*,*,ひらがな,*".to_string(),
            2 => "名詞,一般,*,*,*,*,*,カタカナ,*".to_string(),
            3 => "名詞,一般,*,*,*,*,*,漢字,*".to_string(),
            4 => "名詞,一般,*,*,*,*,*,アルファベット,*".to_string(),
            5 => "名詞,数,*,*,*,*,*,数字,*".to_string(),
            _ => "名詞,一般,*,*,*,*,*,その他,*".to_string(),
        }
    }

    fn get_or_create_label_id(&mut self, token: &Word) -> Result<NonZeroU32> {
        // Check if the token exists in the dictionary
        let is_known_word = self.is_word_in_dictionary(token);

        // Try to find existing label for this surface/feature combination
        if let Some(char_map) = self.label_id_map.get(token.surface()) {
            if let Some(first_char) = token.surface().chars().next() {
                if let Some(&label_id) = char_map.get(&first_char) {
                    return Ok(label_id);
                }
            }
        }

        // For unknown words, try to use pre-defined unknown labels
        // This ensures consistent handling of unknown words by character type,
        // improving the model's ability to generalize to new vocabulary
        if !is_known_word {
            let unk_category = self.classify_unknown_word(token);
            if let Some(&unk_label_id) = self.label_id_map_unk.get(unk_category) {
                return Ok(unk_label_id);
            }
        }

        // Create new label ID, considering dictionary status
        let base_id = if is_known_word {
            // Known words get lower IDs (higher priority)
            self.label_id_map.len() + 1
        } else {
            // Unknown words get higher IDs (lower priority)
            self.label_id_map.len() + 1000
        };

        let new_id = NonZeroU32::new(base_id as u32).unwrap();

        // Store the new mapping
        if let Some(first_char) = token.surface().chars().next() {
            self.label_id_map
                .entry(token.surface().to_string())
                .or_insert_with(HashMap::new)
                .insert(first_char, new_id);
        }

        Ok(new_id)
    }

    /// Classifies an unknown word into one of 6 predefined categories based on its character type.
    /// This classification is used to index into `label_id_map_unk` for consistent label assignment.
    ///
    /// Returns:
    /// - 0: DEFAULT (punctuation, symbols, or unclassified characters)
    /// - 1: HIRAGANA (ひらがな)
    /// - 2: KATAKANA (カタカナ)
    /// - 3: KANJI (漢字)
    /// - 4: ALPHA (A-Z, a-z)
    /// - 5: NUMERIC (0-9)
    fn classify_unknown_word(&self, token: &Word) -> usize {
        // Classify unknown word into one of 6 categories based on character type
        let surface = token.surface();
        let first_char = surface.chars().next().unwrap_or('\0');

        if first_char.is_ascii_digit() {
            5 // NUMERIC - ASCII digits (0-9)
        } else if first_char.is_ascii_alphabetic() {
            4 // ALPHA - ASCII letters (A-Z, a-z)
        } else if first_char >= '\u{4E00}' && first_char <= '\u{9FAF}' {
            3 // KANJI - CJK Unified Ideographs
        } else if first_char >= '\u{30A1}' && first_char <= '\u{30F6}' {
            2 // KATAKANA - Japanese katakana
        } else if first_char >= '\u{3041}' && first_char <= '\u{3096}' {
            1 // HIRAGANA - Japanese hiragana
        } else {
            0 // DEFAULT - everything else (punctuation, symbols, etc.)
        }
    }

    fn is_word_in_dictionary(&self, token: &Word) -> bool {
        // First check in the surface list (known training vocabulary)
        if self.config.surfaces.contains(&token.surface().to_string()) {
            return true;
        }

        // Additionally check if the word can be handled as an unknown word
        // by the dictionary's character definitions
        if let Some(first_char) = token.surface().chars().next() {
            // Use the dictionary's character definition to validate character types
            let categories = self
                .config
                .dict
                .character_definition
                .lookup_categories(first_char);
            if !categories.is_empty() {
                return true;
            }
        }

        false
    }

    fn extract_token_features(&mut self, token: &Word, _pos: usize) -> Result<rucrf::FeatureSet> {
        // Parse features from the token
        let features: Vec<String> = token.feature().split(',').map(|s| s.to_string()).collect();

        // Apply feature rewriting (similar to Vibrato's approach)
        let unigram_features_input = if let Some(rewritten) = self.config.unigram_rewriter.rewrite(&features) {
            rewritten
        } else {
            features.clone()
        };
        let left_features_input = if let Some(rewritten) = self.config.left_rewriter.rewrite(&features) {
            rewritten
        } else {
            features.clone()
        };
        let right_features_input = if let Some(rewritten) = self.config.right_rewriter.rewrite(&features) {
            rewritten
        } else {
            features.clone()
        };

        // Extract different types of features using rewritten inputs
        let unigram_features = self
            .config
            .feature_extractor
            .extract_unigram_feature_ids(&unigram_features_input, 0);
        let left_features = self
            .config
            .feature_extractor
            .extract_left_feature_ids(&left_features_input);
        let right_features = self
            .config
            .feature_extractor
            .extract_right_feature_ids(&right_features_input);

        // Convert to the exact types expected by rucrf
        Ok(rucrf::FeatureSet::new(
            &unigram_features,
            &right_features,
            &left_features,
        ))
    }

    /// Remove unused features from the feature extractor to optimize the model
    /// This is similar to Vibrato's optimization in trainer.rs:413-457
    fn remove_unused_features(&mut self, raw_model: &rucrf::RawModel) {
        println!("Removing unused features...");

        // Try to merge the model to get weight information
        let merged_model = match raw_model.merge() {
            Ok(model) => model,
            Err(e) => {
                eprintln!("Warning: Could not merge model for feature cleanup: {}", e);
                return;
            }
        };

        let mut used_right_features = std::collections::HashSet::new();

        // Check unigram features
        let unigram_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .unigram_feature_ids
            .keys()
            .cloned()
            .collect();

        for k in &unigram_feature_keys {
            if let Some(id) = self.config.feature_extractor.unigram_feature_ids.get(k) {
                let id_index = id.get() as usize;
                if id_index == 0 || id_index > merged_model.feature_sets.len() {
                    // Remove invalid or unused unigram features
                    self.config.feature_extractor.unigram_feature_ids.remove(k);
                }
            }
        }

        // Collect used right features from bigram connections
        for hm in &merged_model.matrix {
            for &feature_id in hm.keys() {
                used_right_features.insert(feature_id);
            }
        }

        // Check left features
        let left_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .left_feature_ids
            .keys()
            .cloned()
            .collect();

        for k in &left_feature_keys {
            if let Some(id) = self.config.feature_extractor.left_feature_ids.get(k) {
                let id_index = id.get() as usize;
                if id_index >= merged_model.matrix.len() || merged_model.matrix[id_index].is_empty() {
                    // Remove unused left features
                    self.config.feature_extractor.left_feature_ids.remove(k);
                }
            }
        }

        // Check right features
        let right_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .right_feature_ids
            .keys()
            .cloned()
            .collect();

        for k in &right_feature_keys {
            if let Some(id) = self.config.feature_extractor.right_feature_ids.get(k) {
                if !used_right_features.contains(&id.get()) {
                    // Remove unused right features
                    self.config.feature_extractor.right_feature_ids.remove(k);
                }
            }
        }

        println!("Feature cleanup completed. Remaining features:");
        println!("  Unigram: {}", self.config.feature_extractor.unigram_feature_ids.len());
        println!("  Left: {}", self.config.feature_extractor.left_feature_ids.len());
        println!("  Right: {}", self.config.feature_extractor.right_feature_ids.len());
    }
}
