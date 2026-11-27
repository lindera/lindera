//! The features of a decision of the parser.
//!
//! A decision is about two bunsetsu, the source and the target: does the
//! source depend on the target? Its features are strings, with the names
//! CaboCha gives them. Most are made once for each bunsetsu of a sentence,
//! and a decision puts together those of the bunsetsu it looks at.
//!
//! What the strings are made from is in the [`Schema`]: the head and the
//! function word, the levels of the part of speech, the conjugation form,
//! the particles, the symbols. Nothing here names a part of speech or a
//! column.

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::schema::Schema;

/// The value of a feature column that holds nothing.
const EMPTY: &str = "*";

/// A group of features of a decision.
///
/// The name of a feature starts as CaboCha writes it; the examples are for
/// 彼 / は as the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Group {
    /// The number of bunsetsu from the source to the target, in three
    /// classes: `DIST:1`, `DIST:2-5`, `DIST:6-`.
    Distance,
    /// The source: the surface, the levels of the part of speech and the
    /// conjugation form of its head (`SHS:彼`, `SHP0:名詞`), its function
    /// word (`SFS:は`), its first and its last morpheme (`SLS:`, `SRS:`);
    /// its punctuation (`SPUNC:、`), its brackets (`SOB:「`, `SOB:1`,
    /// `SCB:」`, `SCB:1`), each of its particles (`SCASE:は`), and whether
    /// it is the first or the last bunsetsu of the sentence (`SBOS:1`,
    /// `SEOS:1`).
    Source,
    /// The target: what [`Group::Source`] has, with `D` in front
    /// (`DHS:読ん`).
    Target,
    /// The bunsetsu before the source: the surface of its function word
    /// (`LF:`).
    Left,
    /// The bunsetsu after the target: the surfaces of its first morpheme,
    /// its head and its function word (`RL:`, `RH:`, `RF:`).
    Right,
    /// The bunsetsu already attached to the source (`a:`) and to the target
    /// (`A:`), each by its function word: the surface of a word of
    /// [`Schema::lexicalized`], or else the conjugation form, or else the
    /// part of speech. These are the only features that depend on the
    /// decisions made before.
    Children,
    /// The bunsetsu between the source and the target: their punctuation
    /// (`GPUNC:、`), their brackets (`GOB:「`, `GCB:」`), the function word
    /// of each when it is a particle (`GCASE:を`), and whether brackets
    /// open or close in between: `GNB:1` for neither, `GOB:1`, `GCB:1`,
    /// `GBB:1` for both.
    Gap,
    /// The morpheme after the head of the source (`SNS:`, `SNP0:`, `SNF:`)
    /// and of the target (`DNS:`): its surface, the levels of its part of
    /// speech and its conjugation form. CaboCha has no such feature. Its
    /// head is the last word of a bunsetsu that is not a function word;
    /// the head of a [`Schema`] can be the content word before it (利用 in
    /// 利用 / し / た, 読む in 読む / こと / を), and the word that follows
    /// says what the bunsetsu is as a whole.
    AfterHead,
}

impl Group {
    /// Every group, with its name, in the order of the list of a
    /// [`FeatureSet`].
    const ALL: [(Group, &'static str); 8] = [
        (Group::Distance, "distance"),
        (Group::Source, "source"),
        (Group::Target, "target"),
        (Group::Left, "left"),
        (Group::Right, "right"),
        (Group::Children, "children"),
        (Group::Gap, "gap"),
        (Group::AfterHead, "after"),
    ];

    /// The name of the group: `distance`, `source`, `target`, `left`,
    /// `right`, `children`, `gap` or `after`.
    ///
    /// # Returns
    ///
    /// The name.
    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(group, _)| *group == self)
            .map_or("", |(_, name)| name)
    }

    /// The group with a name.
    ///
    /// # Arguments
    ///
    /// * `name` - A name that [`Group::name`] gives.
    ///
    /// # Returns
    ///
    /// The group, or `None` for another name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, other)| *other == name)
            .map(|(group, _)| *group)
    }
}

/// The groups of features a model uses.
///
/// A feature set is data of a model, as the template of the chunker is: the
/// features of training and of parsing are made from the same value, and
/// two sets are compared without a change of code. It is written as the
/// names of its groups with commas, as in `distance,source,target,gap`.
///
/// A set can also limit the levels of the part of speech its features
/// take ([`FeatureSet::with_part_of_speech_levels`]), written after the
/// groups as in `distance,source,pos-levels=2`. Without a limit the
/// features take every level of the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSet {
    /// The groups, in the order of [`Group::ALL`], each at most once.
    groups: Vec<Group>,
    /// The number of levels of the part of speech the features take, when
    /// it is not all of the schema.
    part_of_speech_levels: Option<usize>,
}

/// What a limit of the levels of the part of speech is written with in the
/// text of a feature set.
const LEVELS_KEY: &str = "pos-levels=";

impl FeatureSet {
    /// Makes a feature set of groups.
    ///
    /// # Arguments
    ///
    /// * `groups` - The groups, in any order.
    ///
    /// # Returns
    ///
    /// The set, or [`Error::Parser`] for no group or a group given twice.
    pub fn new(groups: &[Group]) -> Result<Self> {
        if groups.is_empty() {
            return Err(Error::Parser("a feature set needs a group".to_string()));
        }
        for (index, group) in groups.iter().enumerate() {
            if groups[..index].contains(group) {
                return Err(Error::Parser(format!(
                    "the group {} is given twice",
                    group.name()
                )));
            }
        }
        Ok(Self {
            groups: Group::ALL
                .iter()
                .map(|(group, _)| *group)
                .filter(|group| groups.contains(group))
                .collect(),
            part_of_speech_levels: None,
        })
    }

    /// Limits the levels of the part of speech the features take.
    ///
    /// The features of a morpheme name the levels of its part of speech
    /// (`HP0:名詞`, `HP1:普通名詞`), and the feature of a bunsetsu that
    /// ends in a word without a conjugation form is its part of speech,
    /// the levels joined (`A:名詞-普通名詞`). With a limit, both stop at
    /// that many levels, though the schema has more: the chunker and the
    /// patterns of the schema still read all of them. CaboCha reads two
    /// levels of UniDic in its parser and four in its chunker.
    ///
    /// # Arguments
    ///
    /// * `levels` - The number of levels, 1 or more.
    ///
    /// # Returns
    ///
    /// The set with the limit, or [`Error::Parser`] for 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use lindera_dependency::parser::FeatureSet;
    ///
    /// let set = FeatureSet::cabocha().with_part_of_speech_levels(2)?;
    /// assert_eq!(set.part_of_speech_levels(), Some(2));
    /// assert_eq!(
    ///     set.to_string(),
    ///     "distance,source,target,left,right,children,gap,pos-levels=2"
    /// );
    /// assert_eq!(FeatureSet::parse(&set.to_string())?, set);
    /// # Ok::<(), lindera_dependency::Error>(())
    /// ```
    pub fn with_part_of_speech_levels(mut self, levels: usize) -> Result<Self> {
        if levels == 0 {
            return Err(Error::Parser(
                "the features need a level of the part of speech: pos-levels is 1 or more"
                    .to_string(),
            ));
        }
        self.part_of_speech_levels = Some(levels);
        Ok(self)
    }

    /// The limit of the levels of the part of speech.
    ///
    /// # Returns
    ///
    /// The number of levels the features take, or `None` for every level
    /// of the schema.
    pub fn part_of_speech_levels(&self) -> Option<usize> {
        self.part_of_speech_levels
    }

    /// The columns of the part of speech the features read, of a schema.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema.
    ///
    /// # Returns
    ///
    /// The columns of the schema, cut at the limit of the set.
    fn levels(&self, schema: &Schema) -> Range<usize> {
        let all = schema.part_of_speech.clone();
        match self.part_of_speech_levels {
            Some(levels) => all.start..all.end.min(all.start.saturating_add(levels)),
            None => all,
        }
    }

    /// The feature set of CaboCha: every group but [`Group::AfterHead`].
    ///
    /// # Returns
    ///
    /// The set.
    pub fn cabocha() -> Self {
        Self {
            groups: Group::ALL
                .iter()
                .map(|(group, _)| *group)
                .filter(|group| *group != Group::AfterHead)
                .collect(),
            part_of_speech_levels: None,
        }
    }

    /// Reads a feature set from its text.
    ///
    /// # Arguments
    ///
    /// * `text` - The names of the groups, separated by commas; spaces
    ///   around a name are ignored. `pos-levels=<number>` among them is
    ///   the limit of [`FeatureSet::with_part_of_speech_levels`].
    ///
    /// # Returns
    ///
    /// The set, or [`Error::Parser`] for a name that is not a group, no
    /// group, a group given twice, or a limit that is not a number of 1 or
    /// more, or is given twice.
    pub fn parse(text: &str) -> Result<Self> {
        let mut groups = Vec::new();
        let mut levels = None;
        for name in text.split(',') {
            let name = name.trim();
            if let Some(value) = name.strip_prefix(LEVELS_KEY) {
                if levels.is_some() {
                    return Err(Error::Parser(format!("{LEVELS_KEY} is given twice")));
                }
                levels = Some(value.parse::<usize>().map_err(|_| {
                    Error::Parser(format!(
                        "{name:?} is not a number of levels of the part of speech"
                    ))
                })?);
                continue;
            }
            groups
                .push(Group::from_name(name).ok_or_else(|| {
                    Error::Parser(format!("{name:?} is not a group of features"))
                })?);
        }
        let set = Self::new(&groups)?;
        match levels {
            Some(levels) => set.with_part_of_speech_levels(levels),
            None => Ok(set),
        }
    }

    /// The groups of the set.
    ///
    /// # Returns
    ///
    /// The groups, in the order `distance`, `source`, `target`, `left`,
    /// `right`, `children`, `gap`, `after`.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// Whether the set has a group.
    ///
    /// # Arguments
    ///
    /// * `group` - The group.
    ///
    /// # Returns
    ///
    /// Whether the features of the group are made.
    pub fn has(&self, group: Group) -> bool {
        self.groups.contains(&group)
    }
}

impl Default for FeatureSet {
    /// Every group: the feature set of CaboCha and [`Group::AfterHead`],
    /// which is what a model is trained with unless another set is given.
    fn default() -> Self {
        Self {
            groups: Group::ALL.iter().map(|(group, _)| *group).collect(),
            part_of_speech_levels: None,
        }
    }
}

impl fmt::Display for FeatureSet {
    /// Writes the names of the groups with commas, and the limit of the
    /// levels of the part of speech after them when the set has one: the
    /// text [`FeatureSet::parse`] reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, group) in self.groups.iter().enumerate() {
            if index > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", group.name())?;
        }
        if let Some(levels) = self.part_of_speech_levels {
            write!(f, ",{LEVELS_KEY}{levels}")?;
        }
        Ok(())
    }
}

/// The features of one bunsetsu, in each of the roles it can have in a
/// decision.
///
/// `T` is what a feature string was turned into: an id of the classifier
/// when parsing, the string itself for training instances.
#[derive(Debug, Clone, PartialEq)]
struct Roles<T> {
    /// As the source of a decision.
    source: Vec<T>,
    /// As the target of a decision.
    target: Vec<T>,
    /// As the bunsetsu before the source.
    left: Vec<T>,
    /// As the bunsetsu after the target.
    right: Vec<T>,
    /// As a bunsetsu between the source and the target, without the state
    /// of the brackets.
    gap: Vec<T>,
    /// The morpheme after its head, as the source of a decision.
    after_source: Vec<T>,
    /// The morpheme after its head, as the target of a decision.
    after_target: Vec<T>,
    /// As a bunsetsu attached to the source.
    child_of_source: Option<T>,
    /// As a bunsetsu attached to the target.
    child_of_target: Option<T>,
    /// Whether the bunsetsu has an opening bracket.
    opens: bool,
    /// Whether the bunsetsu has a closing bracket.
    closes: bool,
}

/// The features of the bunsetsu of a sentence, from which the features of
/// each decision are put together.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Prepared<T> {
    /// The features of each bunsetsu.
    bunsetsu: Vec<Roles<T>>,
    /// The three classes of the distance: 1, 2 to 5, 6 or more.
    distance: [Option<T>; 3],
    /// The four states of the brackets in the gap: none, opening only,
    /// closing only, both.
    brackets: [Option<T>; 4],
}

impl<T: Clone + Ord> Prepared<T> {
    /// Makes the features of the bunsetsu of a sentence.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morphemes.
    /// * `set` - The groups to make.
    /// * `morphemes` - The morphemes of the sentence.
    /// * `bunsetsu` - The bunsetsu, as ranges of indices into `morphemes`;
    ///   none is empty or past the morphemes.
    /// * `intern` - Turns a feature string into a `T`, or into `None` for a
    ///   feature to leave out.
    ///
    /// # Returns
    ///
    /// The features.
    pub(crate) fn new<M: Morpheme>(
        schema: &Schema,
        set: &FeatureSet,
        morphemes: &[M],
        bunsetsu: &[Range<usize>],
        mut intern: impl FnMut(&str) -> Option<T>,
    ) -> Self {
        let last = bunsetsu.len().saturating_sub(1);
        let roles = bunsetsu
            .iter()
            .enumerate()
            .map(|(index, range)| {
                let strings = Strings::new(
                    schema,
                    set.levels(schema),
                    &morphemes[range.clone()],
                    index == 0,
                    index == last,
                );
                let mut role = |group: Group, prefix: &str, names: &[String]| -> Vec<T> {
                    if !set.has(group) {
                        return Vec::new();
                    }
                    names
                        .iter()
                        .filter_map(|name| intern(&format!("{prefix}{name}")))
                        .collect()
                };
                let source = role(Group::Source, "S", &strings.statics);
                let target = role(Group::Target, "D", &strings.statics);
                let left = role(Group::Left, "", &strings.left);
                let right = role(Group::Right, "", &strings.right);
                let gap = role(Group::Gap, "", &strings.gap);
                let after_source = role(Group::AfterHead, "S", &strings.after);
                let after_target = role(Group::AfterHead, "D", &strings.after);
                let child = std::slice::from_ref(&strings.child);
                let child_of_source = role(Group::Children, "a:", child).pop();
                let child_of_target = role(Group::Children, "A:", child).pop();
                Roles {
                    source,
                    target,
                    left,
                    right,
                    gap,
                    after_source,
                    after_target,
                    child_of_source,
                    child_of_target,
                    opens: strings.opens,
                    closes: strings.closes,
                }
            })
            .collect();
        let mut named = |group: Group, name: &str| {
            if set.has(group) { intern(name) } else { None }
        };
        Self {
            bunsetsu: roles,
            distance: [
                named(Group::Distance, "DIST:1"),
                named(Group::Distance, "DIST:2-5"),
                named(Group::Distance, "DIST:6-"),
            ],
            brackets: [
                named(Group::Gap, "GNB:1"),
                named(Group::Gap, "GOB:1"),
                named(Group::Gap, "GCB:1"),
                named(Group::Gap, "GBB:1"),
            ],
        }
    }

    /// Puts together the features of a decision.
    ///
    /// # Arguments
    ///
    /// * `source` - The index of the source bunsetsu.
    /// * `target` - The index of the target bunsetsu, after the source.
    /// * `children` - For each bunsetsu, the bunsetsu attached to it so
    ///   far.
    /// * `features` - Where the features are written: sorted, each once.
    pub(crate) fn decision(
        &self,
        source: usize,
        target: usize,
        children: &[Vec<usize>],
        features: &mut Vec<T>,
    ) {
        features.clear();
        let class = match target - source {
            1 => 0,
            2..=5 => 1,
            _ => 2,
        };
        features.extend(self.distance[class].clone());
        features.extend_from_slice(&self.bunsetsu[source].source);
        features.extend_from_slice(&self.bunsetsu[target].target);
        features.extend_from_slice(&self.bunsetsu[source].after_source);
        features.extend_from_slice(&self.bunsetsu[target].after_target);
        if source > 0 {
            features.extend_from_slice(&self.bunsetsu[source - 1].left);
        }
        if let Some(after) = self.bunsetsu.get(target + 1) {
            features.extend_from_slice(&after.right);
        }
        // A group that is not in the feature set gives nothing below: the
        // features of its bunsetsu are empty, and its named ones are `None`.
        for &child in &children[source] {
            features.extend(self.bunsetsu[child].child_of_source.clone());
        }
        for &child in &children[target] {
            features.extend(self.bunsetsu[child].child_of_target.clone());
        }
        let mut opens = false;
        let mut closes = false;
        for between in &self.bunsetsu[source + 1..target] {
            features.extend_from_slice(&between.gap);
            opens |= between.opens;
            closes |= between.closes;
        }
        let state = usize::from(opens) + 2 * usize::from(closes);
        features.extend(self.brackets[state].clone());
        features.sort_unstable();
        features.dedup();
    }
}

/// The feature strings of one bunsetsu, before the role is put in front.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Strings {
    /// The features of the bunsetsu as a source or a target, without the
    /// `S` or `D` in front.
    statics: Vec<String>,
    /// The features as the bunsetsu before the source.
    left: Vec<String>,
    /// The features as the bunsetsu after the target.
    right: Vec<String>,
    /// The features as a bunsetsu in the gap.
    gap: Vec<String>,
    /// The features of the morpheme after the head, without the `S` or `D`
    /// in front; none when the head is the last morpheme.
    after: Vec<String>,
    /// The feature as a child, without the `a:` or `A:` in front.
    child: String,
    /// Whether the bunsetsu has an opening bracket.
    opens: bool,
    /// Whether the bunsetsu has a closing bracket.
    closes: bool,
}

impl Strings {
    /// Makes the feature strings of a bunsetsu.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morphemes.
    /// * `morphemes` - The morphemes of the bunsetsu, at least one.
    /// * `first` - Whether it is the first bunsetsu of the sentence.
    /// * `last` - Whether it is the last bunsetsu of the sentence.
    ///
    /// # Returns
    ///
    /// The strings.
    fn new<M: Morpheme>(
        schema: &Schema,
        levels: Range<usize>,
        morphemes: &[M],
        first: bool,
        last: bool,
    ) -> Self {
        let mut statics = Vec::new();
        let mut gap = Vec::new();
        let mut opens = false;
        let mut closes = false;
        for (index, morpheme) in morphemes.iter().enumerate() {
            let surface = schema.surface_of(morpheme);
            if schema.is_comma(morpheme) || schema.is_period(morpheme) {
                gap.push(format!("GPUNC:{surface}"));
                statics.push(format!("PUNC:{surface}"));
            }
            if schema.is_open_bracket(morpheme) {
                gap.push(format!("GOB:{surface}"));
                statics.push(format!("OB:{surface}"));
                statics.push("OB:1".to_string());
                opens = true;
            }
            if schema.is_close_bracket(morpheme) {
                gap.push(format!("GCB:{surface}"));
                statics.push(format!("CB:{surface}"));
                statics.push("CB:1".to_string());
                closes = true;
            }
            if schema.is_particle(morphemes, index) {
                statics.push(format!("CASE:{surface}"));
            }
        }

        let (head, function) = schema.positions(morphemes).unwrap_or((0, 0));
        let end = morphemes.len().saturating_sub(1);
        for (role, index) in [('H', head), ('F', function), ('L', 0), ('R', end)] {
            if let Some(morpheme) = morphemes.get(index) {
                push_morpheme(schema, levels.clone(), role, morpheme, &mut statics);
            }
        }
        let mut after = Vec::new();
        if let Some(morpheme) = morphemes.get(head + 1) {
            push_morpheme(schema, levels.clone(), 'N', morpheme, &mut after);
        }
        if first {
            statics.push("BOS:1".to_string());
        }
        if last {
            statics.push("EOS:1".to_string());
        }

        let surface = |index: usize| {
            morphemes
                .get(index)
                .map_or(Cow::Borrowed(""), |morpheme| schema.surface_of(morpheme))
        };
        let left = vec![format!("LF:{}", surface(function))];
        let right = vec![
            format!("RL:{}", surface(0)),
            format!("RH:{}", surface(head)),
            format!("RF:{}", surface(function)),
        ];
        if schema.is_particle(morphemes, function) {
            gap.push(format!("GCASE:{}", surface(function)));
        }

        let child = if schema.is_lexicalized(morphemes, function) {
            surface(function).to_string()
        } else {
            morphemes
                .get(function)
                .and_then(|morpheme| conjugation_form(schema, morpheme))
                .map(str::to_string)
                .or_else(|| {
                    morphemes
                        .get(function)
                        .map(|morpheme| schema.levels_of(morpheme, levels.clone()))
                })
                .unwrap_or_default()
        };

        Self {
            statics,
            left,
            right,
            gap,
            after,
            child,
            opens,
            closes,
        }
    }
}

/// The conjugation form of a morpheme.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morpheme.
/// * `morpheme` - The morpheme.
///
/// # Returns
///
/// The form, or `None` when the schema has no column for it, the morpheme
/// does not have the column, or the column holds `*`.
fn conjugation_form<'a, M: Morpheme>(schema: &Schema, morpheme: &'a M) -> Option<&'a str> {
    schema
        .conjugation_form
        .and_then(|index| morpheme.feature(index))
        .filter(|form| *form != EMPTY)
}

/// Appends the features of one morpheme of a bunsetsu: its surface, the
/// levels of its part of speech up to the first that holds nothing, and its
/// conjugation form.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morpheme.
/// * `levels` - The columns of the part of speech the features read.
/// * `role` - The letter of the morpheme in the bunsetsu: `H` for the head,
///   `F` for the function word, `L` for the first, `R` for the last, `N`
///   for the one after the head.
/// * `morpheme` - The morpheme.
/// * `features` - The features to append to.
fn push_morpheme<M: Morpheme>(
    schema: &Schema,
    levels: Range<usize>,
    role: char,
    morpheme: &M,
    features: &mut Vec<String>,
) {
    features.push(format!("{role}S:{}", schema.surface_of(morpheme)));
    for (level, index) in levels.enumerate() {
        match morpheme.feature(index) {
            Some(value) if value != EMPTY => features.push(format!("{role}P{level}:{value}")),
            _ => break,
        }
    }
    if let Some(form) = conjugation_form(schema, morpheme) {
        features.push(format!("{role}F:{form}"));
    }
}

#[cfg(test)]
// A sentence that is one bunsetsu is given as `[0..n]`: an array of one
// range, which is what is meant.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;
    use crate::lattice::LatticeMorpheme;
    use crate::parser::fixtures;

    /// The features of the bunsetsu of a sentence, as strings.
    fn prepared(
        set: &FeatureSet,
        sentence: &(Vec<LatticeMorpheme>, Vec<Range<usize>>),
    ) -> Prepared<String> {
        Prepared::new(
            &Schema::ipadic(),
            set,
            &sentence.0,
            &sentence.1,
            |feature| Some(feature.to_string()),
        )
    }

    /// The features of a decision, as strings.
    fn decision(
        prepared: &Prepared<String>,
        source: usize,
        target: usize,
        children: &[Vec<usize>],
    ) -> Vec<String> {
        let mut features = Vec::new();
        prepared.decision(source, target, children, &mut features);
        features
    }

    /// No bunsetsu attached to any of `count` bunsetsu.
    fn no_children(count: usize) -> Vec<Vec<usize>> {
        vec![Vec::new(); count]
    }

    /// Whether a feature string is of a group.
    fn is_of(group: Group, feature: &str) -> bool {
        match group {
            Group::Distance => feature.starts_with("DIST:"),
            Group::Source => feature.starts_with('S') && !feature.starts_with("SN"),
            Group::Target => {
                feature.starts_with('D')
                    && !feature.starts_with("DIST:")
                    && !feature.starts_with("DN")
            }
            Group::Left => feature.starts_with("LF:"),
            Group::Right => feature.starts_with('R'),
            Group::Children => feature.starts_with("a:") || feature.starts_with("A:"),
            Group::Gap => feature.starts_with('G'),
            Group::AfterHead => feature.starts_with("SN") || feature.starts_with("DN"),
        }
    }

    #[test]
    fn the_surfaces_of_the_features_are_the_normalized_ones_of_the_schema() {
        // Ａ ｦ / Ｂ （ ｦ / Ｃ ｦ / Ｄ ｦ / 本: every surface of a feature, and
        // the bracket.
        let noun = "名詞,一般,*,*,*,*,*";
        let particle = "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ";
        let morphemes = vec![
            LatticeMorpheme::new("Ａ", noun),
            LatticeMorpheme::new("ｦ", particle),
            LatticeMorpheme::new("Ｂ", noun),
            LatticeMorpheme::new("（", "名詞,サ変接続,*,*,*,*,*"),
            LatticeMorpheme::new("ｦ", particle),
            LatticeMorpheme::new("Ｃ", noun),
            LatticeMorpheme::new("ｦ", particle),
            LatticeMorpheme::new("Ｄ", noun),
            LatticeMorpheme::new("ｦ", particle),
            LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
        ];
        let ranges = vec![0..2, 2..5, 5..7, 7..9, 9..10];
        // The first bunsetsu is attached to the second.
        let mut children = no_children(5);
        children[1].push(0);
        let features = |schema: &Schema, source: usize, target: usize| -> Vec<String> {
            let prepared = Prepared::new(
                schema,
                &FeatureSet::cabocha(),
                &morphemes,
                &ranges,
                |feature| Some(feature.to_string()),
            );
            decision(&prepared, source, target, &children)
        };
        let schema = Schema {
            normalization: crate::schema::Normalization::new(&[
                ("Ａ", "A"),
                ("Ｂ", "B"),
                ("Ｃ", "C"),
                ("Ｄ", "D"),
                ("（", "("),
                ("ｦ", "ヲ"),
            ])
            .unwrap(),
            ..Schema::ipadic()
        };
        // Ｂ （ ｦ -> Ｄ ｦ, over Ｃ ｦ, with Ａ ｦ before and 本 after.
        let normalized = features(&schema, 1, 3);
        for feature in [
            // The source and the target.
            "SHS:B",
            "SLS:B",
            "SFS:ヲ",
            "SRS:ヲ",
            "SOB:(",
            "SOB:1",
            "SCASE:ヲ",
            "DHS:D",
            "DCASE:ヲ",
            // The bunsetsu before the source, and the one attached to it.
            "LF:ヲ",
            "a:ヲ",
            // The bunsetsu in between.
            "GCASE:ヲ",
        ] {
            assert!(
                normalized.iter().any(|one| one == feature),
                "{feature}: {normalized:?}"
            );
        }
        // Ａ ｦ -> Ｂ （ ｦ, with Ｃ ｦ after, and the bracket in between for
        // Ａ ｦ -> Ｃ ｦ.
        let first = features(&schema, 0, 1);
        for feature in ["RL:C", "RH:C", "RF:ヲ", "DOB:("] {
            assert!(
                first.iter().any(|one| one == feature),
                "{feature}: {first:?}"
            );
        }
        let over = features(&schema, 0, 2);
        assert!(over.iter().any(|one| one == "GOB:("), "{over:?}");
        for all in [&normalized, &first, &over] {
            assert!(
                !all.iter()
                    .any(|one| "ＡＢＣＤ（ｦ".chars().any(|wide| one.contains(wide))),
                "{all:?}"
            );
        }
        // Without a normalization the surfaces are as they are.
        let plain = features(&Schema::ipadic(), 1, 3);
        for feature in [
            "SHS:Ｂ",
            "SFS:ｦ",
            "SOB:（",
            "SCASE:ｦ",
            "DHS:Ｄ",
            "LF:ｦ",
            "a:ｦ",
            "GCASE:ｦ",
        ] {
            assert!(
                plain.iter().any(|one| one == feature),
                "{feature}: {plain:?}"
            );
        }
    }

    #[test]
    fn a_decision_has_the_features_of_cabocha() {
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::sentence());
        // 彼 は -> 「 本, with 」 を after the target.
        let mut expected = vec![
            "DIST:1",
            // The source: its head 彼, its function word は, its first
            // morpheme 彼 and its last は.
            "SHS:彼",
            "SHP0:名詞",
            "SHP1:代名詞",
            "SHP2:一般",
            "SFS:は",
            "SFP0:助詞",
            "SFP1:係助詞",
            "SLS:彼",
            "SLP0:名詞",
            "SLP1:代名詞",
            "SLP2:一般",
            "SRS:は",
            "SRP0:助詞",
            "SRP1:係助詞",
            "SCASE:は",
            "SBOS:1",
            // The target: the bracket is no head, so 本 is the head and
            // the function word.
            "DHS:本",
            "DHP0:名詞",
            "DHP1:一般",
            "DFS:本",
            "DFP0:名詞",
            "DFP1:一般",
            "DLS:「",
            "DLP0:記号",
            "DLP1:括弧開",
            "DRS:本",
            "DRP0:名詞",
            "DRP1:一般",
            "DOB:「",
            "DOB:1",
            // After the target: 」 を, whose head stays at its first
            // morpheme.
            "RL:」",
            "RH:」",
            "RF:を",
            // Nothing in between.
            "GNB:1",
        ];
        expected.sort_unstable();
        assert_eq!(decision(&prepared, 0, 1, &no_children(5)), expected);
    }

    #[test]
    fn the_conjugation_form_and_the_marks_of_a_bunsetsu_are_features() {
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::sentence());
        // 静か に 、 -> 読ん だ 。
        let features = decision(&prepared, 3, 4, &no_children(5));
        let missing: Vec<&str> = [
            "SPUNC:、",
            "SCASE:に",
            "LF:を",
            "DHF:連用タ接続",
            "DFF:基本形",
            "DPUNC:。",
            "DEOS:1",
        ]
        .into_iter()
        .filter(|expected| !features.iter().any(|feature| feature == expected))
        .collect();
        assert!(missing.is_empty(), "{missing:?} not in {features:?}");
        // The target is the last bunsetsu: nothing is after it, and the
        // source is not the first.
        let present: Vec<&str> = ["RL:", "RH:", "RF:", "SBOS:", "SEOS:", "DBOS:"]
            .into_iter()
            .filter(|absent| features.iter().any(|feature| feature.starts_with(absent)))
            .collect();
        assert!(present.is_empty(), "{present:?} in {features:?}");
        // A closing bracket: 」 を -> 静か に 、.
        let features = decision(&prepared, 2, 3, &no_children(5));
        let missing: Vec<&str> = ["SCB:」", "SCB:1"]
            .into_iter()
            .filter(|expected| !features.iter().any(|feature| feature == expected))
            .collect();
        assert!(missing.is_empty(), "{missing:?} not in {features:?}");
    }

    #[test]
    fn the_levels_of_the_part_of_speech_stop_at_the_first_empty_one() {
        // A level after an empty one is not read, as in CaboCha.
        let morphemes = [LatticeMorpheme::new("語", "名詞,*,一般,*,*,*,語,ゴ,ゴ")];
        let strings = Strings::new(&Schema::ipadic(), 0..4, &morphemes, false, false);
        let levels: Vec<&str> = strings
            .statics
            .iter()
            .map(String::as_str)
            .filter(|feature| feature.starts_with("HP"))
            .collect();
        assert_eq!(levels, ["HP0:名詞"]);
    }

    #[test]
    fn a_limit_holds_the_features_to_that_many_levels_of_the_part_of_speech() {
        // A schema of four levels, as the one of UniDic.
        let schema = Schema {
            part_of_speech: 0..4,
            conjugation_form: Some(5),
            ..Schema::default()
        };
        let all = FeatureSet::cabocha();
        let two = FeatureSet::cabocha().with_part_of_speech_levels(2).unwrap();
        assert_eq!(all.levels(&schema), 0..4);
        assert_eq!(two.levels(&schema), 0..2);
        // A limit past the levels of the schema is all of them.
        let nine = FeatureSet::cabocha().with_part_of_speech_levels(9).unwrap();
        assert_eq!(nine.levels(&schema), 0..4);
        // A schema that starts later is cut from its start.
        let later = Schema {
            part_of_speech: 3..7,
            ..Schema::default()
        };
        assert_eq!(two.levels(&later), 3..5);

        // 太郎: four levels, and no conjugation form.
        let morphemes = [LatticeMorpheme::new(
            "太郎",
            "名詞,固有名詞,人名,名,*,*,タロウ,タロウ",
        )];
        let levels = |set: &FeatureSet| -> (Vec<String>, String) {
            let strings = Strings::new(&schema, set.levels(&schema), &morphemes, false, false);
            let names = strings
                .statics
                .iter()
                .filter(|feature| feature.starts_with("HP"))
                .cloned()
                .collect();
            (names, strings.child)
        };
        // The features of the levels, and the feature of the bunsetsu as a
        // child: its part of speech, as it has no conjugation form.
        assert_eq!(
            levels(&all),
            (
                vec![
                    "HP0:名詞".to_string(),
                    "HP1:固有名詞".to_string(),
                    "HP2:人名".to_string(),
                    "HP3:名".to_string()
                ],
                "名詞-固有名詞-人名-名".to_string()
            )
        );
        assert_eq!(
            levels(&two),
            (
                vec!["HP0:名詞".to_string(), "HP1:固有名詞".to_string()],
                "名詞-固有名詞".to_string()
            )
        );
        // The limit reaches the features of a decision: 太郎 -> 太郎.
        let pair = [morphemes[0].clone(), morphemes[0].clone()];
        let decisions = |set: &FeatureSet| -> Vec<String> {
            let prepared = Prepared::new(&schema, set, &pair, &[0..1, 1..2], |feature| {
                Some(feature.to_string())
            });
            decision(&prepared, 0, 1, &no_children(2))
        };
        assert!(decisions(&all).iter().any(|one| one == "SHP3:名"));
        assert!(!decisions(&two).iter().any(|one| one == "SHP2:人名"));
        assert!(decisions(&two).iter().any(|one| one == "SHP1:固有名詞"));
    }

    #[test]
    fn the_limit_of_the_levels_is_part_of_the_text_of_a_set() {
        let set = FeatureSet::parse("gap, pos-levels=2 ,source").unwrap();
        assert_eq!(set.groups(), [Group::Source, Group::Gap]);
        assert_eq!(set.part_of_speech_levels(), Some(2));
        // It is written after the groups, and read back.
        assert_eq!(set.to_string(), "source,gap,pos-levels=2");
        assert_eq!(FeatureSet::parse(&set.to_string()).unwrap(), set);
        // A set without a limit is written as before, and is another set.
        let plain = FeatureSet::parse("source,gap").unwrap();
        assert_eq!(plain.part_of_speech_levels(), None);
        assert_eq!(plain.to_string(), "source,gap");
        assert_ne!(plain, set);
        assert_eq!(FeatureSet::cabocha().part_of_speech_levels(), None);
        assert_eq!(FeatureSet::default().part_of_speech_levels(), None);

        let message = |text: &str| FeatureSet::parse(text).unwrap_err().to_string();
        assert_eq!(
            message("source,pos-levels=0"),
            "the features need a level of the part of speech: pos-levels is 1 or more"
        );
        assert_eq!(
            message("source,pos-levels=two"),
            "\"pos-levels=two\" is not a number of levels of the part of speech"
        );
        assert_eq!(
            message("source,pos-levels="),
            "\"pos-levels=\" is not a number of levels of the part of speech"
        );
        assert_eq!(
            message("pos-levels=2,source,pos-levels=3"),
            "pos-levels= is given twice"
        );
        // The limit alone is not a set.
        assert_eq!(message("pos-levels=2"), "a feature set needs a group");
        assert!(matches!(
            FeatureSet::cabocha().with_part_of_speech_levels(0),
            Err(Error::Parser(_))
        ));
    }

    #[test]
    fn the_distance_has_three_classes() {
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::repeated(8));
        let classes: Vec<String> = [1, 2, 5, 6, 7]
            .iter()
            .map(|&target| {
                decision(&prepared, 0, target, &no_children(8))
                    .into_iter()
                    .find(|feature| feature.starts_with("DIST:"))
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(
            classes,
            ["DIST:1", "DIST:2-5", "DIST:2-5", "DIST:6-", "DIST:6-"]
        );
    }

    #[test]
    fn the_gap_has_the_marks_in_between_and_the_state_of_the_brackets() {
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::sentence());
        let gap = |source: usize, target: usize| -> Vec<String> {
            decision(&prepared, source, target, &no_children(5))
                .into_iter()
                .filter(|feature| feature.starts_with('G'))
                .collect()
        };
        // Nothing in between: the brackets of the source and of the target
        // do not count.
        assert_eq!(gap(1, 2), ["GNB:1"]);
        // 「 本 in between: opening only.
        assert_eq!(gap(0, 2), ["GOB:1", "GOB:「"]);
        // 」 を in between: closing only, and the particle.
        assert_eq!(gap(1, 3), ["GCASE:を", "GCB:1", "GCB:」"]);
        // Both, and the comma of 静か に 、.
        assert_eq!(
            gap(0, 4),
            [
                "GBB:1",
                "GCASE:に",
                "GCASE:を",
                "GCB:」",
                "GOB:「",
                "GPUNC:、"
            ]
        );
    }

    #[test]
    fn the_children_are_named_by_their_function_word() {
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::sentence());
        let children = |features: Vec<String>| -> Vec<String> {
            features
                .into_iter()
                .filter(|feature| is_of(Group::Children, feature))
                .collect()
        };
        // Nothing attached yet.
        assert_eq!(
            children(decision(&prepared, 1, 3, &no_children(5))),
            [""; 0]
        );
        // 彼 は is attached to the source, 」 を to the target: particles,
        // by their surface.
        let attached = vec![vec![], vec![0], vec![], vec![2], vec![]];
        assert_eq!(
            children(decision(&prepared, 1, 3, &attached)),
            ["A:を", "a:は"]
        );
        // 「 本 ends in a noun, which has no conjugation form: its part of
        // speech. 読ん だ 。 ends in an auxiliary: its conjugation form.
        let attached = vec![vec![], vec![], vec![1], vec![4], vec![]];
        assert_eq!(
            children(decision(&prepared, 2, 3, &attached)),
            ["A:基本形", "a:名詞-一般"]
        );
    }

    #[test]
    fn a_feature_is_in_a_decision_once() {
        // 本 を twice in between: one GCASE:を.
        let prepared = prepared(&FeatureSet::cabocha(), &fixtures::repeated(4));
        let features = decision(&prepared, 0, 3, &no_children(4));
        let particles = features
            .iter()
            .filter(|feature| *feature == "GCASE:を")
            .count();
        assert_eq!(particles, 1);
        let mut sorted = features.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(features, sorted);
    }

    #[test]
    fn a_feature_that_is_not_interned_is_left_out() {
        let (morphemes, ranges) = fixtures::sentence();
        let known = ["GNB:1", "RF:を", "SFS:は", "DIST:1"];
        let prepared: Prepared<u32> = Prepared::new(
            &Schema::ipadic(),
            &FeatureSet::cabocha(),
            &morphemes,
            &ranges,
            |feature| {
                known
                    .iter()
                    .position(|one| *one == feature)
                    .map(|index| index as u32)
            },
        );
        let mut features = Vec::new();
        prepared.decision(0, 1, &no_children(5), &mut features);
        // In the order of the numbers, not of the making.
        assert_eq!(features, [0, 1, 2, 3]);
    }

    #[test]
    fn a_group_left_out_gives_none_of_its_features() {
        let sentence = fixtures::sentence();
        let attached = vec![vec![], vec![0], vec![], vec![2], vec![]];
        let all = decision(
            &prepared(&FeatureSet::default(), &sentence),
            1,
            3,
            &attached,
        );
        let mut failures = Vec::new();
        for (group, name) in Group::ALL {
            // The decision has features of every group.
            if !all.iter().any(|feature| is_of(group, feature)) {
                failures.push(format!("the decision has no feature of {name}"));
            }
            let others: Vec<Group> = Group::ALL
                .iter()
                .map(|(other, _)| *other)
                .filter(|other| *other != group)
                .collect();
            let without = decision(
                &prepared(&FeatureSet::new(&others).unwrap(), &sentence),
                1,
                3,
                &attached,
            );
            let expected: Vec<String> = all
                .iter()
                .filter(|feature| !is_of(group, feature))
                .cloned()
                .collect();
            if without != expected {
                failures.push(format!("without {name}: {without:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn the_morpheme_after_the_head_is_a_feature_of_the_source_and_of_the_target() {
        // 利用 し た / 読む こと を / 知っ た: the heads are 利用 and 読む, as
        // the schema of IPADIC has them, and し and こと follow them.
        let morphemes: Vec<LatticeMorpheme> = [
            ("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
            ("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
            ("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
            ("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
            ("こと", "名詞,非自立,一般,*,*,*,こと,コト,コト"),
            ("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
            ("知っ", "動詞,自立,*,*,五段・ラ行,連用タ接続,知る,シッ,シッ"),
            ("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
        ]
        .iter()
        .map(|(surface, features)| LatticeMorpheme::new(*surface, *features))
        .collect();
        let sentence = (morphemes, vec![0..3, 3..6, 6..8]);
        let after = |set: &FeatureSet, source: usize, target: usize| -> Vec<String> {
            decision(&prepared(set, &sentence), source, target, &no_children(3))
                .into_iter()
                .filter(|feature| is_of(Group::AfterHead, feature))
                .collect()
        };
        assert_eq!(
            after(&FeatureSet::default(), 0, 1),
            [
                "DNP0:名詞",
                "DNP1:非自立",
                "DNP2:一般",
                "DNS:こと",
                "SNF:連用形",
                "SNP0:動詞",
                "SNP1:自立",
                "SNS:し"
            ]
        );
        // The head of 知っ た is 知っ, and た follows it.
        assert_eq!(
            after(&FeatureSet::default(), 1, 2),
            [
                "DNF:基本形",
                "DNP0:助動詞",
                "DNS:た",
                "SNP0:名詞",
                "SNP1:非自立",
                "SNP2:一般",
                "SNS:こと"
            ]
        );
        // CaboCha's set has none of them, and a head that ends its
        // bunsetsu gives none.
        assert!(after(&FeatureSet::cabocha(), 0, 1).is_empty());
        let (morphemes, _) = &sentence;
        let alone = (morphemes[3..4].to_vec(), vec![0..1]);
        let prepared = prepared(&FeatureSet::default(), &alone);
        assert!(prepared.bunsetsu[0].after_source.is_empty());
        assert!(prepared.bunsetsu[0].after_target.is_empty());
    }

    #[test]
    fn a_feature_set_is_read_from_its_text_and_written_back() {
        assert_eq!(
            FeatureSet::cabocha().to_string(),
            "distance,source,target,left,right,children,gap"
        );
        // The default has the morpheme after the head as well.
        assert_eq!(
            FeatureSet::default().to_string(),
            "distance,source,target,left,right,children,gap,after"
        );
        assert!(!FeatureSet::cabocha().has(Group::AfterHead));
        assert_eq!(Group::ALL.len(), 8);
        // The order of the text does not matter, nor do spaces.
        let set = FeatureSet::parse("gap, source ,distance").unwrap();
        assert_eq!(set.groups(), [Group::Distance, Group::Source, Group::Gap]);
        assert_eq!(set.to_string(), "distance,source,gap");
        assert_eq!(FeatureSet::parse(&set.to_string()).unwrap(), set);
        assert!(set.has(Group::Gap));
        assert!(!set.has(Group::Children));
        for (group, name) in Group::ALL {
            assert_eq!(group.name(), name);
            assert_eq!(Group::from_name(name), Some(group));
        }
    }

    #[test]
    fn a_feature_set_that_cannot_be_read_is_refused() {
        let accepted: Vec<&str> = ["", "distance,", "distance,head", "gap,distance,gap"]
            .into_iter()
            .filter(|text| !matches!(FeatureSet::parse(text), Err(Error::Parser(_))))
            .collect();
        assert!(accepted.is_empty(), "not refused: {accepted:?}");
        assert!(matches!(FeatureSet::new(&[]), Err(Error::Parser(_))));
    }
}
