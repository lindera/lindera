use regex::Regex;
use std::collections::HashSet;

use crate::feature_csv::{push_column, split_feature};

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

/// Returns the column that one part of a rewrite rule writes.
///
/// # Arguments
///
/// * `rewrite` - The part of the rule.
/// * `features` - The columns of the feature string being rewritten.
///
/// # Returns
///
/// The referenced column, empty when the features have no such column, or
/// the text.
fn rewrite_column<'a, S: AsRef<str>>(rewrite: &'a Rewrite, features: &'a [S]) -> &'a str {
    match rewrite {
        Rewrite::Reference(idx) => features.get(*idx).map_or("", AsRef::as_ref),
        Rewrite::Text(text) => text,
    }
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
    ///
    /// # Arguments
    ///
    /// * `pattern` - Field patterns matched positionally from field 0; `*`
    ///   matches any single field, `A|B` any of the listed values.
    /// * `rewrite` - Replacement fields; `$N` (1-based) copies input field
    ///   `N`, anything else is literal text.
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
            // `$N` is 1-based (MeCab convention). `$0`, or a digit string
            // too long for usize, is out-of-spec input and falls back to
            // literal text rather than panicking (#975).
            let reference = self
                .ref_pattern
                .captures(p)
                .and_then(|cap| cap.get(1))
                .and_then(|digits| digits.as_str().parse::<usize>().ok())
                .and_then(|n| n.checked_sub(1));
            parsed_rewrite.push(match reference {
                Some(idx) => Rewrite::Reference(idx),
                None => Rewrite::Text(p.to_string()),
            });
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
        let rule = self.find_rule(features)?;
        Some(
            rule.iter()
                .map(|rewrite| rewrite_column(rewrite, features).to_string())
                .collect(),
        )
    }

    /// Writes the rewritten features into `out`, joined with `,` and each
    /// written by [`push_column`], if a rule matches; the same as joining
    /// what [`Self::rewrite`] returns, without a `String` per column.
    ///
    /// # Arguments
    ///
    /// * `features` - The columns of the feature string.
    /// * `out` - Cleared and given the rewritten feature string when a rule
    ///   matches; left as it is otherwise.
    ///
    /// # Returns
    ///
    /// Whether a rule matched.
    pub(crate) fn rewrite_into<S>(&self, features: &[S], out: &mut String) -> bool
    where
        S: AsRef<str>,
    {
        let Some(rule) = self.find_rule(features) else {
            return false;
        };
        out.clear();
        for (index, rewrite) in rule.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_column(out, rewrite_column(rewrite, features));
        }
        true
    }

    /// Returns the rule of the first registered pattern that the features
    /// match.
    ///
    /// # Arguments
    ///
    /// * `features` - The columns of the feature string.
    ///
    /// # Returns
    ///
    /// The rule, or `None` if no pattern matches.
    fn find_rule<S>(&self, features: &[S]) -> Option<&[Rewrite]>
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
                    Action::Rewrite(rule) => return Some(rule),
                }
            }
        }
        None
    }

    /// Creates a feature rewriter from a reader.
    ///
    /// The format should be:
    /// Each line contains a pattern and replacement separated by a tab.
    /// Pattern is a list of feature columns, where:
    /// - "*" matches any feature
    /// - "(A|B|C)" matches any of A, B, or C
    /// - "text" matches exactly "text"
    ///
    /// Replacement can contain:
    /// - "$1", "$2", etc. to reference pattern matches
    /// - literal text
    ///
    /// The pattern and the replacement are split into columns as MeCab
    /// splits them, so a quoted column with `,` in it is one column (#1184).
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
                let pattern = split_feature(parts[0]);
                let replacement = split_feature(parts[1]);
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
}

/// The unigram, left and right features that
/// [`DictionaryRewriter::rewrite_into`] writes, kept from one feature to the
/// next.
#[derive(Default)]
pub(crate) struct RewriteBuffers {
    /// The rewritten unigram feature.
    pub(crate) ufeature: String,
    /// The rewritten left feature.
    pub(crate) lfeature: String,
    /// The rewritten right feature.
    pub(crate) rfeature: String,
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
    ///
    /// The pattern and the replacement of a rule are split into columns as
    /// MeCab splits them, so a quoted column with `,` in it is one column
    /// (#1184).
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
                let pattern = split_feature(parts[0]);
                let replacement = split_feature(parts[1].trim());

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
        })
    }

    /// Rewrites input features into three outputs (ufeature, lfeature, rfeature)
    /// without caching.
    ///
    /// Returns the original feature string for any section whose rewriter has no rules
    /// or no matching pattern (passthrough behavior).
    ///
    /// As in MeCab, the feature is split into columns with `tokenizeCSV`'s
    /// rules, and the rewritten columns are joined with `,`, with a column
    /// that contains `,` or `"` quoted (#1184). Split the result into columns
    /// the same way (`feature_csv::split_feature`): a split on `,` would cut
    /// such a column, as in Jieba's quoted definitions.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    ///
    /// # Returns
    ///
    /// The unigram, left and right features.
    pub fn rewrite(&self, feature: &str) -> (String, String, String) {
        let mut buffers = RewriteBuffers::default();
        self.rewrite_into(feature, &mut buffers);
        let RewriteBuffers {
            ufeature,
            lfeature,
            rfeature,
        } = buffers;
        (ufeature, lfeature, rfeature)
    }

    /// Rewrites a feature as [`Self::rewrite`] does, into buffers that the
    /// caller reuses from one feature to the next, so that the rewrite
    /// allocates only when a buffer has to grow (#1286).
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    /// * `buffers` - Given the unigram, left and right features.
    pub(crate) fn rewrite_into(&self, feature: &str, buffers: &mut RewriteBuffers) {
        let fields = split_feature(feature);
        for (rewriter, out) in [
            (&self.unigram_rewriter, &mut buffers.ufeature),
            (&self.left_rewriter, &mut buffers.lfeature),
            (&self.right_rewriter, &mut buffers.rfeature),
        ] {
            if !rewriter.rewrite_into(&fields, out) {
                out.clear();
                out.push_str(feature);
            }
        }
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
    use crate::feature_csv::join_feature;
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

    /// Pins `rewrite` over an ipadic-shaped rewrite.def: three sections,
    /// depth-7/8 patterns, `$N` references, and a passthrough path.
    ///
    /// This replaces the removed `test_dictionary_rewriter_cached`, which
    /// passed even with the cache logic deleted (#975).
    #[test]
    fn test_dictionary_rewriter_ipadic_shape() {
        let rewrite_def = "\
[unigram rewrite]
*,*,*,*,*,*,*,*\t$1,$2,$3,$4,$5,$6,$7,$8

[left rewrite]
名詞,一般,*,*,*,*,*\t$1,$2,$3,$4,$5,$6,NOUN

[right rewrite]
名詞,一般,*,*,*,*,*\t$1,$2,$3,$4,$5,$6,$7
";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        // All three sections match: unigram keeps 8 fields, left replaces
        // field 7 with a literal, right keeps 7 fields (dropping 8..).
        let (u, l, r) = rewriter.rewrite("名詞,一般,*,*,*,*,外食,ガイショク,ガイショク");
        assert_eq!(u, "名詞,一般,*,*,*,*,外食,ガイショク");
        assert_eq!(l, "名詞,一般,*,*,*,*,NOUN");
        assert_eq!(r, "名詞,一般,*,*,*,*,外食");

        // Two entries differing only after the matched prefix must still
        // differ in the sections that reference those fields -- the reason a
        // prefix cache key was rejected in #975.
        let (u2, _, r2) = rewriter.rewrite("名詞,一般,*,*,*,*,会見,カイケン,カイケン");
        assert_ne!(u, u2);
        assert_ne!(r, r2);

        // Passthrough: a feature no left/right rule matches comes back whole.
        let (u3, l3, r3) = rewriter.rewrite("動詞,自立,*,*,*,*,動く,ウゴク,ウゴク");
        assert_eq!(u3, "動詞,自立,*,*,*,*,動く,ウゴク");
        assert_eq!(l3, "動詞,自立,*,*,*,*,動く,ウゴク,ウゴク");
        assert_eq!(r3, "動詞,自立,*,*,*,*,動く,ウゴク,ウゴク");
    }

    /// A `$0` reference is out-of-spec (`$N` is 1-based, per MeCab) and must
    /// be treated as literal text rather than panicking (#975; the previous
    /// parser underflowed on the `- 1`).
    #[test]
    fn test_dollar_zero_is_literal_text() {
        let rewrite_def = "\
[right rewrite]
名詞\t$0,$1
";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let (_, _, r) = rewriter.rewrite("名詞,一般");
        assert_eq!(r, "$0,名詞");
    }

    /// A quoted column with `,` in it is one column, and a rewritten column
    /// with `,` in it is quoted (#1184). The rules and the feature are those
    /// of the issue's reproduction; for them, MeCab's learner gives the
    /// unigram feature `n,dian4,店,店,"shop, store",1`, and `mecab-dict-gen`
    /// writes `"shop, store"` to `left-id.def` and `1` to `right-id.def`.
    #[test]
    fn test_dictionary_rewriter_quoted_column_is_one_column() {
        let rewrite_def = "\
[unigram rewrite]
* $1,$2,$3,$4,$5,$6

[left rewrite]
* $5

[right rewrite]
* $6
";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let (u, l, r) = rewriter.rewrite("n,dian4,店,店,\"shop, store\",1");
        assert_eq!(u, "n,dian4,店,店,\"shop, store\",1");
        assert_eq!(l, "\"shop, store\"");
        assert_eq!(r, "1");
    }

    /// The columns of a rule are split as the feature is, so a pattern can
    /// match a quoted column with `,` in it (#1184).
    #[test]
    fn test_dictionary_rewriter_rule_with_a_quoted_column() {
        let rewrite_def = "[right rewrite]\n*,\"a,b\"\t$1,\"x,y\"\n";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let (_, _, r) = rewriter.rewrite("n,\"a,b\"");
        assert_eq!(r, "n,\"x,y\"");

        // `a` and `b` in two columns do not match the quoted `a,b`.
        let (_, _, r) = rewriter.rewrite("n,a,b");
        assert_eq!(r, "n,a,b");
    }

    #[test]
    fn test_feature_rewriter_rule_with_a_quoted_column() {
        let rewrite_def = "\"a,b\",*\t\"X,Y\",$2\n";
        let rewriter = FeatureRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        let result = rewriter.rewrite(&["a,b", "c"]);
        assert_eq!(result, Some(vec!["X,Y".to_string(), "c".to_string()]));
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

    #[test]
    fn test_feature_rewriter_partial_match_no_hang() {
        // Test that partial match followed by mismatch correctly falls through
        // without infinite loop (regression test for backtracking bug)
        let rewrite_def = "名詞,一般\tNOUN,GENERAL\n";
        let rewriter = FeatureRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        // Exact match: should rewrite
        let result = rewriter.rewrite(&["名詞", "一般"]);
        assert_eq!(
            result,
            Some(vec!["NOUN".to_string(), "GENERAL".to_string()])
        );

        // Partial match at first field, mismatch at second: should return None
        let result = rewriter.rewrite(&["名詞", "代名詞", "一般"]);
        assert_eq!(result, None);

        // No match at all: should return None
        let result = rewriter.rewrite(&["助詞", "係助詞"]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_feature_rewriter_multiple_rules_backtracking() {
        // Multiple rules: first partially matches, second fully matches
        let rewrite_def = "名詞,一般\tNOUN,GENERAL\n名詞,*\tNOUN,$2\n";
        let rewriter = FeatureRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        // First rule matches exactly
        let result = rewriter.rewrite(&["名詞", "一般"]);
        assert_eq!(
            result,
            Some(vec!["NOUN".to_string(), "GENERAL".to_string()])
        );

        // First rule partially matches (名詞 ok, 代名詞 != 一般), falls through to second rule
        let result = rewriter.rewrite(&["名詞", "代名詞"]);
        assert_eq!(result, Some(vec!["NOUN".to_string(), "代名詞".to_string()]));

        // No match at all
        let result = rewriter.rewrite(&["助詞", "係助詞"]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_dictionary_rewriter_partial_match_integration() {
        // Integration test: DictionaryRewriter with partial match inputs
        // This is the exact scenario that caused the hang in Python tests
        let rewrite_def = "名詞,一般\tNOUN,GENERAL\n";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        // Input whose first field matches "名詞" but second is "代名詞" != "一般"
        // Should passthrough (no matching rule)
        let (u, l, r) = rewriter.rewrite("名詞,代名詞,一般,*,*,*,これ,コレ,コレ");
        assert_eq!(u, "名詞,代名詞,一般,*,*,*,これ,コレ,コレ");
        assert_eq!(l, "名詞,代名詞,一般,*,*,*,これ,コレ,コレ");
        assert_eq!(r, "名詞,代名詞,一般,*,*,*,これ,コレ,コレ");

        // Input that fully matches the rule
        // Legacy format: only right rewriter has rules, unigram/left passthrough
        let (u, l, r) = rewriter.rewrite("名詞,一般");
        assert_eq!(u, "名詞,一般");
        assert_eq!(l, "名詞,一般");
        assert_eq!(r, "NOUN,GENERAL");
    }

    /// `rewrite_into` writes what joining the columns of `rewrite` gives, and
    /// leaves the buffer as it is when no rule matches (#1286).
    #[test]
    fn rewrite_into_writes_the_joined_rewrite() {
        // `$3` is past the last column of `a,1`; the second rule writes a
        // quote and an empty column, and copies a column with `,` in it; the
        // third writes text with `,` in it.
        let rewrite_def = "a,*\t$1,$3,$2\nb,*\t\"x\"\"y\",,$2\nc\t\"p,q\"\n";
        let rewriter = FeatureRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();

        for features in [
            vec!["a", "1"],
            vec!["b", "shop, store"],
            vec!["c"],
            vec!["d", "1"],
        ] {
            let expected = rewriter
                .rewrite(&features)
                .map(|columns| join_feature(&columns));
            let mut out = String::from("untouched");
            let matched = rewriter.rewrite_into(&features, &mut out);
            assert_eq!(matched, expected.is_some(), "{features:?}");
            assert_eq!(
                out,
                expected.unwrap_or_else(|| "untouched".to_string()),
                "{features:?}"
            );
        }
        let mut out = String::new();
        rewriter.rewrite_into(&["b", "shop, store"], &mut out);
        assert_eq!(out, "\"x\"\"y\",,\"shop, store\"");
    }

    /// The buffers that `DictionaryRewriter::rewrite_into` reuses hold only
    /// the rewrite of the last feature, however long the one before was,
    /// whether a rule rewrote it or it passed through.
    #[test]
    fn rewrite_into_leaves_nothing_of_the_previous_feature() {
        let rewrite_def = "[unigram rewrite]\n*,*\t$1\n\n[left rewrite]\nlong,*\t$2,$2\n\n[right rewrite]\nlong,*\t$2\n";
        let rewriter =
            DictionaryRewriter::from_reader(Cursor::new(rewrite_def.as_bytes())).unwrap();
        let mut buffers = RewriteBuffers::default();

        rewriter.rewrite_into("long,abcdefghijklmnop", &mut buffers);
        rewriter.rewrite_into("s,t", &mut buffers);

        let RewriteBuffers {
            ufeature,
            lfeature,
            rfeature,
        } = buffers;
        assert_eq!((ufeature, lfeature, rfeature), rewriter.rewrite("s,t"));
        assert_eq!(
            rewriter.rewrite("s,t"),
            ("s".to_string(), "s,t".to_string(), "s,t".to_string())
        );
    }
}
