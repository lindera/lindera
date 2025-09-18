use std::io::{BufReader, Read};
use std::path::Path;
use std::collections::HashMap;

use anyhow::Result;

use crate::dictionary::Dictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::metadata::Metadata;
use crate::dictionary::prefix_dictionary::PrefixDictionary;
use crate::dictionary::unknown_dictionary::UnknownDictionary;
// Dictionary builders are not needed for the simplified approach
use super::feature_extractor::FeatureExtractor;
use super::feature_rewriter::FeatureRewriter;

/// Configuration for training.
pub struct TrainerConfig {
    pub(crate) dict: Dictionary,
    pub(crate) surfaces: Vec<String>,
    /// Maps surface forms to their original feature strings from the lexicon
    pub(crate) surface_features: HashMap<String, String>,
    pub(crate) feature_extractor: FeatureExtractor,
    pub(crate) unigram_rewriter: FeatureRewriter,
    pub(crate) left_rewriter: FeatureRewriter,
    pub(crate) right_rewriter: FeatureRewriter,
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
            if parts.len() >= 5 {
                let surface = parts[0].to_string();
                // Extract features from columns 4 onwards (skip surface,left_id,right_id,cost)
                let features = parts[4..].join(",");
                surfaces.push(surface.clone());
                surface_features.insert(surface, features);
            }
        }

        // Create feature extractor from templates
        let mut feature_extractor = FeatureExtractor::new();
        let mut feature_content = String::new();
        {
            let mut template_reader = BufReader::new(feature_templates_rdr);
            std::io::Read::read_to_string(&mut template_reader, &mut feature_content)?;
        }

        for line in feature_content.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            // Parse template format (simplified)
            if line.starts_with("UNIGRAM:") {
                feature_extractor.add_unigram_template(line[8..].to_string());
            } else if line.starts_with("LEFT:") {
                feature_extractor.add_left_template(line[5..].to_string());
            } else if line.starts_with("RIGHT:") {
                feature_extractor.add_right_template(line[6..].to_string());
            }
        }

        // Create feature rewriters
        let unigram_rewriter = FeatureRewriter::new();
        let left_rewriter = FeatureRewriter::new();
        let right_rewriter = FeatureRewriter::from_reader(rewrite_rules_rdr)?;

        // Build dictionary from readers
        let dict =
            Self::build_dictionary_from_readers(&lexicon_content, char_prop_rdr, unk_handler_rdr)?;

        Ok(Self {
            dict,
            surfaces,
            surface_features,
            feature_extractor,
            unigram_rewriter,
            left_rewriter,
            right_rewriter,
        })
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

    /// Builds a dictionary from raw file contents
    fn build_dictionary_from_readers<R2, R3>(
        lexicon_content: &str,
        char_prop_rdr: R2,
        unk_handler_rdr: R3,
    ) -> Result<Dictionary>
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
        let char_def = Self::build_char_def_from_content(&char_prop_content)?;

        // Build unknown dictionary
        let unknown_dict = Self::build_unknown_dict_from_content(&unk_content, &char_def)?;

        // Build prefix dictionary (lexicon)
        let prefix_dict = Self::build_prefix_dict_from_content(lexicon_content)?;

        // Create minimal connection cost matrix
        let conn_matrix = Self::create_minimal_connection_matrix()?;

        Ok(Dictionary {
            prefix_dictionary: prefix_dict,
            connection_cost_matrix: conn_matrix,
            character_definition: char_def,
            unknown_dictionary: unknown_dict,
            metadata: Metadata::default(),
        })
    }

    fn build_char_def_from_content(_content: &str) -> Result<CharacterDefinition> {
        // For training purposes, create a minimal character definition
        // In a real implementation, this would parse the char.def format
        use crate::dictionary::character_definition::{CategoryData, LookupTable};
        let mut category_definitions = Vec::new();
        let mut category_names = Vec::new();

        // Add default categories for basic character types
        category_names.push("DEFAULT".to_string());
        category_names.push("HIRAGANA".to_string());
        category_names.push("KATAKANA".to_string());
        category_names.push("KANJI".to_string());
        category_names.push("ALPHA".to_string());
        category_names.push("NUMERIC".to_string());

        for _name in category_names.iter() {
            category_definitions.push(CategoryData {
                invoke: true,
                group: true,
                length: 1,
            });
        }

        // Create a simple lookup table that maps all characters to DEFAULT
        let boundaries = vec![0u32, 0x10FFFF]; // All Unicode range
        let mapping = LookupTable::from_fn(boundaries, &|_c, buff| {
            buff.push(crate::dictionary::character_definition::CategoryId(0)); // Map to DEFAULT category
        });

        Ok(CharacterDefinition {
            category_definitions,
            category_names,
            mapping,
        })
    }

    fn build_unknown_dict_from_content(
        _content: &str,
        _char_def: &CharacterDefinition,
    ) -> Result<UnknownDictionary> {
        // Create minimal unknown dictionary for training
        Ok(UnknownDictionary {
            category_references: vec![vec![0]; 6], // One for each category
            costs: vec![],                         // Will be filled during training
        })
    }

    fn build_prefix_dict_from_content(_content: &str) -> Result<PrefixDictionary> {
        use crate::util::Data;
        use yada::DoubleArray;

        // Create minimal prefix dictionary structure for training
        // In production, this would parse the lexicon CSV format
        let da_data = Data::from(vec![]);
        let da = DoubleArray::new(da_data);

        Ok(PrefixDictionary {
            da,
            vals_data: Data::from(vec![]),
            words_idx_data: Data::from(vec![]),
            words_data: Data::from(vec![]),
            is_system: true,
        })
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

        Ok(ConnectionCostMatrix::load(matrix_data))
    }
}
