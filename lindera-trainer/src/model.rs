use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::num::NonZeroU32;

use anyhow::Result;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::corpus::Word;
use crate::feature_csv::{split_feature, wildcard_feature};
use crate::lexicon_csv::{csv_field, read_lexicon_rows};
use lindera_dictionary::viterbi::{LexType, WordEntry, WordId};

/// The `metadata.version` this build writes into `model.dat` and requires
/// when reading one. Bumped whenever the layout or the meaning of
/// [`SerializableModel`] changes; 2.0.0 keeps every `unk.def` line as its own
/// entry (`SerializableModel::unk_entries`, #1110).
pub const MODEL_FORMAT_VERSION: &str = "2.0.0";

/// Checks that a model was written in the format this build reads.
///
/// # Arguments
///
/// * `version` - The model's `metadata.version`, or `None` when it has none.
///
/// # Returns
///
/// `Ok(())` when the version is [`MODEL_FORMAT_VERSION`], or an error that
/// tells the user to re-train.
fn check_format_version(version: Option<&str>) -> Result<()> {
    if version == Some(MODEL_FORMAT_VERSION) {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "model format version {} is not {MODEL_FORMAT_VERSION}, so the model was written by \
         a different version of lindera-trainer; re-run `lindera train` to regenerate it.",
        version.unwrap_or("(none)")
    ))
}

/// Convert CRF weight to MeCab-compatible cost using cost-factor.
///
/// MeCab's `tocost(d, n)` formula: `clamp(-n * d, -32767, 32767)`
/// where `d` is the cost-factor and `n` is the CRF weight.
fn tocost(weight: f64, cost_factor: i32) -> i16 {
    let raw = -(cost_factor as f64) * weight;
    raw.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

/// Returns whether `bytes` begins a JSON object, ignoring leading whitespace.
///
/// Used only to decide whether a payload that rkyv already rejected is worth
/// handing to the legacy JSON reader. rkyv is always tried first, so a false
/// positive here cannot misroute a valid model.
///
/// # Arguments
///
/// * `bytes` - Raw model file contents.
///
/// # Returns
///
/// `true` when the first non-whitespace byte is `{`.
fn looks_like_json_object(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| *byte == b'{')
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

/// Returns the seed row whose feature is the most similar to a user lexicon
/// feature, for `Model::infer_context_ids`.
///
/// Each column that is the same in both features, up to the shorter of the
/// two, adds `(n - j) * 10` to the score of a row, where `n` is that length
/// and `j` the column, so the earlier columns, such as the part of speech,
/// weigh more. The features are split with `split_feature`, so the columns
/// after a quoted column with `,` in it line up (#1184). The first row with
/// the highest score wins.
///
/// # Arguments
///
/// * `user_feature` - The user lexicon entry's feature.
/// * `seed_features` - The features of the seed rows.
///
/// # Returns
///
/// The index of the row and its score, or `None` when no row has a column
/// in common with `user_feature`.
fn best_matching_seed_row(user_feature: &str, seed_features: &[String]) -> Option<(usize, usize)> {
    let user_columns = split_feature(user_feature);
    let mut best: Option<(usize, usize)> = None;
    for (index, seed_feature) in seed_features.iter().enumerate() {
        let seed_columns = split_feature(seed_feature);
        let max_fields = user_columns.len().min(seed_columns.len());
        let score = user_columns
            .iter()
            .zip(&seed_columns)
            .enumerate()
            .filter(|(_, (user, seed))| user == seed)
            .map(|(j, _)| (max_fields - j) * 10)
            .sum();
        if score > best.map_or(0, |(_, best_score)| best_score) {
            best = Some((index, score));
        }
    }
    best
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
    /// Connection cost matrix: (right_id, left_id) -> cost.
    ///
    /// Ordered maps rather than hashed ones, at both levels. rkyv archives a
    /// `HashMap` as a SwissTable whose slot assignment resolves collisions in
    /// source iteration order, and `HashMap` iteration order is seeded per
    /// instance -- so serializing the same trained model twice produced files
    /// of identical length with scattered byte differences (#974). A
    /// `BTreeMap` serializes from its key-ordered iterator into a tree whose
    /// shape depends only on the entry count, which makes `model.dat`
    /// byte-reproducible. No stored value changes: both levels are only ever
    /// filled key-by-key from `Vec`-ordered sources.
    pub connection_matrix: BTreeMap<usize, BTreeMap<usize, f64>>,
    /// Highest left context id of the model, whether or not a weight survived
    /// for it; `matrix.def` has `max_left_id + 1` columns.
    pub max_left_id: usize,
    /// Highest right context id of the model, whether or not a weight
    /// survived for it; `matrix.def` has `max_right_id + 1` rows.
    pub max_right_id: usize,
    /// Feature set information (left_id, right_id, weight) for each label
    pub feature_sets: Vec<FeatureSetInfo>,
    /// The `unk.def` entries the model was trained with, in file order, as
    /// `(category, feature string)` pairs: every line, several per category
    /// included (#1110). Entry `i` is label
    /// `labels.len() - unk_entries.len() + i` and is written to `unk.def` as
    /// line `i`.
    ///
    /// A `Vec`, which serializes in its own order, so `model.dat` stays
    /// byte-reproducible without the ordered map the per-category field
    /// needed (#974).
    pub unk_entries: Vec<(String, String)>,
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
    /// The feature strings of the virtual labels, in label order. They
    /// follow the exported labels in the CRF model (see
    /// `Trainer::virtual_label`) and are never exported themselves; they only
    /// name context ids in `left-id.def` and `right-id.def`.
    pub(crate) virtual_features: Vec<String>,
    pub(crate) user_entries: Vec<(Word, WordEntry, NonZeroU32)>,
    pub(crate) merged_model: Option<lindera_crf::MergedModel>,
    pub(crate) regularization_cost: f64,
    pub(crate) max_iterations: u64,
}

impl Model {
    /// Creates a new model with metadata.
    ///
    /// # Arguments
    ///
    /// * `raw_model` - The trained CRF model.
    /// * `config` - The training configuration.
    /// * `feature_weights` - The weight of every seed row.
    /// * `labels` - The exported labels: the seed surfaces, then the category
    ///   of every `unk.def` entry.
    /// * `virtual_features` - The feature strings of the virtual labels,
    ///   which follow the exported labels in `raw_model`.
    /// * `regularization_cost` - The regularization coefficient used.
    /// * `max_iterations` - The iteration limit used.
    ///
    /// # Returns
    ///
    /// The model.
    pub(crate) fn new_with_metadata(
        raw_model: lindera_crf::RawModel,
        config: super::config::TrainerConfig,
        feature_weights: Vec<f64>,
        labels: Vec<String>,
        virtual_features: Vec<String>,
        regularization_cost: f64,
        max_iterations: u64,
    ) -> Self {
        Self {
            raw_model,
            config,
            feature_weights,
            labels,
            virtual_features,
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
    /// [`write_dictionary`](Self::write_dictionary) gives an entry with
    /// `0,0,0` the context ids that `lex.csv` gives the seed row whose
    /// feature is the most similar to the entry's (#1190). Each column that
    /// is the same in both features, at the same position and up to the
    /// shorter of the two, scores `(n - j) * 10`, where `n` is that length and
    /// `j` the column, so the earlier columns, such as the part of speech,
    /// weigh more; the first row with the highest score wins. An entry whose
    /// feature has no column in common with any seed row makes
    /// `write_dictionary` fail, as `mecab-dict-index` fails for a feature
    /// without a context id; give its parameters instead.
    ///
    /// The file is read as the seed lexicon is (see
    /// [`TrainerConfig::from_readers`](crate::config::TrainerConfig::from_readers)):
    /// as CSV records, so a field may be quoted, with no comment lines, and
    /// with the feature kept as written (#1170). Records with fewer than five
    /// fields are skipped.
    ///
    /// # Arguments
    ///
    /// * `rdr` - Read sink of the user-defined lexicon file.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when the file cannot be read or a quoted field
    /// spans lines; no entry is added then.
    pub fn read_user_lexicon<R: Read>(&mut self, mut rdr: R) -> Result<()> {
        self.merged_model = None;
        let mut content = String::new();
        rdr.read_to_string(&mut content)?;

        for row in read_lexicon_rows(&content, "user lexicon")? {
            let surface = row.surface.as_str();
            let left_id = row.left_id.parse::<i32>().unwrap_or(0);
            let right_id = row.right_id.parse::<i32>().unwrap_or(0);
            let cost = row.cost.parse::<i16>().unwrap_or(0);
            let features = row.feature.as_str();

            let word = Word::new(surface, features);

            // Create word ID for user dictionary entry
            let word_id = WordId::new(LexType::User, self.user_entries.len() as u32);

            let entry = WordEntry::new(word_id, cost, left_id as u16, right_id as u16);

            // Extract features and create feature set for this user entry
            let first_char = surface.chars().next().unwrap_or('\0');
            let cate_id = self.get_category_id(first_char);

            // Apply dictionary rewriter to get ufeature, lfeature, rfeature
            let (ufeature, lfeature, rfeature) = self.config.dictionary_rewriter.rewrite(features);
            // Split as MeCab splits a feature, so a quoted column with `,` in
            // it is one column (#1184).
            let (u_columns, l_columns, r_columns) = (
                split_feature(&ufeature),
                split_feature(&lfeature),
                split_feature(&rfeature),
            );
            let u_vec: Vec<&str> = u_columns.iter().map(|column| column.as_ref()).collect();
            let l_vec: Vec<&str> = l_columns.iter().map(|column| column.as_ref()).collect();
            let r_vec: Vec<&str> = r_columns.iter().map(|column| column.as_ref()).collect();

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

            let _feature_set =
                lindera_crf::FeatureSet::new(&unigram_features, &right_features, &left_features);
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

        Ok(())
    }

    /// Returns the `%t` feature value of a character.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character, usually the first character of a surface.
    ///
    /// # Returns
    ///
    /// The default category of `ch` from `char.def` (MeCab's `default_type`),
    /// or 0 (DEFAULT) if it has none.
    fn get_category_id(&self, ch: char) -> u32 {
        self.config.default_category(ch).unwrap_or(0)
    }

    /// Writes the model to a writer.
    pub fn write_model<W: Write>(&self, writer: &mut W) -> Result<()> {
        let feature_weights = self.feature_weights.clone();

        // Extract connection cost matrix from the trained model
        let merged_model = self.raw_model.merge()?;

        // Compute optimal cost factor from actual weight range
        let cost_factor = calculate_cost_factor(&merged_model);

        // The matrix must cover every context id the merge assigned, whether
        // or not a weight survived for it: the merge numbers the ids densely
        // from 1 (0 is BOS/EOS), and every lexicon and unknown entry carries
        // one. Counting only the cells that kept a weight shrank the left
        // axis, and to nothing when regularization zeroed every bigram
        // weight, so the built dictionary indexed past its matrix.
        let max_right_id = merged_model.right_conn_to_left_feats.len();
        let max_left_id = merged_model.left_conn_to_right_feats.len();

        let mut connection_matrix: BTreeMap<usize, BTreeMap<usize, f64>> = BTreeMap::new();
        for (right_id, left_map) in merged_model.matrix.iter().enumerate() {
            let inner_map: BTreeMap<usize, f64> = left_map
                .iter()
                .map(|(&left_id, &weight)| (left_id as usize, weight))
                .collect();

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

        // Build left/right ID maps from merged model's connection IDs
        // before feature_sets is moved into SerializableModel.
        // Use feature strings (POS info), not surface forms. The labels are
        // the seed rows, then the unk.def entries, then the virtual labels.
        let features = &self.config.features;
        let unk_entries = &self.config.unk_entries;
        let unk_start_idx = self.config.surfaces.len();
        let virtual_start_idx = unk_start_idx + unk_entries.len();

        let get_feature_string = |i: usize| -> String {
            if i < unk_start_idx {
                features.get(i).cloned().unwrap_or_else(|| "*".to_string())
            } else if i < virtual_start_idx {
                unk_entries[i - unk_start_idx].1.clone()
            } else {
                self.virtual_features
                    .get(i - virtual_start_idx)
                    .cloned()
                    .unwrap_or_else(|| "*".to_string())
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
                version: MODEL_FORMAT_VERSION.to_string(),
                regularization: self.regularization_cost,
                iterations: self.max_iterations,
                feature_count: self.feature_weights.len(),
                label_count: self.labels.len(),
            },
            connection_matrix,
            max_left_id,
            max_right_id,
            feature_sets,
            unk_entries: self.config.unk_entries.clone(),
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
    /// Accepts the current rkyv binary format and, for backward
    /// compatibility, the legacy JSON format.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader containing the serialized model data
    ///
    /// # Returns
    ///
    /// The deserialized model.
    ///
    /// # Errors
    ///
    /// Returns an error naming the format that failed. A model written by a
    /// build with a different `SerializableModel` layout is reported as such,
    /// with the instruction to re-train -- it used to fall through to the
    /// legacy JSON reader and surface as an unrelated UTF-8 error (#974). So
    /// is a model, in either format, whose `metadata.version` is not
    /// [`MODEL_FORMAT_VERSION`] (#1110).
    pub fn read_model<R: Read>(mut reader: R) -> Result<SerializableModel> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        if buffer.is_empty() {
            return Err(anyhow::anyhow!(
                "model file is empty; re-run `lindera train` to produce one"
            ));
        }

        // Current format: rkyv. Tried first because it is self-validating, so
        // a successful parse proves the on-disk layout matched this build.
        let rkyv_error = match rkyv::from_bytes::<SerializableModel, rkyv::rancor::Error>(&buffer) {
            Ok(mut model) => {
                check_format_version(Some(&model.metadata.version))?;
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
            Err(err) => err,
        };

        // Legacy format: JSON, attempted only when the payload actually looks
        // like JSON. Routing on the leading byte as the *primary* check would
        // be wrong -- a valid rkyv payload starts with the low byte of
        // `feature_weights[0]` as a little-endian f64, which is `{` about one
        // time in 256.
        if looks_like_json_object(&buffer) {
            let json_str = String::from_utf8(buffer).map_err(|err| {
                anyhow::anyhow!("model file starts like legacy JSON but is not valid UTF-8: {err}")
            })?;
            let value: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|err| anyhow::anyhow!("failed to parse legacy JSON model: {err}"))?;
            // Checked before the fields are read, so that a model of another
            // format version gets the instruction to re-train rather than a
            // missing-field error.
            check_format_version(
                value
                    .get("metadata")
                    .and_then(|metadata| metadata.get("version"))
                    .and_then(serde_json::Value::as_str),
            )?;
            return serde_json::from_value(value)
                .map_err(|err| anyhow::anyhow!("failed to parse legacy JSON model: {err}"));
        }

        Err(anyhow::anyhow!(
            "failed to deserialize model: {rkyv_error}. The file is not a legacy JSON \
             model either, so it was most likely written by a different version of \
             lindera-trainer; re-run `lindera train` to regenerate it."
        ))
    }

    /// Gets the merged model, creating it if necessary
    fn get_merged_model(&self) -> Result<lindera_crf::MergedModel> {
        Ok(self.raw_model.merge()?)
    }

    /// Generate a default feature string with appropriate number of fields
    /// based on existing entries in the dictionary
    fn generate_default_features(&self) -> String {
        // Try to infer field count from the unk.def entries
        if let Some((_, first_unk)) = self.config.unk_entries.first() {
            return wildcard_feature(first_unk);
        }

        // Fallback: try from config.features
        if let Some(first_feature) = self.config.features.first() {
            return wildcard_feature(first_feature);
        }

        // Ultimate fallback (should rarely happen)
        "*".to_string()
    }

    /// Extracts part-of-speech information for each label
    fn extract_pos_info(&self) -> Vec<String> {
        // Get POS info from config.features (parallel to surfaces/labels)
        let mut pos_info = Vec::new();

        for i in 0..self.labels.len() {
            // First check if this is within the vocabulary (config.features range)
            if i < self.config.features.len() {
                pos_info.push(self.config.features[i].clone());
            } else {
                // An unk.def entry: its own feature string, by entry index
                let unk_features = self
                    .config
                    .unk_entries
                    .get(i - self.config.features.len())
                    .map(|(_, feature)| feature.clone())
                    .unwrap_or_else(|| self.generate_default_features());
                pos_info.push(unk_features);
            }
        }

        pos_info
    }

    /// Writes the dictionary files in Lindera format: `lex.csv`,
    /// `matrix.def`, `unk.def` and the user lexicon that `read_user_lexicon`
    /// read.
    ///
    /// A user lexicon entry whose left id, right id and cost are all zero
    /// gets the context ids that `lex.csv` gives the seed row whose feature
    /// is the most similar to its own (#1190). The user lexicon is made
    /// before anything is written, so that nothing is written when the
    /// context ids of one of its entries cannot be inferred.
    ///
    /// # Arguments
    ///
    /// * `lexicon_wtr` - Write sink for `lex.csv`.
    /// * `connector_wtr` - Write sink for `matrix.def`.
    /// * `unk_handler_wtr` - Write sink for `unk.def`.
    /// * `user_lexicon_wtr` - Write sink for the user lexicon, which gets
    ///   nothing when no user lexicon was read.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error when a writer fails. It is also an
    /// error, and nothing is written, when a user lexicon entry whose left
    /// id, right id and cost are all zero has a feature with no column in
    /// common with any seed row, so that its context ids cannot be inferred.
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
        // Make the user lexicon first, so that an entry whose context ids
        // cannot be inferred fails before anything is written.
        let mut user_lexicon = Vec::new();
        self.write_user_lexicon(&mut user_lexicon)?;

        // Write lexicon with trained weights
        self.write_lexicon(lexicon_wtr)?;

        // Write connection cost matrix with trained costs
        self.write_connection_costs(connector_wtr)?;

        // Write unknown word handler with trained parameters
        self.write_unknown_dictionary(unk_handler_wtr)?;

        // Write user lexicon with trained weights
        user_lexicon_wtr.write_all(&user_lexicon)?;

        Ok(())
    }

    /// Writes `lex.csv`: one row per seed lexicon row, in seed order, with
    /// its trained context ids and cost.
    ///
    /// A surface that contains `,`, `"` or a line break is quoted, with its
    /// quotes doubled, so that `lindera build` reads it back as it is; the
    /// feature is written as it was read (#1170).
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for `lex.csv`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        for (i, surface) in self.config.surfaces.iter().enumerate() {
            let surface = csv_field(surface);
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

    /// Writes `unk.def`: one line per `unk.def` entry the model was trained
    /// with, in file order, with the entry's category name and feature string
    /// and its trained context ids and cost (#1110).
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for `unk.def`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()> {
        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        let unk_start_idx = self.config.surfaces.len();

        for (i, (category_name, features)) in self.config.unk_entries.iter().enumerate() {
            let feature_set_idx = unk_start_idx + i;
            if feature_set_idx < merged_model.feature_sets.len() {
                let feature_set = merged_model.feature_sets[feature_set_idx];
                let cost = tocost(feature_set.weight, cost_factor);

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

    /// Writes the user lexicon with trained parameters.
    ///
    /// Entries are written in the order `read_user_lexicon` read them, so the
    /// output is byte-reproducible and diffable against the input file (#981).
    /// An entry whose left id, right id and cost are all zero receives the
    /// context ids of the seed row whose feature is the most similar to its
    /// own (see `infer_context_ids`) and a trained cost; any other entry is
    /// re-emitted with its input parameters unchanged, per the contract
    /// documented on `read_user_lexicon`. A surface that contains `,`, `"` or
    /// a line break is quoted, with its quotes doubled; the feature is written
    /// as it was read (#1170).
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for the user lexicon CSV.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success; writing nothing when no user lexicon was read.
    /// An error when the context ids of an entry cannot be inferred, after
    /// the entries before it have been written.
    fn write_user_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
        if self.user_entries.is_empty() {
            return Ok(());
        }

        let merged_model = self.get_merged_model()?;
        let cost_factor = calculate_cost_factor(&merged_model);

        for (word, entry, _label_id) in &self.user_entries {
            let surface = word.surface();
            let features = word.feature();
            let surface_field = csv_field(surface);
            if entry.left_id() == 0 && entry.right_id() == 0 && entry.word_cost() == 0 {
                let (left_id, right_id) =
                    self.infer_context_ids(&merged_model, surface, features)?;
                let raw_cost = self.get_user_word_cost(surface) as f64 / 1000.0;
                let cost = tocost(raw_cost, cost_factor);
                writeln!(
                    writer,
                    "{surface_field},{left_id},{right_id},{cost},{features}"
                )?;
            } else {
                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    surface_field,
                    entry.left_id(),
                    entry.right_id(),
                    entry.word_cost(),
                    features
                )?;
            }
        }

        Ok(())
    }

    fn get_user_word_cost(&self, _surface: &str) -> i32 {
        // Return trained cost for user lexicon words
        // Could be based on trained model weights
        800 // Slightly lower cost than default for user words
    }

    /// Returns the input cost of an unknown-word category.
    ///
    /// It reads the cost column of `unk.def`, not the trained weights; a
    /// category with several entries gives the cost of its last line.
    ///
    /// # Arguments
    ///
    /// * `category` - The category id, its index in `char.def`.
    ///
    /// # Returns
    ///
    /// The cost, or 2000 when the category or its cost is unknown.
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

    /// Returns the context ids of a user lexicon entry whose left id, right
    /// id and cost are all zero.
    ///
    /// They are the ids that `lex.csv` gives the seed row whose feature is
    /// the most similar to the entry's (see `best_matching_seed_row`): those
    /// of the row's label in the merged model. Before #1190 the index of
    /// that row was looked up in the user lexicon instead, so the entry got
    /// the ids of another user lexicon entry.
    ///
    /// An entry whose feature has no column in common with any seed row is
    /// an error, as MeCab's `mecab-dict-index` fails for a user dictionary
    /// entry whose rewritten feature is not in `left-id.def` or
    /// `right-id.def`. It used to get the index of its first character's
    /// category in `char.def` as both ids, which is not a context id.
    ///
    /// # Arguments
    ///
    /// * `merged_model` - The merged CRF model, whose feature sets are in
    ///   label order, the seed rows first.
    /// * `surface` - The entry's surface, for the error message.
    /// * `features` - The entry's feature.
    ///
    /// # Returns
    ///
    /// The left and right context ids, or an error that names the entry
    /// when no seed row has a column in common with its feature.
    fn infer_context_ids(
        &self,
        merged_model: &lindera_crf::MergedModel,
        surface: &str,
        features: &str,
    ) -> Result<(u32, u32)> {
        // The labels after the seed rows are the unk.def entries and the
        // virtual labels.
        let seed_rows = self.labels.len().min(self.config.features.len());
        let Some((index, _)) = best_matching_seed_row(features, &self.config.features[..seed_rows])
        else {
            return Err(anyhow::anyhow!(
                "user lexicon entry {surface:?} with the feature {features:?}: no seed row \
                 has a column in common with the feature, so its context ids cannot be \
                 inferred; give its left id, right id and cost"
            ));
        };
        let feature_set = merged_model
            .feature_sets
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("seed row {index} has no label in the merged model"))?;
        Ok((feature_set.left_id.get(), feature_set.right_id.get()))
    }

    /// Calculate maximum context ID from training data
    #[allow(dead_code)]
    fn calculate_max_context_id(&self) -> u32 {
        let mut max_id = 0u32;

        // Get maximum ID from all user_entries
        for (_, entry, _) in &self.user_entries {
            max_id = max_id.max(entry.left_id()).max(entry.right_id());
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
    /// - Left features: per left connection id, the quoted right-side (`%R`)
    ///   feature strings it groups, `*` for absent slots
    /// - Right features: per right connection id, the quoted left-side (`%L`)
    ///   feature strings it groups
    /// - Costs: one `{left}/{right}\t{cost}` line per raw bigram feature
    ///   pair, where id 0 renders as `BOS` / `EOS` and names come from the
    ///   feature extractor
    ///
    /// All three files are byte-reproducible for the same model: the loops
    /// run over vectors or key-sorted map entries, never over raw map
    /// iteration order (#981).
    ///
    /// # Arguments
    ///
    /// * `left_wtr` - Write sink for the left-feature file.
    /// * `right_wtr` - Write sink for the right-feature file.
    /// * `cost_wtr` - Write sink for the feature-pair cost file.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
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

        // Real feature names, inverted from the extractor's string-to-id maps
        // (the same inversion write_left_id_def uses): id -> feature string.
        let right_features: HashMap<u32, &str> = self
            .config
            .feature_extractor
            .right_feature_ids
            .iter()
            .map(|(s, &id)| (id.get(), s.as_str()))
            .collect();
        let left_features: HashMap<u32, &str> = self
            .config
            .feature_extractor
            .left_feature_ids
            .iter()
            .map(|(s, &id)| (id.get(), s.as_str()))
            .collect();

        // Write left features
        let mut left_wtr = BufWriter::new(left_wtr);
        for (conn_id, left_feat_ids) in merged_model.left_conn_to_right_feats.iter().enumerate() {
            write!(&mut left_wtr, "{}\t", conn_id + 1)?;
            for (i, feat_id) in left_feat_ids.iter().enumerate() {
                if i != 0 {
                    write!(&mut left_wtr, ",")?;
                }
                if let Some(feat_id) = feat_id {
                    let feat_str = right_features.get(&feat_id.get()).copied().unwrap_or("*");
                    write!(&mut left_wtr, "\"{feat_str}\"")?;
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
                    let feat_str = left_features.get(&feat_id.get()).copied().unwrap_or("*");
                    write!(&mut right_wtr, "\"{feat_str}\"")?;
                } else {
                    write!(&mut right_wtr, "*")?;
                }
            }
            writeln!(&mut right_wtr)?;
        }

        // Write bigram costs with feature pair names. The source is the raw
        // model's bigram weight table -- left bigram feature id (0 = BOS) to
        // right bigram feature id (0 = EOS) to weight index -- which is the
        // id space these name maps describe. The inner maps are key-sorted so
        // the output is byte-reproducible (#981).
        let mut cost_wtr = BufWriter::new(cost_wtr);
        for (left_feat_id, hm) in self.raw_model.bigram_weight_indices().iter().enumerate() {
            let left_feat_str = if left_feat_id == 0 {
                "BOS"
            } else {
                left_features
                    .get(&(left_feat_id as u32))
                    .copied()
                    .unwrap_or("*")
            };
            let mut entries: Vec<(u32, u32)> = hm.iter().map(|(&k, &v)| (k, v)).collect();
            entries.sort_unstable_by_key(|&(k, _)| k);
            for (right_feat_id, weight_index) in entries {
                let right_feat_str = if right_feat_id == 0 {
                    "EOS"
                } else {
                    right_features.get(&right_feat_id).copied().unwrap_or("*")
                };
                let weight = self
                    .raw_model
                    .weights()
                    .get(weight_index as usize)
                    .copied()
                    .ok_or_else(|| {
                        anyhow::anyhow!("bigram weight index {weight_index} out of range")
                    })?;
                let cost = tocost(weight, cost_factor);
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
        // Try to infer field count from the unk.def entries
        if let Some((_, first_unk)) = self.unk_entries.first() {
            return wildcard_feature(first_unk);
        }

        // Fallback: try from pos_info
        if let Some(first_pos) = self.pos_info.first() {
            return wildcard_feature(first_pos);
        }

        // Ultimate fallback (should rarely happen)
        "*".to_string()
    }

    /// Writes `lex.csv`: one row per seed lexicon label, in label order, with
    /// its trained context ids and cost. `lindera export` and the bindings'
    /// export functions write `lex.csv` with this method.
    ///
    /// A surface (a label) that contains `,`, `"` or a line break is quoted,
    /// with its quotes doubled, so that `lindera build` reads it back as it
    /// is; the feature is written as it was read (#1170).
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for `lex.csv`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn write_lexicon<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        // Unknown word entries are at the end of labels, skip them (they go to unk.def)
        let unk_start_idx = self.labels.len().saturating_sub(self.unk_entries.len());

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
                    csv_field(label),
                    fs.left_id,
                    fs.right_id,
                    cost,
                    pos_info
                )?;
            }
        }

        Ok(())
    }

    /// Writes the dense connection cost matrix (MeCab-compatible `matrix.def`).
    ///
    /// The matrix covers every context id of the model, plus 0 for BOS/EOS,
    /// even when no bigram weight survived training: the lexicon and unknown
    /// entries carry those ids, and the runtime looks every transition up.
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for `matrix.def`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn write_connection_costs<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
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

        Ok(())
    }

    /// Update metadata.json with trained model values.
    ///
    /// The emitted `model_info` section deliberately carries no timestamp, so
    /// exporting the same model twice produces byte-identical output (#981);
    /// serde_json's map is ordered, so key order is stable too.
    ///
    /// # Arguments
    ///
    /// * `base_metadata_path` - Path of the base `metadata.json` to update.
    /// * `writer` - Write sink for the updated JSON.
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

        // Add model_info section with training statistics. The matrix size is
        // the one `write_connection_costs` writes: rows are right ids, columns
        // left ids, each with 0 for BOS/EOS.
        metadata["model_info"] = serde_json::json!({
            "feature_count": self.feature_weights.len(),
            "label_count": self.labels.len(),
            "max_left_context_id": self.max_left_id,
            "max_right_context_id": self.max_right_id,
            "connection_matrix_size": format!("{}x{}", self.max_right_id + 1, self.max_left_id + 1),
            "version": self.metadata.version,
            "training_iterations": self.metadata.iterations,
            "regularization": self.metadata.regularization,
        });

        // Write updated metadata
        let formatted = serde_json::to_string_pretty(&metadata)?;
        writer.write_all(formatted.as_bytes())?;

        Ok(())
    }

    /// Writes `unk.def`: one line per `unk.def` entry the model was trained
    /// with, in file order, with the entry's category name and feature string
    /// and its trained context ids and cost (#1110).
    ///
    /// # Arguments
    ///
    /// * `writer` - Write sink for `unk.def`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn write_unknown_dictionary<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        let unk_start_idx = self.labels.len().saturating_sub(self.unk_entries.len());

        for (i, (category_name, features)) in self.unk_entries.iter().enumerate() {
            let feature_idx = unk_start_idx + i;

            if feature_idx < self.feature_sets.len() {
                let fs = &self.feature_sets[feature_idx];

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
    use crate::{Trainer, TrainerConfig};
    use std::io::Cursor;

    use super::best_matching_seed_row;

    /// The columns after a quoted column with `,` in it line up (#1184).
    /// Split on `,`, the user feature would have four columns, and the first
    /// row, whose `"a` and `b"` pieces match, would score 60 and win.
    #[test]
    fn best_matching_seed_row_reads_a_quoted_column_as_one_column() {
        let seeds = ["n,\"a,b\"".to_string(), "n,\"c,d\",x".to_string()];

        // Row 0: `n` and `a,b` of two columns, 20 + 10. Row 1: `n` and `x`
        // of three columns, 30 + 10.
        assert_eq!(best_matching_seed_row("n,\"a,b\",x", &seeds), Some((1, 40)));
    }

    #[test]
    fn best_matching_seed_row_takes_the_first_best_row() {
        let seeds = [
            "動詞,自立".to_string(),
            "名詞,一般".to_string(),
            "名詞,一般".to_string(),
        ];

        assert_eq!(best_matching_seed_row("名詞,一般", &seeds), Some((1, 30)));
        assert_eq!(best_matching_seed_row("助詞,格助詞", &seeds), None);
    }

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

// Regression tests for #974: `model.dat` must be byte-reproducible.
//
// Feature ids and connection costs are stable across runs, but the archived
// bytes were not: rkyv lays an archived `HashMap` out as a SwissTable whose
// slot assignment resolves collisions in source iteration order, and
// `HashMap` iteration order is seeded per instance.
#[cfg(test)]
mod determinism_tests {
    use super::{FeatureSetInfo, MODEL_FORMAT_VERSION, ModelMetadata, SerializableModel};
    use std::collections::BTreeMap;

    /// Builds a model whose order-sensitive map is populated by walking
    /// `keys` in the given order. Everything else is fixed.
    fn model_with_maps(keys: &[usize]) -> SerializableModel {
        let mut connection_matrix = BTreeMap::new();
        for &k in keys {
            // The inner key set is derived from `k`, not from the traversal
            // order, so two builds over the same keys in different orders
            // produce logically identical maps -- which is exactly what this
            // test needs to isolate ordering from content.
            let mut inner = BTreeMap::new();
            for j in 0..4 {
                inner.insert(k + j, (k * 64 + j) as f64);
            }
            connection_matrix.insert(k, inner);
        }
        SerializableModel {
            feature_weights: vec![0.5; 4],
            labels: vec!["a".to_string()],
            pos_info: vec!["名詞,一般".to_string()],
            metadata: ModelMetadata {
                version: MODEL_FORMAT_VERSION.to_string(),
                regularization: 0.01,
                iterations: 10,
                feature_count: 4,
                label_count: 1,
            },
            connection_matrix,
            max_left_id: 63,
            max_right_id: 63,
            feature_sets: vec![FeatureSetInfo {
                left_id: 1,
                right_id: 1,
                weight: 0.25,
            }],
            unk_entries: vec![("CAT000".to_string(), "feat0,*,*".to_string())],
            char_def_content: String::new(),
            feature_def_content: String::new(),
            rewrite_def_content: String::new(),
            cost_factor: 700,
            left_id_map: vec![(1, "*".to_string())],
            right_id_map: vec![(1, "*".to_string())],
        }
    }

    /// The archived bytes must depend only on the logical contents of the
    /// map, never on the order it was populated in.
    ///
    /// 32 keys is deliberate: rkyv sizes the SwissTable at ~89% load and its
    /// probe window is 16 slots, so under `HashMap` the ascending and
    /// descending builds necessarily resolve collisions differently. Two
    /// separately constructed `HashMap`s also get different `RandomState`
    /// seeds, so this is not a coin flip.
    #[test]
    fn serialized_model_is_independent_of_map_insertion_order() {
        let forward: Vec<usize> = (0..32).collect();
        let reverse: Vec<usize> = (0..32).rev().collect();

        let a = match rkyv::to_bytes::<rkyv::rancor::Error>(&model_with_maps(&forward)) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to serialize ascending model: {err}"),
        };
        let b = match rkyv::to_bytes::<rkyv::rancor::Error>(&model_with_maps(&reverse)) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to serialize descending model: {err}"),
        };

        assert_eq!(
            a.as_slice(),
            b.as_slice(),
            "archived bytes must not depend on map insertion order"
        );
    }

    /// Serializing the very same value twice must also be stable -- a weaker
    /// property than the one above, but it is the one a caller writing the
    /// same model to two files relies on.
    #[test]
    fn serializing_the_same_model_twice_is_stable() {
        let keys: Vec<usize> = (0..32).collect();
        let model = model_with_maps(&keys);

        let a = match rkyv::to_bytes::<rkyv::rancor::Error>(&model) {
            Ok(bytes) => bytes,
            Err(err) => panic!("first serialization failed: {err}"),
        };
        let b = match rkyv::to_bytes::<rkyv::rancor::Error>(&model) {
            Ok(bytes) => bytes,
            Err(err) => panic!("second serialization failed: {err}"),
        };

        assert_eq!(a.as_slice(), b.as_slice());
    }

    /// The connection matrix's type, spelled out so the compile-time guard
    /// below names it exactly.
    type OrderedConnectionMatrix = BTreeMap<usize, BTreeMap<usize, f64>>;

    /// Compile-time guard. `model.dat` is byte-reproducible only while this
    /// field is an ordered map. This function is never called; it exists so
    /// that switching the field back to `HashMap` fails to compile rather
    /// than silently regressing (#974). (`unk_entries` is a `Vec`, which
    /// needs no guard.)
    #[allow(dead_code)]
    fn assert_connection_matrix_is_ordered(model: &SerializableModel) -> &OrderedConnectionMatrix {
        &model.connection_matrix
    }

    /// #1110: a model written in another format version (1.0.0 kept one
    /// unknown-word entry per category) is rejected with the instruction to
    /// re-train, even when its bytes happen to deserialize.
    #[test]
    fn read_model_rejects_another_format_version() {
        let mut model = model_with_maps(&[0, 1]);
        let current = match rkyv::to_bytes::<rkyv::rancor::Error>(&model) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to serialize model: {err}"),
        };
        if let Err(err) = super::Model::read_model(current.as_slice()) {
            panic!("a {MODEL_FORMAT_VERSION} model must load: {err}");
        }

        model.metadata.version = "1.0.0".to_string();
        let old = match rkyv::to_bytes::<rkyv::rancor::Error>(&model) {
            Ok(bytes) => bytes,
            Err(err) => panic!("failed to serialize model: {err}"),
        };
        let err = match super::Model::read_model(old.as_slice()) {
            Ok(_) => panic!("a 1.0.0 model must not load"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("lindera train"), "unhelpful message: {err}");
        assert!(
            err.contains("1.0.0"),
            "message must name the version: {err}"
        );
    }

    /// #1110: a legacy JSON model in the 1.0.0 layout (per-category
    /// `unk_category_names` / `unk_categories`, no `unk_entries`) gets the
    /// same instruction to re-train, not a missing-field error; one in the
    /// current version and layout still loads.
    #[test]
    fn read_model_rejects_a_legacy_json_model_of_another_format_version() {
        let mut value = match serde_json::to_value(model_with_maps(&[0, 1])) {
            Ok(value) => value,
            Err(err) => panic!("failed to serialize model as JSON: {err}"),
        };
        let current = value.to_string();
        if let Err(err) = super::Model::read_model(current.as_bytes()) {
            panic!("a {MODEL_FORMAT_VERSION} JSON model must load: {err}");
        }

        let serde_json::Value::Object(fields) = &mut value else {
            panic!("a model serializes as a JSON object");
        };
        fields.remove("unk_entries");
        fields.insert(
            "unk_category_names".to_string(),
            serde_json::json!(["DEFAULT"]),
        );
        fields.insert(
            "unk_categories".to_string(),
            serde_json::json!({"DEFAULT": "名詞,一般"}),
        );
        match fields.get_mut("metadata") {
            Some(metadata) => metadata["version"] = serde_json::json!("1.0.0"),
            None => panic!("a model has metadata"),
        }
        let old = value.to_string();

        let err = match super::Model::read_model(old.as_bytes()) {
            Ok(_) => panic!("a 1.0.0 JSON model must not load"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("lindera train"), "unhelpful message: {err}");
        assert!(
            err.contains("1.0.0"),
            "message must name the version: {err}"
        );
        assert!(
            !err.contains("missing field"),
            "message must not be the serde error: {err}"
        );
    }

    /// A payload that is neither valid rkyv nor JSON must say so and tell the
    /// user to re-train, rather than surfacing an unrelated UTF-8 error.
    #[test]
    fn read_model_rejects_unrecognized_payload_with_an_actionable_error() {
        let garbage = vec![0xffu8; 64];

        let err = match super::Model::read_model(garbage.as_slice()) {
            Ok(_) => panic!("garbage must not deserialize"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("lindera train"), "unhelpful message: {err}");
    }

    /// A payload that looks like JSON but is malformed is reported as a JSON
    /// parse failure, so the legacy path stays diagnosable.
    #[test]
    fn read_model_reports_malformed_legacy_json() {
        let err = match super::Model::read_model(&b"{ not json"[..]) {
            Ok(_) => panic!("malformed JSON must not deserialize"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("legacy JSON"), "unhelpful message: {err}");
    }

    /// An empty file is its own case, and used to reach the JSON reader.
    #[test]
    fn read_model_rejects_an_empty_file() {
        let err = match super::Model::read_model(&b""[..]) {
            Ok(_) => panic!("an empty file must not deserialize"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("empty"), "unhelpful message: {err}");
    }

    /// `update_metadata_json` must be byte-reproducible and must not emit a
    /// timestamp; its output must round-trip through
    /// `lindera_dictionary`'s `Metadata` loader.
    ///
    /// Before #981 it stamped `updated_at` from `SystemTime::now()` (and
    /// `unwrap()`ed in production code), so two exports of the same model
    /// could never compare equal.
    #[test]
    fn metadata_json_is_reproducible_and_timestamp_free() {
        use lindera_dictionary::dictionary::metadata::Metadata;

        let model = model_with_maps(&[0, 7, 21]);

        let dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(err) => panic!("failed to create temp dir: {err}"),
        };
        let base_path = dir.path().join("metadata.json");
        let base_json = match serde_json::to_string_pretty(&Metadata::default()) {
            Ok(json) => json,
            Err(err) => panic!("failed to serialize base metadata: {err}"),
        };
        if let Err(err) = std::fs::write(&base_path, base_json) {
            panic!("failed to write base metadata: {err}");
        }

        let update = || -> Vec<u8> {
            let mut buf = Vec::new();
            if let Err(err) = model.update_metadata_json(&base_path, &mut buf) {
                panic!("update_metadata_json failed: {err}");
            }
            buf
        };

        let first = update();
        let second = update();
        assert_eq!(first, second, "metadata.json output must be reproducible");

        let value: serde_json::Value = match serde_json::from_slice(&first) {
            Ok(value) => value,
            Err(err) => panic!("output is not valid JSON: {err}"),
        };
        let model_info = match value.get("model_info") {
            Some(serde_json::Value::Object(map)) => map,
            other => panic!("model_info must be an object, got {other:?}"),
        };
        assert!(
            !model_info.contains_key("updated_at"),
            "model_info must not carry a timestamp"
        );
        assert!(model_info.contains_key("feature_count"));
        assert!(model_info.contains_key("version"));

        // The emitted file must load through the dictionary-side struct,
        // pinning writer/struct consistency end to end.
        if let Err(err) = Metadata::load(&first) {
            panic!("exported metadata.json must load as Metadata: {err}");
        }
    }
}
