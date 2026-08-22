use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::LazyLock;

/// Feature template patterns, compiled once on first use.
///
/// `%F[n]`, `%F?[n]`, `%t`, `%w` (surface), `%u` (all ufeature)
static UNIGRAM_FEATURE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%((F|F\?)\[([0-9]+)\]|t|w|u)").expect("valid unigram regex"));
/// `%L[n]`, `%L?[n]`, `%l` (all lfeature), `%r` (all rfeature)
static LEFT_FEATURE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%((L|L\?)\[([0-9]+)\]|l|r)").expect("valid left regex"));
static RIGHT_FEATURE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%((R|R\?)\[([0-9]+)\]|l|r)").expect("valid right regex"));

/// Feature type for template parsing
#[derive(Debug, Clone)]
enum FeatureType {
    Index(usize),
    CharacterType,
    /// %w — surface form (unigram only)
    SurfaceForm,
    /// %u — full ufeature string (unigram only)
    AllUnigramFeature,
    /// %l — full lfeature string (bigram left)
    AllLeftFeature,
    /// %r — full rfeature string (bigram right)
    AllRightFeature,
}

/// Parsed template structure
#[derive(Debug, Clone)]
struct ParsedTemplate {
    raw_template: String,
    required_indices: Vec<usize>,
    captures: Vec<(Range<usize>, FeatureType)>,
}

/// Context for template application, providing additional information
/// for meta characters like %w, %u, %l, %r.
#[derive(Debug, Clone, Default)]
pub struct TemplateContext<'a> {
    /// Surface form (%w)
    pub surface: Option<&'a str>,
    /// Full ufeature string (%u)
    pub ufeature: Option<&'a str>,
    /// Full lfeature string (%l) — used in bigram left template
    pub lfeature: Option<&'a str>,
    /// Full rfeature string (%r) — used in bigram right template
    pub rfeature: Option<&'a str>,
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

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
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
        // Parse unigram templates
        let mut parsed_unigram_templates = Vec::new();
        for template in unigram_templates {
            let raw_template = template.to_string();
            let mut required_indices = Vec::new();
            let mut captures = Vec::new();

            for m in UNIGRAM_FEATURE_PATTERN.captures_iter(&raw_template) {
                let pattern = m.get(0).unwrap();
                let matched = m.get(1).unwrap().as_str();
                match matched {
                    "t" => {
                        captures.push((pattern.start()..pattern.end(), FeatureType::CharacterType));
                    }
                    "w" => {
                        captures.push((pattern.start()..pattern.end(), FeatureType::SurfaceForm));
                    }
                    "u" => {
                        captures.push((
                            pattern.start()..pattern.end(),
                            FeatureType::AllUnigramFeature,
                        ));
                    }
                    _ => {
                        let idx: usize = m.get(3).unwrap().as_str().parse().unwrap();
                        match m.get(2).unwrap().as_str() {
                            "F" => {
                                captures.push((
                                    pattern.start()..pattern.end(),
                                    FeatureType::Index(idx),
                                ));
                            }
                            "F?" => {
                                required_indices.push(idx);
                                captures.push((
                                    pattern.start()..pattern.end(),
                                    FeatureType::Index(idx),
                                ));
                            }
                            _ => unreachable!(),
                        }
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

                for m in LEFT_FEATURE_PATTERN.captures_iter(&raw_template) {
                    let pattern = m.get(0).unwrap();
                    let matched = m.get(1).unwrap().as_str();
                    match matched {
                        "l" => {
                            captures.push((
                                pattern.start()..pattern.end(),
                                FeatureType::AllLeftFeature,
                            ));
                        }
                        "r" => {
                            captures.push((
                                pattern.start()..pattern.end(),
                                FeatureType::AllRightFeature,
                            ));
                        }
                        _ => {
                            let idx: usize = m.get(3).unwrap().as_str().parse().unwrap();
                            match m.get(2).unwrap().as_str() {
                                "L" => {
                                    captures.push((
                                        pattern.start()..pattern.end(),
                                        FeatureType::Index(idx),
                                    ));
                                }
                                "L?" => {
                                    required_indices.push(idx);
                                    captures.push((
                                        pattern.start()..pattern.end(),
                                        FeatureType::Index(idx),
                                    ));
                                }
                                _ => unreachable!(),
                            }
                        }
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

                for m in RIGHT_FEATURE_PATTERN.captures_iter(&raw_template) {
                    let pattern = m.get(0).unwrap();
                    let matched = m.get(1).unwrap().as_str();
                    match matched {
                        "l" => {
                            captures.push((
                                pattern.start()..pattern.end(),
                                FeatureType::AllLeftFeature,
                            ));
                        }
                        "r" => {
                            captures.push((
                                pattern.start()..pattern.end(),
                                FeatureType::AllRightFeature,
                            ));
                        }
                        _ => {
                            let idx: usize = m.get(3).unwrap().as_str().parse().unwrap();
                            match m.get(2).unwrap().as_str() {
                                "R" => {
                                    captures.push((
                                        pattern.start()..pattern.end(),
                                        FeatureType::Index(idx),
                                    ));
                                }
                                "R?" => {
                                    required_indices.push(idx);
                                    captures.push((
                                        pattern.start()..pattern.end(),
                                        FeatureType::Index(idx),
                                    ));
                                }
                                _ => unreachable!(),
                            }
                        }
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
        template: &ParsedTemplate,
        features: &[String],
        cate_id: u32,
        ctx: &TemplateContext,
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
                FeatureType::CharacterType => cate_id.to_string(),
                FeatureType::SurfaceForm => ctx.surface.unwrap_or("").to_string(),
                FeatureType::AllUnigramFeature => ctx.ufeature.unwrap_or("").to_string(),
                FeatureType::AllLeftFeature => ctx.lfeature.unwrap_or("").to_string(),
                FeatureType::AllRightFeature => ctx.rfeature.unwrap_or("").to_string(),
            };

            result.replace_range(range.clone(), &replacement);
        }

        Some(result)
    }

    /// Get or create feature ID (with NonZeroU32)
    /// Interns `feature_str` in `ids`, minting the next id from `next_id` on
    /// a miss.
    ///
    /// Takes the map and the counter as separate arguments rather than
    /// `&mut self` so the callers can hold a shared borrow of their template
    /// vector at the same time; that disjointness is what lets the extract
    /// loops iterate the templates in place instead of cloning them (#965).
    ///
    /// The `get` lookup on the hit path matters: the `entry` API needs an
    /// owned key, so going straight to it allocated a `String` on *every*
    /// call, and the ids are interned precisely because the same feature
    /// strings recur across lexicon entries.
    ///
    /// # 引数
    ///
    /// * `ids` - The feature-string to id map to intern into.
    /// * `next_id` - The counter for this id space; incremented on a miss.
    /// * `feature_str` - The generated feature string to intern.
    ///
    /// # 戻り値
    ///
    /// The existing id, or the newly minted one.
    fn intern_feature_id(
        ids: &mut HashMap<String, NonZeroU32>,
        next_id: &mut u32,
        feature_str: &str,
    ) -> NonZeroU32 {
        if let Some(&id) = ids.get(feature_str) {
            return id;
        }
        // `from_templates` starts the counters at 1, but `new()` starts them
        // at 0, where `NonZeroU32::new` yields `None`. The retired helpers
        // called `.unwrap()` there and would have panicked; clamping to 1
        // instead keeps the production path free of `unwrap()` per CLAUDE.md.
        // Advancing from the id actually minted -- rather than from
        // `*next_id` -- is what keeps ids distinct in that case, instead of
        // handing out 1 twice. Unreachable today (the template vectors are
        // private and only `from_templates` fills them, so a `new()`-built
        // extractor has nothing to iterate), but the helper is correct for
        // any starting value rather than relying on that.
        let new_id = NonZeroU32::new(*next_id).unwrap_or(NonZeroU32::MIN);
        ids.insert(feature_str.to_string(), new_id);
        *next_id = new_id.get().saturating_add(1);
        new_id
    }

    /// Extracts unigram feature IDs from features.
    pub fn extract_unigram_feature_ids(
        &mut self,
        features: &[String],
        cate_id: u32,
    ) -> Vec<NonZeroU32> {
        self.extract_unigram_feature_ids_with_ctx(features, cate_id, &TemplateContext::default())
    }

    /// Extracts unigram feature IDs from features with template context.
    pub fn extract_unigram_feature_ids_with_ctx(
        &mut self,
        features: &[String],
        cate_id: u32,
        ctx: &TemplateContext,
    ) -> Vec<NonZeroU32> {
        // Destructured so the template borrow and the id-map borrow are
        // disjoint; iterating `self.unigram_templates` directly while calling
        // a `&mut self` method is what forced the per-call clone this
        // replaces (#965).
        let Self {
            unigram_templates,
            unigram_feature_ids,
            unigram_next_id,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(unigram_templates.len());
        for template in unigram_templates.iter() {
            // A template whose `%F?` field is undefined is skipped entirely,
            // so the result is shorter than the template list. The left/right
            // variants below deliberately differ.
            if let Some(feature_str) = Self::apply_parsed_template(template, features, cate_id, ctx)
            {
                feature_ids.push(Self::intern_feature_id(
                    unigram_feature_ids,
                    unigram_next_id,
                    &feature_str,
                ));
            }
        }

        feature_ids
    }

    /// Extracts left context feature IDs from features (with Optional).
    pub fn extract_left_feature_ids(&mut self, features: &[String]) -> Vec<Option<NonZeroU32>> {
        self.extract_left_feature_ids_with_ctx(features, &TemplateContext::default())
    }

    /// Extracts left context feature IDs from features with template context.
    pub fn extract_left_feature_ids_with_ctx(
        &mut self,
        features: &[String],
        ctx: &TemplateContext,
    ) -> Vec<Option<NonZeroU32>> {
        // See `extract_unigram_feature_ids_with_ctx` for why this is
        // destructured rather than iterating through `self` (#965).
        let Self {
            left_templates,
            left_feature_ids,
            left_next_id,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(left_templates.len());
        for template in left_templates.iter() {
            // Unlike unigram extraction, a skipped template pushes `None`
            // rather than being dropped, so the result stays index-aligned
            // with the template list. Callers depend on that alignment.
            if let Some(feature_str) = Self::apply_parsed_template(template, features, 0, ctx) {
                feature_ids.push(Some(Self::intern_feature_id(
                    left_feature_ids,
                    left_next_id,
                    &feature_str,
                )));
            } else {
                feature_ids.push(None);
            }
        }

        feature_ids
    }

    /// Extracts right context feature IDs from features (with Optional).
    pub fn extract_right_feature_ids(&mut self, features: &[String]) -> Vec<Option<NonZeroU32>> {
        self.extract_right_feature_ids_with_ctx(features, &TemplateContext::default())
    }

    /// Extracts right context feature IDs from features with template context.
    pub fn extract_right_feature_ids_with_ctx(
        &mut self,
        features: &[String],
        ctx: &TemplateContext,
    ) -> Vec<Option<NonZeroU32>> {
        // See `extract_unigram_feature_ids_with_ctx` for why this is
        // destructured rather than iterating through `self` (#965).
        let Self {
            right_templates,
            right_feature_ids,
            right_next_id,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(right_templates.len());
        for template in right_templates.iter() {
            // Unlike unigram extraction, a skipped template pushes `None`
            // rather than being dropped, so the result stays index-aligned
            // with the template list. Callers depend on that alignment.
            if let Some(feature_str) = Self::apply_parsed_template(template, features, 0, ctx) {
                feature_ids.push(Some(Self::intern_feature_id(
                    right_feature_ids,
                    right_next_id,
                    &feature_str,
                )));
            } else {
                feature_ids.push(None);
            }
        }

        feature_ids
    }
}

// These tests pin the behavior that the exported `left-id.def` / `right-id.def`
// depend on: which feature strings are generated, in what order, and how ids
// are minted. Feature ids are assigned sequentially in template iteration
// order and written out verbatim by `Model::write_left_id_def` /
// `write_right_id_def`, so an ordering or off-by-one slip would silently
// change a trained dictionary without failing anything. There was no coverage
// here before (#965).
#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an extractor from `&str` templates. Bigram templates are given
    /// as `(left, right)` pairs, matching `TrainerConfig`'s split on `/`.
    fn extractor(unigram: &[&str], bigram: &[(&str, &str)]) -> FeatureExtractor {
        FeatureExtractor::from_templates(unigram, bigram)
    }

    fn features(fields: &[&str]) -> Vec<String> {
        fields.iter().map(|f| f.to_string()).collect()
    }

    /// The ids an extractor hands out are 1-based and assigned in template
    /// order, so the generated strings can be recovered by inverting the map.
    fn unigram_strings_in_id_order(fe: &FeatureExtractor) -> Vec<String> {
        let mut pairs: Vec<(u32, &String)> = fe
            .unigram_feature_ids
            .iter()
            .map(|(k, v)| (v.get(), k))
            .collect();
        pairs.sort_by_key(|(id, _)| *id);
        pairs.into_iter().map(|(_, k)| k.clone()).collect()
    }

    /// Unigram templates are applied in declaration order, and each distinct
    /// generated string is interned with the next sequential id starting at 1.
    #[test]
    fn unigram_ids_follow_template_order() {
        let mut fe = extractor(&["U0:%F[0]", "U1:%F[1]", "U2:%F[0],%F[1]"], &[]);

        let ids = fe.extract_unigram_feature_ids(&features(&["名詞", "一般"]), 0);

        assert_eq!(
            ids.iter().map(|i| i.get()).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            unigram_strings_in_id_order(&fe),
            vec!["U0:名詞", "U1:一般", "U2:名詞,一般"]
        );
    }

    /// `%F?[n]` marks the index required: when that field is `*` or empty the
    /// template is skipped entirely. Unigram extraction **drops** the skipped
    /// template rather than pushing a placeholder, so the returned vector is
    /// shorter than the template list.
    #[test]
    fn unigram_skips_templates_whose_required_field_is_undefined() {
        let mut fe = extractor(&["U0:%F[0]", "U1:%F?[1]", "U2:%F[0],%F?[1]"], &[]);

        // Field 1 is "*", so both templates that require it are skipped.
        let ids = fe.extract_unigram_feature_ids(&features(&["名詞", "*"]), 0);

        assert_eq!(ids.len(), 1, "only U0 should survive");
        assert_eq!(unigram_strings_in_id_order(&fe), vec!["U0:名詞"]);

        // An empty field is treated the same way as "*".
        let mut fe = extractor(&["U0:%F[0]", "U1:%F?[1]"], &[]);
        let ids = fe.extract_unigram_feature_ids(&features(&["名詞", ""]), 0);
        assert_eq!(ids.len(), 1);

        // A required index past the end of the feature vector also skips.
        let mut fe = extractor(&["U0:%F[0]", "U1:%F?[5]"], &[]);
        let ids = fe.extract_unigram_feature_ids(&features(&["名詞"]), 0);
        assert_eq!(ids.len(), 1);
    }

    /// Left and right extraction **push `None`** for a skipped template
    /// instead of dropping it, so the returned vector stays index-aligned with
    /// the template list. This asymmetry with unigram extraction is
    /// load-bearing and must not be "unified".
    #[test]
    fn left_and_right_push_none_for_skipped_templates() {
        let mut fe = extractor(&[], &[("L0:%L[0]", "R0:%R[0]"), ("L1:%L?[1]", "R1:%R?[1]")]);

        let left = fe.extract_left_feature_ids(&features(&["名詞", "*"]));
        let right = fe.extract_right_feature_ids(&features(&["名詞", "*"]));

        assert_eq!(left.len(), 2, "index alignment with the template list");
        assert!(left[0].is_some());
        assert!(
            left[1].is_none(),
            "the %L? template must yield None, not be dropped"
        );

        assert_eq!(right.len(), 2);
        assert!(right[0].is_some());
        assert!(right[1].is_none());
    }

    /// An already-interned string returns its existing id and does **not**
    /// bump the counter, so ids stay dense across repeated calls. This is the
    /// `new_id == feature_id` condition inside the `get_or_create_*` helpers.
    #[test]
    fn repeated_calls_reuse_ids_without_advancing_the_counter() {
        let mut fe = extractor(&["U0:%F[0]"], &[("L0:%L[0]", "R0:%R[0]")]);

        let first = fe.extract_unigram_feature_ids(&features(&["名詞"]), 0);
        let again = fe.extract_unigram_feature_ids(&features(&["名詞"]), 0);
        assert_eq!(first, again, "the same input must return the same ids");

        // A different value mints the next id, with no gap left by the repeat.
        let other = fe.extract_unigram_feature_ids(&features(&["動詞"]), 0);
        assert_eq!(other[0].get(), 2);
        assert_eq!(fe.unigram_feature_ids.len(), 2);

        // Same for the left/right maps, which have their own counters.
        let l1 = fe.extract_left_feature_ids(&features(&["名詞"]));
        let l2 = fe.extract_left_feature_ids(&features(&["名詞"]));
        assert_eq!(l1, l2);
        let l3 = fe.extract_left_feature_ids(&features(&["動詞"]));
        assert_eq!(l3[0].map(|i| i.get()), Some(2));
        assert_eq!(fe.left_feature_ids.len(), 2);

        let r1 = fe.extract_right_feature_ids(&features(&["名詞"]));
        let r2 = fe.extract_right_feature_ids(&features(&["名詞"]));
        assert_eq!(r1, r2);
        assert_eq!(fe.right_feature_ids.len(), 1);
    }

    /// The three id spaces are independent: the same generated string in the
    /// unigram, left and right maps gets its own id from its own counter.
    #[test]
    fn unigram_left_and_right_id_spaces_are_independent() {
        let mut fe = extractor(&["X:%F[0]"], &[("X:%L[0]", "X:%R[0]")]);

        let u = fe.extract_unigram_feature_ids(&features(&["名詞"]), 0);
        let l = fe.extract_left_feature_ids(&features(&["名詞"]));
        let r = fe.extract_right_feature_ids(&features(&["名詞"]));

        assert_eq!(u[0].get(), 1);
        assert_eq!(l[0].map(|i| i.get()), Some(1));
        assert_eq!(r[0].map(|i| i.get()), Some(1));
        assert_eq!(fe.unigram_feature_ids.len(), 1);
        assert_eq!(fe.left_feature_ids.len(), 1);
        assert_eq!(fe.right_feature_ids.len(), 1);
    }

    /// `%t` substitutes the character-category id, and out-of-range `%F[n]`
    /// (without `?`) substitutes `*` rather than skipping the template.
    #[test]
    fn character_type_and_out_of_range_index_substitution() {
        let mut fe = extractor(&["T:%t", "O:%F[9]"], &[]);

        let ids = fe.extract_unigram_feature_ids(&features(&["名詞"]), 7);

        assert_eq!(ids.len(), 2, "neither template is skipped");
        assert_eq!(unigram_strings_in_id_order(&fe), vec!["T:7", "O:*"]);
    }

    /// The context meta-characters substitute the strings the caller supplies:
    /// `%w` the surface, `%u` the whole ufeature, `%l` / `%r` the whole
    /// lfeature / rfeature. An absent one becomes the empty string.
    #[test]
    fn context_meta_characters_substitute_from_the_context() {
        let mut fe = extractor(&["W:%w", "U:%u"], &[("L:%l", "R:%r")]);

        let ctx = TemplateContext {
            surface: Some("東京"),
            ufeature: Some("名詞,固有名詞"),
            lfeature: Some("L-FEAT"),
            rfeature: Some("R-FEAT"),
        };

        fe.extract_unigram_feature_ids_with_ctx(&features(&["名詞"]), 0, &ctx);
        assert_eq!(
            unigram_strings_in_id_order(&fe),
            vec!["W:東京", "U:名詞,固有名詞"]
        );

        fe.extract_left_feature_ids_with_ctx(&features(&["名詞"]), &ctx);
        assert!(fe.left_feature_ids.contains_key("L:L-FEAT"));

        fe.extract_right_feature_ids_with_ctx(&features(&["名詞"]), &ctx);
        assert!(fe.right_feature_ids.contains_key("R:R-FEAT"));

        // With no context, the meta-characters collapse to empty strings.
        let mut fe = extractor(&["W:%w"], &[]);
        fe.extract_unigram_feature_ids(&features(&["名詞"]), 0);
        assert_eq!(unigram_strings_in_id_order(&fe), vec!["W:"]);
    }

    /// Multiple captures inside one template are all substituted, including
    /// repeats of the same index, and the surrounding literal text is kept.
    #[test]
    fn multiple_captures_in_one_template() {
        let mut fe = extractor(&["M:%F[0]/%F[1]/%F[0]"], &[]);

        fe.extract_unigram_feature_ids(&features(&["名詞", "一般"]), 0);

        assert_eq!(unigram_strings_in_id_order(&fe), vec!["M:名詞/一般/名詞"]);
    }

    /// An extractor with no templates yields no ids and mints nothing --
    /// the loop body never runs.
    #[test]
    fn empty_template_list_yields_no_ids() {
        let mut fe = extractor(&[], &[]);

        assert!(
            fe.extract_unigram_feature_ids(&features(&["名詞"]), 0)
                .is_empty()
        );
        assert!(fe.extract_left_feature_ids(&features(&["名詞"])).is_empty());
        assert!(
            fe.extract_right_feature_ids(&features(&["名詞"]))
                .is_empty()
        );
        assert!(fe.unigram_feature_ids.is_empty());
    }
}
