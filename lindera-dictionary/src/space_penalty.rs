//! Left-space penalty: an extra cost for lattice candidates that begin right
//! after whitespace, selected by the candidate's first part-of-speech tag.
//!
//! This reproduces mecab-ko's `left-space-penalty-factor` (from mecab-ko-dic's
//! `dicrc`) and Lucene nori's `computeSpacePenalty`: in Korean, particles
//! (`J*`), endings (`E*`), the copula (`VCP`) and derivational suffixes (`XS*`)
//! attach to the preceding word without a space, so a candidate with one of
//! those tags that starts after a space is almost always wrong. Penalizing it
//! lets the Viterbi search prefer, for example, `시/NNG` over `시/EP` in
//! `서울 시 에서`.
//!
//! mecab-ko keys the penalty on `pos-id.def` ids, which Lindera does not
//! store. The ids it lists all collapse to the entry's *first* tag (the part
//! before the first `+`; for `Inflect` rows this equals the
//! `first_part_of_speech` field), so [`SpacePenaltyConfig`] expresses the
//! rules as tag lists instead. "Whitespace" is a character of the
//! dictionary's `SPACE` category (`char.def`, the set MeCab skips and
//! `keep_whitespace` filters on); a dictionary without that category falls
//! back to [`char::is_whitespace`]. Only characters inside the sentence
//! count, so a candidate at the very start of a sentence is not penalized
//! (mecab-ko's `rlength > length` test behaves the same way).
//!
//! The penalty is applied in [`Lattice`](crate::viterbi::Lattice) through a
//! [`SpacePenaltyTable`], a per-word-id lookup precomputed once from the
//! dictionary so the hot path never parses a part-of-speech string.
//!
//! Note that Lindera keeps whitespace as a lattice node (the `SPACE`
//! unknown word) and connects its neighbours through it, whereas MeCab drops
//! whitespace and connects the surrounding words directly. The penalty
//! therefore fixes the penalized readings but does not by itself make every
//! spaced sentence come out as mecab-ko would.

use std::collections::HashMap;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::dictionary::character_definition::{CategoryId, CharacterDefinition};
use crate::dictionary::schema::Schema;
use crate::dictionary::{Dictionary, UserDictionary};
use crate::error::LinderaErrorKind;
use crate::viterbi::{LexType, WordId};

/// Schema field names consulted, in order, to find the part-of-speech tag
/// column: `part_of_speech_tag` (ko-dic) and `part_of_speech` (IPADIC,
/// UniDic, CC-CEDICT, Jieba, SudachiDict).
pub const POS_FIELD_CANDIDATES: [&str; 2] = ["part_of_speech_tag", "part_of_speech"];

/// Number of leading CSV columns (`surface`, `left_context_id`,
/// `right_context_id`, `cost`) that precede the detail fields.
const COMMON_FIELD_COUNT: usize = 4;

/// One penalty rule: the cost added to a candidate whose first
/// part-of-speech tag is in `pos` when the candidate starts after whitespace.
///
/// The type is `#[non_exhaustive]` so that fields can be added without a
/// breaking change; create it with [`SpacePenaltyRule::new`] or deserialize
/// it from JSON/YAML.
#[derive(
    Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[non_exhaustive]
pub struct SpacePenaltyRule {
    /// First part-of-speech tags this rule applies to (e.g. `["JKS", "JX"]`).
    pub pos: Vec<String>,
    /// Cost added to the candidate's path cost.
    pub cost: i32,
}

impl SpacePenaltyRule {
    /// Creates a rule from tag names and a cost.
    ///
    /// # Arguments
    ///
    /// * `pos` - First part-of-speech tags the rule applies to.
    /// * `cost` - The cost to add.
    ///
    /// # Returns
    ///
    /// The rule.
    pub fn new<S: Into<String>>(pos: impl IntoIterator<Item = S>, cost: i32) -> Self {
        Self {
            pos: pos.into_iter().map(Into::into).collect(),
            cost,
        }
    }
}

/// The left-space penalty rules (mecab-ko's `left-space-penalty-factor`).
///
/// A candidate is matched against the rules by its first part-of-speech tag;
/// the first rule listing that tag wins, and unlisted tags cost nothing. In
/// JSON/YAML the config looks like:
///
/// ```json
/// {
///   "rules": [
///     { "pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000 },
///     { "pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000 }
///   ]
/// }
/// ```
///
/// The type is `#[non_exhaustive]` so that fields can be added without a
/// breaking change; create it with [`SpacePenaltyConfig::new`],
/// [`Default::default`] or by deserializing it.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[non_exhaustive]
pub struct SpacePenaltyConfig {
    /// The rules, consulted in order; the first match wins.
    pub rules: Vec<SpacePenaltyRule>,
}

impl SpacePenaltyConfig {
    /// Creates a config from rules.
    ///
    /// # Arguments
    ///
    /// * `rules` - The rules, in priority order.
    ///
    /// # Returns
    ///
    /// The config.
    pub fn new(rules: Vec<SpacePenaltyRule>) -> Self {
        Self { rules }
    }

    /// Returns the penalty for a part-of-speech tag.
    ///
    /// # Arguments
    ///
    /// * `pos_tag` - The full tag as stored in the dictionary (e.g. `JKS` or
    ///   `VV+EC`); only the part before the first `+` is matched.
    ///
    /// # Returns
    ///
    /// The cost of the first matching rule, or `0` when no rule lists the tag.
    pub fn cost_for_tag(&self, pos_tag: &str) -> i32 {
        let first = first_pos(pos_tag);
        self.rules
            .iter()
            .find(|rule| rule.pos.iter().any(|p| p == first))
            .map_or(0, |rule| rule.cost)
    }
}

/// Returns the first component of a `+`-joined part-of-speech tag.
///
/// # Arguments
///
/// * `pos_tag` - A tag such as `NNG` or `VV+EC`.
///
/// # Returns
///
/// The text before the first `+` (the whole tag when there is none).
#[inline]
pub fn first_pos(pos_tag: &str) -> &str {
    pos_tag.split('+').next().unwrap_or(pos_tag)
}

/// Resolves the detail-field index of the part-of-speech tag in a dictionary
/// schema (see [`POS_FIELD_CANDIDATES`]).
///
/// # Arguments
///
/// * `schema` - The system dictionary schema.
///
/// # Returns
///
/// The index into an entry's detail fields (the fields after the four common
/// columns), or an error when the schema has no part-of-speech column.
pub fn pos_detail_index(schema: &Schema) -> LinderaResult<usize> {
    POS_FIELD_CANDIDATES
        .iter()
        .find_map(|name| schema.get_field_index(name))
        .and_then(|index| index.checked_sub(COMMON_FIELD_COUNT))
        .ok_or_else(|| {
            LinderaErrorKind::Dictionary.with_error(anyhow::anyhow!(
                "space penalty requires a part-of-speech field ({}) in the dictionary schema",
                POS_FIELD_CANDIDATES.join(" or ")
            ))
        })
}

/// Per-word-id space penalty lookup, precomputed from a
/// [`SpacePenaltyConfig`] for one system dictionary (and optionally one user
/// dictionary) so the lattice never parses a part-of-speech string.
///
/// Each lexicon stores one byte per word id: the 1-based index of the
/// matching rule, or `0` for "no penalty". The rule costs live in a separate
/// small vector, so a lookup is two loads.
///
/// The table also carries the whitespace classifier the penalty is gated on
/// (see [`SpacePenaltyTable::is_space`]), since that is per-dictionary state
/// with the same lifetime as the rule indexes.
#[derive(Clone, Debug)]
pub struct SpacePenaltyTable {
    /// Rule index per system word id.
    system: Vec<u8>,
    /// Rule index per user word id (empty without a user dictionary).
    user: Vec<u8>,
    /// Rule index per unknown-word entry id.
    unknown: Vec<u8>,
    /// `costs[0] == 0`; `costs[i]` is the cost of rule `i - 1`.
    costs: Vec<i32>,
    /// Precomputed `SPACE`-category membership for the ASCII/Latin-1 range,
    /// mirroring the table `Segmenter` keeps for `keep_whitespace`. Every
    /// character `char.def` files classify as `SPACE` in practice lives here
    /// (0x20, 0x09, 0x0A, 0x0B, 0x0D), so this is the path that actually
    /// runs, one indexed load per character instead of a category lookup.
    space_ascii: [bool; 256],
    /// The dictionary's `SPACE` category, for codepoints outside
    /// `space_ascii`; `None` when the dictionary defines no such category,
    /// which selects the `char::is_whitespace` fallback.
    space_category: Option<CategoryId>,
}

impl SpacePenaltyTable {
    /// Precomputes the lookup for a dictionary pair.
    ///
    /// The user dictionary passed here must be the one used for
    /// segmentation: user word ids index `user` directly.
    ///
    /// # Arguments
    ///
    /// * `config` - The penalty rules.
    /// * `dictionary` - The system dictionary; its schema locates the
    ///   part-of-speech field, and its unknown-word entries are covered too.
    /// * `user_dictionary` - The user dictionary, if any.
    ///
    /// # Returns
    ///
    /// The table, or an error when the schema has no part-of-speech column or
    /// the config has more than 255 rules.
    pub fn build(
        config: &SpacePenaltyConfig,
        dictionary: &Dictionary,
        user_dictionary: Option<&UserDictionary>,
    ) -> LinderaResult<Self> {
        if config.rules.len() > u8::MAX as usize {
            return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "space penalty supports at most {} rules, got {}",
                u8::MAX,
                config.rules.len()
            )));
        }
        let pos_index = pos_detail_index(&dictionary.metadata.dictionary_schema)?;

        let mut costs = Vec::with_capacity(config.rules.len() + 1);
        costs.push(0);
        costs.extend(config.rules.iter().map(|rule| rule.cost));

        // First rule listing a tag wins, so insert in reverse and let earlier
        // rules overwrite later ones.
        let mut rule_of_tag: HashMap<&str, u8> = HashMap::new();
        for (i, rule) in config.rules.iter().enumerate().rev() {
            for pos in &rule.pos {
                rule_of_tag.insert(pos.as_str(), (i + 1) as u8);
            }
        }
        let rule_for = |pos_tag: Option<&str>| -> u8 {
            pos_tag
                .and_then(|tag| rule_of_tag.get(first_pos(tag)))
                .copied()
                .unwrap_or(0)
        };

        let system = (0..dictionary.prefix_dictionary.word_count())
            .map(|id| rule_for(dictionary.word_details_iter(id).nth(pos_index)))
            .collect();

        let user = user_dictionary.map_or_else(Vec::new, |ud| {
            (0..ud.dict.word_count())
                .map(|id| rule_for(ud.word_details_iter(id).nth(pos_index)))
                .collect()
        });

        let unknown = (0..dictionary.unknown_dictionary.costs.len())
            .map(|id| rule_for(dictionary.unknown_word_details_iter(id).nth(pos_index)))
            .collect();

        // Whitespace classifier, resolved once here rather than per character
        // per sentence. Built from the dictionary's own `char.def`, like the
        // equivalent table in `Segmenter::new`.
        let char_definitions = &dictionary.character_definition;
        let space_category = char_definitions.category_id_by_name("SPACE");
        let mut space_ascii = [false; 256];
        for (codepoint, is_space) in space_ascii.iter_mut().enumerate() {
            if let Some(c) = char::from_u32(codepoint as u32) {
                *is_space = match space_category {
                    Some(space_id) => char_definitions.lookup_categories(c).contains(&space_id),
                    None => c.is_whitespace(),
                };
            }
        }

        Ok(Self {
            system,
            user,
            unknown,
            costs,
            space_ascii,
            space_category,
        })
    }

    /// Returns whether `c` counts as whitespace for the penalty: a character
    /// carrying the dictionary's `SPACE` category (`char.def`), which is the
    /// set MeCab skips and `keep_whitespace` filters on, or, for a dictionary
    /// that defines no `SPACE` category, one satisfying
    /// [`char::is_whitespace`].
    ///
    /// ASCII/Latin-1 codepoints are answered from a precomputed table, which
    /// covers every character real `char.def` files put in `SPACE`; anything
    /// above falls back to a category lookup.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to classify.
    /// * `char_definitions` - The same dictionary's character definitions,
    ///   read only on the non-ASCII path.
    ///
    /// # Returns
    ///
    /// `true` when a candidate starting after `c` is subject to the penalty.
    #[inline]
    pub fn is_space(&self, c: char, char_definitions: &CharacterDefinition) -> bool {
        let codepoint = c as u32;
        if codepoint < 256 {
            self.space_ascii[codepoint as usize]
        } else {
            match self.space_category {
                Some(space_id) => char_definitions.lookup_categories(c).contains(&space_id),
                None => c.is_whitespace(),
            }
        }
    }

    /// Returns the penalty for a candidate that starts after whitespace.
    ///
    /// # Arguments
    ///
    /// * `word_id` - The candidate's word id (any lexicon).
    ///
    /// # Returns
    ///
    /// The rule cost, or `0` for an unlisted tag or an unknown word id.
    #[inline]
    pub fn cost(&self, word_id: WordId) -> i32 {
        let table = match word_id.lex_type() {
            LexType::System => &self.system,
            LexType::User => &self.user,
            LexType::Unknown => &self.unknown,
        };
        let rule = table.get(word_id.id() as usize).copied().unwrap_or(0);
        self.costs[rule as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ko_dic_rules() -> SpacePenaltyConfig {
        SpacePenaltyConfig::new(vec![
            SpacePenaltyRule::new(
                ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"],
                3000,
            ),
            SpacePenaltyRule::new(
                ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"],
                6000,
            ),
        ])
    }

    #[test]
    fn first_pos_takes_the_part_before_the_first_plus() {
        assert_eq!(first_pos("NNG"), "NNG");
        assert_eq!(first_pos("VV+EC"), "VV");
        assert_eq!(first_pos("XSV+ETM"), "XSV");
        assert_eq!(first_pos(""), "");
    }

    #[test]
    fn cost_for_tag_matches_on_the_first_pos() {
        let config = ko_dic_rules();
        assert_eq!(config.cost_for_tag("JKB"), 6000);
        assert_eq!(config.cost_for_tag("EP"), 3000);
        // Inflect rows: `EP+EC` -> EP -> 3000, `VCP+EP` -> VCP -> 3000.
        assert_eq!(config.cost_for_tag("EP+EC"), 3000);
        assert_eq!(config.cost_for_tag("VCP+EP"), 3000);
        // `VV+EC` starts with VV, which is unlisted.
        assert_eq!(config.cost_for_tag("VV+EC"), 0);
        assert_eq!(config.cost_for_tag("NNG"), 0);
    }

    #[test]
    fn cost_for_tag_prefers_the_first_matching_rule() {
        let config = SpacePenaltyConfig::new(vec![
            SpacePenaltyRule::new(["JKS"], 100),
            SpacePenaltyRule::new(["JKS", "JX"], 200),
        ]);
        assert_eq!(config.cost_for_tag("JKS"), 100);
        assert_eq!(config.cost_for_tag("JX"), 200);
    }

    #[test]
    fn config_round_trips_through_json() {
        let config = ko_dic_rules();
        let json = serde_json::to_string(&config).unwrap();
        let back: SpacePenaltyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);

        let parsed: SpacePenaltyConfig = serde_json::from_str(
            r#"{"rules":[{"pos":["JKS"],"cost":6000},{"pos":["EP"],"cost":3000}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.rules.len(), 2);
        assert_eq!(parsed.cost_for_tag("JKS"), 6000);
    }

    #[test]
    fn constructors_set_every_field() {
        // The types are `#[non_exhaustive]`, so code outside this crate can
        // only build them through these constructors (or serde); they must
        // keep populating every field.
        let rule = SpacePenaltyRule::new(["JKS", "JX"], 6000);
        assert_eq!(rule.pos, vec!["JKS".to_string(), "JX".to_string()]);
        assert_eq!(rule.cost, 6000);

        let config = SpacePenaltyConfig::new(vec![rule.clone()]);
        assert_eq!(config.rules, vec![rule]);

        let empty = SpacePenaltyConfig::default();
        assert!(empty.rules.is_empty());
        assert_eq!(empty.cost_for_tag("JKS"), 0);
    }

    #[test]
    fn pos_detail_index_prefers_ko_dic_then_generic_name() {
        let ko = Schema::new(
            [
                "surface",
                "left_context_id",
                "right_context_id",
                "cost",
                "part_of_speech_tag",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        );
        assert_eq!(pos_detail_index(&ko).unwrap(), 0);

        let generic = Schema::new(
            [
                "surface",
                "left_context_id",
                "right_context_id",
                "cost",
                "display_surface",
                "part_of_speech",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        );
        assert_eq!(pos_detail_index(&generic).unwrap(), 1);

        let none = Schema::new(
            [
                "surface",
                "left_context_id",
                "right_context_id",
                "cost",
                "reading",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        );
        assert!(pos_detail_index(&none).is_err());
    }
}
