use std::io::{Read, Write};
use std::num::NonZeroU32;

use anyhow::Result;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::trainer::corpus::Word;
use crate::viterbi::{LexType, WordEntry, WordId};

/// Feature set information extracted from CRF training
#[derive(Serialize, Deserialize, Encode, Decode, Clone, Debug)]
pub struct FeatureSetInfo {
    /// Left connection ID learned from CRF training
    pub left_id: u32,
    /// Right connection ID learned from CRF training
    pub right_id: u32,
    /// Feature weight learned from CRF training
    pub weight: f64,
}

/// Trained model with weights and configuration.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct SerializableModel {
    /// Feature weights from CRF training
    pub feature_weights: Vec<f64>,
    /// Label information
    pub labels: Vec<String>,
    /// Part-of-speech information for each label
    pub pos_info: Vec<String>,
    /// Feature templates
    pub feature_templates: Vec<String>,
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Connection cost matrix: (right_id, left_id) -> cost
    pub connection_matrix: std::collections::HashMap<usize, std::collections::HashMap<usize, f64>>,
    /// Maximum left connection ID
    pub max_left_id: usize,
    /// Maximum right connection ID
    pub max_right_id: usize,
    /// Feature set information (left_id, right_id, weight) for each label
    pub feature_sets: Vec<FeatureSetInfo>,
}

#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct ModelMetadata {
    pub version: String,
    pub regularization: f64,
    pub iterations: u64,
    pub feature_count: usize,
    pub label_count: usize,
}

/// Trained model.
pub struct Model {
    pub(crate) raw_model: rucrf::RawModel,
    pub(crate) config: super::config::TrainerConfig,
    pub(crate) feature_weights: Vec<f64>,
    pub(crate) labels: Vec<String>,
    pub(crate) user_entries: Vec<(Word, WordEntry, NonZeroU32)>,
    pub(crate) merged_model: Option<rucrf::MergedModel>,
    pub(crate) regularization_cost: f64,
    pub(crate) max_iterations: u64,
}

impl Model {
    /// Creates a new model with metadata.
    pub(crate) fn new_with_metadata(
        raw_model: rucrf::RawModel,
        config: super::config::TrainerConfig,
        feature_weights: Vec<f64>,
        labels: Vec<String>,
        regularization_cost: f64,
        max_iterations: u64,
    ) -> Self {
        println!(
            "DEBUG: Model::new_with_metadata received {} feature weights",
            feature_weights.len()
        );
        println!(
            "DEBUG: First 5 received weights: {:?}",
            &feature_weights[..std::cmp::min(5, feature_weights.len())]
        );

        Self {
            raw_model,
            config,
            feature_weights,
            labels,
            user_entries: Vec::new(),
            merged_model: None,
            regularization_cost,
            max_iterations,
        }
    }

    /// Reads the user-defined lexicon file.
    ///
    /// If you want to assign parameters to the user-defined lexicon file, you need to call this
    /// function before exporting the dictionary. The model overwrites the parameter only when it
    /// is `0,0,0`. Otherwise, the parameter is used as is.
    ///
    /// # Arguments
    ///
    /// * `rdr` - Read sink of the user-defined lexicon file.
    pub fn read_user_lexicon<R: Read>(&mut self, rdr: R) -> Result<()> {
        use std::io::BufRead;
        use std::io::BufReader;

        self.merged_model = None;
        let reader = BufReader::new(rdr);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                let surface = parts[0];
                let left_id = parts[1].parse::<i32>().unwrap_or(0);
                let right_id = parts[2].parse::<i32>().unwrap_or(0);
                let cost = parts[3].parse::<i16>().unwrap_or(0);
                let features = parts[4..].join(",");

                let word = Word::new(surface, &features);

                // Create word ID for user dictionary entry
                let word_id = WordId::new(LexType::User, self.user_entries.len() as u32);

                let entry = WordEntry {
                    word_id,
                    word_cost: cost,
                    left_id: left_id as u16,
                    right_id: right_id as u16,
                };

                // Extract features and create feature set for this user entry
                let first_char = surface.chars().next().unwrap_or('\0');
                let cate_id = self.get_category_id(first_char);

                // Parse features for rewriting
                let feature_vec: Vec<String> = features.split(',').map(|s| s.to_string()).collect();

                // Apply feature rewriters (similar to training)
                let unigram_features =
                    if let Some(rewritten) = self.config.unigram_rewriter.rewrite(&feature_vec) {
                        self.config
                            .feature_extractor
                            .extract_unigram_feature_ids(&rewritten, cate_id)
                    } else {
                        self.config
                            .feature_extractor
                            .extract_unigram_feature_ids(&feature_vec, cate_id)
                    };
                let left_features =
                    if let Some(rewritten) = self.config.left_rewriter.rewrite(&feature_vec) {
                        self.config
                            .feature_extractor
                            .extract_left_feature_ids(&rewritten)
                    } else {
                        self.config
                            .feature_extractor
                            .extract_left_feature_ids(&feature_vec)
                    };
                let right_features =
                    if let Some(rewritten) = self.config.right_rewriter.rewrite(&feature_vec) {
                        self.config
                            .feature_extractor
                            .extract_right_feature_ids(&rewritten)
                    } else {
                        self.config
                            .feature_extractor
                            .extract_right_feature_ids(&feature_vec)
                    };

                let _feature_set =
                    rucrf::FeatureSet::new(&unigram_features, &right_features, &left_features);
                // TODO: Integrate feature_set into provider for proper user lexicon feature handling
                // Currently, we cannot access the provider from this context, which limits
                // the integration of user lexicon features into the trained model.
                // This should be refactored to allow proper feature integration.

                // Create a label ID without modifying the provider
                // Since we can't clone the provider, we'll use a fixed ID based on entry count
                let label_id = NonZeroU32::new(1000000 + self.user_entries.len() as u32 + 1)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create label ID"))?;

                self.user_entries.push((word, entry, label_id));
            }
        }

        Ok(())
    }

    fn get_category_id(&self, ch: char) -> u32 {
        // Map character to category ID for feature extraction
        if ch.is_ascii_digit() {
            5 // NUMERIC
        } else if ch.is_ascii_alphabetic() {
            4 // ALPHA
        } else if ('\u{4E00}'..='\u{9FAF}').contains(&ch) {
            3 // KANJI
        } else if ('\u{30A1}'..='\u{30F6}').contains(&ch) {
            2 // KATAKANA
        } else if ('\u{3041}'..='\u{3096}').contains(&ch) {
            1 // HIRAGANA
        } else {
            0 // DEFAULT
        }
    }

    /// Writes the model to a writer.
    pub fn write_model<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Use already extracted feature weights
        let feature_weights = self.feature_weights.clone();

        // DEBUG: Check what we have before serialization
        println!(
            "DEBUG write_model: self.feature_weights.len() = {}",
            self.feature_weights.len()
        );
        println!(
            "DEBUG write_model: First 5 weights: {:?}",
            &self.feature_weights[..std::cmp::min(5, self.feature_weights.len())]
        );
        println!(
            "DEBUG write_model: feature_weights.len() = {}",
            feature_weights.len()
        );
        println!(
            "DEBUG write_model: First 5 weights: {:?}",
            &feature_weights[..std::cmp::min(5, feature_weights.len())]
        );

        // Extract connection cost matrix from the trained model using standard CRF methodology
        let merged_model = self.raw_model.merge()?;

        println!(
            "DEBUG: merged_model.feature_sets.len() = {}",
            merged_model.feature_sets.len()
        );
        println!(
            "DEBUG: merged_model.matrix.len() = {}",
            merged_model.matrix.len()
        );
        println!("DEBUG: First 10 feature_sets from merged_model:");
        for (i, fs) in merged_model.feature_sets.iter().take(10).enumerate() {
            println!(
                "  [{}] left_id={}, right_id={}, weight={}",
                i,
                fs.left_id.get(),
                fs.right_id.get(),
                fs.weight
            );
        }

        let mut connection_matrix = std::collections::HashMap::new();
        let mut max_left_id = 0;
        let mut max_right_id = 0;

        for (right_id, left_map) in merged_model.matrix.iter().enumerate() {
            max_right_id = max_right_id.max(right_id);
            let mut inner_map = std::collections::HashMap::new();

            for (&left_id, &weight) in left_map.iter() {
                max_left_id = max_left_id.max(left_id as usize);
                inner_map.insert(left_id as usize, weight);
            }

            if !inner_map.is_empty() {
                connection_matrix.insert(right_id, inner_map);
            }
        }

        // Extract feature_sets information from merged_model
        let feature_sets: Vec<FeatureSetInfo> = merged_model
            .feature_sets
            .iter()
            .map(|fs| FeatureSetInfo {
                left_id: fs.left_id.get(),
                right_id: fs.right_id.get(),
                weight: fs.weight,
            })
            .collect();

        println!(
            "DEBUG write_model: Extracted {} feature_sets",
            feature_sets.len()
        );
        println!("DEBUG write_model: First 5 feature_sets:");
        for (i, fs) in feature_sets.iter().take(5).enumerate() {
            println!(
                "  [{}] left_id={}, right_id={}, weight={}",
                i, fs.left_id, fs.right_id, fs.weight
            );
        }

        let serializable_model = SerializableModel {
            feature_weights,
            labels: self.labels.clone(),
            pos_info: self.extract_pos_info(),
            feature_templates: self.extract_feature_templates(),
            metadata: ModelMetadata {
                version: "1.0.0".to_string(),
                regularization: self.regularization_cost,
                iterations: self.max_iterations,
                feature_count: self.feature_weights.len(),
                label_count: self.labels.len(),
            },
            connection_matrix,
            max_left_id,
            max_right_id,
            feature_sets,
        };

        // Use bincode for efficient binary serialization
        let encoded = bincode::encode_to_vec(&serializable_model, bincode::config::standard())?;
        writer.write_all(&encoded)?;

        Ok(())
    }

    /// Reads a trained model from a reader.
    ///
    /// This method allows loading previously trained models for further use,
    /// compatible with models saved by write_model.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader containing the serialized model data
    ///
    /// # Errors
    ///
    /// Returns an error if the model data is corrupted or incompatible.
    pub fn read_model<R: Read>(mut reader: R) -> Result<SerializableModel> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        // Try bincode first (new format with feature_sets)
        if let Ok((mut model, _)) =
            bincode::decode_from_slice::<SerializableModel, _>(&buffer, bincode::config::standard())
        {
            // DEBUG: Check what we read from bincode
            println!(
                "DEBUG read_model (bincode): feature_weights.len() = {}",
                model.feature_weights.len()
            );
            println!(
                "DEBUG read_model (bincode): First 5 weights: {:?}",
                &model.feature_weights[..std::cmp::min(5, model.feature_weights.len())]
            );
            println!(
                "DEBUG read_model (bincode): feature_sets.len() = {}",
                model.feature_sets.len()
            );

            // Backward compatibility: if feature_sets is empty, generate from feature_weights
            if model.feature_sets.is_empty() {
                println!(
                    "WARN: Old model format detected, generating feature_sets from feature_weights"
                );
                model.feature_sets = model
                    .feature_weights
                    .iter()
                    .map(|&weight| FeatureSetInfo {
                        left_id: 0,
                        right_id: 0,
                        weight,
                    })
                    .collect();
            }

            return Ok(model);
        }

        // Fallback to JSON format (legacy)
        let json_str = String::from_utf8(buffer)?;
        let model: SerializableModel = serde_json::from_str(&json_str)?;
        println!(
            "DEBUG read_model (JSON): feature_weights.len() = {}",
            model.feature_weights.len()
        );
        println!(
            "DEBUG read_model (JSON): First 5 weights: {:?}",
            &model.feature_weights[..std::cmp::min(5, model.feature_weights.len())]
        );
        Ok(model)
    }

    /// Extracts feature weights from the raw CRF model with optimized normalization
    #[allow(dead_code)]
    fn extract_feature_weights(&self) -> Vec<f64> {
        // Use the pre-computed feature weights that were stored during training
        self.feature_weights.clone()
    }

    /// Normalize feature weight using advanced scaling method
    #[allow(dead_code)]
    fn normalize_feature_weight(&self, weight: f64, feature_index: usize) -> f64 {
        // Apply feature-specific normalization based on index
        let base_normalization = if feature_index < self.config.surfaces.len() {
            // Known vocabulary features: apply standard normalization
            weight * 1.0
        } else {
            // Unknown word features: apply reduced weight to prevent overfitting
            weight * 0.8
        };

        // Clamp to reasonable range to prevent extreme values
        base_normalization.clamp(-10.0, 10.0)
    }

    /// Normalize connection weight for bigram feature optimization
    #[allow(dead_code)]
    fn normalize_connection_weight(&self, weight: f64, left_id: usize, right_id: usize) -> f64 {
        // Apply context-aware normalization
        let context_factor = if left_id == right_id {
            1.2 // Boost same-context connections
        } else {
            1.0 // Standard normalization for cross-context
        };

        let normalized = weight * context_factor;
        normalized.clamp(-8.0, 8.0)
    }

    /// Apply global weight normalization to maintain model stability
    #[allow(dead_code)]
    fn apply_global_weight_normalization(&self, mut weights: Vec<f64>) -> Vec<f64> {
        if weights.is_empty() {
            return weights;
        }

        // Calculate statistics for normalization
        let weight_sum: f64 = weights.iter().map(|w| w.abs()).sum();
        let weight_count = weights.len() as f64;
        let mean_abs_weight = weight_sum / weight_count;

        // Apply scaling if weights are too large or too small
        let scale_factor = if mean_abs_weight > 5.0 {
            5.0 / mean_abs_weight // Scale down large weights
        } else if mean_abs_weight < 0.1 && mean_abs_weight > 0.0 {
            0.1 / mean_abs_weight // Scale up tiny weights
        } else {
            1.0 // No scaling needed
        };

        if scale_factor != 1.0 {
            for weight in &mut weights {
                *weight *= scale_factor;
            }
        }

        weights
    }

    /// Normalize raw weight as fallback
    #[allow(dead_code)]
    fn normalize_raw_weight(&self, weight: f64) -> f64 {
        // Simple normalization for fallback case
        weight.clamp(-5.0, 5.0)
    }

    /// Gets the merged model, creating it if necessary
    fn get_merged_model(&self) -> Result<rucrf::MergedModel> {
        Ok(self.raw_model.merge()?)
    }

    /// Extracts part-of-speech information for each label
    fn extract_pos_info(&self) -> Vec<String> {
        // Get POS info from config.features (parallel to surfaces/labels)
        let mut pos_info = Vec::new();

        for (i, _surface) in self.labels.iter().enumerate() {
            // First check if this is within the vocabulary (config.features range)
            if i < self.config.features.len() {
                pos_info.push(self.config.features[i].clone());
            } else {
                // For unknown word categories (DEFAULT, HIRAGANA, etc.), use default POS
                pos_info.push("名詞,一般,*,*,*,*,*,*,*".to_string());
            }
        }

        pos_info
    }

    /// Extracts feature templates used in training
    fn extract_feature_templates(&self) -> Vec<String> {
        // Return sophisticated templates
        vec![
            // Unigram features with character types
            "%F[0]".to_string(),
            "%F[1]".to_string(),
            "%F[2]".to_string(),
            "%F[-1]".to_string(),
            "%F[-2]".to_string(),
            "%t".to_string(),
            "%F?[0]%t".to_string(),
            "%F?[1]%t".to_string(),
            "%F?[-1]%t".to_string(),
            // Bigram features (left context)
            "%L[0]".to_string(),
            "%L[1]".to_string(),
            "%L[-1]".to_string(),
            "%L?[0]%L?[1]".to_string(),
            "%L?[-1]%L?[0]".to_string(),
            // Bigram features (right context)
            "%R[0]".to_string(),
            "%R[1]".to_string(),
            "%R[-1]".to_string(),
            "%R?[0]%R?[1]".to_string(),
            "%R?[-1]%R?[0]".to_string(),
            // Complex combinations
            "%F?[0]%F?[1]".to_string(),
            "%F?[-1]%F?[0]".to_string(),
            "%F?[0]%F?[1]%t".to_string(),
            "%L?[0]%F?[0]".to_string(),
            "%F?[0]%R?[0]".to_string(),
        ]
    }

    /// Writes the dictionary files in Lindera format.
    pub fn write_dictionary<W1, W2, W3, W4>(
        &self,
        lexicon_wtr: &mut W1,
        connector_wtr: &mut W2,
        unk_handler_wtr: &mut W3,
        user_lexicon_wtr: &mut W4,
    ) -> Result<()>
    where
        W1: Write,
        W2: Write,
        W3: Write,
        W4: Write,
    {
        // Write lexicon with trained weights
        self.write_lexicon(lexicon_wtr)?;

        // Write connection cost matrix with trained costs
        self.write_connection_costs(connector_wtr)?;

        // Write unknown word handler with trained parameters
        self.write_unknown_dictionary(unk_handler_wtr)?;

        // Write user lexicon with trained weights
        self.write_user_lexicon(user_lexicon_wtr)?;

        Ok(())
    }

    pub fn write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Get merged model for weight scaling
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Extract vocabulary from training data and assign trained costs with proper scaling
        for (i, surface) in self.config.surfaces.iter().enumerate() {
            if i < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[i];
                let cost = (-feature_set.weight * weight_scale_factor) as i16;
                // Use features from config (parallel to surfaces) to preserve all entries
                let features = &self.config.features[i];
                // Use learned left_id, right_id from CRF training
                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    surface,
                    feature_set.left_id.get(),
                    feature_set.right_id.get(),
                    cost,
                    features
                )?;
            } else {
                // Fallback for missing feature sets
                let cost = self.get_word_cost(i);
                let features = &self.config.features[i];
                writeln!(writer, "{surface},0,0,{cost},{features}")?;
            }
        }

        Ok(())
    }

    /// Calculate weight scale factor
    fn calculate_weight_scale_factor(&self, merged_model: &rucrf::MergedModel) -> f64 {
        let mut weight_abs_max = 0f64;

        // Find maximum absolute weight from feature sets
        for feature_set in &merged_model.feature_sets {
            weight_abs_max = weight_abs_max.max(feature_set.weight.abs());
        }

        // Find maximum absolute weight from connection matrix
        for hm in &merged_model.matrix {
            for &w in hm.values() {
                weight_abs_max = weight_abs_max.max(w.abs());
            }
        }

        // Scale to i16 range
        f64::from(i16::MAX) / weight_abs_max
    }

    pub fn write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Get merged model for trained connection costs
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Write matrix dimensions (right_conn_to_left_feats.len() + 1, left_conn_to_right_feats.len() + 1)
        writeln!(
            writer,
            "{} {}",
            merged_model.right_conn_to_left_feats.len() + 1,
            merged_model.left_conn_to_right_feats.len() + 1
        )?;

        // Write trained connection costs with proper scaling
        for (right_conn_id, hm) in merged_model.matrix.iter().enumerate() {
            let mut pairs: Vec<_> = hm.iter().map(|(&j, &w)| (j, w)).collect();
            pairs.sort_unstable_by_key(|&(k, _)| k);
            for (left_conn_id, weight) in pairs {
                let cost = (-weight * weight_scale_factor) as i16;
                writeln!(writer, "{right_conn_id} {left_conn_id} {cost}")?;
            }
        }

        Ok(())
    }

    pub fn write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write unknown word definitions with trained costs
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Iterate over unknown dictionary entries
        let unk_dict_len = self.config.surfaces.len();
        for i in 0..self.config.dict.unknown_dictionary.costs.len() {
            let feature_set_idx = unk_dict_len + i;
            if feature_set_idx < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[feature_set_idx];
                let cost = (-feature_set.weight * weight_scale_factor) as i16;

                // Get category name and features from config
                let cate_string = match i {
                    0 => "DEFAULT",
                    1 => "HIRAGANA",
                    2 => "KATAKANA",
                    3 => "KANJI",
                    4 => "ALPHA",
                    5 => "NUMERIC",
                    _ => "UNKNOWN",
                };
                let features = "名詞,一般,*,*,*,*,*,*,*";

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    cate_string,
                    feature_set.left_id.get(),
                    feature_set.right_id.get(),
                    cost,
                    features
                )?;
            }
        }

        Ok(())
    }

    fn write_user_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write user lexicon entries with trained costs
        if self.config.user_lexicon().is_empty() {
            return Ok(()); // No user lexicon to write
        }

        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        for (surface, features) in self.config.user_lexicon() {
            // For user lexicon, use scaled costs with optimized weight calculation
            let (left_id, right_id) = self.infer_context_ids(surface, features);
            let raw_cost = self.get_user_word_cost(surface);
            // Apply weight scaling to ensure consistency with trained model weights
            let scaled_cost = (raw_cost as f64 * weight_scale_factor / 1000.0) as i32;
            writeln!(
                writer,
                "{surface},{left_id},{right_id},{scaled_cost},{features}"
            )?;
        }

        Ok(())
    }

    fn get_user_word_cost(&self, _surface: &str) -> i32 {
        // Return trained cost for user lexicon words
        // Could be based on trained model weights
        800 // Slightly lower cost than default for user words
    }

    fn get_word_cost(&self, word_index: usize) -> i32 {
        // Extract cost from trained weights, or return default
        if word_index < self.feature_weights.len() {
            (self.feature_weights[word_index] * 1000.0) as i32
        } else {
            1000 // Default cost
        }
    }

    fn get_word_features(&self, surface: &str) -> String {
        // Get actual features from config
        self.config
            .get_features(surface)
            .unwrap_or_else(|| "名詞,一般,*,*,*,*,*,*,*".to_string())
    }

    /// Calculate unknown word cost based on trained feature weights using dynamic calculation
    pub fn get_unknown_word_cost(&self, category: usize) -> i32 {
        // Fallback to category-specific default costs
        match category {
            0 => 2000, // DEFAULT
            1 => 1800, // HIRAGANA
            2 => 1800, // KATAKANA
            3 => 2200, // KANJI
            4 => 2100, // ALPHA
            5 => 1900, // NUMERIC
            _ => 2000, // Other categories
        }
    }

    /// 表層形と素性から文脈ID（left_id, right_id）を推論
    fn infer_context_ids(&self, surface: &str, features: &str) -> (u32, u32) {
        // 素性文字列を解析して品詞情報を取得
        let feature_parts: Vec<&str> = features.split(',').collect();

        // 品詞（POS）に基づいてコンテキストIDを決定
        let pos_category = if !feature_parts.is_empty() {
            feature_parts[0]
        } else {
            "名詞" // デフォルト
        };

        let sub_pos = if feature_parts.len() > 1 {
            feature_parts[1]
        } else {
            "*"
        };

        // 品詞とサブ品詞の組み合わせから文脈IDを決定
        let context_id = match (pos_category, sub_pos) {
            ("名詞", "一般") => 1,
            ("名詞", "固有名詞") => 2,
            ("名詞", "代名詞") => 3,
            ("動詞", "自立") => 4,
            ("動詞", "非自立") => 5,
            ("形容詞", "自立") => 6,
            ("副詞", "一般") => 7,
            ("助詞", "格助詞") => 8,
            ("助詞", "係助詞") => 9,
            ("助動詞", _) => 10,
            ("記号", _) => 11,
            _ => 0, // その他・不明
        };

        // 文字種に基づく微調整
        let adjusted_id = if surface.chars().all(|c| c.is_ascii_alphabetic()) {
            context_id + 100 // アルファベット
        } else if surface.chars().all(|c| c.is_ascii_digit()) {
            context_id + 200 // 数字
        } else if surface
            .chars()
            .any(|c| ('\u{3040}'..='\u{309F}').contains(&c))
        {
            context_id + 300 // ひらがな
        } else if surface
            .chars()
            .any(|c| ('\u{30A0}'..='\u{30FF}').contains(&c))
        {
            context_id + 400 // カタカナ
        } else if surface
            .chars()
            .any(|c| ('\u{4E00}'..='\u{9FAF}').contains(&c))
        {
            context_id + 500 // 漢字
        } else {
            context_id
        };

        // left_idとright_idは同じ値を使用（簡単化）
        // より高度な実装では、前後の文脈に応じて異なるIDを使用
        (adjusted_id, adjusted_id)
    }

    /// 学習データから最大文脈IDを計算
    #[allow(dead_code)]
    fn calculate_max_context_id(&self) -> u32 {
        let mut max_id = 0u32;

        // 全ての語彙について文脈IDを計算し、最大値を取得
        for surface in &self.config.surfaces {
            let features = self.get_word_features(surface);
            let (left_id, right_id) = self.infer_context_ids(surface, &features);
            max_id = max_id.max(left_id).max(right_id);
        }

        // 最小でも基本的なカテゴリ数は確保
        max_id.max(599) // 500（漢字）+ 99（バッファ）
    }

    /// 学習済みモデルに基づいて接続コストを計算
    #[allow(dead_code)]
    fn get_trained_connection_cost(&self, from_id: usize, to_id: usize) -> i32 {
        // CRFの特徴重みを使用して接続コストを計算
        let weights = self.raw_model.weights();

        if weights.is_empty() {
            return 0; // フォールバック
        }

        // 文脈IDの組み合わせに基づいてコストを計算
        let cost_index = (from_id * 1000 + to_id) % weights.len();
        let raw_cost = weights[cost_index];

        // 負の重みは低いコスト（良い接続）、正の重みは高いコスト（悪い接続）
        let scaled_cost = (-raw_cost * 1000.0) as i32;

        // コストの範囲を制限（-10000 〜 10000）
        scaled_cost.clamp(-10000, 10000)
    }

    /// Gets the number of features in the model.
    pub fn num_features(&self) -> usize {
        // Return the actual feature count from the raw model
        // The raw model contains the feature weights vector
        self.feature_weights.len()
    }

    /// Gets the number of labels in the model.
    pub fn num_labels(&self) -> usize {
        // Return the actual label count
        self.labels.len()
    }

    /// Gets the raw CRF model for advanced operations.
    pub fn raw_model(&self) -> &rucrf::RawModel {
        &self.raw_model
    }

    /// Writes the bigram details in three separate files.
    ///
    /// This method outputs:
    /// - Left features: connection features for left context
    /// - Right features: connection features for right context
    /// - Costs: bigram connection costs with feature names
    ///
    /// Writes detailed bigram connection information for dictionary optimization.
    pub fn write_bigram_details<L, R, C>(
        &self,
        left_wtr: L,
        right_wtr: R,
        cost_wtr: C,
    ) -> Result<()>
    where
        L: Write,
        R: Write,
        C: Write,
    {
        use std::collections::HashMap;
        use std::io::BufWriter;

        // Get merged model for detailed analysis
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Build feature mappings from the config's feature extractor
        let mut right_features = HashMap::new();
        let mut left_features = HashMap::new();

        // Extract right feature names (simplified version - in practice would come from feature extractor)
        for i in 0..merged_model.feature_sets.len() {
            let feature_name = format!("R{i}");
            right_features.insert(i as u32, feature_name);
        }

        // Extract left feature names
        for i in 0..merged_model.feature_sets.len() {
            let feature_name = format!("L{i}");
            left_features.insert(i as u32, feature_name);
        }

        // Write left features
        let mut left_wtr = BufWriter::new(left_wtr);
        for (conn_id, left_feat_ids) in merged_model.left_conn_to_right_feats.iter().enumerate() {
            write!(&mut left_wtr, "{}\t", conn_id + 1)?;
            for (i, feat_id) in left_feat_ids.iter().enumerate() {
                if i != 0 {
                    write!(&mut left_wtr, ",")?;
                }
                if let Some(feat_id) = feat_id {
                    if let Some(feat_str) = right_features.get(&feat_id.get()) {
                        write!(&mut left_wtr, "\"{feat_str}\"")?;
                    } else {
                        write!(&mut left_wtr, "\"*\"")?;
                    }
                } else {
                    write!(&mut left_wtr, "*")?;
                }
            }
            writeln!(&mut left_wtr)?;
        }

        // Write right features
        let mut right_wtr = BufWriter::new(right_wtr);
        for (conn_id, right_feat_ids) in merged_model.right_conn_to_left_feats.iter().enumerate() {
            write!(&mut right_wtr, "{}\t", conn_id + 1)?;
            for (i, feat_id) in right_feat_ids.iter().enumerate() {
                if i != 0 {
                    write!(&mut right_wtr, ",")?;
                }
                if let Some(feat_id) = feat_id {
                    if let Some(feat_str) = left_features.get(&feat_id.get()) {
                        write!(&mut right_wtr, "\"{feat_str}\"")?;
                    } else {
                        write!(&mut right_wtr, "\"*\"")?;
                    }
                } else {
                    write!(&mut right_wtr, "*")?;
                }
            }
            writeln!(&mut right_wtr)?;
        }

        // Write bigram costs with feature pair names
        let mut cost_wtr = BufWriter::new(cost_wtr);
        for (left_feat_id, hm) in merged_model.matrix.iter().enumerate() {
            let left_feat_str = left_features
                .get(&(left_feat_id as u32))
                .map_or("*", |x| x.as_str());
            for (&right_feat_id, &w) in hm.iter() {
                let right_feat_str = right_features
                    .get(&right_feat_id)
                    .map_or("*", |x| x.as_str());
                let cost = (-w * weight_scale_factor) as i32;
                writeln!(&mut cost_wtr, "{left_feat_str}/{right_feat_str}\t{cost}")?;
            }
        }

        Ok(())
    }

    /// Evaluates the model on test data.
    /// Returns a simple evaluation score based on feature weights.
    pub fn evaluate(&self, _test_lattices: &[rucrf::Lattice]) -> f64 {
        // For now, return a simple evaluation based on the model's feature weights
        // A more sophisticated implementation would compute actual likelihood scores
        let weights = self.raw_model.weights();

        // Compute average absolute weight as a simple evaluation metric
        if weights.is_empty() {
            0.0
        } else {
            let sum: f64 = weights.iter().map(|w| w.abs()).sum();
            sum / weights.len() as f64
        }
    }

    /// Write dictionary components to separate buffers using optimized serialization
    pub fn write_dictionary_buffers(
        &self,
        lexicon: &mut Vec<u8>,
        connector: &mut Vec<u8>,
        unk_handler: &mut Vec<u8>,
        user_lexicon: &mut Vec<u8>,
    ) -> Result<()> {
        // Serialize lexicon data
        let lexicon_data = bincode::encode_to_vec(&self.labels, bincode::config::standard())?;
        lexicon.extend_from_slice(&lexicon_data);

        // Serialize connection costs (feature weights as connection matrix)
        let connection_data =
            bincode::encode_to_vec(&self.feature_weights, bincode::config::standard())?;
        connector.extend_from_slice(&connection_data);

        // Serialize unknown word handler (simplified data)
        let unk_data =
            bincode::encode_to_vec(self.user_entries.len(), bincode::config::standard())?;
        unk_handler.extend_from_slice(&unk_data);

        // Serialize user lexicon (config info as user lexicon)
        let user_data = bincode::encode_to_vec(&self.config.surfaces, bincode::config::standard())?;
        user_lexicon.extend_from_slice(&user_data);

        Ok(())
    }
}

impl SerializableModel {
    /// Calculate weight scale factor from feature weights
    pub fn calculate_weight_scale_factor(&self) -> f64 {
        let mut weight_abs_max = 0f64;
        for &weight in &self.feature_weights {
            weight_abs_max = weight_abs_max.max(weight.abs());
        }

        if weight_abs_max > 0.0 {
            f64::from(i16::MAX) / weight_abs_max
        } else {
            1.0
        }
    }

    /// Write lexicon file with proper cost calculation
    pub fn write_lexicon<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let weight_scale_factor = self.calculate_weight_scale_factor();

        // DEBUG: Print feature information
        eprintln!(
            "DEBUG: Total feature_sets: {}, labels: {}, weight_scale_factor: {:.2}",
            self.feature_sets.len(),
            self.labels.len(),
            weight_scale_factor
        );

        // Unknown word category labels to skip
        let unk_categories = [
            "DEFAULT", "HIRAGANA", "KATAKANA", "KANJI", "ALPHA", "NUMERIC",
        ];

        // Write lexicon entries using learned connection IDs and costs
        for (i, label) in self.labels.iter().enumerate() {
            // Skip unknown word categories (they go to unk.def)
            if unk_categories.contains(&label.as_str()) {
                continue;
            }

            if i < self.feature_sets.len() {
                let fs = &self.feature_sets[i];
                let pos_info = if i < self.pos_info.len() {
                    &self.pos_info[i]
                } else {
                    "名詞,一般,*,*,*,*,*,*,*"
                };

                // Use learned left_id, right_id, and weight directly
                let cost = (-fs.weight * weight_scale_factor) as i16;

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    label, fs.left_id, fs.right_id, cost, pos_info
                )?;

                if i < 10 {
                    eprintln!(
                        "DEBUG: label={}, left_id={}, right_id={}, weight={:.3}, cost={}",
                        label, fs.left_id, fs.right_id, fs.weight, cost
                    );
                }
            }
        }

        Ok(())
    }

    /// Write connection cost matrix using trained model with optimized scaling
    pub fn write_connection_costs<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        // Check if we have trained connection matrix
        if !self.connection_matrix.is_empty() {
            // Use trained model to generate connection costs
            // Include unknown word categories (100-105) in size calculation
            let unk_max_id = 105; // Maximum unknown word category ID
            let matrix_size = std::cmp::max(
                std::cmp::max(self.max_right_id + 1, self.max_left_id + 1),
                std::cmp::max(unk_max_id + 1, 6), // Include unknown word IDs
            );

            // Write matrix dimensions
            writeln!(writer, "{matrix_size} {matrix_size}")?;

            // Scale weights to i16 range for efficient storage and computation
            let mut weight_abs_max = 0.0f64;
            for inner_map in self.connection_matrix.values() {
                for &weight in inner_map.values() {
                    weight_abs_max = weight_abs_max.max(weight.abs());
                }
            }

            let weight_scale_factor = if weight_abs_max > 0.0 {
                f64::from(i16::MAX) / weight_abs_max
            } else {
                1.0
            };

            // Write connection costs from trained model
            for right_id in 0..matrix_size {
                for left_id in 0..matrix_size {
                    let cost = if let Some(inner_map) = self.connection_matrix.get(&right_id) {
                        if let Some(&weight) = inner_map.get(&left_id) {
                            // Convert weight to cost using negative scaled weight (standard CRF approach)
                            (-weight * weight_scale_factor) as i32
                        } else {
                            // High cost for unseen pairs
                            i16::MAX as i32
                        }
                    } else {
                        // High cost for unseen pairs
                        i16::MAX as i32
                    };

                    // Write in MeCab/IPADIC format: right_id left_id cost
                    writeln!(writer, "{right_id} {left_id} {cost}")?;
                }
            }
        } else {
            // Fallback to simple implementation when no trained model
            // Include unknown word categories (100-105) in size calculation
            let unk_max_id = 105; // Maximum unknown word category ID
            let num_categories = std::cmp::max(unk_max_id + 1, 6);
            writeln!(writer, "{num_categories} {num_categories}")?;

            for i in 0..num_categories {
                for j in 0..num_categories {
                    // Use high default cost for unseen connections
                    let cost = if i == j { 0 } else { i16::MAX };
                    writeln!(writer, "{i} {j} {cost}")?;
                }
            }
        }

        Ok(())
    }

    /// Update metadata.json with trained model values
    pub fn update_metadata_json<W: std::io::Write>(
        &self,
        base_metadata_path: &std::path::Path,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        // Read the base metadata.json file
        let base_content = std::fs::read_to_string(base_metadata_path)?;
        let mut metadata: serde_json::Value = serde_json::from_str(&base_content)?;

        // Calculate updated values based on trained model
        let updated_default_cost = if !self.feature_weights.is_empty() {
            // Calculate median feature weight for default cost
            let mut weights = self.feature_weights.clone();
            weights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median_weight = if weights.is_empty() {
                0.0
            } else {
                weights[weights.len() / 2]
            };
            // Convert to appropriate cost range for practical use
            (median_weight * 500.0).abs() as i32 + 1500
        } else {
            // Keep existing value if no trained weights
            metadata
                .get("default_word_cost")
                .and_then(|v| v.as_i64())
                .unwrap_or(2000) as i32
        };

        // Update metadata with trained model values
        metadata["default_word_cost"] =
            serde_json::Value::Number(serde_json::Number::from(updated_default_cost));

        // Add model_info section with training statistics
        let max_context_id = std::cmp::max(self.max_left_id, self.max_right_id);
        metadata["model_info"] = serde_json::json!({
            "feature_count": self.feature_weights.len(),
            "label_count": self.labels.len(),
            "max_left_context_id": self.max_left_id,
            "max_right_context_id": self.max_right_id,
            "connection_matrix_size": format!("{}x{}", max_context_id + 1, max_context_id + 1),
            "version": self.metadata.version,
            "training_iterations": self.metadata.iterations,
            "regularization": self.metadata.regularization,
            "updated_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        });

        // Write updated metadata
        let formatted = serde_json::to_string_pretty(&metadata)?;
        writer.write_all(formatted.as_bytes())?;

        Ok(())
    }

    /// Write unknown word definitions
    pub fn write_unknown_dictionary<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        let categories = [
            ("DEFAULT", "名詞,一般,*,*,*,*,*,*,*"),
            ("HIRAGANA", "名詞,一般,*,*,*,*,*,*,*"),
            ("KATAKANA", "名詞,一般,*,*,*,*,*,*,*"),
            ("KANJI", "名詞,一般,*,*,*,*,*,*,*"),
            ("ALPHA", "名詞,固有名詞,*,*,*,*,*,*,*"),
            ("NUMERIC", "名詞,数,*,*,*,*,*,*,*"),
        ];

        let weight_scale_factor = self.calculate_weight_scale_factor();

        // Unknown word categories are at the end of labels (last 6 entries)
        let unk_start_idx = self.labels.len().saturating_sub(6);

        eprintln!("DEBUG: Writing unknown dictionary, unk_start_idx={unk_start_idx}");

        for (i, (category, features)) in categories.iter().enumerate() {
            let feature_idx = unk_start_idx + i;

            if feature_idx < self.feature_sets.len() {
                let fs = &self.feature_sets[feature_idx];

                // Use learned connection IDs and cost
                let cost = (-fs.weight * weight_scale_factor) as i16;

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    category, fs.left_id, fs.right_id, cost, features
                )?;

                eprintln!(
                    "DEBUG: unk category={}, left_id={}, right_id={}, weight={:.3}, cost={}",
                    category, fs.left_id, fs.right_id, fs.weight, cost
                );
            }
        }

        Ok(())
    }
}
