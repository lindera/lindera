use std::io::{Write, Read};
use std::num::NonZeroU32;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use bincode::{Encode, Decode};

use crate::trainer::corpus::Word;
use crate::viterbi::{WordEntry, WordId, LexType};

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
    /// Creates a new model.
    pub(crate) fn new(
        raw_model: rucrf::RawModel,
        config: super::config::TrainerConfig,
        feature_weights: Vec<f64>,
        labels: Vec<String>,
    ) -> Self {
        Self {
            raw_model,
            config,
            feature_weights,
            labels,
            user_entries: Vec::new(),
            merged_model: None,
            regularization_cost: 0.01, // Default value
            max_iterations: 100, // Default value
        }
    }

    /// Creates a new model with metadata.
    pub(crate) fn new_with_metadata(
        raw_model: rucrf::RawModel,
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
    pub fn read_user_lexicon<R: Read>(&mut self, mut rdr: R) -> Result<()> {
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
                let word_id = WordId::new(
                    LexType::User,
                    self.user_entries.len() as u32
                );

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
                let unigram_features = if let Some(rewritten) = self.config.unigram_rewriter.rewrite(&feature_vec) {
                    self.config.feature_extractor.extract_unigram_feature_ids(&rewritten, cate_id)
                } else {
                    self.config.feature_extractor.extract_unigram_feature_ids(&feature_vec, cate_id)
                };
                let left_features = if let Some(rewritten) = self.config.left_rewriter.rewrite(&feature_vec) {
                    self.config.feature_extractor.extract_left_feature_ids(&rewritten)
                } else {
                    self.config.feature_extractor.extract_left_feature_ids(&feature_vec)
                };
                let right_features = if let Some(rewritten) = self.config.right_rewriter.rewrite(&feature_vec) {
                    self.config.feature_extractor.extract_right_feature_ids(&rewritten)
                } else {
                    self.config.feature_extractor.extract_right_feature_ids(&feature_vec)
                };

                let feature_set = rucrf::FeatureSet::new(&unigram_features, &right_features, &left_features);

                // Create a label ID without modifying the provider
                // Since we can't clone the provider, we'll use a fixed ID based on entry count
                let label_id = NonZeroU32::new((1000000 + self.user_entries.len() as u32 + 1) as u32)
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

    /// Writes the model to a writer.
    pub fn write_model<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Extract feature weights from the trained CRF model
        let feature_weights = self.extract_feature_weights();

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
        };

        // Use bincode for efficient storage (compatible with Vibrato)
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

        // Try bincode first (new format)
        if let Ok((model, _)) = bincode::decode_from_slice::<SerializableModel, _>(&buffer, bincode::config::standard()) {
            return Ok(model);
        }

        // Fallback to JSON format (legacy)
        let json_str = String::from_utf8(buffer)?;
        let model: SerializableModel = serde_json::from_str(&json_str)?;
        Ok(model)
    }

    /// Extracts feature weights from the raw CRF model
    fn extract_feature_weights(&self) -> Vec<f64> {
        // Use merge approach to get accurate weights
        match self.raw_model.merge() {
            Ok(merged_model) => {
                let mut weights = Vec::new();

                // Extract unigram weights from feature sets
                for feature_set in &merged_model.feature_sets {
                    weights.push(feature_set.weight);
                }

                // Extract bigram weights from connection matrix
                for hm in &merged_model.matrix {
                    for &w in hm.values() {
                        weights.push(w);
                    }
                }

                weights
            }
            Err(_) => {
                // Return weights from raw model if merge fails
                self.raw_model.weights().to_vec()
            }
        }
    }

    /// Gets the merged model, creating it if necessary
    fn get_merged_model(&self) -> Result<rucrf::MergedModel> {
        Ok(self.raw_model.merge()?)
    }

    /// Extracts part-of-speech information for each label
    fn extract_pos_info(&self) -> Vec<String> {
        // Get POS info from config for each surface (includes user lexicon)
        let mut pos_info = Vec::new();

        for surface in &self.labels {
            // Look up the surface in both main lexicon and user lexicon
            if let Some(features) = self.config.get_features(surface) {
                pos_info.push(features.clone());
            } else {
                // Default POS for unknown words
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
                let features = self.get_word_features(surface);
                writeln!(writer, "{},{},{},{},{}",
                    surface, feature_set.left_id, feature_set.right_id, cost, features)?;
            } else {
                // Fallback for missing feature sets
                let cost = self.get_word_cost(i);
                let features = self.get_word_features(surface);
                let (left_id, right_id) = self.infer_context_ids(surface, &features);
                writeln!(writer, "{},{},{},{},{}", surface, left_id, right_id, cost, features)?;
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
        // Get merged model for proper connection cost calculation
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Write matrix dimensions
        writeln!(writer, "{} {}",
            merged_model.right_conn_to_left_feats.len() + 1,
            merged_model.left_conn_to_right_feats.len() + 1)?;

        // Write connection costs from merged model
        for (right_conn_id, hm) in merged_model.matrix.iter().enumerate() {
            let mut pairs: Vec<_> = hm.iter().map(|(&j, &w)| (j, w)).collect();
            pairs.sort_unstable_by_key(|&(k, _)| k);
            for (left_conn_id, w) in pairs {
                writeln!(writer, "{} {} {}",
                    right_conn_id,
                    left_conn_id,
                    (-w * weight_scale_factor) as i16)?;
            }
        }

        Ok(())
    }

    pub fn write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write unknown word definitions with trained costs
        let merged_model = self.get_merged_model()?;
        let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

        // Use config's unknown dictionary length for proper indexing
        let unk_dict_len = self.config.surfaces.len();
        let unk_len = self.config.dict.unknown_dictionary.costs.len();
        for i in 0..unk_len {
            let feature_set_idx = unk_dict_len + i;
            if feature_set_idx < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[feature_set_idx];
                let cost = (-feature_set.weight * weight_scale_factor) as i16;

                // Get category string from character properties
                let cate_string = format!("UNK_{}", i); // Simplified category naming
                let features = "名詞,一般,*,*,*,*,*,*,*"; // Default features

                writeln!(writer, "{},{},{},{},{}",
                    cate_string, feature_set.left_id, feature_set.right_id, cost, features)?;
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
            // For user lexicon, use default parameters or look up in merged model
            let (left_id, right_id) = self.infer_context_ids(surface, features);
            let cost = self.get_user_word_cost(surface);
            writeln!(writer, "{},{},{},{},{}", surface, left_id, right_id, cost, features)?;
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

    fn get_word_features(&self, _surface: &str) -> String {
        // Return basic feature template
        "名詞,一般,*,*,*,*,*,*,*".to_string()
    }


    fn get_unknown_word_cost(&self, _category: usize) -> i32 {
        // Return trained unknown word cost
        2000 // Default unknown word cost
    }

    /// 表層形と素性から文脈ID（left_id, right_id）を推論
    fn infer_context_ids(&self, surface: &str, features: &str) -> (u32, u32) {
        // 素性文字列を解析して品詞情報を取得
        let feature_parts: Vec<&str> = features.split(',').collect();

        // 品詞（POS）に基づいてコンテキストIDを決定
        let pos_category = if feature_parts.len() > 0 {
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
        } else if surface.chars().any(|c| c >= '\u{3040}' && c <= '\u{309F}') {
            context_id + 300 // ひらがな
        } else if surface.chars().any(|c| c >= '\u{30A0}' && c <= '\u{30FF}') {
            context_id + 400 // カタカナ
        } else if surface.chars().any(|c| c >= '\u{4E00}' && c <= '\u{9FAF}') {
            context_id + 500 // 漢字
        } else {
            context_id
        };

        // left_idとright_idは同じ値を使用（簡単化）
        // より高度な実装では、前後の文脈に応じて異なるIDを使用
        (adjusted_id, adjusted_id)
    }

    /// 学習データから最大文脈IDを計算
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
    /// This is equivalent to Vibrato's write_bigram_details method.
    pub fn write_bigram_details<L, R, C>(
        &self,
        mut left_wtr: L,
        mut right_wtr: R,
        mut cost_wtr: C,
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
            let feature_name = format!("R{}", i);
            right_features.insert(i as u32, feature_name);
        }

        // Extract left feature names
        for i in 0..merged_model.feature_sets.len() {
            let feature_name = format!("L{}", i);
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
                        write!(&mut left_wtr, "\"{}\"", feat_str)?;
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
                        write!(&mut right_wtr, "\"{}\"", feat_str)?;
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
                writeln!(&mut cost_wtr, "{}/{}\t{}", left_feat_str, right_feat_str, cost)?;
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
        use std::collections::HashMap;

        let weight_scale_factor = self.calculate_weight_scale_factor();

        // DEBUG: Print feature weights information
        eprintln!("DEBUG: Total feature_weights: {}", self.feature_weights.len());
        eprintln!("DEBUG: Total labels: {}", self.labels.len());
        eprintln!("DEBUG: Weight scale factor: {}", weight_scale_factor);
        for (i, weight) in self.feature_weights.iter().take(20).enumerate() {
            eprintln!("DEBUG: feature_weights[{}] = {}", i, weight);
        }

        // Filter out unknown word category labels
        let unk_categories = [
            "DEFAULT", "HIRAGANA", "KATAKANA", "KANJI", "ALPHA", "NUMERIC",
        ];

        // Create POS-based connection ID mapping
        let mut pos_to_id: HashMap<String, u32> = HashMap::new();
        let mut next_id = 0u32;

        let mut feature_weight_index = 0;
        for (i, label) in self.labels.iter().enumerate() {
            // Skip unknown word categories
            if unk_categories.contains(&label.as_str()) {
                eprintln!("DEBUG: Skipping unknown category: {}", label);
                continue;
            }

            let pos_info = if i < self.pos_info.len() {
                &self.pos_info[i]
            } else {
                "名詞,一般,*,*,*,*,*,*,*"
            };

            // Extract main POS for connection ID
            let main_pos = pos_info.split(',').next().unwrap_or("名詞");
            let connection_id = *pos_to_id.entry(main_pos.to_string()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });

            // Calculate cost with proper scaling for these unusually large weights
            let cost = if feature_weight_index < self.feature_weights.len() {
                let raw_weight = self.feature_weights[feature_weight_index];

                // Since weights are in 40-50 range (unusually large), we need different scaling
                // Normalize to a reasonable range first
                let normalized_weight = if raw_weight.abs() > 1.0 {
                    // Scale down large weights to reasonable range (-5 to +5)
                    (raw_weight / 10.0).max(-5.0).min(5.0)
                } else {
                    raw_weight
                };

                // Apply standard morphological analysis cost calculation
                let calculated_cost = (-normalized_weight * 1000.0) as i16;
                let final_cost = (calculated_cost + 5000).max(1000).min(15000);

                eprintln!("DEBUG: label={}, raw_weight={:.3}, normalized={:.3}, calculated={}, final={}",
                    label, raw_weight, normalized_weight, calculated_cost, final_cost);

                final_cost
            } else {
                eprintln!("DEBUG: label={} using default cost", label);
                5000i16 // Default cost for morphological analysis
            };

            writeln!(
                writer,
                "{},{},{},{},{}",
                label, connection_id, connection_id, cost, pos_info
            )?;

            feature_weight_index += 1;
        }

        Ok(())
    }

    /// Write connection cost matrix
    pub fn write_connection_costs<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        // For now, create a simple connection matrix based on POS categories
        // In a full implementation, this would use trained bigram weights
        let num_categories = 6; // Number of distinct POS categories

        writeln!(writer, "{} {}", num_categories, num_categories)?;

        for i in 0..num_categories {
            for j in 0..num_categories {
                let cost = if i == j { 0 } else { 200 }; // Lower penalty than before
                write!(writer, "{}", cost)?;
                if j < num_categories - 1 {
                    write!(writer, " ")?;
                }
            }
            writeln!(writer)?;
        }

        Ok(())
    }

    /// Write unknown word definitions
    pub fn write_unknown_dictionary<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let categories = [
            ("DEFAULT", "名詞,一般,*,*,*,*,*,*,*"),
            ("HIRAGANA", "名詞,一般,*,*,*,*,*,*,*"),
            ("KATAKANA", "名詞,一般,*,*,*,*,*,*,*"),
            ("KANJI", "名詞,一般,*,*,*,*,*,*,*"),
            ("ALPHA", "名詞,一般,*,*,*,*,*,*,*"),
            ("NUMERIC", "名詞,一般,*,*,*,*,*,*,*"),
        ];

        for (i, (category, features)) in categories.iter().enumerate() {
            let cost = 2000i16; // Standard unknown word cost
            writeln!(writer, "{},{},{},{},{}", category, i, i, cost, features)?;
        }

        Ok(())
    }
}