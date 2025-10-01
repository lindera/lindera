use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::Path;

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
    /// Feature strings for each entry (parallel to surfaces)
    pub(crate) features: Vec<String>,
    /// Maps surface forms to their original feature strings from the lexicon
    pub(crate) surface_features: HashMap<String, String>,
    /// User lexicon entries for additional vocabulary
    pub(crate) user_lexicon: HashMap<String, String>,
    pub(crate) feature_extractor: FeatureExtractor,
    pub(crate) unigram_rewriter: FeatureRewriter,
    pub(crate) left_rewriter: FeatureRewriter,
    pub(crate) right_rewriter: FeatureRewriter,
}

impl TrainerConfig {
    /// Access system lexicon for morphological analysis
    pub fn system_lexicon(&self) -> &PrefixDictionary {
        &self.dict.prefix_dictionary
    }

    /// Access dictionary (for compatibility)
    pub fn dict(&self) -> &Dictionary {
        &self.dict
    }

    /// Access unknown word handler for out-of-vocabulary processing
    pub fn unk_handler(&self) -> &crate::dictionary::unknown_dictionary::UnknownDictionary {
        &self.dict.unknown_dictionary
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
            if parts.len() >= 5 {
                let surface = parts[0].to_string();
                // Extract features from columns 4 onwards (skip surface,left_id,right_id,cost)
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
            // Parse template format
            if let Some(stripped) = line.strip_prefix("UNIGRAM:") {
                unigram_templates.push(stripped.to_string());
            } else if let Some(template_part) = line.strip_prefix("BIGRAM") {
                // Parse bigram template like "BIGRAM B00:%L[0]/%R[0]"
                // Remove label prefix (e.g., "B00:") and split by /
                let template = template_part.trim().trim_start_matches(':').trim();
                // Find the part after the label (after the first colon if present)
                let template_body = if let Some(idx) = template.find(':') {
                    &template[idx + 1..]
                } else {
                    template
                };
                if let Some((left, right)) = template_body.split_once('/') {
                    bigram_templates.push((left.to_string(), right.to_string()));
                }
            } else {
                // Default unigram template
                unigram_templates.push(line.to_string());
            }
        }

        // Create feature extractor with parsed templates
        let feature_extractor =
            FeatureExtractor::from_templates(&unigram_templates, &bigram_templates);

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
            features,
            surface_features,
            user_lexicon: HashMap::new(), // Initialize empty user lexicon
            feature_extractor,
            unigram_rewriter,
            left_rewriter,
            right_rewriter,
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

    /// Get the user lexicon mapping
    pub fn user_lexicon(&self) -> &HashMap<String, String> {
        &self.user_lexicon
    }

    /// Add user lexicon entry (user dictionary support)
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

    /// Load user lexicon from CSV content
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

    fn build_char_def_from_content(content: &str) -> Result<CharacterDefinition> {
        use crate::dictionary::character_definition::{CategoryData, CategoryId, LookupTable};
        use std::collections::HashMap;

        let mut category_definitions = Vec::new();
        let mut category_names = Vec::new();
        let mut category_map = HashMap::new(); // Name -> Index
        let mut char_ranges = Vec::new();

        // Always add DEFAULT as category 0
        category_names.push("DEFAULT".to_string());
        category_map.insert("DEFAULT".to_string(), 0);
        category_definitions.push(CategoryData {
            invoke: false,
            group: true,
            length: 0,
        });

        // Parse the char.def file
        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse character range mappings (e.g., "0x3041..0x3096 HIRAGANA")
            if line.starts_with("0x") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let range_str = parts[0];
                    let category = parts[1];

                    // Parse range (e.g., "0x3041..0x3096")
                    if let Some(range_parts) = range_str.split_once("..") {
                        let start = u32::from_str_radix(&range_parts.0[2..], 16)?;
                        let end = u32::from_str_radix(&range_parts.1[2..], 16)?;

                        // Get or create category index
                        let cat_idx =
                            *category_map.entry(category.to_string()).or_insert_with(|| {
                                let idx = category_names.len();
                                category_names.push(category.to_string());
                                // Default category data - will be overridden if defined
                                category_definitions.push(CategoryData {
                                    invoke: true,
                                    group: true,
                                    length: 0,
                                });
                                idx
                            });

                        char_ranges.push((start, end, cat_idx));
                    }
                }
            } else {
                // Parse category definitions (e.g., "HIRAGANA 1 1 0")
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0];
                    let invoke = parts[1] != "0";
                    let group = parts[2] != "0";
                    let length = parts[3].parse::<u8>().unwrap_or(0);

                    // Get or create category index
                    let cat_idx = *category_map.entry(name.to_string()).or_insert_with(|| {
                        let idx = category_names.len();
                        category_names.push(name.to_string());
                        category_definitions.push(CategoryData {
                            invoke,
                            group,
                            length: length.into(),
                        });
                        idx
                    });

                    // Update category definition if it already exists
                    if cat_idx < category_definitions.len() {
                        category_definitions[cat_idx] = CategoryData {
                            invoke,
                            group,
                            length: length.into(),
                        };
                    }
                }
            }
        }

        // Sort char ranges by start position
        char_ranges.sort_by_key(|&(start, _, _)| start);

        // Build boundaries and mapping function
        let mut boundaries = vec![0u32];
        for &(start, end, _) in &char_ranges {
            if start > boundaries[boundaries.len() - 1] {
                boundaries.push(start);
            }
            boundaries.push(end + 1);
        }
        if boundaries[boundaries.len() - 1] < 0x10FFFF {
            boundaries.push(0x10FFFF);
        }

        // Create lookup table with proper category mappings
        let ranges_clone = char_ranges.clone();
        let mapping = LookupTable::from_fn(boundaries, &|c, buff| {
            let code = c;

            // Find which category this character belongs to
            for &(start, end, cat_idx) in &ranges_clone {
                if code >= start && code <= end {
                    buff.push(CategoryId(cat_idx));
                    return;
                }
            }

            // Default to category 0 (DEFAULT)
            buff.push(CategoryId(0));
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
