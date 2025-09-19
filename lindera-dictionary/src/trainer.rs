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
            max_iter,
        ))
    }

    /// Extracts feature weights from the trained model with Vibrato-compatible processing
    fn extract_feature_weights_static(
        raw_model: &rucrf::RawModel,
        surface_count: usize,
    ) -> Vec<f64> {
        // Use merge approach to get accurate weights (following Vibrato pattern)
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
                println!(
                    "WARNING: Failed to merge model for weight extraction: {}",
                    e
                );
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
        let mut lattice = Lattice::new(input_len)?;

        // First, add positive edges (correct segmentation from training data)
        let mut pos = 0;
        let mut positive_edges = Vec::new();
        for token in &example.tokens {
            let token_len = token.surface().chars().count();

            // Get or create label ID for this token
            let label_id = self.get_or_create_label_id(token)?;

            // Use the dedicated extract_token_features method for comprehensive feature extraction
            let feature_set = self.extract_token_features(token, pos)?;
            let _feature_id = self.provider.add_feature_set(feature_set)?;

            // Create edge using label_id (following Vibrato's approach)
            let edge = Edge::new(pos + token_len, label_id);
            lattice.add_edge(pos, edge)?;
            positive_edges.push((pos, pos + token_len));

            pos += token_len;
        }

        // Add negative edges with proper unknown word handling (following Vibrato's approach)
        // Generate unknown word features based on character types
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

                // Extract substring and determine character type for unknown word features
                let substring: String = input_chars[start_pos..end_pos].iter().collect();
                let first_char = substring.chars().next().unwrap_or('\0');
                let char_type = self.get_char_type(first_char);

                // Generate enhanced unknown word features based on character type and surface analysis
                let enhanced_features = self.generate_unknown_word_features(&substring, char_type);
                let features = if enhanced_features.len() > 1 {
                    // Use first feature as base, combine others as additional features
                    let mut combined_features = enhanced_features[0]
                        .split(',')
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>();
                    // Add enhanced features as additional context
                    combined_features.extend(enhanced_features[1..].iter().cloned());
                    combined_features
                } else {
                    // Fallback to basic feature parsing
                    let unknown_feature = self.get_unknown_feature(char_type);
                    unknown_feature.split(',').map(|s| s.to_string()).collect()
                };

                // Extract features for unknown word
                let unigram_features = self
                    .config
                    .feature_extractor
                    .extract_unigram_feature_ids(&features, char_type as u32);
                let left_features = self
                    .config
                    .feature_extractor
                    .extract_left_feature_ids(&features);
                let right_features = self
                    .config
                    .feature_extractor
                    .extract_right_feature_ids(&features);

                let feature_set =
                    rucrf::FeatureSet::new(&unigram_features, &right_features, &left_features);
                let feature_id = self.provider.add_feature_set(feature_set)?;
                let edge = Edge::new(end_pos, feature_id);
                lattice.add_edge(start_pos, edge)?;

                added_count += 1;
                if added_count >= 2 {
                    // Limit to 2 negative edges per position
                    break;
                }
            }
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

    /// Generate enhanced features for unknown words based on surface form analysis
    fn generate_unknown_word_features(&self, surface: &str, char_type: usize) -> Vec<String> {
        let mut features = Vec::new();

        // Basic character type feature
        let base_feature = self.get_unknown_feature(char_type);
        features.push(base_feature);

        // Length-based features
        let len = surface.chars().count();
        match len {
            1 => features.push("UNK_LEN=1".to_string()),
            2 => features.push("UNK_LEN=2".to_string()),
            3..=5 => features.push("UNK_LEN=SHORT".to_string()),
            6..=10 => features.push("UNK_LEN=MEDIUM".to_string()),
            _ => features.push("UNK_LEN=LONG".to_string()),
        }

        // Character pattern features
        let chars: Vec<char> = surface.chars().collect();
        if chars.len() > 1 {
            // Mixed character type detection
            let has_hiragana = chars.iter().any(|&c| self.is_hiragana(c));
            let has_katakana = chars.iter().any(|&c| self.is_katakana(c));
            let has_kanji = chars.iter().any(|&c| self.is_kanji(c));
            let has_alpha = chars.iter().any(|&c| c.is_ascii_alphabetic());
            let has_digit = chars.iter().any(|&c| c.is_ascii_digit());

            let type_count = [has_hiragana, has_katakana, has_kanji, has_alpha, has_digit]
                .iter()
                .filter(|&&x| x)
                .count();

            if type_count > 1 {
                features.push("UNK_MIXED=TRUE".to_string());
            }

            // Specific patterns
            if has_kanji && has_hiragana {
                features.push("UNK_KANJI_HIRA=TRUE".to_string());
            }
            if has_katakana && has_alpha {
                features.push("UNK_KATA_ALPHA=TRUE".to_string());
            }
        }

        // Positional character features (first and last char types)
        if let Some(first_char) = chars.first() {
            features.push(format!("UNK_FIRST={}", self.get_char_type(*first_char)));
        }
        if let Some(last_char) = chars.last() {
            features.push(format!("UNK_LAST={}", self.get_char_type(*last_char)));
        }

        features
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

    /// Classifies an unknown word into one of 6 predefined categories based on comprehensive character analysis.
    /// This classification uses the improved character type detection methods for better accuracy.
    ///
    /// Returns:
    /// - 0: DEFAULT (punctuation, symbols, or unclassified characters)
    /// - 1: HIRAGANA (ひらがな)
    /// - 2: KATAKANA (カタカナ)
    /// - 3: KANJI (漢字)
    /// - 4: ALPHA (A-Z, a-z)
    /// - 5: NUMERIC (0-9)
    fn classify_unknown_word(&self, token: &Word) -> usize {
        let surface = token.surface();
        let chars: Vec<char> = surface.chars().collect();

        if chars.is_empty() {
            return 0; // DEFAULT for empty strings
        }

        // For single character words, use direct classification
        if chars.len() == 1 {
            return self.get_char_type(chars[0]);
        }

        // For multi-character words, use majority voting with special rules
        let mut type_counts = [0; 6]; // Count for each character type

        for &ch in &chars {
            let char_type = self.get_char_type(ch);
            type_counts[char_type] += 1;
        }

        // Find the most frequent character type
        let (most_frequent_type, max_count) = type_counts
            .iter()
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(idx, count)| (idx, *count))
            .unwrap_or((0, 0));

        // Special rules for mixed character types
        if type_counts[3] > 0 && type_counts[1] > 0 {
            // Kanji + Hiragana = Kanji (compound words)
            return 3;
        }

        if type_counts[2] > 0 && type_counts[4] > 0 {
            // Katakana + Alpha = Katakana (foreign words)
            return 2;
        }

        if type_counts[5] > 0 && max_count == type_counts[5] {
            // If numbers are present and dominant, classify as numeric
            return 5;
        }

        // Return the most frequent type, or DEFAULT if tie
        if max_count > 0 {
            most_frequent_type
        } else {
            0 // DEFAULT fallback
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
        let unigram_features_input =
            if let Some(rewritten) = self.config.unigram_rewriter.rewrite(&features) {
                rewritten
            } else {
                features.clone()
            };
        let left_features_input =
            if let Some(rewritten) = self.config.left_rewriter.rewrite(&features) {
                rewritten
            } else {
                features.clone()
            };
        let right_features_input =
            if let Some(rewritten) = self.config.right_rewriter.rewrite(&features) {
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
                if id_index >= merged_model.matrix.len() || merged_model.matrix[id_index].is_empty()
                {
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
        println!(
            "  Unigram: {}",
            self.config.feature_extractor.unigram_feature_ids.len()
        );
        println!(
            "  Left: {}",
            self.config.feature_extractor.left_feature_ids.len()
        );
        println!(
            "  Right: {}",
            self.config.feature_extractor.right_feature_ids.len()
        );
    }
}
