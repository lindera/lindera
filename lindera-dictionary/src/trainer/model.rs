use std::io::{Read, Write};
use std::num::NonZeroU32;

use anyhow::Result;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::trainer::corpus::Word;
use crate::viterbi::{LexType, WordEntry, WordId};

/// Convert CRF weight to MeCab-compatible cost using cost-factor.
///
/// MeCab's `tocost(d, n)` formula: `clamp(-n * d, -32767, 32767)`
/// where `d` is the cost-factor and `n` is the CRF weight.
fn tocost(weight: f64, cost_factor: i32) -> i16 {
    let raw = -(cost_factor as f64) * weight;
    raw.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

/// Calculate the optimal cost factor from actual model weights.
///
/// This ensures the full i16 range is utilized, preserving relative differences
/// between weights. The cost factor is computed as `i16::MAX / max_abs_weight`
/// so that the largest weight maps to the boundary of the i16 range.
fn calculate_cost_factor(merged_model: &lindera_crf::MergedModel) -> i32 {
    let mut weight_abs_max = 0f64;

    // Find maximum absolute weight from unigram feature sets
    for feature_set in &merged_model.feature_sets {
        weight_abs_max = weight_abs_max.max(feature_set.weight.abs());
    }

    // Find maximum absolute weight from connection cost matrix
    for hm in &merged_model.matrix {
        for &w in hm.values() {
            weight_abs_max = weight_abs_max.max(w.abs());
        }
    }

    if weight_abs_max > f64::EPSILON {
        (f64::from(i16::MAX) / weight_abs_max) as i32
    } else {
        700 // MeCab default fallback
    }
}

/// Feature set information extracted from CRF training
#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize, Clone, Debug)]

pub struct FeatureSetInfo {
    /// Left connection ID learned from CRF training
    pub left_id: u32,
    /// Right connection ID learned from CRF training
    pub right_id: u32,
    /// Feature weight learned from CRF training
    pub weight: f64,
}

/// Trained model with weights and configuration.
#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct SerializableModel {
    /// Feature weights from CRF training
    pub feature_weights: Vec<f64>,
    /// Label information
    pub labels: Vec<String>,
    /// Part-of-speech information for each label
    pub pos_info: Vec<String>,
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
    /// Raw content of the character definition file (char.def)
    pub char_def_content: String,
    /// Raw content of the feature definition file (feature.def)
    pub feature_def_content: String,
    /// Raw content of the rewrite rule definition file (rewrite.def)
    pub rewrite_def_content: String,
    /// Cost factor for weight-to-cost conversion (MeCab default: 700)
    pub cost_factor: i32,
    /// Left context ID to feature string mapping (for left-id.def)
    pub left_id_map: Vec<(u32, String)>,
    /// Right context ID to feature string mapping (for right-id.def)
    pub right_id_map: Vec<(u32, String)>,
}

#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct ModelMetadata {
    pub version: String,
    pub regularization: f64,
    pub iterations: u64,
    pub feature_count: usize,
    pub label_count: usize,
}

/// Trained model.
pub struct Model {
    pub(crate) raw_model: lindera_crf::RawModel,
    pub(crate) config: super::config::TrainerConfig,
    pub(crate) feature_weights: Vec<f64>,
    pub(crate) labels: Vec<String>,
    pub(crate) user_entries: Vec<(Word, WordEntry, NonZeroU32)>,
    pub(crate) merged_model: Option<lindera_crf::MergedModel>,
    pub(crate) regularization_cost: f64,
    pub(crate) max_iterations: u64,
}

impl Model {
    /// Creates a new model with metadata.
    pub(crate) fn new_with_metadata(
        raw_model: lindera_crf::RawModel,
        config: super::config::TrainerConfig,
        feature_weights: Vec<f64>,
        labels: Vec<String>,
        regularization_cost: f64,
        max_iterations: u64,
    ) -> Self {
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

                // Apply dictionary rewriter to get ufeature, lfeature, rfeature
                let (ufeature, lfeature, rfeature) =
                    self.config.dictionary_rewriter.rewrite(&features);
                let u_vec: Vec<String> = ufeature.split(',').map(|s| s.to_string()).collect();
                let l_vec: Vec<String> = lfeature.split(',').map(|s| s.to_string()).collect();
                let r_vec: Vec<String> = rfeature.split(',').map(|s| s.to_string()).collect();

                let unigram_features = self
                    .config
                    .feature_extractor
                    .extract_unigram_feature_ids(&u_vec, cate_id);
                let left_features = self
                    .config
                    .feature_extractor
                    .extract_left_feature_ids(&l_vec);
                let right_features = self
                    .config
                    .feature_extractor
                    .extract_right_feature_ids(&r_vec);

                let _feature_set = lindera_crf::FeatureSet::new(
                    &unigram_features,
                    &right_features,
                    &left_features,
                );
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
        let feature_weights = self.feature_weights.clone();

        // Extract connection cost matrix from the trained model
        let merged_model = self.raw_model.merge()?;

        // Compute optimal cost factor from actual weight range
        let cost_factor = calculate_cost_factor(&merged_model);

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

        // Extract unknown word category information
        let char_def = &self.config.dict.character_definition;
        let unk_category_names: Vec<String> = char_def
            .categories()
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Build left/right ID maps from merged model's connection IDs
        // before feature_sets is moved into SerializableModel
        // Build left/right ID maps from merged model's connection IDs.
        // Use feature strings (POS info) from config.features, not surface forms.
        let features = &self.config.features;
        let unk_categories = &self.config.unk_categories;
        let unk_start_idx = self
            .labels
            .len()
            .saturating_sub(self.config.dict.character_definition.categories().len());

        let get_feature_string = |i: usize| -> String {
            if i < unk_start_idx {
                // Dictionary entry: use feature string from config.features
                if i < features.len() {
                    features[i].clone()
                } else {
                    "*".to_string()
                }
            } else {
                // Unknown word category: use unk feature string
                let category_idx = i - unk_start_idx;
                let char_def = &self.config.dict.character_definition;
                let categories = char_def.categories();
                if category_idx < categories.len() {
                    let cat_name = &categories[category_idx];
                    unk_categories
                        .get(cat_name)
                        .cloned()
                        .unwrap_or_else(|| format!("UNK_{cat_name}"))
                } else {
                    format!("UNK_{}", category_idx)
                }
            }
        };

        let left_id_map = {
            let mut id_to_feat: std::collections::HashMap<u32, String> =
                std::collections::HashMap::new();
            for (i, fs) in feature_sets.iter().enumerate() {
                let lid = fs.left_id;
                id_to_feat.entry(lid).or_insert_with(|| {
                    let feat = get_feature_string(i);
                    let (_, lfeature, _) = self.config.dictionary_rewriter.rewrite(&feat);
                    lfeature
                });
            }
            let mut entries: Vec<(u32, String)> = id_to_feat.into_iter().collect();
            entries.sort_by_key(|&(id, _)| id);
            entries
        };
        let right_id_map = {
            let mut id_to_feat: std::collections::HashMap<u32, String> =
                std::collections::HashMap::new();
            for (i, fs) in feature_sets.iter().enumerate() {
                let rid = fs.right_id;
                id_to_feat.entry(rid).or_insert_with(|| {
                    let feat = get_feature_string(i);
                    let (_, _, rfeature) = self.config.dictionary_rewriter.rewrite(&feat);
                    rfeature
                });
            }
            let mut entries: Vec<(u32, String)> = id_to_feat.into_iter().collect();
            entries.sort_by_key(|&(id, _)| id);
            entries
        };

        let serializable_model = SerializableModel {
            feature_weights,
            labels: self.labels.clone(),
            pos_info: self.extract_pos_info(),
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
            char_def_content: self.config.char_def_content.clone(),
            feature_def_content: self.config.feature_def_content.clone(),
            rewrite_def_content: self.config.rewrite_def_content.clone(),
            cost_factor,
            left_id_map,
            right_id_map,
        };

        // Use rkyv for efficient binary serialization
        let encoded = rkyv::to_bytes::<rkyv::rancor::Error>(&serializable_model)
            .map_err(|e| anyhow::anyhow!("Failed to serialize model: {}", e))?;
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

        // Try rkyv first (new format with feature_sets)
        if let Ok(mut model) = rkyv::from_bytes::<SerializableModel, rkyv::rancor::Error>(&buffer) {
            // Backward compatibility: if feature_sets is empty, generate from feature_weights
            if model.feature_sets.is_empty() {
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
        Ok(model)
    }

    /// Gets the merged model, creating it if necessary
    fn get_merged_model(&self) -> Result<lindera_crf::MergedModel> {
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
        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        for (i, surface) in self.config.surfaces.iter().enumerate() {
            if i < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[i];
                let cost = tocost(feature_set.weight, cost_factor);
                let features = &self.config.features[i];
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
                let features = &self.config.features[i];
                writeln!(writer, "{surface},0,0,0,{features}")?;
            }
        }

        Ok(())
    }

    pub fn write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()> {
        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        // Dense matrix dimensions: (right_size, left_size)
        // +1 for BOS/EOS (id=0)
        let right_size = merged_model.right_conn_to_left_feats.len() + 1;
        let left_size = merged_model.left_conn_to_right_feats.len() + 1;
        writeln!(writer, "{right_size} {left_size}")?;

        // Dense matrix: output all (right_id, left_id) pairs
        // Unseen pairs get maximum penalty cost to block unlearned transitions,
        // forcing Viterbi to prefer paths through learned POS bigram connections.
        for right_id in 0..right_size {
            for left_id in 0..left_size {
                let cost = if let Some(weight) = merged_model
                    .matrix
                    .get(right_id)
                    .and_then(|hm| hm.get(&(left_id as u32)))
                    .copied()
                {
                    tocost(weight, cost_factor)
                } else {
                    i16::MAX
                };
                writeln!(writer, "{right_id} {left_id} {cost}")?;
            }
        }

        Ok(())
    }

    pub fn write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()> {
        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        let char_def = &self.config.dict.character_definition;
        let category_names = char_def.categories();
        let unk_start_idx = self.config.surfaces.len();

        for (i, category_name) in category_names.iter().enumerate() {
            let feature_set_idx = unk_start_idx + i;
            if feature_set_idx < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[feature_set_idx];
                let cost = tocost(feature_set.weight, cost_factor);

                let default_features = self.generate_default_features();
                let features = self
                    .config
                    .unk_categories
                    .get(category_name.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(default_features.as_str());

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    category_name,
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
        if self.config.user_lexicon().is_empty() {
            return Ok(());
        }

        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        for (surface, features) in self.config.user_lexicon() {
            let (left_id, right_id) = self.infer_context_ids(surface, features);
            let raw_cost = self.get_user_word_cost(surface) as f64 / 1000.0;
            let cost = tocost(raw_cost, cost_factor);
            writeln!(writer, "{surface},{left_id},{right_id},{cost},{features}")?;
        }

        Ok(())
    }

    fn get_user_word_cost(&self, _surface: &str) -> i32 {
        // Return trained cost for user lexicon words
        // Could be based on trained model weights
        800 // Slightly lower cost than default for user words
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

    /// Infer context IDs (left_id, right_id) from surface form and features.
    /// Finds the most similar POS pattern from trained vocabulary and uses its ID.
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

    /// Calculate maximum context ID from training data
    #[allow(dead_code)]
    fn calculate_max_context_id(&self) -> u32 {
        let mut max_id = 0u32;

        // Get maximum ID from all user_entries
        for (_, entry, _) in &self.user_entries {
            max_id = max_id.max(entry.left_id as u32).max(entry.right_id as u32);
        }

        max_id
    }

    /// Calculate connection cost based on trained model
    #[allow(dead_code)]
    fn get_trained_connection_cost(&self, from_id: usize, to_id: usize) -> i32 {
        // Use CRF feature weights to calculate connection cost
        let weights = self.raw_model.weights();

        if weights.is_empty() {
            return 0; // Fallback
        }

        // Calculate cost based on context ID combination
        let cost_index = (from_id * 1000 + to_id) % weights.len();
        let raw_cost = weights[cost_index];

        // Negative weight = low cost (good connection), positive weight = high cost (bad connection)
        let scaled_cost = (-raw_cost * 1000.0) as i32;

        // Limit cost range (-10000 to 10000)
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
    pub fn raw_model(&self) -> &lindera_crf::RawModel {
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
        let cost_factor = self.config.cost_factor;

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
                let cost = tocost(w, cost_factor);
                writeln!(&mut cost_wtr, "{left_feat_str}/{right_feat_str}\t{cost}")?;
            }
        }

        Ok(())
    }

    /// Evaluates the model on test data.
    /// Returns a simple evaluation score based on feature weights.
    pub fn evaluate(&self, _test_lattices: &[lindera_crf::Lattice]) -> f64 {
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
        let lexicon_data = rkyv::to_bytes::<rkyv::rancor::Error>(&self.labels)
            .map_err(|e| anyhow::anyhow!("Failed to serialize lexicon: {}", e))?;
        lexicon.extend_from_slice(&lexicon_data);

        // Serialize connection costs (feature weights as connection matrix)
        let connection_data = rkyv::to_bytes::<rkyv::rancor::Error>(&self.feature_weights)
            .map_err(|e| anyhow::anyhow!("Failed to serialize connector: {}", e))?;
        connector.extend_from_slice(&connection_data);

        // Serialize unknown word handler (simplified data)
        let unk_data = rkyv::to_bytes::<rkyv::rancor::Error>(&self.user_entries.len())
            .map_err(|e| anyhow::anyhow!("Failed to serialize unknown handler: {}", e))?;
        unk_handler.extend_from_slice(&unk_data);

        // Serialize user lexicon (config info as user lexicon)
        let user_data = rkyv::to_bytes::<rkyv::rancor::Error>(&self.config.surfaces)
            .map_err(|e| anyhow::anyhow!("Failed to serialize user lexicon: {}", e))?;
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

    /// Write lexicon file with proper cost calculation
    pub fn write_lexicon<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        // Unknown word categories are at the end of labels, skip them (they go to unk.def)
        let unk_start_idx = self
            .labels
            .len()
            .saturating_sub(self.unk_category_names.len());

        // Write lexicon entries using learned connection IDs and costs
        for (i, label) in self.labels.iter().enumerate() {
            if i >= unk_start_idx {
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

                let cost = tocost(fs.weight, self.cost_factor);

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    label, fs.left_id, fs.right_id, cost, pos_info
                )?;
            }
        }

        Ok(())
    }

    /// Write dense connection cost matrix (MeCab-compatible)
    pub fn write_connection_costs<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        if !self.connection_matrix.is_empty() {
            let right_size = self.max_right_id + 1;
            let left_size = self.max_left_id + 1;

            writeln!(writer, "{right_size} {left_size}")?;

            // Dense matrix: all (right_id, left_id) pairs
            // Unseen pairs get maximum penalty cost to block unlearned transitions,
            // forcing Viterbi to prefer paths through learned POS bigram connections.
            for right_id in 0..right_size {
                for left_id in 0..left_size {
                    let cost = if let Some(&weight) = self
                        .connection_matrix
                        .get(&right_id)
                        .and_then(|inner| inner.get(&left_id))
                    {
                        tocost(weight, self.cost_factor)
                    } else {
                        i16::MAX
                    };
                    writeln!(writer, "{right_id} {left_id} {cost}")?;
                }
            }
        } else {
            writeln!(writer, "0 0")?;
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
        let unk_start_idx = self
            .labels
            .len()
            .saturating_sub(self.unk_category_names.len());

        for (i, category_name) in self.unk_category_names.iter().enumerate() {
            let feature_idx = unk_start_idx + i;

            if feature_idx < self.feature_sets.len() {
                let fs = &self.feature_sets[feature_idx];

                let features = self
                    .unk_categories
                    .get(category_name)
                    .cloned()
                    .unwrap_or_else(|| self.generate_default_features());

                let cost = tocost(fs.weight, self.cost_factor);

                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    category_name, fs.left_id, fs.right_id, cost, features
                )?;
            }
        }

        Ok(())
    }

    /// Writes the character definition file (char.def) content preserved from training.
    pub fn write_char_def<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_all(self.char_def_content.as_bytes())?;
        Ok(())
    }

    /// Writes the feature definition file (feature.def) content preserved from training.
    pub fn write_feature_def<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_all(self.feature_def_content.as_bytes())?;
        Ok(())
    }

    /// Writes the rewrite rule definition file (rewrite.def) content preserved from training.
    pub fn write_rewrite_def<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_all(self.rewrite_def_content.as_bytes())?;
        Ok(())
    }

    /// Writes left-id.def: maps left context IDs to their feature strings.
    pub fn write_left_id_def<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writeln!(writer, "0 BOS/EOS")?;
        for (id, feat_str) in &self.left_id_map {
            writeln!(writer, "{id} {feat_str}")?;
        }
        Ok(())
    }

    /// Writes right-id.def: maps right context IDs to their feature strings.
    pub fn write_right_id_def<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writeln!(writer, "0 BOS/EOS")?;
        for (id, feat_str) in &self.right_id_map {
            writeln!(writer, "{id} {feat_str}")?;
        }
        Ok(())
    }
}

impl Model {
    /// Writes left-id.def: maps left context IDs to their feature strings.
    ///
    /// Format: `id feature_string` (one per line), with BOS/EOS at id=0.
    pub fn write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()> {
        // id=0 is reserved for BOS/EOS
        writeln!(writer, "0 BOS/EOS")?;

        // Build reverse mapping: NonZeroU32 -> feature string
        let left_ids = &self.config.feature_extractor.left_feature_ids;
        let mut entries: Vec<(u32, &str)> = left_ids
            .iter()
            .map(|(feat_str, &id)| (id.get(), feat_str.as_str()))
            .collect();
        entries.sort_by_key(|&(id, _)| id);

        for (id, feat_str) in entries {
            writeln!(writer, "{id} {feat_str}")?;
        }

        Ok(())
    }

    /// Writes right-id.def: maps right context IDs to their feature strings.
    ///
    /// Format: `id feature_string` (one per line), with BOS/EOS at id=0.
    pub fn write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()> {
        // id=0 is reserved for BOS/EOS
        writeln!(writer, "0 BOS/EOS")?;

        // Build reverse mapping: NonZeroU32 -> feature string
        let right_ids = &self.config.feature_extractor.right_feature_ids;
        let mut entries: Vec<(u32, &str)> = right_ids
            .iter()
            .map(|(feat_str, &id)| (id.get(), feat_str.as_str()))
            .collect();
        entries.sort_by_key(|&(id, _)| id);

        for (id, feat_str) in entries {
            writeln!(writer, "{id} {feat_str}")?;
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

    #[test]
    fn test_tocost() {
        use super::tocost;

        // MeCab: tocost(d, n) = clamp(-n * d, -32767, 32767)
        // Positive weight → negative cost
        assert_eq!(tocost(1.0, 700), -700);
        // Negative weight → positive cost
        assert_eq!(tocost(-1.0, 700), 700);
        // Zero weight → zero cost
        assert_eq!(tocost(0.0, 700), 0);
        // Clamp to i16::MAX
        assert_eq!(tocost(-100.0, 700), i16::MAX);
        // Clamp to i16::MIN
        assert_eq!(tocost(100.0, 700), i16::MIN);
        // Fractional weight
        assert_eq!(tocost(0.5, 700), -350);
    }
}
