use regex::Regex;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::{Arc, LazyLock};

use crate::feature_csv::split_feature;

pub use crate::feature_id_map::FeatureIdMap;

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
#[derive(Debug)]
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

/// The left or right context feature IDs of a label, one per template, `None`
/// where the template was skipped; shared by the labels with the same
/// rewritten left and right features (#1263).
pub(crate) type BigramFeatureIds = Arc<[Option<NonZeroU32>]>;

/// Feature extractor for training with advanced capabilities.
pub struct FeatureExtractor {
    unigram_templates: Vec<ParsedTemplate>,
    left_templates: Vec<ParsedTemplate>,
    right_templates: Vec<ParsedTemplate>,
    /// The ids of the unigram features (see [`Self::unigram_feature_ids`]).
    unigram_feature_ids: FeatureIdMap,
    /// The ids of the left context features.
    left_feature_ids: FeatureIdMap,
    /// The ids of the right context features.
    right_feature_ids: FeatureIdMap,
    /// Set when a map had no id left for a new feature, which was then
    /// skipped (see [`Self::ids_exhausted`]).
    ids_exhausted: bool,
    /// The left and right context feature IDs by rewritten left feature, then
    /// by rewritten right feature (see [`Self::bigram_feature_ids`]). Nested
    /// so that a lookup takes the two `&str` as they are.
    bigram_feature_ids: HashMap<String, HashMap<String, (BigramFeatureIds, BigramFeatureIds)>>,
    /// The buffer that `apply_parsed_template` expands a template into, kept
    /// so that a feature string is allocated only when it is interned
    /// (#1263).
    expansion: String,
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
            unigram_feature_ids: FeatureIdMap::default(),
            left_feature_ids: FeatureIdMap::default(),
            right_feature_ids: FeatureIdMap::default(),
            ids_exhausted: false,
            bigram_feature_ids: HashMap::new(),
            expansion: String::new(),
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
            unigram_feature_ids: FeatureIdMap::default(),
            left_feature_ids: FeatureIdMap::default(),
            right_feature_ids: FeatureIdMap::default(),
            ids_exhausted: false,
            bigram_feature_ids: HashMap::new(),
            expansion: String::new(),
        }
    }

    /// Applies a parsed template, writing its generated feature string into
    /// `out`.
    ///
    /// An associated function rather than a method: it reads nothing from the
    /// extractor, and keeping `&self` off it is part of what lets the extract
    /// loops borrow the template vector and the id map disjointly (#965).
    ///
    /// The template is written front to back, each part of it once: the
    /// captures are in template order. Writing into a buffer that the caller
    /// keeps, rather than building a `String` per template and another per
    /// substituted value, is what lets the caller allocate only for a feature
    /// string it has not interned yet (#1263).
    ///
    /// # Arguments
    ///
    /// * `template` - The parsed template to apply.
    /// * `features` - The entry's feature fields, indexed by `%F`/`%L`/`%R`.
    /// * `cate_id` - The character category id substituted for `%t`.
    /// * `ctx` - Surface and full-feature strings for `%w`/`%u`/`%l`/`%r`.
    /// * `out` - Cleared, then given the generated feature string.
    ///
    /// # Returns
    ///
    /// `true` when `out` holds the generated feature string, or `false` when
    /// the template does not apply to this entry: a `?`-marked field is `*`
    /// or empty, or a `%F[n]` / `%L[n]` / `%R[n]` reads past the last column,
    /// with or without `?`. MeCab's learner skips the template in both cases
    /// (`getIndex` in `feature_index.cpp`) (#1189).
    fn apply_parsed_template(
        template: &ParsedTemplate,
        features: &[&str],
        cate_id: u32,
        ctx: &TemplateContext,
        out: &mut String,
    ) -> bool {
        // Check required indices (for conditional features like %F?)
        for &required_idx in &template.required_indices {
            if required_idx >= features.len() {
                return false; // Index out of bounds
            }
            let feature_val = features[required_idx];
            if feature_val == "*" || feature_val.is_empty() {
                return false; // Skip if required feature is undefined
            }
        }

        out.clear();
        let mut copied_to = 0;
        for (range, feature_type) in &template.captures {
            out.push_str(&template.raw_template[copied_to..range.start]);
            match feature_type {
                // An index past the last column skips the template rather
                // than reading as `*`, as MeCab's learner does (#1189).
                FeatureType::Index(idx) => match features.get(*idx) {
                    Some(value) => out.push_str(value),
                    None => return false,
                },
                // Writing to a `String` cannot fail.
                FeatureType::CharacterType => {
                    let _ = write!(out, "{cate_id}");
                }
                FeatureType::SurfaceForm => out.push_str(ctx.surface.unwrap_or("")),
                FeatureType::AllUnigramFeature => out.push_str(ctx.ufeature.unwrap_or("")),
                FeatureType::AllLeftFeature => out.push_str(ctx.lfeature.unwrap_or("")),
                FeatureType::AllRightFeature => out.push_str(ctx.rfeature.unwrap_or("")),
            }
            copied_to = range.end;
        }
        out.push_str(&template.raw_template[copied_to..]);

        true
    }

    /// Interns `feature_str` in `ids`, which gives a new feature the next id.
    ///
    /// Takes the map and the flag as separate arguments rather than
    /// `&mut self` so the callers can hold a shared borrow of their template
    /// vector at the same time; that disjointness is what lets the extract
    /// loops iterate the templates in place instead of cloning them (#965).
    ///
    /// # Arguments
    ///
    /// * `ids` - The map to intern into.
    /// * `exhausted` - Set when `ids` has no id left for a new feature.
    /// * `feature_str` - The generated feature string to intern.
    ///
    /// # Returns
    ///
    /// The id, or `None` when the feature is new and `ids` has given out its
    /// largest id; the caller then skips the feature as it skips a template.
    fn intern_feature_id(
        ids: &mut FeatureIdMap,
        exhausted: &mut bool,
        feature_str: &str,
    ) -> Option<NonZeroU32> {
        let id = ids.intern(feature_str);
        *exhausted |= id.is_none();
        id
    }

    /// Extracts unigram feature IDs from features.
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten unigram feature fields, one slice element per
    ///   column, with a quoted column with `,` in it as one column, as MeCab
    ///   splits a feature.
    /// * `cate_id` - Character category id for the `%t` placeholder.
    ///
    /// # Returns
    ///
    /// The interned feature ids; templates whose `%F?` field is undefined or
    /// that read past the last column are skipped, so the result may be
    /// shorter than the template list.
    pub fn extract_unigram_feature_ids(
        &mut self,
        features: &[&str],
        cate_id: u32,
    ) -> Vec<NonZeroU32> {
        self.extract_unigram_feature_ids_with_ctx(features, cate_id, &TemplateContext::default())
    }

    /// Extracts unigram feature IDs from features with template context.
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten unigram feature fields.
    /// * `cate_id` - Character category id for the `%t` placeholder.
    /// * `ctx` - Values for the `%w` / `%u` / `%l` / `%r` placeholders.
    ///
    /// # Returns
    ///
    /// The interned feature ids; skipped templates are dropped.
    pub fn extract_unigram_feature_ids_with_ctx(
        &mut self,
        features: &[&str],
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
            ids_exhausted,
            expansion,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(unigram_templates.len());
        for template in unigram_templates.iter() {
            // A template whose `%F?` field is undefined or that reads past the
            // last column is skipped entirely, so the result is shorter than
            // the template list. The left/right variants below deliberately
            // differ.
            if Self::apply_parsed_template(template, features, cate_id, ctx, expansion)
                && let Some(id) =
                    Self::intern_feature_id(unigram_feature_ids, ids_exhausted, expansion)
            {
                feature_ids.push(id);
            }
        }

        feature_ids
    }

    /// Extracts left context feature IDs from features (with Optional).
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten left-context feature fields.
    ///
    /// # Returns
    ///
    /// One entry per template, `None` where the template was skipped, so the
    /// result stays index-aligned with the template list.
    pub fn extract_left_feature_ids(&mut self, features: &[&str]) -> Vec<Option<NonZeroU32>> {
        self.extract_left_feature_ids_with_ctx(features, &TemplateContext::default())
    }

    /// Extracts left context feature IDs from features with template context.
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten left-context feature fields.
    /// * `ctx` - Values for the `%w` / `%u` / `%l` / `%r` placeholders.
    ///
    /// # Returns
    ///
    /// One entry per template, `None` where the template was skipped.
    pub fn extract_left_feature_ids_with_ctx(
        &mut self,
        features: &[&str],
        ctx: &TemplateContext,
    ) -> Vec<Option<NonZeroU32>> {
        // See `extract_unigram_feature_ids_with_ctx` for why this is
        // destructured rather than iterating through `self` (#965).
        let Self {
            left_templates,
            left_feature_ids,
            ids_exhausted,
            expansion,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(left_templates.len());
        for template in left_templates.iter() {
            // Unlike unigram extraction, a skipped template pushes `None`
            // rather than being dropped, so the result stays index-aligned
            // with the template list. Callers depend on that alignment.
            if Self::apply_parsed_template(template, features, 0, ctx, expansion) {
                feature_ids.push(Self::intern_feature_id(
                    left_feature_ids,
                    ids_exhausted,
                    expansion,
                ));
            } else {
                feature_ids.push(None);
            }
        }

        feature_ids
    }

    /// Extracts right context feature IDs from features (with Optional).
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten right-context feature fields.
    ///
    /// # Returns
    ///
    /// One entry per template, `None` where the template was skipped, so the
    /// result stays index-aligned with the template list.
    pub fn extract_right_feature_ids(&mut self, features: &[&str]) -> Vec<Option<NonZeroU32>> {
        self.extract_right_feature_ids_with_ctx(features, &TemplateContext::default())
    }

    /// Extracts right context feature IDs from features with template context.
    ///
    /// # Arguments
    ///
    /// * `features` - Rewritten right-context feature fields.
    /// * `ctx` - Values for the `%w` / `%u` / `%l` / `%r` placeholders.
    ///
    /// # Returns
    ///
    /// One entry per template, `None` where the template was skipped.
    pub fn extract_right_feature_ids_with_ctx(
        &mut self,
        features: &[&str],
        ctx: &TemplateContext,
    ) -> Vec<Option<NonZeroU32>> {
        // See `extract_unigram_feature_ids_with_ctx` for why this is
        // destructured rather than iterating through `self` (#965).
        let Self {
            right_templates,
            right_feature_ids,
            ids_exhausted,
            expansion,
            ..
        } = self;

        let mut feature_ids = Vec::with_capacity(right_templates.len());
        for template in right_templates.iter() {
            // Unlike unigram extraction, a skipped template pushes `None`
            // rather than being dropped, so the result stays index-aligned
            // with the template list. Callers depend on that alignment.
            if Self::apply_parsed_template(template, features, 0, ctx, expansion) {
                feature_ids.push(Self::intern_feature_id(
                    right_feature_ids,
                    ids_exhausted,
                    expansion,
                ));
            } else {
                feature_ids.push(None);
            }
        }

        feature_ids
    }

    /// Returns the left and right context feature IDs of a label from its
    /// rewritten left and right features, computing them once per pair of
    /// features.
    ///
    /// The left and right templates read the columns of the two features
    /// (`%L[n]`, `%R[n]`) and the whole features (`%l`, `%r`), and nothing
    /// else: their patterns match no `%w`, `%t` or `%u`. The two features
    /// therefore decide the IDs, and the labels with the same pair share one
    /// copy of them. A dictionary has few such pairs, as its rewrite rules
    /// reduce the two features to little more than the part of speech: the
    /// 347,344 rows of mecab-ipadic have 1,329. Expanding the templates of
    /// every label took most of the time before training, and a copy of the
    /// IDs per label took 146 MiB (#1263).
    ///
    /// The first time a pair comes, the left and then the right IDs are made
    /// by [`Self::extract_left_feature_ids_with_ctx`] and
    /// [`Self::extract_right_feature_ids_with_ctx`], from the features split
    /// as MeCab splits them (#1184), so new IDs are minted in the order they
    /// always were.
    ///
    /// # Arguments
    ///
    /// * `lfeature` - The rewritten left feature.
    /// * `rfeature` - The rewritten right feature.
    ///
    /// # Returns
    ///
    /// The left and the right context feature IDs, one per template, `None`
    /// where the template was skipped.
    pub(crate) fn bigram_feature_ids(
        &mut self,
        lfeature: &str,
        rfeature: &str,
    ) -> (BigramFeatureIds, BigramFeatureIds) {
        if let Some((left, right)) = self
            .bigram_feature_ids
            .get(lfeature)
            .and_then(|by_rfeature| by_rfeature.get(rfeature))
        {
            return (Arc::clone(left), Arc::clone(right));
        }
        let (l_columns, r_columns) = (split_feature(lfeature), split_feature(rfeature));
        let l_vec: Vec<&str> = l_columns.iter().map(|column| column.as_ref()).collect();
        let r_vec: Vec<&str> = r_columns.iter().map(|column| column.as_ref()).collect();
        let ctx = TemplateContext {
            lfeature: Some(lfeature),
            rfeature: Some(rfeature),
            ..TemplateContext::default()
        };
        let left: BigramFeatureIds = self.extract_left_feature_ids_with_ctx(&l_vec, &ctx).into();
        let right: BigramFeatureIds = self.extract_right_feature_ids_with_ctx(&r_vec, &ctx).into();
        self.bigram_feature_ids
            .entry(lfeature.to_string())
            .or_default()
            .insert(
                rfeature.to_string(),
                (Arc::clone(&left), Arc::clone(&right)),
            );
        (left, right)
    }

    /// Returns the ids of the unigram features.
    ///
    /// The k-th feature string that the unigram templates generate gets id k,
    /// from 1. Training removes the features that no weight uses; a removed
    /// feature that a later label generates again gets a new id.
    pub fn unigram_feature_ids(&self) -> &FeatureIdMap {
        &self.unigram_feature_ids
    }

    /// Returns the ids of the left context features, which the left side of
    /// the bigram templates generates; numbered as the unigram features are
    /// (see [`Self::unigram_feature_ids`]).
    pub fn left_feature_ids(&self) -> &FeatureIdMap {
        &self.left_feature_ids
    }

    /// Returns the ids of the right context features, which the right side of
    /// the bigram templates generates; numbered as the unigram features are
    /// (see [`Self::unigram_feature_ids`]).
    pub fn right_feature_ids(&self) -> &FeatureIdMap {
        &self.right_feature_ids
    }

    /// Returns whether a map had no id left for a new feature, which was then
    /// skipped; the caller turns it into an error.
    pub(crate) fn ids_exhausted(&self) -> bool {
        self.ids_exhausted
    }
    /// Makes the three maps, which must still be empty, give out ids up to
    /// `max_id` only, so that tests can run out of ids.
    ///
    /// # Arguments
    ///
    /// * `max_id` - The largest id to give out.
    ///
    /// # Returns
    ///
    /// The extractor.
    #[cfg(test)]
    pub(crate) fn with_max_feature_id(mut self, max_id: u32) -> Self {
        self.unigram_feature_ids = FeatureIdMap::with_max_id(max_id);
        self.left_feature_ids = FeatureIdMap::with_max_id(max_id);
        self.right_feature_ids = FeatureIdMap::with_max_id(max_id);
        self
    }

    /// Removes the features whose ids the given functions reject, then
    /// forgets the left and right context feature IDs that
    /// [`Self::bigram_feature_ids`] computed so far.
    ///
    /// Called once the unused features are known after training. Forgetting
    /// the cached IDs in the same call keeps the cache from ever holding an
    /// ID whose feature the maps no longer have: a label made later interns
    /// its left and right features again, as it did before the IDs were
    /// cached (#1263).
    ///
    /// # Arguments
    ///
    /// * `keep_unigram` - Returns whether to keep a unigram feature id.
    /// * `keep_left` - Returns whether to keep a left context feature id.
    /// * `keep_right` - Returns whether to keep a right context feature id.
    pub(crate) fn retain_features(
        &mut self,
        keep_unigram: impl FnMut(NonZeroU32) -> bool,
        keep_left: impl FnMut(NonZeroU32) -> bool,
        keep_right: impl FnMut(NonZeroU32) -> bool,
    ) {
        self.unigram_feature_ids.retain(keep_unigram);
        self.left_feature_ids.retain(keep_left);
        self.right_feature_ids.retain(keep_right);
        self.bigram_feature_ids = HashMap::new();
    }

    /// Returns the number of pairs of left and right features whose IDs
    /// [`Self::bigram_feature_ids`] holds.
    #[cfg(test)]
    pub(crate) fn cached_bigram_feature_id_pairs(&self) -> usize {
        self.bigram_feature_ids.values().map(HashMap::len).sum()
    }
}

// These tests pin which feature strings are generated, in what order, and how
// ids are minted. Feature ids are assigned sequentially in template iteration
// order; the trained weights are indexed by them and
// `Model::write_bigram_details` names them, so an ordering or off-by-one slip
// would silently change what is trained or written without failing anything.
// There was no coverage here before (#965).
#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an extractor from `&str` templates. Bigram templates are given
    /// as `(left, right)` pairs, matching `TrainerConfig`'s split on `/`.
    fn extractor(unigram: &[&str], bigram: &[(&str, &str)]) -> FeatureExtractor {
        FeatureExtractor::from_templates(unigram, bigram)
    }

    fn features<'a>(fields: &[&'a str]) -> Vec<&'a str> {
        fields.to_vec()
    }

    /// The ids an extractor hands out are 1-based and assigned in template
    /// order, and the map lists its features in id order.
    fn unigram_strings_in_id_order(fe: &FeatureExtractor) -> Vec<String> {
        fe.unigram_feature_ids()
            .iter()
            .map(|(feature, _)| feature.to_string())
            .collect()
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

    /// `%t` substitutes the character-category id, and a template whose
    /// `%F[n]` (without `?`) reads past the last column is skipped, as
    /// MeCab's learner skips it, rather than reading the column as `*`
    /// (#1189). One such index skips the template even when its other
    /// indices are in range.
    #[test]
    fn character_type_substitution_and_unigram_templates_past_the_last_column() {
        let mut fe = extractor(&["T:%t", "O:%F[9]", "P:%F[0],%F[1]"], &[]);

        let ids = fe.extract_unigram_feature_ids(&features(&["名詞"]), 7);

        assert_eq!(ids.len(), 1, "only T should survive");
        assert_eq!(unigram_strings_in_id_order(&fe), vec!["T:7"]);
    }

    /// A `%L[n]` or `%R[n]` (without `?`) that reads past the last column
    /// skips its template, which yields `None` in its slot as a `%L?` or
    /// `%R?` does (#1189). The CRF uses a bigram feature only when both
    /// sides are `Some`, so the feature is dropped, as MeCab's learner drops
    /// the whole bigram template.
    #[test]
    fn left_and_right_templates_past_the_last_column_yield_none() {
        let mut fe = extractor(&[], &[("L0:%L[0]", "R0:%R[0]"), ("L1:%L[1]", "R1:%R[1]")]);

        let left = fe.extract_left_feature_ids(&features(&["名詞"]));
        let right = fe.extract_right_feature_ids(&features(&["名詞"]));

        assert_eq!(left.len(), 2, "index alignment with the template list");
        assert!(left[0].is_some());
        assert!(left[1].is_none(), "L1 reads past the last column");
        assert_eq!(right.len(), 2);
        assert!(right[0].is_some());
        assert!(right[1].is_none(), "R1 reads past the last column");
        assert_eq!(fe.left_feature_ids.len(), 1);
        assert_eq!(fe.right_feature_ids.len(), 1);
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

    /// A template is written front to back into a reused buffer (#1263):
    /// a capture at the start, captures with nothing between them, and the
    /// text after the last capture all come out in place, and a template
    /// does not keep anything of the one written before it.
    #[test]
    fn captures_at_the_ends_and_side_by_side() {
        let mut fe = extractor(&["%F[0]%F[1]", "%t%F[0]-", "a%w%tz", "%F[1]"], &[]);
        let ctx = TemplateContext {
            surface: Some("東京"),
            ..TemplateContext::default()
        };

        fe.extract_unigram_feature_ids_with_ctx(&features(&["名詞", "一般"]), 3, &ctx);

        assert_eq!(
            unigram_strings_in_id_order(&fe),
            vec!["名詞一般", "3名詞-", "a東京3z", "一般"]
        );
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
