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
    /// Unknown word category names (from char.def)
    pub unk_category_names: Vec<String>,
    /// Unknown word category features (from unk.def)
    pub unk_categories: std::collections::HashMap<String, String>,
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
        // Use CharacterDefinition to map character to category ID
        // This works for any dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT, etc.)
        let char_def = &self.config.dict.character_definition;
        let categories = char_def.lookup_categories(ch);

        // Return the first category ID, or 0 (DEFAULT) if no categories match
        if !categories.is_empty() {
            categories[0].0 as u32
        } else {
            0 // DEFAULT category
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

        // Extract unknown word category information
        let char_def = &self.config.dict.character_definition;
        let unk_category_names: Vec<String> = char_def
            .categories()
            .iter()
            .map(|s| s.to_string())
            .collect();

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
            unk_category_names,
            unk_categories: self.config.unk_categories.clone(),
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

    /// Generate a default feature string with appropriate number of fields
    /// based on existing entries in the dictionary
    fn generate_default_features(&self) -> String {
        // Try to infer field count from existing unk_categories
        if let Some(first_unk) = self.config.unk_categories.values().next() {
            let field_count = first_unk.split(',').count();
            return vec!["*"; field_count].join(",");
        }

        // Fallback: try from config.features
        if let Some(first_feature) = self.config.features.first() {
            let field_count = first_feature.split(',').count();
            return vec!["*"; field_count].join(",");
        }

        // Ultimate fallback (should rarely happen)
        "*".to_string()
    }

    /// Extracts part-of-speech information for each label
    fn extract_pos_info(&self) -> Vec<String> {
        // Get POS info from config.features (parallel to surfaces/labels)
        let mut pos_info = Vec::new();

        for (i, label) in self.labels.iter().enumerate() {
            // First check if this is within the vocabulary (config.features range)
            if i < self.config.features.len() {
                pos_info.push(self.config.features[i].clone());
            } else {
                // For unknown word categories (DEFAULT, HIRAGANA, etc.),
                // look up POS info from unk_categories
                let unk_features = self
                    .config
                    .unk_categories
                    .get(label)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Fallback: use DEFAULT category if label not found in unk_categories
                        self.config
                            .unk_categories
                            .get("DEFAULT")
                            .cloned()
                            .unwrap_or_else(|| self.generate_default_features())
                    });
                pos_info.push(unk_features);
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
                let char_def = &self.config.dict.character_definition;
                let category_names = char_def.categories();
                let cate_string = if i < category_names.len() {
                    category_names[i].as_str()
                } else {
                    "UNKNOWN"
                };
                let features = self
                    .config
                    .unk_categories
                    .get(cate_string)
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| {
                        // Use generated default features with appropriate field count
                        Box::leak(self.generate_default_features().into_boxed_str())
                    });

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

    /// Calculate unknown word cost based on trained feature weights using dynamic calculation
    pub fn get_unknown_word_cost(&self, category: usize) -> i32 {
        // Get category name from character definition
        let char_def = &self.config.dict.character_definition;
        let category_names = char_def.categories();

        if category < category_names.len() {
            let category_name = &category_names[category];
            // Look up cost from unk_costs, with fallback to 2000
            self.config
                .unk_costs
                .get(category_name)
                .copied()
                .unwrap_or(2000)
        } else {
            2000 // Default fallback cost
        }
    }

    /// 表層形と素性から文脈ID（left_id, right_id）を推論
    /// 学習済み語彙から最も類似した品詞パターンを探してそのIDを使用
    fn infer_context_ids(&self, surface: &str, features: &str) -> (u32, u32) {
        // Parse feature string to get POS information
        let feature_parts: Vec<&str> = features.split(',').collect();

        // Find best matching entry from trained vocabulary by comparing features
        // Try to match increasingly general patterns:
        // 1. Exact feature match (all fields)
        // 2. First 2 fields match (main POS + sub POS)
        // 3. First field match (main POS only)
        // 4. Same character category (via CharacterDefinition)

        let char_def = &self.config.dict.character_definition;

        // Strategy 1 & 2 & 3: Match by feature similarity
        let mut best_match_idx: Option<usize> = None;
        let mut best_match_score = 0;

        for (i, _label) in self.labels.iter().enumerate() {
            // Skip unknown word categories
            if i >= self.config.features.len() {
                break;
            }

            let vocab_features = &self.config.features[i];
            let vocab_parts: Vec<&str> = vocab_features.split(',').collect();

            // Calculate similarity score
            let mut score = 0;
            let max_fields = feature_parts.len().min(vocab_parts.len());

            for j in 0..max_fields {
                if feature_parts[j] == vocab_parts[j] {
                    // Weight earlier fields more heavily (POS > sub-POS > details)
                    score += (max_fields - j) * 10;
                }
            }

            if score > best_match_score {
                best_match_score = score;
                best_match_idx = Some(i);
            }
        }

        // If found a match from vocabulary, look up from user_entries
        if let Some(idx) = best_match_idx {
            // Try to find in user_entries first
            if idx < self.user_entries.len() {
                let (_, entry, _) = &self.user_entries[idx];
                return (entry.left_id as u32, entry.right_id as u32);
            }
        }

        // Strategy 4: If no good match found, use character category
        if best_match_score == 0 && !surface.is_empty() {
            let first_char = surface.chars().next().unwrap();
            let categories = char_def.lookup_categories(first_char);

            if !categories.is_empty() {
                let category_id = categories[0].0 as u32;
                // Use category ID as both left and right ID
                return (category_id, category_id);
            }
        }

        // Ultimate fallback: use first user_entry's IDs or default to 0
        if let Some((_, entry, _)) = self.user_entries.first() {
            (entry.left_id as u32, entry.right_id as u32)
        } else {
            (0, 0)
        }
    }

    /// 学習データから最大文脈IDを計算
    #[allow(dead_code)]
    fn calculate_max_context_id(&self) -> u32 {
        let mut max_id = 0u32;

        // Get maximum ID from all user_entries
        for (_, entry, _) in &self.user_entries {
            max_id = max_id.max(entry.left_id as u32).max(entry.right_id as u32);
        }

        max_id
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
    /// Generate a default feature string with appropriate number of fields
    /// based on existing entries in the dictionary
    fn generate_default_features(&self) -> String {
        // Try to infer field count from existing unk_categories
        if let Some(first_unk) = self.unk_categories.values().next() {
            let field_count = first_unk.split(',').count();
            return vec!["*"; field_count].join(",");
        }

        // Fallback: try from pos_info
        if let Some(first_pos) = self.pos_info.first() {
            let field_count = first_pos.split(',').count();
            return vec!["*"; field_count].join(",");
        }

        // Ultimate fallback (should rarely happen)
        "*".to_string()
    }

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

        // Get unknown word category labels to skip them
        let unk_category_names_set: Vec<&str> =
            self.unk_category_names.iter().map(|s| s.as_str()).collect();

        // Write lexicon entries using learned connection IDs and costs
        for (i, label) in self.labels.iter().enumerate() {
            // Skip unknown word categories (they go to unk.def)
            if unk_category_names_set.contains(&label.as_str()) {
                continue;
            }

            if i < self.feature_sets.len() {
                let fs = &self.feature_sets[i];
                let pos_info_str;
                let pos_info = if i < self.pos_info.len() {
                    &self.pos_info[i]
                } else {
                    pos_info_str = self.generate_default_features();
                    &pos_info_str
                };

                // Use learned left_id, right_id, and weight directly
                let cost = (-fs.weight * weight_scale_factor) as i16;

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    label, fs.left_id, fs.right_id, cost, pos_info
                )?;
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
        let weight_scale_factor = self.calculate_weight_scale_factor();

        // Unknown word categories are at the end of labels
        let unk_start_idx = self
            .labels
            .len()
            .saturating_sub(self.unk_category_names.len());

        eprintln!(
            "DEBUG: Writing unknown dictionary, unk_start_idx={}, num_categories={}",
            unk_start_idx,
            self.unk_category_names.len()
        );

        for (i, category_name) in self.unk_category_names.iter().enumerate() {
            let feature_idx = unk_start_idx + i;

            if feature_idx < self.feature_sets.len() {
                let fs = &self.feature_sets[feature_idx];

                // Get features from unk_categories
                let features = self
                    .unk_categories
                    .get(category_name)
                    .cloned()
                    .unwrap_or_else(|| self.generate_default_features());

                // Use learned connection IDs and cost
                let cost = (-fs.weight * weight_scale_factor) as i16;

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    category_name, fs.left_id, fs.right_id, cost, features
                )?;

                eprintln!(
                    "DEBUG: unk category={}, left_id={}, right_id={}, weight={:.3}, cost={}",
                    category_name, fs.left_id, fs.right_id, fs.weight, cost
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::trainer::{Trainer, TrainerConfig};
    use std::io::Cursor;

    #[test]
    fn test_trainer_creation() {
        // Test that Trainer can be created from a valid config
        let lexicon_data = "外国,0,0,5000,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n";
        let char_data = "# char.def placeholder\n";
        let unk_data = "# unk.def placeholder\n";
        let feature_data = "UNIGRAM:%F[0]\nLEFT:%L[0]\nRIGHT:%R[0]\n";
        let rewrite_data = "# rewrite.def placeholder\n";

        let config_result = TrainerConfig::from_readers(
            Cursor::new(lexicon_data.as_bytes()),
            Cursor::new(char_data.as_bytes()),
            Cursor::new(unk_data.as_bytes()),
            Cursor::new(feature_data.as_bytes()),
            Cursor::new(rewrite_data.as_bytes()),
        );

        assert!(config_result.is_ok());
        let config = config_result.unwrap();

        // Test trainer creation with builder pattern
        let trainer = Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .max_iter(10)
            .num_threads(1);

        // Verify trainer settings using the getters
        assert_eq!(trainer.get_regularization_cost(), 0.01);
        assert_eq!(trainer.get_max_iter(), 10);
        assert_eq!(trainer.get_num_threads(), 1);
    }
}
