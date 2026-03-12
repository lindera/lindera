use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pattern {
    Any,
    Exact(String),
    Multiple(HashSet<String>),
}

#[derive(Debug, Clone)]
enum Rewrite {
    Reference(usize),
    Text(String),
}

#[derive(Debug, Clone)]
struct Edge {
    pattern: Pattern,
    target: usize,
}

#[derive(Debug, Clone)]
enum Action {
    Transition(Edge),
    Rewrite(Vec<Rewrite>),
}

#[derive(Debug, Clone, Default)]
struct Node {
    actions: Vec<Action>,
}

/// Feature rewriter builder for constructing a prefix trie of rewrite patterns.
pub struct FeatureRewriterBuilder {
    nodes: Vec<Node>,
    ref_pattern: Regex,
}

impl Default for FeatureRewriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureRewriterBuilder {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::default()],
            ref_pattern: Regex::new(r"^\$([0-9]+)$").unwrap(),
        }
    }

    /// Adds the rewrite rule associated with the pattern.
    /// If the pattern is shorter than the rewrite rule,
    /// the remainings are automatically padded with "*".
    pub fn add_rule<S>(&mut self, pattern: &[S], rewrite: &[S])
    where
        S: AsRef<str>,
    {
        let mut cursor = 0;
        'a: for p in pattern {
            let p = p.as_ref();
            let parsed = if p == "*" {
                Pattern::Any
            } else if p.starts_with('(') && p.ends_with(')') {
                let mut s = HashSet::new();
                for t in p[1..p.len() - 1].split('|') {
                    s.insert(t.to_string());
                }
                Pattern::Multiple(s)
            } else {
                Pattern::Exact(p.to_string())
            };
            for action in &self.nodes[cursor].actions {
                if let Action::Transition(edge) = action
                    && parsed == edge.pattern
                {
                    cursor = edge.target;
                    continue 'a;
                }
            }
            let target = self.nodes.len();
            self.nodes[cursor].actions.push(Action::Transition(Edge {
                pattern: parsed,
                target,
            }));
            self.nodes.push(Node::default());
            cursor = target;
        }
        let mut parsed_rewrite = vec![];
        for p in rewrite {
            let p = p.as_ref();
            parsed_rewrite.push(self.ref_pattern.captures(p).map_or_else(
                || Rewrite::Text(p.to_string()),
                |cap| {
                    let idx = cap.get(1).unwrap().as_str().parse::<usize>().unwrap() - 1;
                    Rewrite::Reference(idx)
                },
            ));
        }
        self.nodes[cursor]
            .actions
            .push(Action::Rewrite(parsed_rewrite));
    }
}

/// Feature rewriter for training.
pub struct FeatureRewriter {
    nodes: Vec<Node>,
}

impl From<FeatureRewriterBuilder> for FeatureRewriter {
    fn from(builder: FeatureRewriterBuilder) -> Self {
        Self {
            nodes: builder.nodes,
        }
    }
}

impl Default for FeatureRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureRewriter {
    /// Creates a new feature rewriter.
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::default()],
        }
    }

    /// Returns the rewritten features if matched.
    /// If multiple patterns are matched, the earlier registered one is applied.
    pub fn rewrite<S>(&self, features: &[S]) -> Option<Vec<String>>
    where
        S: AsRef<str>,
    {
        let mut stack = vec![(0, 0)];
        'a: while let Some((node_idx, edge_idx)) = stack.pop() {
            for (i, action) in self.nodes[node_idx]
                .actions
                .iter()
                .enumerate()
                .skip(edge_idx)
            {
                match action {
                    Action::Transition(edge) => {
                        if let Some(f) = features.get(stack.len()) {
                            let f = f.as_ref();
                            let is_match = match &edge.pattern {
                                Pattern::Any => true,
                                Pattern::Multiple(s) => s.contains(f),
                                Pattern::Exact(s) => f == s,
                            };
                            if is_match {
                                stack.push((node_idx, i + 1));
                                stack.push((edge.target, 0));
                                continue 'a;
                            }
                        }
                    }
                    Action::Rewrite(rule) => {
                        let mut result = Vec::with_capacity(rule.len());
                        for rewrite in rule {
                            match rewrite {
                                Rewrite::Reference(idx) => {
                                    if let Some(f) = features.get(*idx) {
                                        result.push(f.as_ref().to_string());
                                    } else {
                                        result.push(String::new());
                                    }
                                }
                                Rewrite::Text(text) => {
                                    result.push(text.clone());
                                }
                            }
                        }
                        return Some(result);
                    }
                }
            }
        }
        None
    }

    /// Creates a feature rewriter from a reader.
    ///
    /// The format should be:
    /// Each line contains a pattern and replacement separated by a tab.
    /// Pattern can be comma-separated list of features, where:
    /// - "*" matches any feature
    /// - "(A|B|C)" matches any of A, B, or C
    /// - "text" matches exactly "text"
    ///
    /// Replacement can contain:
    /// - "$1", "$2", etc. to reference pattern matches
    /// - literal text
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        use std::io::{BufRead, BufReader};

        let mut builder = FeatureRewriterBuilder::new();
        let reader = BufReader::new(reader);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let pattern: Vec<&str> = parts[0].split(',').collect();
                let replacement: Vec<&str> = parts[1].split(',').collect();
                builder.add_rule(&pattern, &replacement);
            }
        }

        Ok(FeatureRewriter::from(builder))
    }
}

/// MeCab-compatible dictionary rewriter that handles three rewrite sections:
/// `[unigram rewrite]`, `[left rewrite]`, and `[right rewrite]`.
///
/// Each section contains rewrite rules that transform input features into
/// different representations for unigram features, left context IDs,
/// and right context IDs respectively.
pub struct DictionaryRewriter {
    unigram_rewriter: FeatureRewriter,
    left_rewriter: FeatureRewriter,
    right_rewriter: FeatureRewriter,
    cache: HashMap<String, (String, String, String)>,
}

impl Default for DictionaryRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryRewriter {
    /// Creates a new empty dictionary rewriter.
    pub fn new() -> Self {
        Self {
            unigram_rewriter: FeatureRewriter::new(),
            left_rewriter: FeatureRewriter::new(),
            right_rewriter: FeatureRewriter::new(),
            cache: HashMap::new(),
        }
    }

    /// Creates a dictionary rewriter from a reader containing rewrite.def content.
    ///
    /// The file format follows MeCab's rewrite.def:
    /// ```text
    /// [unigram rewrite]
    /// pattern1,pattern2\t$1,$2
    /// ...
    /// [left rewrite]
    /// pattern1,pattern2\t$1
    /// ...
    /// [right rewrite]
    /// pattern1,pattern2\t$1
    /// ...
    /// ```
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        use std::io::{BufRead, BufReader};

        let mut unigram_builder = FeatureRewriterBuilder::new();
        let mut left_builder = FeatureRewriterBuilder::new();
        let mut right_builder = FeatureRewriterBuilder::new();

        // 0 = no section yet, 1 = unigram, 2 = left, 3 = right
        let mut current_section = 0u8;

        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for section markers
            match trimmed {
                "[unigram rewrite]" => {
                    current_section = 1;
                    continue;
                }
                "[left rewrite]" => {
                    current_section = 2;
                    continue;
                }
                "[right rewrite]" => {
                    current_section = 3;
                    continue;
                }
                _ => {}
            }

            if current_section == 0 {
                // No section header found yet; treat as legacy single-section format
                // for backward compatibility with existing Lindera rewrite.def files
                current_section = 3; // default to right rewrite
            }

            // Parse rewrite rule: pattern and replacement separated by tab or whitespace
            let parts: Vec<&str> = trimmed.splitn(2, ['\t', ' ']).collect();
            if parts.len() >= 2 {
                let pattern: Vec<&str> = parts[0].split(',').collect();
                let replacement_str = parts[1].trim();
                let replacement: Vec<&str> = replacement_str.split(',').collect();

                let builder = match current_section {
                    1 => &mut unigram_builder,
                    2 => &mut left_builder,
                    3 => &mut right_builder,
                    _ => unreachable!(),
                };
                builder.add_rule(&pattern, &replacement);
            }
        }

        Ok(Self {
            unigram_rewriter: FeatureRewriter::from(unigram_builder),
            left_rewriter: FeatureRewriter::from(left_builder),
            right_rewriter: FeatureRewriter::from(right_builder),
            cache: HashMap::new(),
        })
    }

    /// Rewrites input features into three outputs (ufeature, lfeature, rfeature)
    /// without caching.
    ///
    /// Returns the original feature string for any section whose rewriter has no rules
    /// or no matching pattern (passthrough behavior).
    pub fn rewrite(&self, feature: &str) -> (String, String, String) {
        let fields: Vec<&str> = feature.split(',').collect();

        let ufeature = self
            .unigram_rewriter
            .rewrite(&fields)
            .map(|v| v.join(","))
            .unwrap_or_else(|| feature.to_string());

        let lfeature = self
            .left_rewriter
            .rewrite(&fields)
            .map(|v| v.join(","))
            .unwrap_or_else(|| feature.to_string());

        let rfeature = self
            .right_rewriter
            .rewrite(&fields)
            .map(|v| v.join(","))
            .unwrap_or_else(|| feature.to_string());

        (ufeature, lfeature, rfeature)
    }

    /// Rewrites input features with caching (MeCab's rewrite2 equivalent).
    pub fn rewrite_cached(&mut self, feature: &str) -> (String, String, String) {
        if let Some(cached) = self.cache.get(feature) {
            return cached.clone();
        }

        let result = self.rewrite(feature);
        self.cache.insert(feature.to_string(), result.clone());
        result
    }

    /// Clears the rewrite cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Returns the unigram rewriter (for direct access when needed).
    pub fn unigram_rewriter(&self) -> &FeatureRewriter {
        &self.unigram_rewriter
    }

    /// Returns the left rewriter (for direct access when needed).
    pub fn left_rewriter(&self) -> &FeatureRewriter {
        &self.left_rewriter
    }

    /// Returns the right rewriter (for direct access when needed).
    pub fn right_rewriter(&self) -> &FeatureRewriter {
        &self.right_rewriter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_dictionary_rewriter_three_sections() {
        let rewrite_def = "\
[unigram rewrite]
*,*,*,*,*,*\t$1,$2,$3,$4,$5,$6

[left rewrite]
*,*,*,*,*,*\t$1,$2

[right rewrite]
*,*,*,*,*,*\t$1,$2,$3
";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let (u, l, r) = rewriter.rewrite("名詞,一般,*,*,*,*");
        assert_eq!(u, "名詞,一般,*,*,*,*");
        assert_eq!(l, "名詞,一般");
        assert_eq!(r, "名詞,一般,*");
    }

    #[test]
    fn test_dictionary_rewriter_cached() {
        let rewrite_def = "\
[unigram rewrite]
*\t$1

[left rewrite]
*\t$1

[right rewrite]
*\t$1
";
        let mut rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let result1 = rewriter.rewrite_cached("名詞");
        let result2 = rewriter.rewrite_cached("名詞");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_dictionary_rewriter_empty() {
        let rewriter = DictionaryRewriter::new();
        // Empty rewriter should passthrough
        let (u, l, r) = rewriter.rewrite("名詞,一般");
        assert_eq!(u, "名詞,一般");
        assert_eq!(l, "名詞,一般");
        assert_eq!(r, "名詞,一般");
    }

    #[test]
    fn test_dictionary_rewriter_legacy_format() {
        // Legacy format without section headers (treated as right rewrite)
        let rewrite_def = "*,*,*,*\t$1,$2\n";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let (u, l, r) = rewriter.rewrite("名詞,一般,*,*");
        // unigram/left: passthrough (no rules)
        assert_eq!(u, "名詞,一般,*,*");
        assert_eq!(l, "名詞,一般,*,*");
        // right: rewritten
        assert_eq!(r, "名詞,一般");
    }
}
