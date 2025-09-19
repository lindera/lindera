use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Range;

/// Feature type for template parsing
#[derive(Debug, Clone)]
enum FeatureType {
    Index(usize),
    CharacterType,
}

/// Parsed template structure
#[derive(Debug, Clone)]
struct ParsedTemplate {
    raw_template: String,
    required_indices: Vec<usize>,
    captures: Vec<(Range<usize>, FeatureType)>,
}

/// Feature extractor for training with advanced capabilities.
pub struct FeatureExtractor {
    unigram_templates: Vec<ParsedTemplate>,
    left_templates: Vec<ParsedTemplate>,
    right_templates: Vec<ParsedTemplate>,
    pub unigram_feature_ids: HashMap<String, NonZeroU32>,
    pub left_feature_ids: HashMap<String, NonZeroU32>,
    pub right_feature_ids: HashMap<String, NonZeroU32>,
    unigram_next_id: u32,
    left_next_id: u32,
    right_next_id: u32,
}

impl FeatureExtractor {
    /// Creates a new feature extractor with advanced template parsing.
    pub fn new() -> Self {
        Self {
            unigram_templates: Vec::new(),
            left_templates: Vec::new(),
            right_templates: Vec::new(),
            unigram_feature_ids: HashMap::new(),
            left_feature_ids: HashMap::new(),
            right_feature_ids: HashMap::new(),
            unigram_next_id: 0,
            left_next_id: 0,
            right_next_id: 0,
        }
    }

    /// Creates a new feature extractor from templates.
    pub fn from_templates<S>(unigram_templates: &[S], bigram_templates: &[(S, S)]) -> Self
    where
        S: ToString,
    {
        // Regex patterns for advanced feature parsing
        let unigram_feature_pattern = Regex::new(r"%((F|F\?)\[([0-9]+)\]|t)").unwrap();
        let left_feature_pattern = Regex::new(r"%(L|L\?)\[([0-9]+)\]").unwrap();
        let right_feature_pattern = Regex::new(r"%(R|R\?)\[([0-9]+)\]").unwrap();

        // Parse unigram templates
        let mut parsed_unigram_templates = Vec::new();
        for template in unigram_templates {
            let raw_template = template.to_string();
            let mut required_indices = Vec::new();
            let mut captures = Vec::new();

            for m in unigram_feature_pattern.captures_iter(&raw_template) {
                let pattern = m.get(0).unwrap();
                if m.get(1).unwrap().as_str() == "t" {
                    captures.push((pattern.start()..pattern.end(), FeatureType::CharacterType));
                } else {
                    let idx: usize = m.get(3).unwrap().as_str().parse().unwrap();
                    match m.get(2).unwrap().as_str() {
                        "F" => {
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        "F?" => {
                            required_indices.push(idx);
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        _ => unreachable!(),
                    }
                }
            }

            parsed_unigram_templates.push(ParsedTemplate {
                raw_template,
                required_indices,
                captures,
            });
        }

        // Parse bigram templates (left and right)
        let mut parsed_left_templates = Vec::new();
        let mut parsed_right_templates = Vec::new();

        for (left_template, right_template) in bigram_templates {
            // Parse left template
            {
                let raw_template = left_template.to_string();
                let mut required_indices = Vec::new();
                let mut captures = Vec::new();

                for m in left_feature_pattern.captures_iter(&raw_template) {
                    let pattern = m.get(0).unwrap();
                    let idx: usize = m.get(2).unwrap().as_str().parse().unwrap();
                    match m.get(1).unwrap().as_str() {
                        "L" => {
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        "L?" => {
                            required_indices.push(idx);
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        _ => unreachable!(),
                    }
                }

                parsed_left_templates.push(ParsedTemplate {
                    raw_template,
                    required_indices,
                    captures,
                });
            }

            // Parse right template
            {
                let raw_template = right_template.to_string();
                let mut required_indices = Vec::new();
                let mut captures = Vec::new();

                for m in right_feature_pattern.captures_iter(&raw_template) {
                    let pattern = m.get(0).unwrap();
                    let idx: usize = m.get(2).unwrap().as_str().parse().unwrap();
                    match m.get(1).unwrap().as_str() {
                        "R" => {
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        "R?" => {
                            required_indices.push(idx);
                            captures
                                .push((pattern.start()..pattern.end(), FeatureType::Index(idx)));
                        }
                        _ => unreachable!(),
                    }
                }

                parsed_right_templates.push(ParsedTemplate {
                    raw_template,
                    required_indices,
                    captures,
                });
            }
        }

        Self {
            unigram_templates: parsed_unigram_templates,
            left_templates: parsed_left_templates,
            right_templates: parsed_right_templates,
            unigram_feature_ids: HashMap::new(),
            left_feature_ids: HashMap::new(),
            right_feature_ids: HashMap::new(),
            unigram_next_id: 1, // Start from 1 (0 reserved)
            left_next_id: 1,
            right_next_id: 1,
        }
    }

    /// Apply a parsed template to generate feature string
    fn apply_parsed_template(
        &self,
        template: &ParsedTemplate,
        features: &[String],
        cate_id: u32,
    ) -> Option<String> {
        // Check required indices (for conditional features like %F?)
        for &required_idx in &template.required_indices {
            if required_idx >= features.len() {
                return None; // Index out of bounds
            }
            let feature_val = &features[required_idx];
            if feature_val == "*" || feature_val.is_empty() {
                return None; // Skip if required feature is undefined
            }
        }

        let mut result = template.raw_template.clone();

        // Process captures in reverse order to maintain string positions
        for (range, feature_type) in template.captures.iter().rev() {
            let replacement = match feature_type {
                FeatureType::Index(idx) => {
                    if *idx >= features.len() {
                        "*".to_string() // Default for out of bounds
                    } else {
                        features[*idx].clone()
                    }
                }
                FeatureType::CharacterType => {
                    cate_id.to_string() // Use character category ID
                }
            };

            result.replace_range(range.clone(), &replacement);
        }

        Some(result)
    }

    /// Get or create feature ID (with NonZeroU32)
    fn get_or_create_unigram_feature_id(&mut self, feature_str: &str) -> NonZeroU32 {
        if let Some(&id) = self.unigram_feature_ids.get(feature_str) {
            id
        } else {
            let new_id = NonZeroU32::new(self.unigram_next_id).unwrap();
            let feature_id = *self
                .unigram_feature_ids
                .entry(feature_str.to_string())
                .or_insert(new_id);
            if new_id == feature_id {
                self.unigram_next_id += 1;
            }
            feature_id
        }
    }

    fn get_or_create_left_feature_id(&mut self, feature_str: &str) -> Option<NonZeroU32> {
        let new_id = NonZeroU32::new(self.left_next_id).unwrap();
        let feature_id = *self
            .left_feature_ids
            .entry(feature_str.to_string())
            .or_insert(new_id);
        if new_id == feature_id {
            self.left_next_id += 1;
        }
        Some(feature_id)
    }

    fn get_or_create_right_feature_id(&mut self, feature_str: &str) -> Option<NonZeroU32> {
        let new_id = NonZeroU32::new(self.right_next_id).unwrap();
        let feature_id = *self
            .right_feature_ids
            .entry(feature_str.to_string())
            .or_insert(new_id);
        if new_id == feature_id {
            self.right_next_id += 1;
        }
        Some(feature_id)
    }

    /// Extracts unigram feature IDs from features.
    pub fn extract_unigram_feature_ids(
        &mut self,
        features: &[String],
        cate_id: u32,
    ) -> Vec<NonZeroU32> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.unigram_templates.clone();
        for template in templates {
            if let Some(feature_str) = self.apply_parsed_template(&template, features, cate_id) {
                let id = self.get_or_create_unigram_feature_id(&feature_str);
                feature_ids.push(id);
            }
        }

        feature_ids
    }

    /// Extracts left context feature IDs from features (with Optional).
    pub fn extract_left_feature_ids(&mut self, features: &[String]) -> Vec<Option<NonZeroU32>> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.left_templates.clone();
        for template in templates {
            if let Some(feature_str) = self.apply_parsed_template(&template, features, 0) {
                let id = self.get_or_create_left_feature_id(&feature_str);
                feature_ids.push(id);
            } else {
                feature_ids.push(None); // Handle undefined features
            }
        }

        feature_ids
    }

    /// Extracts right context feature IDs from features (with Optional).
    pub fn extract_right_feature_ids(&mut self, features: &[String]) -> Vec<Option<NonZeroU32>> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.right_templates.clone();
        for template in templates {
            if let Some(feature_str) = self.apply_parsed_template(&template, features, 0) {
                let id = self.get_or_create_right_feature_id(&feature_str);
                feature_ids.push(id);
            } else {
                feature_ids.push(None); // Handle undefined features
            }
        }

        feature_ids
    }
}
