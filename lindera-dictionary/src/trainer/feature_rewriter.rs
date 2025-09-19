use regex::Regex;
use std::collections::HashSet;

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
                if let Action::Transition(edge) = action {
                    if parsed == edge.pattern {
                        cursor = edge.target;
                        continue 'a;
                    }
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
                                stack.push((node_idx, i));
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
