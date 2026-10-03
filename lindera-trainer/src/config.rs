use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use super::feature_extractor::FeatureExtractor;
use super::feature_rewriter::DictionaryRewriter;
use lindera_dictionary::builder::character_definition::CharacterDefinitionBuilderOptions;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::{
    CategoryId, CharacterDefinition, LookupTable,
};
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;

/// Configuration for training.
pub struct TrainerConfig {
    pub(crate) dict: Dictionary,
    /// Default category of every code point: the first category of the last
    /// `char.def` line that covers it (MeCab's `default_type`). The `%t`
    /// feature uses it, because the category list in `dict` is sorted by
    /// category id and so does not start with the default category.
    pub(crate) default_categories: LookupTable<CategoryId>,
    pub(crate) surfaces: Vec<String>,
    /// Feature strings for each entry (parallel to surfaces)
    pub(crate) features: Vec<String>,
    /// Maps surface forms to their original feature strings from the lexicon
    pub(crate) surface_features: HashMap<String, String>,
    /// User lexicon entries for additional vocabulary
    pub(crate) user_lexicon: HashMap<String, String>,
    pub(crate) feature_extractor: FeatureExtractor,
    pub(crate) dictionary_rewriter: DictionaryRewriter,
    /// Cost factor for converting CRF weights to i16 costs (MeCab's cost-factor)
    pub(crate) cost_factor: i32,
    /// Metadata from which encoding and schema information is derived
    pub(crate) metadata: Metadata,
    /// Maps unknown word category names to their feature strings from unk.def
    /// Format: category -> "pos,feature1,feature2,..."
    ///
    /// Ordered rather than hashed: this map is cloned verbatim into
    /// `SerializableModel::unk_categories`, whose serialized byte layout must
    /// not depend on iteration order (#974).
    pub(crate) unk_categories: BTreeMap<String, String>,
    /// Maps unknown word category names to their costs from unk.def
    /// Format: category -> cost
    pub(crate) unk_costs: HashMap<String, i32>,
    /// Raw content of the character definition file (char.def)
    /// Preserved from training input for export
    pub(crate) char_def_content: String,
    /// Raw content of the feature definition file (feature.def)
    /// Preserved from training input for export
    pub(crate) feature_def_content: String,
    /// Raw content of the rewrite rule definition file (rewrite.def)
    /// Preserved from training input for export
    pub(crate) rewrite_def_content: String,
}

impl TrainerConfig {
    /// Access system lexicon for morphological analysis
    pub fn system_lexicon(&self) -> &PrefixDictionary {
        self.dict.prefix_dictionary.as_ref()
    }

    /// Access dictionary (for compatibility)
    pub fn dict(&self) -> &Dictionary {
        &self.dict
    }

    /// Access unknown word handler for out-of-vocabulary processing
    pub fn unk_handler(
        &self,
    ) -> &lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary {
        &self.dict.unknown_dictionary
    }

    /// Returns the default category of a character, the value of the `%t`
    /// feature.
    ///
    /// # Arguments
    ///
    /// * `c` - The character, usually the first character of a surface.
    ///
    /// # Returns
    ///
    /// The id of the first category of the last `char.def` line that covers
    /// `c` (MeCab's `default_type`), the id of `DEFAULT` when no line covers
    /// it, or `None` when no line covers it and `char.def` does not use
    /// `DEFAULT`.
    pub(crate) fn default_category(&self, c: char) -> Option<u32> {
        self.default_categories
            .eval(c as u32)
            .first()
            .map(|category_id| category_id.0 as u32)
    }
}

impl TrainerConfig {
    /// Creates a new trainer configuration from readers.
    ///
    /// # Arguments
    ///
    /// * `lexicon_rdr` - Reader for the seed lexicon file (lex.csv)
    /// * `char_prop_rdr` - Reader for the character property file (char.def)
    /// * `unk_handler_rdr` - Reader for the unknown word file (unk.def)
    /// * `feature_templates_rdr` - Reader for the feature templates file (feature.def)
    /// * `rewrite_rules_rdr` - Reader for the rewrite rules file (rewrite.def)
    pub fn from_readers<R1, R2, R3, R4, R5>(
        lexicon_rdr: R1,
        char_prop_rdr: R2,
        unk_handler_rdr: R3,
        feature_templates_rdr: R4,
        rewrite_rules_rdr: R5,
    ) -> Result<Self>
    where
        R1: Read,
        R2: Read,
        R3: Read,
        R4: Read,
        R5: Read,
    {
        // Parse lexicon to extract surfaces and features
        let mut surfaces = Vec::new();
        let mut features = Vec::new();
        let mut surface_features = HashMap::new();
        let mut lexicon_content = String::new();
        {
            let mut lexicon_reader = BufReader::new(lexicon_rdr);
            std::io::Read::read_to_string(&mut lexicon_reader, &mut lexicon_content)?;
        }

        for line in lexicon_content.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();

            // Accept any dictionary format with at least 5 columns
            // Format: surface,left_id,right_id,cost,feature1,feature2,...
            // - IPADIC:    13 columns (pos + 8 feature fields)
            // - UniDic:    21+ columns (pos + 16+ feature fields)
            // - ko-dic:    8 columns (pos + 3 feature fields)
            // - CC-CEDICT: 8 columns (pos + 3 feature fields)
            if parts.len() >= 5 {
                let surface = parts[0].to_string();
                // Extract features from columns 4 onwards (skip surface,left_id,right_id,cost)
                // This works for any dictionary format
                let feature_str = parts[4..].join(",");
                surfaces.push(surface.clone());
                features.push(feature_str.clone());
                surface_features.insert(surface, feature_str);
            }
        }

        // Create feature extractor from templates
        let mut feature_content = String::new();
        {
            let mut template_reader = BufReader::new(feature_templates_rdr);
            std::io::Read::read_to_string(&mut template_reader, &mut feature_content)?;
        }

        // Parse templates into unigram and bigram categories
        let mut unigram_templates = Vec::new();
        let mut bigram_templates = Vec::new();

        for line in feature_content.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            // Parse template format: MeCab-compatible feature.def
            // UNIGRAM U00:%F[0]  or  UNIGRAM:%F[0]
            // BIGRAM B00:%L[0]/%R[0]  or  BIGRAM:%L[0]/%R[0]
            if let Some(rest) = line.strip_prefix("UNIGRAM") {
                // Extract the template part after optional label (e.g., "U00:")
                let rest = rest.trim_start().trim_start_matches(':').trim_start();
                let template = if let Some(idx) = rest.find('%') {
                    &rest[idx..]
                } else {
                    rest
                };
                unigram_templates.push(template.to_string());
            } else if let Some(rest) = line.strip_prefix("BIGRAM") {
                // Extract the template part after optional label (e.g., "B00:")
                let rest = rest.trim_start().trim_start_matches(':').trim_start();
                let template = if let Some(idx) = rest.find('%') {
                    &rest[idx..]
                } else {
                    rest
                };
                if let Some((left, right)) = template.split_once('/') {
                    bigram_templates.push((left.to_string(), right.to_string()));
                }
            } else {
                // Default unigram template (bare template without prefix)
                unigram_templates.push(line.to_string());
            }
        }

        // Create feature extractor with parsed templates
        let feature_extractor =
            FeatureExtractor::from_templates(&unigram_templates, &bigram_templates);

        // Read rewrite rules content
        let mut rewrite_def_content = String::new();
        {
            let mut rewrite_reader = BufReader::new(rewrite_rules_rdr);
            std::io::Read::read_to_string(&mut rewrite_reader, &mut rewrite_def_content)?;
        }

        // Create dictionary rewriter with 3-section support
        let dictionary_rewriter =
            DictionaryRewriter::from_reader(std::io::Cursor::new(rewrite_def_content.as_bytes()))?;

        // Parse unk.def to extract category-to-features mapping
        let mut unk_content = String::new();
        {
            let mut unk_reader = BufReader::new(unk_handler_rdr);
            std::io::Read::read_to_string(&mut unk_reader, &mut unk_content)?;
        }

        let mut unk_categories = BTreeMap::new();
        let mut unk_costs = HashMap::new();
        for line in unk_content.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            // Format: category,left_id,right_id,cost,feature1,feature2,...
            if parts.len() >= 5 {
                let category = parts[0].to_string();
                let features = parts[4..].join(",");
                unk_categories.insert(category.clone(), features);

                // Parse cost (4th column)
                if let Ok(cost) = parts[3].parse::<i32>() {
                    unk_costs.insert(category, cost);
                }
            }
        }

        // Read character properties content
        let mut char_def_content = String::new();
        {
            let mut char_prop_reader = BufReader::new(char_prop_rdr);
            std::io::Read::read_to_string(&mut char_prop_reader, &mut char_def_content)?;
        }

        // Build dictionary from readers (need to re-create readers from content strings)
        use std::io::Cursor;
        let (dict, default_categories) = Self::build_dictionary_from_readers(
            &lexicon_content,
            Cursor::new(char_def_content.as_bytes()),
            Cursor::new(unk_content.as_bytes()),
        )?;

        Ok(Self {
            dict,
            default_categories,
            surfaces,
            features,
            surface_features,
            user_lexicon: HashMap::new(), // Initialize empty user lexicon
            feature_extractor,
            dictionary_rewriter,
            cost_factor: 700,              // MeCab default cost-factor
            metadata: Metadata::default(), // Use default metadata for backward compatibility
            unk_categories,
            unk_costs,
            char_def_content,
            feature_def_content: feature_content,
            rewrite_def_content,
        })
    }

    /// Get the surfaces extracted from the lexicon
    pub fn surfaces(&self) -> &[String] {
        &self.surfaces
    }

    /// Get the surface features mapping
    pub fn surface_features(&self) -> &HashMap<String, String> {
        &self.surface_features
    }

    /// Get the user lexicon mapping.
    ///
    /// This map only feeds [`TrainerConfig::get_features`]; dictionary export
    /// reads the user entries loaded via `Model::read_user_lexicon` instead
    /// (#981).
    ///
    /// # 戻り値
    ///
    /// The surface-to-features map.
    pub fn user_lexicon(&self) -> &HashMap<String, String> {
        &self.user_lexicon
    }

    /// Add user lexicon entry (user dictionary support).
    ///
    /// # 引数
    ///
    /// * `surface` - Surface form of the entry.
    /// * `features` - Comma-joined feature string of the entry.
    #[deprecated(
        since = "5.3.0",
        note = "this map only feeds TrainerConfig::get_features; dictionary \
                export reads user entries loaded via Model::read_user_lexicon"
    )]
    pub fn add_user_lexicon_entry(&mut self, surface: String, features: String) {
        self.user_lexicon.insert(surface, features);
    }

    /// Get features for a given surface form
    pub fn get_features(&self, surface: &str) -> Option<String> {
        // First check user lexicon, then surface features
        self.user_lexicon
            .get(surface)
            .or_else(|| self.surface_features.get(surface))
            .cloned()
    }

    /// Load user lexicon from CSV content.
    ///
    /// # 引数
    ///
    /// * `content` - User lexicon CSV content.
    ///
    /// # 戻り値
    ///
    /// `Ok(())`; malformed lines are skipped.
    #[deprecated(
        since = "5.3.0",
        note = "this map only feeds TrainerConfig::get_features; dictionary \
                export reads user entries loaded via Model::read_user_lexicon"
    )]
    pub fn load_user_lexicon_from_content(&mut self, content: &str) -> Result<()> {
        for line in content.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                let surface = parts[0].to_string();
                // Extract features from columns 4 onwards (skip surface,left_id,right_id,cost)
                let features = parts[4..].join(",");
                self.user_lexicon.insert(surface, features);
            }
        }
        Ok(())
    }

    /// Creates a new trainer configuration from file paths.
    pub fn from_paths(
        lexicon_path: &Path,
        char_prop_path: &Path,
        unk_handler_path: &Path,
        feature_templates_path: &Path,
        rewrite_rules_path: &Path,
    ) -> Result<Self> {
        use std::fs::File;

        Self::from_readers(
            File::open(lexicon_path)?,
            File::open(char_prop_path)?,
            File::open(unk_handler_path)?,
            File::open(feature_templates_path)?,
            File::open(rewrite_rules_path)?,
        )
    }

    /// Get the metadata
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Builds a dictionary from raw file contents.
    ///
    /// # Arguments
    ///
    /// * `lexicon_content` - The seed lexicon (lex.csv) content.
    /// * `char_prop_rdr` - Reader for the character property file (char.def).
    /// * `unk_handler_rdr` - Reader for the unknown word file (unk.def).
    ///
    /// # Returns
    ///
    /// The minimal dictionary used during training, and the default category
    /// table built from the same `char.def`.
    fn build_dictionary_from_readers<R2, R3>(
        lexicon_content: &str,
        char_prop_rdr: R2,
        unk_handler_rdr: R3,
    ) -> Result<(Dictionary, LookupTable<CategoryId>)>
    where
        R2: Read,
        R3: Read,
    {
        // Read character properties
        let mut char_prop_content = String::new();
        let mut char_prop_reader = BufReader::new(char_prop_rdr);
        std::io::Read::read_to_string(&mut char_prop_reader, &mut char_prop_content)?;

        // Read unknown word definitions
        let mut unk_content = String::new();
        let mut unk_reader = BufReader::new(unk_handler_rdr);
        std::io::Read::read_to_string(&mut unk_reader, &mut unk_content)?;

        // Build character definition
        let (char_def, default_categories) = Self::build_char_def_from_content(&char_prop_content)?;

        // Build unknown dictionary
        let unknown_dict = Self::build_unknown_dict_from_content(&unk_content, &char_def)?;

        // Build prefix dictionary (lexicon)
        let prefix_dict = Self::build_prefix_dict_from_content(lexicon_content)?;

        // Create minimal connection cost matrix
        let conn_matrix = Self::create_minimal_connection_matrix()?;

        let dict = Dictionary {
            prefix_dictionary: Arc::new(prefix_dict),
            connection_cost_matrix: Arc::new(conn_matrix),
            character_definition: Arc::new(char_def),
            unknown_dictionary: Arc::new(unknown_dict),
            metadata: Arc::new(Metadata::default()),
        };
        Ok((dict, default_categories))
    }

    /// Reads `char.def` with the dictionary builder, so training sees the
    /// same character categories as the dictionary built from the exported
    /// files.
    ///
    /// # Arguments
    ///
    /// * `content` - The text of `char.def`.
    ///
    /// # Returns
    ///
    /// The character definition, and the table of default categories (MeCab's
    /// `default_type` of every code point) used for the `%t` feature.
    fn build_char_def_from_content(
        content: &str,
    ) -> Result<(CharacterDefinition, LookupTable<CategoryId>)> {
        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        let char_def = builder
            .build_from_str(content)
            .map_err(|err| anyhow::anyhow!("failed to parse char.def: {err}"))?;
        Ok((char_def, builder.build_default_category_table()))
    }

    fn build_unknown_dict_from_content(
        _content: &str,
        _char_def: &CharacterDefinition,
    ) -> Result<UnknownDictionary> {
        // Create minimal unknown dictionary for training
        Ok(UnknownDictionary {
            category_references: vec![vec![0]; 6], // One for each category
            costs: vec![],                         // Will be filled during training
            words_idx_data: vec![],
            words_data: vec![],
        })
    }

    fn build_prefix_dict_from_content(_content: &str) -> Result<PrefixDictionary> {
        // Create minimal (empty) prefix dictionary structure for training.
        // In production, this would parse the lexicon CSV format.
        PrefixDictionary::from_word_entry_map(&std::collections::BTreeMap::new())
            .map_err(|err| anyhow::anyhow!("failed to build empty prefix dictionary: {err}"))
    }

    fn create_minimal_connection_matrix() -> Result<ConnectionCostMatrix> {
        // Create minimal 6x6 connection matrix for the basic categories
        let matrix_size = 6u16;
        let mut matrix_data = vec![0u8; 4]; // Header: forward_size(2) + backward_size(2)

        // Write matrix dimensions
        matrix_data[0..2].copy_from_slice(&matrix_size.to_le_bytes());
        matrix_data[2..4].copy_from_slice(&matrix_size.to_le_bytes());

        // Add connection costs (all zero for simplicity)
        let cost_data_size = (matrix_size as usize) * (matrix_size as usize) * 2; // 2 bytes per cost
        matrix_data.extend(vec![0u8; cost_data_size]);

        Ok(ConnectionCostMatrix::load(matrix_data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_ipadic_format_13_columns() {
        // IPADIC format: 13 columns
        let seed_csv = "東京,0,0,5000,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー\n\
                        行く,1,1,4000,動詞,自立,*,*,五段・カ行促音便,基本形,行く,イク,イク\n";
        let char_def = "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n0x3042..0x3096 HIRAGANA\n";
        let unk_def = "DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n";
        let feature_def = "UNIGRAM:%F[0]\nUNIGRAM:%F[1]\n";
        let rewrite_def = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        assert_eq!(config.surfaces().len(), 2);
        assert!(config.surfaces().contains(&"東京".to_string()));
        assert!(config.surfaces().contains(&"行く".to_string()));

        // Verify features are correctly extracted (9 fields after surface,left_id,right_id,cost)
        let tokyo_features = config.surface_features().get("東京").unwrap();
        assert_eq!(
            tokyo_features,
            "名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー"
        );
    }

    #[test]
    fn test_ko_dic_format_8_columns() {
        // ko-dic format: 8 columns
        let seed_csv = "한국,0,0,5000,NNG,Korea,F,han-guk\n\
                        안녕,1,1,4000,NNG,hello,F,an-nyeong\n";
        let char_def = "DEFAULT 0 1 0\nHANGUL 1 1 0\n0xAC00..0xD7A3 HANGUL\n";
        let unk_def = "DEFAULT,0,0,1500,NNG,unknown,F,*\n";
        let feature_def = "UNIGRAM:%F[0]\n";
        let rewrite_def = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        assert_eq!(config.surfaces().len(), 2);
        assert!(config.surfaces().contains(&"한국".to_string()));
        assert!(config.surfaces().contains(&"안녕".to_string()));

        // Verify features (4 fields after surface,left_id,right_id,cost)
        let korea_features = config.surface_features().get("한국").unwrap();
        assert_eq!(korea_features, "NNG,Korea,F,han-guk");
    }

    #[test]
    fn test_cc_cedict_format_8_columns() {
        // CC-CEDICT format: 8 columns
        let seed_csv = "中国,0,0,5000,n,China,*,zhong1guo2\n\
                        你好,1,1,4000,x,hello,*,ni3hao3\n";
        let char_def = "DEFAULT 0 1 0\nHANZI 1 1 0\n0x4E00..0x9FFF HANZI\n";
        let unk_def = "DEFAULT,0,0,1500,n,unknown,*,*\n";
        let feature_def = "UNIGRAM:%F[0]\n";
        let rewrite_def = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        assert_eq!(config.surfaces().len(), 2);
        assert!(config.surfaces().contains(&"中国".to_string()));
        assert!(config.surfaces().contains(&"你好".to_string()));

        // Verify features (4 fields after surface,left_id,right_id,cost)
        let china_features = config.surface_features().get("中国").unwrap();
        assert_eq!(china_features, "n,China,*,zhong1guo2");
    }

    #[test]
    fn test_unidic_format_21_columns() {
        // UniDic format: 21 columns (simplified example)
        let seed_csv = "東京,0,0,5000,名詞,固有名詞,地名,一般,*,*,トウキョウ,東京,東京,東京,東京,東京,トウキョウ,トーキョー,東京,東京,1\n";
        let char_def = "DEFAULT 0 1 0\nKANJI 0 0 2\n0x4E00..0x9FFF KANJI\n";
        let unk_def = "DEFAULT,0,0,1500,名詞,普通名詞,一般,*,*,*,*,*,*,*,*,*,*,*,*,*,*\n";
        let feature_def = "UNIGRAM:%F[0]\nUNIGRAM:%F[1]\n";
        let rewrite_def = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        assert_eq!(config.surfaces().len(), 1);
        assert!(config.surfaces().contains(&"東京".to_string()));

        // Verify features (17 fields after surface,left_id,right_id,cost)
        let tokyo_features = config.surface_features().get("東京").unwrap();
        assert_eq!(
            tokyo_features,
            "名詞,固有名詞,地名,一般,*,*,トウキョウ,東京,東京,東京,東京,東京,トウキョウ,トーキョー,東京,東京,1"
        );
    }

    #[test]
    fn test_mixed_column_counts() {
        // Test that we can handle files with varying column counts
        let seed_csv = "東京,0,0,5000,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー\n\
                        한국,1,1,4000,NNG,Korea,F,han-guk\n\
                        中国,2,2,3000,n,China,*,zhong1guo2\n";
        let char_def = "DEFAULT 0 1 0\n";
        let unk_def = "DEFAULT,0,0,1500,*,*,*,*\n";
        let feature_def = "UNIGRAM:%F[0]\n";
        let rewrite_def = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(seed_csv),
            Cursor::new(char_def),
            Cursor::new(unk_def),
            Cursor::new(feature_def),
            Cursor::new(rewrite_def),
        )
        .unwrap();

        assert_eq!(config.surfaces().len(), 3);

        // Each row has different number of feature fields, all should be accepted
        assert_eq!(
            config.surface_features().get("東京").unwrap(),
            "名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー"
        );
        assert_eq!(
            config.surface_features().get("한국").unwrap(),
            "NNG,Korea,F,han-guk"
        );
        assert_eq!(
            config.surface_features().get("中国").unwrap(),
            "n,China,*,zhong1guo2"
        );
    }

    #[test]
    fn test_trainer_config_creation() {
        // Test that TrainerConfig can be created with minimal valid data
        let lexicon_data = "外国,0,0,5000,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n人,1,1,5000,名詞,接尾,一般,*,*,*,人,ジン,ジン\n";
        let char_data = "# char.def placeholder\n";
        let unk_data = "# unk.def placeholder\n";
        let feature_data = "UNIGRAM:%F[0]\nLEFT:%L[0]\nRIGHT:%R[0]\n";
        let rewrite_data = "# rewrite.def placeholder\n";

        let result = TrainerConfig::from_readers(
            Cursor::new(lexicon_data.as_bytes()),
            Cursor::new(char_data.as_bytes()),
            Cursor::new(unk_data.as_bytes()),
            Cursor::new(feature_data.as_bytes()),
            Cursor::new(rewrite_data.as_bytes()),
        );

        // Config creation should now succeed with the fixed implementation
        assert!(result.is_ok());
        let config = result.unwrap();
        // Verify that surfaces were extracted correctly using the getter
        assert_eq!(config.surfaces().len(), 2);
        assert!(config.surfaces().contains(&"外国".to_string()));
        assert!(config.surfaces().contains(&"人".to_string()));
    }

    #[test]
    fn test_unk_categories_ipadic() {
        // Test that unk_categories are correctly extracted for IPADIC format
        let lexicon_data = "東京,0,0,5000,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー\n";
        let char_data = "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n";
        let unk_data = "DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\nHIRAGANA,1,1,2000,名詞,代名詞,一般,*,*,*,*,*,*\n";
        let feature_data = "UNIGRAM:%F[0]\n";
        let rewrite_data = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(lexicon_data),
            Cursor::new(char_data),
            Cursor::new(unk_data),
            Cursor::new(feature_data),
            Cursor::new(rewrite_data),
        )
        .unwrap();

        // Verify unk_categories extracted correctly
        assert_eq!(config.unk_categories.len(), 2);
        assert_eq!(
            config.unk_categories.get("DEFAULT").unwrap(),
            "名詞,一般,*,*,*,*,*,*,*"
        );
        assert_eq!(
            config.unk_categories.get("HIRAGANA").unwrap(),
            "名詞,代名詞,一般,*,*,*,*,*,*"
        );
    }

    #[test]
    fn test_unk_categories_ko_dic() {
        // Test that unk_categories work for Korean dictionary format
        let lexicon_data = "한국,0,0,5000,NNG,Korea,F,han-guk\n";
        let char_data = "DEFAULT 0 1 0\n";
        let unk_data = "DEFAULT,0,0,1500,NNG,unknown,F,*\n";
        let feature_data = "UNIGRAM:%F[0]\n";
        let rewrite_data = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(lexicon_data),
            Cursor::new(char_data),
            Cursor::new(unk_data),
            Cursor::new(feature_data),
            Cursor::new(rewrite_data),
        )
        .unwrap();

        assert_eq!(config.unk_categories.len(), 1);
        assert_eq!(
            config.unk_categories.get("DEFAULT").unwrap(),
            "NNG,unknown,F,*"
        );
    }

    #[test]
    fn test_unk_categories_cc_cedict() {
        // Test that unk_categories work for Chinese dictionary format
        let lexicon_data = "中国,0,0,5000,n,China,*,zhong1guo2\n";
        let char_data = "DEFAULT 0 1 0\n";
        let unk_data = "DEFAULT,0,0,1500,n,unknown,*,*\n";
        let feature_data = "UNIGRAM:%F[0]\n";
        let rewrite_data = "*\tUNK\n";

        let config = TrainerConfig::from_readers(
            Cursor::new(lexicon_data),
            Cursor::new(char_data),
            Cursor::new(unk_data),
            Cursor::new(feature_data),
            Cursor::new(rewrite_data),
        )
        .unwrap();

        assert_eq!(config.unk_categories.len(), 1);
        assert_eq!(
            config.unk_categories.get("DEFAULT").unwrap(),
            "n,unknown,*,*"
        );
    }

    fn config_with_char_def(char_data: &str) -> TrainerConfig {
        TrainerConfig::from_readers(
            Cursor::new("一,0,0,5000,名詞,数,*,*,*,*,一,イチ,イチ\n"),
            Cursor::new(char_data),
            Cursor::new("DEFAULT,0,0,1500,名詞,一般,*,*,*,*,*,*,*\n"),
            Cursor::new("UNIGRAM:%F[0]\n"),
            Cursor::new("*\tUNK\n"),
        )
        .unwrap()
    }

    #[test]
    fn test_default_category_follows_mecab() {
        // IPADIC-style char.def: the trainer reads it with the dictionary
        // builder, so single-code-point and overlapping lines count and the
        // last line wins (#1095). %t uses the first category of that line,
        // MeCab's default_type.
        let config = config_with_char_def(
            "DEFAULT 0 1 0\n\
             SPACE 0 1 0\n\
             KANJI 0 0 2\n\
             SYMBOL 1 1 0\n\
             ALPHA 1 1 0\n\
             KANJINUMERIC 1 1 0\n\
             0x0020 SPACE\n\
             0x00D0 SPACE\n\
             0x00C0..0x00FF ALPHA\n\
             0x3005 KANJI\n\
             0x4E00..0x9FA5 KANJI\n\
             0x4E00 KANJINUMERIC KANJI\n\
             0x3000..0x303F SYMBOL\n",
        );
        let char_def = config.dict.character_definition.as_ref();
        let id = |name: &str| char_def.category_id_by_name(name).map(|id| id.0 as u32);

        assert_eq!(config.default_category('一'), id("KANJINUMERIC"));
        assert_eq!(config.default_category('丁'), id("KANJI"));
        assert_eq!(config.default_category(' '), id("SPACE"));
        assert_eq!(config.default_category('Ð'), id("ALPHA"));
        assert_eq!(config.default_category('々'), id("SYMBOL"));
        assert_eq!(config.default_category('a'), id("DEFAULT"));
    }

    #[test]
    fn test_default_category_without_default_category() {
        let config = config_with_char_def("HIRAGANA 1 1 0\n0x3042..0x3096 HIRAGANA\n");

        assert_eq!(config.default_category('あ'), Some(0));
        assert_eq!(config.default_category('a'), None);
    }
}
