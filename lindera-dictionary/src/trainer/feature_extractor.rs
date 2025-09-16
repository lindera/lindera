use std::collections::HashMap;

/// Feature extractor for training.
pub struct FeatureExtractor {
    unigram_templates: Vec<String>,
    left_templates: Vec<String>,
    right_templates: Vec<String>,
    feature_id_map: HashMap<String, u32>,
    next_feature_id: u32,
}

impl FeatureExtractor {
    /// Creates a new feature extractor.
    pub fn new() -> Self {
        Self {
            unigram_templates: Vec::new(),
            left_templates: Vec::new(),
            right_templates: Vec::new(),
            feature_id_map: HashMap::new(),
            next_feature_id: 0,
        }
    }

    /// Adds a unigram feature template.
    pub fn add_unigram_template(&mut self, template: String) {
        self.unigram_templates.push(template);
    }

    /// Adds a left context feature template.
    pub fn add_left_template(&mut self, template: String) {
        self.left_templates.push(template);
    }

    /// Adds a right context feature template.
    pub fn add_right_template(&mut self, template: String) {
        self.right_templates.push(template);
    }

    /// Extracts unigram feature IDs from features.
    pub fn extract_unigram_feature_ids(&mut self, features: &[String], cate_id: u32) -> Vec<u32> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.unigram_templates.clone();
        for template in templates {
            let feature_str = self.apply_template(&template, features, cate_id);
            let id = self.get_or_create_feature_id(&feature_str);
            feature_ids.push(id);
        }

        feature_ids
    }

    /// Extracts left context feature IDs from features.
    pub fn extract_left_feature_ids(&mut self, features: &[String]) -> Vec<u32> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.left_templates.clone();
        for template in templates {
            let feature_str = self.apply_template(&template, features, 0);
            let id = self.get_or_create_feature_id(&feature_str);
            feature_ids.push(id);
        }

        feature_ids
    }

    /// Extracts right context feature IDs from features.
    pub fn extract_right_feature_ids(&mut self, features: &[String]) -> Vec<u32> {
        let mut feature_ids = Vec::new();

        // Clone templates to avoid borrow conflicts
        let templates = self.right_templates.clone();
        for template in templates {
            let feature_str = self.apply_template(&template, features, 0);
            let id = self.get_or_create_feature_id(&feature_str);
            feature_ids.push(id);
        }

        feature_ids
    }

    /// Applies a template to features to generate a feature string.
    fn apply_template(&self, template: &str, features: &[String], cate_id: u32) -> String {
        // This is a simplified implementation
        // In the actual implementation, we need to parse the template and extract features accordingly
        // For now, just return a combination of template and first feature
        if features.is_empty() {
            format!("{}:{}", template, cate_id)
        } else {
            format!("{}:{}:{}", template, features[0], cate_id)
        }
    }

    /// Gets or creates a feature ID for a feature string.
    fn get_or_create_feature_id(&mut self, feature_str: &str) -> u32 {
        if let Some(&id) = self.feature_id_map.get(feature_str) {
            id
        } else {
            let id = self.next_feature_id;
            self.feature_id_map.insert(feature_str.to_string(), id);
            self.next_feature_id += 1;
            id
        }
    }
}