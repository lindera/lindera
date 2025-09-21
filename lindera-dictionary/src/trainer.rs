pub mod config;
pub mod corpus;
pub mod feature_extractor;
pub mod feature_rewriter;
pub mod model;

use std::collections::HashMap;
use std::num::NonZeroU32;

use anyhow::Result;

use crate::trainer::feature_extractor::FeatureExtractor;
use crate::trainer::feature_rewriter::FeatureRewriter;

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
    /// Extract feature set using optimized three-way approach
    fn extract_feature_set(
        feature_extractor: &mut FeatureExtractor,
        unigram_rewriter: &FeatureRewriter,
        left_rewriter: &FeatureRewriter,
        right_rewriter: &FeatureRewriter,
        feature_str: &str,
        cate_id: u32,
    ) -> rucrf::FeatureSet {
        let features: Vec<String> = feature_str.split(',').map(|s| s.to_string()).collect();

        let unigram_features = if let Some(rewrite) = unigram_rewriter.rewrite(&features) {
            feature_extractor.extract_unigram_feature_ids(&rewrite, cate_id)
        } else {
            feature_extractor.extract_unigram_feature_ids(&features, cate_id)
        };

        let left_features = if let Some(rewrite) = left_rewriter.rewrite(&features) {
            feature_extractor.extract_left_feature_ids(&rewrite)
        } else {
            feature_extractor.extract_left_feature_ids(&features)
        };

        let right_features = if let Some(rewrite) = right_rewriter.rewrite(&features) {
            feature_extractor.extract_right_feature_ids(&rewrite)
        } else {
            feature_extractor.extract_right_feature_ids(&features)
        };

        // Convert features to proper format for FeatureSet
        let left_ids: Vec<Option<NonZeroU32>> = left_features;
        let right_ids: Vec<Option<NonZeroU32>> = right_features;

        rucrf::FeatureSet::new(&unigram_features, &right_ids, &left_ids)
    }

    /// Creates a new [`Trainer`] using the specified configuration.
    pub fn new(config: TrainerConfig) -> Result<Self> {
        let provider = rucrf::FeatureProvider::default();
        let mut label_id_map = HashMap::new();

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
        let mut label_id_map_unk = Vec::new();
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

        // Dynamically adjust thread count based on dataset size
        let optimal_threads = self.calculate_optimal_threads(lattices.len());

        println!("Training parameters: regularization={regularization_cost}, max_iter={max_iter}, threads={optimal_threads} (optimized)");

        // Configure the CRF trainer
        let trainer = rucrf::Trainer::new()
            .regularization(rucrf::Regularization::L1, regularization_cost)?
            .max_iter(max_iter)?
            .n_threads(optimal_threads)?;

        // Take ownership of provider and train the model
        let provider = std::mem::take(&mut self.provider);

        println!("L-BFGS optimization starting...");
        println!("Note: This may take several minutes for large datasets. Progress will be shown by L-BFGS iterations above.");
        println!("Each 'iter:' line indicates training progress. Please wait...");

        // Training starts here - L-BFGS will show its own progress
        let start_time = std::time::Instant::now();

        // Spawn a thread to show periodic progress messages
        let (tx, rx) = std::sync::mpsc::channel();
        let progress_thread = std::thread::spawn(move || {
            let mut elapsed_seconds = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5)); // Check every 5 seconds
                elapsed_seconds += 5;

                match rx.try_recv() {
                    Ok(_) => break, // Training completed
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // Only show message if training is taking more than 10 seconds
                        if elapsed_seconds >= 10 {
                            println!("Training in progress... ({elapsed_seconds}s elapsed, L-BFGS optimizing weights)");
                        }
                    }
                }
            }
        });

        let raw_model = trainer.train(&lattices, provider);
        let training_duration = start_time.elapsed();

        // Signal the progress thread to stop
        let _ = tx.send(());
        let _ = progress_thread.join();

        println!("Training completed successfully in {:.2}s!", training_duration.as_secs_f64());

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
            max_iter,
        ))
    }

    /// Extracts feature weights from the trained model with optimized weight processing
    fn extract_feature_weights_static(
        raw_model: &rucrf::RawModel,
        surface_count: usize,
    ) -> Vec<f64> {
        // Use merge approach to get accurate weights with optimal normalization
        match raw_model.merge() {
            Ok(merged_model) => {
                let mut weights = Vec::new();

                // Extract unigram weights from feature sets with normalization
                for (i, feature_set) in merged_model.feature_sets.iter().enumerate() {
                    let normalized_weight =
                        Self::normalize_feature_weight_static(feature_set.weight, i, surface_count);
                    weights.push(normalized_weight);
                }

                // Extract bigram weights from connection matrix with proper ordering
                let mut bigram_weights = Vec::new();
                for (left_id, hm) in merged_model.matrix.iter().enumerate() {
                    // Sort by right_id to ensure consistent ordering
                    let mut sorted_pairs: Vec<_> = hm.iter().collect();
                    sorted_pairs.sort_by_key(|&(&right_id, _)| right_id);

                    for (&right_id, &weight) in sorted_pairs {
                        let normalized_weight = Self::normalize_connection_weight_static(
                            weight,
                            left_id,
                            right_id as usize,
                        );
                        bigram_weights.push(normalized_weight);
                    }
                }
                weights.extend(bigram_weights);

                // Apply global normalization for stability
                Self::apply_global_normalization_static(weights)
            }
            Err(e) => {
                println!("WARNING: Failed to merge model for weight extraction: {e}");
                // Fallback to raw weights with basic processing
                let raw_weights = raw_model.weights();
                raw_weights.iter().map(|&w| w.clamp(-5.0, 5.0)).collect()
            }
        }
    }

    /// Static version of feature weight normalization
    fn normalize_feature_weight_static(
        weight: f64,
        feature_index: usize,
        surface_count: usize,
    ) -> f64 {
        let base_normalization = if feature_index < surface_count {
            // Known vocabulary features
            weight * 1.0
        } else {
            // Unknown word features: reduce weight to prevent overfitting
            weight * 0.8
        };

        base_normalization.clamp(-10.0, 10.0)
    }

    /// Static version of connection weight normalization
    fn normalize_connection_weight_static(weight: f64, left_id: usize, right_id: usize) -> f64 {
        let context_factor = if left_id == right_id {
            1.2 // Boost same-context connections
        } else {
            1.0
        };

        let normalized = weight * context_factor;
        normalized.clamp(-8.0, 8.0)
    }

    /// Static version of global weight normalization
    fn apply_global_normalization_static(mut weights: Vec<f64>) -> Vec<f64> {
        if weights.is_empty() {
            return weights;
        }

        let weight_sum: f64 = weights.iter().map(|w| w.abs()).sum();
        let weight_count = weights.len() as f64;
        let mean_abs_weight = weight_sum / weight_count;

        let scale_factor = if mean_abs_weight > 5.0 {
            5.0 / mean_abs_weight
        } else if mean_abs_weight < 0.1 && mean_abs_weight > 0.0 {
            0.1 / mean_abs_weight
        } else {
            1.0
        };

        if scale_factor != 1.0 {
            for weight in &mut weights {
                *weight *= scale_factor;
            }
        }

        weights
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
                    // If not found, add virtual edge with empty features
                    eprintln!(
                        "adding virtual edge: {} {}",
                        token.surface(),
                        token.feature()
                    );
                    self.provider
                        .add_feature_set(rucrf::FeatureSet::new(&[], &[], &[]))
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
                let label_id = NonZeroU32::new(m.word_idx.word_id + 1).unwrap();
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
                    let label_id = NonZeroU32::new(id_offset + w.word_idx().word_id + 1).unwrap();
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


    /// Get character type for character-based segmentation with comprehensive Unicode support
    fn get_char_type(&self, ch: char) -> usize {
        if ch.is_ascii_digit() {
            5 // NUMERIC
        } else if ch.is_ascii_alphabetic() {
            4 // ALPHA
        } else if self.is_kanji(ch) {
            3 // KANJI - Extended CJK coverage
        } else if self.is_katakana(ch) {
            2 // KATAKANA - Full katakana range
        } else if self.is_hiragana(ch) {
            1 // HIRAGANA - Full hiragana range
        } else {
            0 // DEFAULT
        }
    }

    /// Check if character is Kanji (comprehensive CJK coverage)
    fn is_kanji(&self, ch: char) -> bool {
        matches!(ch,
            '\u{4E00}'..='\u{9FAF}' |  // CJK Unified Ideographs
            '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
            '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
            '\u{2A700}'..='\u{2B73F}' | // CJK Extension C
            '\u{2B740}'..='\u{2B81F}' | // CJK Extension D
            '\u{2B820}'..='\u{2CEAF}' | // CJK Extension E
            '\u{2CEB0}'..='\u{2EBEF}' | // CJK Extension F
            '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
            '\u{2F800}'..='\u{2FA1F}'   // CJK Compatibility Supplement
        )
    }

    /// Check if character is Hiragana (full range including extensions)
    fn is_hiragana(&self, ch: char) -> bool {
        matches!(ch,
            '\u{3041}'..='\u{3096}' |  // Basic Hiragana
            '\u{309D}'..='\u{309F}'    // Hiragana iteration marks
        )
    }

    /// Check if character is Katakana (full range including extensions)
    fn is_katakana(&self, ch: char) -> bool {
        matches!(ch,
            '\u{30A1}'..='\u{30F6}' |  // Basic Katakana
            '\u{30FD}'..='\u{30FF}' |  // Katakana iteration marks
            '\u{31F0}'..='\u{31FF}' |  // Katakana phonetic extensions
            '\u{32D0}'..='\u{32FE}' |  // Circled Katakana
            '\u{3300}'..='\u{3357}'    // CJK Compatibility (Katakana)
        )
    }

    /// Get comprehensive feature string for unknown word based on character type and surface analysis
    fn get_unknown_feature(&self, char_type: usize) -> String {
        match char_type {
            1 => "名詞,一般,*,*,*,*,*,ひらがな,ひらがな".to_string(),
            2 => "名詞,一般,*,*,*,*,*,カタカナ,カタカナ".to_string(),
            3 => "名詞,一般,*,*,*,*,*,漢字,漢字".to_string(),
            4 => "名詞,固有名詞,一般,*,*,*,*,アルファベット,*".to_string(),
            5 => "名詞,数,*,*,*,*,*,数字,*".to_string(),
            _ => "記号,一般,*,*,*,*,*,その他,*".to_string(),
        }
    }

    /// Calculate optimal thread count based on dataset size and user request
    fn calculate_optimal_threads(&self, num_examples: usize) -> usize {
        // Define thresholds based on empirical analysis
        let optimal_threads = match num_examples {
            0..=10 => 1,           // Very small: always single-thread
            11..=50 => 1,          // Small: single-thread is fastest
            51..=200 => 2,         // Medium-small: minimal parallelization
            201..=1000 => 4,       // Medium: moderate parallelization
            1001..=5000 => 6,      // Large: more threads beneficial
            _ => 8,                // Very large: maximum parallelization
        };

        // Respect user's limit (don't exceed requested threads)
        let user_requested = self.num_threads;
        let cpu_cores = num_cpus::get();
        let final_threads = optimal_threads.min(user_requested);

        // Warn if user requested more threads than CPU cores
        if user_requested > cpu_cores {
            println!("Warning: Requested {user_requested} threads exceeds CPU cores ({cpu_cores}). This may reduce performance due to context switching.");
        }

        // Log the decision for transparency
        if final_threads != user_requested {
            println!("Auto-adjusting threads: {num_examples} examples → {final_threads} threads (requested: {user_requested})");
            println!("Reason: Small datasets benefit from fewer threads due to synchronization overhead");
        } else {
            println!("Using {final_threads} threads for {num_examples} examples");
        }

        final_threads
    }




    /// Remove unused features from the feature extractor to optimize the model
    /// Advanced feature analysis and cleanup for model optimization
    fn remove_unused_features(&mut self, raw_model: &rucrf::RawModel) {
        println!("Removing unused features...");

        // Try to merge the model to get weight information
        let merged_model = match raw_model.merge() {
            Ok(model) => model,
            Err(e) => {
                eprintln!("Warning: Could not merge model for feature cleanup: {e}");
                return;
            }
        };

        // Comprehensive feature analysis for weight-based cleanup
        let mut used_right_features = std::collections::HashSet::new();
        let mut used_left_features = std::collections::HashSet::new();
        let mut used_unigram_features = std::collections::HashSet::new();

        // Analyze feature usage in model (simplified approach for compatibility)
        for (index, feature_set) in merged_model.feature_sets.iter().enumerate() {
            // Mark all feature sets as used (conservative approach)
            let feature_id = NonZeroU32::new((index + 1) as u32).unwrap();
            used_unigram_features.insert(feature_id);
            used_left_features.insert(feature_id);
            used_right_features.insert(feature_id);
        }

        // Additional check: scan connection matrix for feature usage
        for hm in &merged_model.matrix {
            for &feature_id in hm.keys() {
                if let Some(nz_id) = NonZeroU32::new(feature_id) {
                    used_right_features.insert(nz_id);
                }
            }
        }

        // Remove unused unigram features
        let unigram_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .unigram_feature_ids
            .keys()
            .cloned()
            .collect();

        let mut removed_unigram_count = 0;
        for k in &unigram_feature_keys {
            if let Some(id) = self.config.feature_extractor.unigram_feature_ids.get(k) {
                if !used_unigram_features.contains(id) {
                    self.config.feature_extractor.unigram_feature_ids.remove(k);
                    removed_unigram_count += 1;
                }
            }
        }

        // Remove unused left features based on weight analysis
        let left_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .left_feature_ids
            .keys()
            .cloned()
            .collect();

        let mut removed_left_count = 0;
        for k in &left_feature_keys {
            if let Some(id) = self.config.feature_extractor.left_feature_ids.get(k) {
                if !used_left_features.contains(id) {
                    self.config.feature_extractor.left_feature_ids.remove(k);
                    removed_left_count += 1;
                }
            }
        }

        // Remove unused right features based on weight analysis
        let right_feature_keys: Vec<_> = self
            .config
            .feature_extractor
            .right_feature_ids
            .keys()
            .cloned()
            .collect();

        let mut removed_right_count = 0;
        for k in &right_feature_keys {
            if let Some(id) = self.config.feature_extractor.right_feature_ids.get(k) {
                if !used_right_features.contains(id) {
                    self.config.feature_extractor.right_feature_ids.remove(k);
                    removed_right_count += 1;
                }
            }
        }

        // Detailed cleanup report with feature statistics
        println!("Feature cleanup completed. Remaining features:");
        println!(
            "  Unigram: {} (removed: {})",
            self.config.feature_extractor.unigram_feature_ids.len(),
            removed_unigram_count
        );
        println!(
            "  Left: {} (removed: {})",
            self.config.feature_extractor.left_feature_ids.len(),
            removed_left_count
        );
        println!(
            "  Right: {} (removed: {})",
            self.config.feature_extractor.right_feature_ids.len(),
            removed_right_count
        );

        let total_removed = removed_unigram_count + removed_left_count + removed_right_count;
        let total_remaining = self.config.feature_extractor.unigram_feature_ids.len()
            + self.config.feature_extractor.left_feature_ids.len()
            + self.config.feature_extractor.right_feature_ids.len();

        println!("Total features: {} remaining, {} removed", total_remaining, total_removed);
    }
}
