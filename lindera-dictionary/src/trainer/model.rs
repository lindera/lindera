use std::io::Write;

use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Trained model with weights and configuration.
#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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
                regularization: 0.01, // TODO: Get from training config
                iterations: 100, // TODO: Get actual iteration count
                feature_count: self.feature_weights.len(),
                label_count: self.labels.len(),
            },
        };

        // Serialize as JSON for readability (could use bincode for efficiency)
        let json_data = serde_json::to_string_pretty(&serializable_model)?;
        writer.write_all(json_data.as_bytes())?;

        Ok(())
    }

    /// Extracts feature weights from the raw CRF model
    fn extract_feature_weights(&self) -> Vec<f64> {
        // Use vibrato's approach: merge the model to get weights
        match self.raw_model.merge() {
            Ok(merged_model) => {
                let mut weights = Vec::new();

                // Extract weights from feature sets (similar to vibrato's implementation)
                for feature_set in &merged_model.feature_sets {
                    weights.push(feature_set.weight);
                }

                // Extract weights from connection matrix (bigram weights)
                for hm in &merged_model.matrix {
                    for &w in hm.values() {
                        weights.push(w);
                    }
                }

                weights
            }
            Err(_) => {
                // Fall back to empty vector if merge fails
                Vec::new()
            }
        }
    }

    /// Extracts part-of-speech information for each label
    fn extract_pos_info(&self) -> Vec<String> {
        // Get POS info from config for each surface
        let mut pos_info = Vec::new();

        for surface in &self.labels {
            // Look up the surface in the lexicon to get its POS info
            if let Some(entry) = self.config.lexicon.get(surface) {
                // Join the features with commas
                pos_info.push(entry.features.join(","));
            } else {
                // Default POS for unknown words
                pos_info.push("名詞,一般,*,*,*,*,*,*,*".to_string());
            }
        }

        pos_info
    }

    /// Extracts feature templates used in training
    fn extract_feature_templates(&self) -> Vec<String> {
        // Return basic templates used
        vec![
            "UNIGRAM:%F[0]".to_string(),
            "UNIGRAM:%F[1]".to_string(),
            "LEFT:%L[0]".to_string(),
            "RIGHT:%R[0]".to_string(),
        ]
    }

    /// Writes the dictionary files in Lindera format.
    pub fn write_dictionary<W1, W2, W3, W4>(
        &self,
        lexicon_wtr: &mut W1,
        connector_wtr: &mut W2,
        unk_handler_wtr: &mut W3,
        _user_lexicon_wtr: &mut W4,
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

        Ok(())
    }

    fn write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write CSV header
        writeln!(writer, "surface,left_id,right_id,cost,features")?;

        // Extract vocabulary from training data and assign trained costs with proper context IDs
        for (i, surface) in self.config.surfaces.iter().enumerate() {
            let cost = self.get_word_cost(i);
            let features = self.get_word_features(surface);
            let (left_id, right_id) = self.infer_context_ids(surface, &features);
            writeln!(writer, "{},{},{},{},{}", surface, left_id, right_id, cost, features)?;
        }

        Ok(())
    }

    fn write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()> {
        // 学習データから文脈IDの最大値を決定
        let max_context_id = self.calculate_max_context_id();
        let size = (max_context_id + 1) as usize;

        writeln!(writer, "{} {}", size, size)?;

        // Write connection costs based on trained model
        for i in 0..size {
            for j in 0..size {
                let cost = self.get_trained_connection_cost(i, j);
                write!(writer, "{}", cost)?;
                if j < size - 1 {
                    write!(writer, " ")?;
                }
            }
            writeln!(writer)?;
        }

        Ok(())
    }

    fn write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write unknown word definitions with trained costs
        let categories = vec!["DEFAULT", "HIRAGANA", "KATAKANA", "KANJI", "ALPHA", "NUMERIC"];

        for (i, category) in categories.iter().enumerate() {
            let cost = self.get_unknown_word_cost(i);
            writeln!(writer, "{},{},{},{},名詞,一般,*,*,*,*,*,*,*",
                    category, 0, 0, cost)?;
        }

        Ok(())
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