use std::collections::HashMap;

/// Feature rewriter for training.
pub struct FeatureRewriter {
    rewrite_rules: HashMap<String, Vec<String>>,
}

impl FeatureRewriter {
    /// Creates a new feature rewriter.
    pub fn new() -> Self {
        Self {
            rewrite_rules: HashMap::new(),
        }
    }

    /// Adds a rewrite rule.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to match in features
    /// * `replacement` - The replacement features
    pub fn add_rule(&mut self, pattern: String, replacement: Vec<String>) {
        self.rewrite_rules.insert(pattern, replacement);
    }

    /// Rewrites features based on the rules.
    ///
    /// Returns Some(rewritten_features) if a rule matches, None otherwise.
    pub fn rewrite(&self, features: &[String]) -> Option<Vec<String>> {
        // Check if any feature matches a rewrite rule
        for feature in features {
            if let Some(replacement) = self.rewrite_rules.get(feature) {
                return Some(replacement.clone());
            }
        }

        // Check if the concatenated features match a rule
        let concatenated = features.join(",");
        if let Some(replacement) = self.rewrite_rules.get(&concatenated) {
            return Some(replacement.clone());
        }

        None
    }

    /// Creates a feature rewriter from a reader.
    ///
    /// The format should be:
    /// Each line contains a pattern and replacement separated by a tab.
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        use std::io::{BufRead, BufReader};

        let mut rewriter = Self::new();
        let reader = BufReader::new(reader);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let pattern = parts[0].to_string();
                let replacement: Vec<String> = parts[1].split(',').map(|s| s.to_string()).collect();
                rewriter.add_rule(pattern, replacement);
            }
        }

        Ok(rewriter)
    }

    /// Rewrites feature IDs based on the rules.
    ///
    /// This method is used during training to transform feature IDs
    /// according to the configured rewrite rules.
    pub fn rewrite_features(&self, feature_ids: &[u32]) -> Vec<u32> {
        // For now, return the feature IDs unchanged
        // In a complete implementation, this would map IDs to string features,
        // apply rewrite rules, and convert back to IDs
        feature_ids.to_vec()
    }
}