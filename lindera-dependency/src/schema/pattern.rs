//! Patterns over the morphemes of a bunsetsu.

use super::Schema;
use crate::bytes::{Reader, push_strings};
use crate::error::Result;
use crate::morpheme::Morpheme;

/// How deep the `after` of a pattern may go in the bytes of a schema.
const MAX_DEPTH: usize = 8;

/// A pattern a morpheme of a bunsetsu matches or not: a prefix of its
/// part-of-speech levels, and optionally its base form and the morpheme
/// before it.
///
/// CaboCha matches a prefix of the whole feature string; a pattern states
/// the same thing over the columns of a [`Schema`], so it holds no column
/// number. Every condition has to hold. The pattern without a condition
/// ([`Pattern::default`]) matches every morpheme.
///
/// # Examples
///
/// ```
/// use lindera_dependency::lattice::LatticeMorpheme;
/// use lindera_dependency::{Pattern, Schema};
///
/// let schema = Schema::ipadic();
/// let bunsetsu = [
///     LatticeMorpheme::new("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
///     LatticeMorpheme::new("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
/// ];
///
/// // する after a verbal noun.
/// let pattern = Pattern::new(&["動詞", "自立"])
///     .with_base_forms(&["する"])
///     .after(Pattern::new(&["名詞", "サ変接続"]));
/// assert!(pattern.matches(&schema, &bunsetsu, 1));
/// assert!(!pattern.matches(&schema, &bunsetsu, 0));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pattern {
    /// The first part-of-speech levels the morpheme has to have, from the
    /// coarsest: `動詞`, `非自立`. Empty for any part of speech.
    pub part_of_speech: Vec<String>,
    /// The base forms the morpheme may have. Empty for any base form.
    pub base_forms: Vec<String>,
    /// What the morpheme before it in the bunsetsu has to match. With it,
    /// the first morpheme of a bunsetsu never matches.
    pub after: Option<Box<Pattern>>,
}

impl Pattern {
    /// Makes a pattern for a part of speech.
    ///
    /// # Arguments
    ///
    /// * `part_of_speech` - The first part-of-speech levels, from the
    ///   coarsest.
    ///
    /// # Returns
    ///
    /// The pattern, without a condition on the base form or on the morpheme
    /// before.
    pub fn new<S: AsRef<str>>(part_of_speech: &[S]) -> Self {
        Self {
            part_of_speech: owned(part_of_speech),
            base_forms: Vec::new(),
            after: None,
        }
    }

    /// Limits the pattern to some base forms.
    ///
    /// # Arguments
    ///
    /// * `base_forms` - The base forms the morpheme may have.
    ///
    /// # Returns
    ///
    /// The pattern with the condition.
    pub fn with_base_forms<S: AsRef<str>>(mut self, base_forms: &[S]) -> Self {
        self.base_forms = owned(base_forms);
        self
    }

    /// Limits the pattern to a morpheme that follows another one in its
    /// bunsetsu.
    ///
    /// # Arguments
    ///
    /// * `before` - What the morpheme before has to match;
    ///   [`Pattern::default`] for any morpheme.
    ///
    /// # Returns
    ///
    /// The pattern with the condition.
    pub fn after(mut self, before: Pattern) -> Self {
        self.after = Some(Box::new(before));
        self
    }

    /// Whether a morpheme of a bunsetsu matches the pattern.
    ///
    /// A column the morpheme does not have matches nothing, and neither
    /// does a base form when the schema has no column for it.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morphemes.
    /// * `bunsetsu` - The morphemes of the bunsetsu.
    /// * `index` - The index of the morpheme in the bunsetsu.
    ///
    /// # Returns
    ///
    /// Whether the morpheme matches; `false` for an index outside the
    /// bunsetsu.
    pub fn matches<M: Morpheme>(&self, schema: &Schema, bunsetsu: &[M], index: usize) -> bool {
        let Some(morpheme) = bunsetsu.get(index) else {
            return false;
        };
        if !has_part_of_speech(schema, morpheme, &self.part_of_speech) {
            return false;
        }
        if !self.base_forms.is_empty() {
            let base_form = schema.base_form.and_then(|column| morpheme.feature(column));
            if !base_form
                .is_some_and(|base_form| self.base_forms.iter().any(|one| one == base_form))
            {
                return false;
            }
        }
        match &self.after {
            None => true,
            Some(before) => index > 0 && before.matches(schema, bunsetsu, index - 1),
        }
    }

    /// Appends the pattern to the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    pub(super) fn write(&self, bytes: &mut Vec<u8>) {
        push_strings(bytes, &self.part_of_speech);
        push_strings(bytes, &self.base_forms);
        match &self.after {
            None => bytes.push(0),
            Some(before) => {
                bytes.push(1);
                before.write(bytes);
            }
        }
    }

    /// Reads a pattern from the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the pattern.
    /// * `depth` - How many patterns this one is the `after` of.
    ///
    /// # Returns
    ///
    /// The pattern, or the error of bytes that end early, of a flag that is
    /// neither 0 nor 1, or of patterns nested deeper than this crate writes.
    pub(super) fn read(reader: &mut Reader, depth: usize) -> Result<Self> {
        if depth > MAX_DEPTH {
            return Err(reader.error(format_args!("has patterns nested deeper than {MAX_DEPTH}")));
        }
        let part_of_speech = reader.strings()?;
        let base_forms = reader.strings()?;
        let after = match reader.u8()? {
            0 => None,
            1 => Some(Box::new(Self::read(reader, depth + 1)?)),
            flag => {
                return Err(reader.error(format_args!("has the unknown pattern flag {flag}")));
            }
        };
        Ok(Self {
            part_of_speech,
            base_forms,
            after,
        })
    }
}

/// Whether the part of speech of a morpheme starts with some levels.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morpheme.
/// * `morpheme` - The morpheme.
/// * `levels` - The levels, from the coarsest.
///
/// # Returns
///
/// Whether each level is the one of the morpheme; `false` when there are
/// more levels than the schema has part-of-speech columns, or than the
/// morpheme has columns.
pub(super) fn has_part_of_speech<M: Morpheme, S: AsRef<str>>(
    schema: &Schema,
    morpheme: &M,
    levels: &[S],
) -> bool {
    if levels.len() > schema.part_of_speech.len() {
        return false;
    }
    levels
        .iter()
        .zip(schema.part_of_speech.clone())
        .all(|(level, column)| morpheme.feature(column) == Some(level.as_ref()))
}

/// Strings as owned strings.
///
/// # Arguments
///
/// * `values` - The strings.
///
/// # Returns
///
/// The owned strings, in the same order.
pub(super) fn owned<S: AsRef<str>>(values: &[S]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.as_ref().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::lattice::LatticeMorpheme;

    /// 利用 / し / た, with the features of IPADIC.
    fn bunsetsu() -> Vec<LatticeMorpheme> {
        vec![
            LatticeMorpheme::new("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
            LatticeMorpheme::new("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
            LatticeMorpheme::new("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
        ]
    }

    #[test]
    fn the_part_of_speech_is_matched_by_a_prefix_of_its_levels() {
        let schema = Schema::ipadic();
        let bunsetsu = bunsetsu();
        assert!(Pattern::new(&["名詞"]).matches(&schema, &bunsetsu, 0));
        assert!(Pattern::new(&["名詞", "サ変接続"]).matches(&schema, &bunsetsu, 0));
        assert!(Pattern::new(&["名詞", "サ変接続", "*", "*"]).matches(&schema, &bunsetsu, 0));
        assert!(!Pattern::new(&["名詞", "一般"]).matches(&schema, &bunsetsu, 0));
        assert!(!Pattern::new(&["動詞"]).matches(&schema, &bunsetsu, 0));
        // A level is compared as a whole, not by its first characters.
        assert!(!Pattern::new(&["名"]).matches(&schema, &bunsetsu, 0));
        // More levels than the schema has are never matched: the fifth
        // column of IPADIC is the conjugation type.
        assert!(
            !Pattern::new(&["動詞", "自立", "*", "*", "サ変・スル"]).matches(&schema, &bunsetsu, 1)
        );
    }

    #[test]
    fn the_columns_of_the_part_of_speech_come_from_the_schema() {
        // A dictionary with one column before the part of speech.
        let schema = Schema {
            part_of_speech: 1..3,
            base_form: Some(3),
            ..Schema::default()
        };
        let bunsetsu = [LatticeMorpheme::new("読む", "x,動詞,自立,読む")];
        assert!(Pattern::new(&["動詞", "自立"]).matches(&schema, &bunsetsu, 0));
        assert!(!Pattern::new(&["x"]).matches(&schema, &bunsetsu, 0));
        assert!(
            Pattern::new(&["動詞"])
                .with_base_forms(&["読む"])
                .matches(&schema, &bunsetsu, 0)
        );
    }

    #[test]
    fn the_pattern_without_a_condition_matches_every_morpheme() {
        let schema = Schema::ipadic();
        let bunsetsu = bunsetsu();
        for index in 0..bunsetsu.len() {
            assert!(Pattern::default().matches(&schema, &bunsetsu, index));
        }
        assert!(!Pattern::default().matches(&schema, &bunsetsu, bunsetsu.len()));
    }

    #[test]
    fn the_base_form_limits_the_pattern() {
        let schema = Schema::ipadic();
        let bunsetsu = bunsetsu();
        let verb = Pattern::new(&["動詞", "自立"]);
        assert!(
            verb.clone()
                .with_base_forms(&["できる", "する"])
                .matches(&schema, &bunsetsu, 1)
        );
        assert!(
            !verb
                .clone()
                .with_base_forms(&["なる"])
                .matches(&schema, &bunsetsu, 1)
        );
        // The surface is not the base form.
        assert!(
            !verb
                .clone()
                .with_base_forms(&["し"])
                .matches(&schema, &bunsetsu, 1)
        );
        // Without a column for the base form, a base form matches nothing.
        let without = Schema {
            base_form: None,
            ..Schema::ipadic()
        };
        assert!(
            !verb
                .with_base_forms(&["する"])
                .matches(&without, &bunsetsu, 1)
        );
    }

    #[test]
    fn after_looks_at_the_morpheme_before_in_the_bunsetsu() {
        let schema = Schema::ipadic();
        let bunsetsu = bunsetsu();
        let verb = Pattern::new(&["動詞"]);
        assert!(
            verb.clone()
                .after(Pattern::new(&["名詞", "サ変接続"]))
                .matches(&schema, &bunsetsu, 1)
        );
        assert!(
            !verb
                .clone()
                .after(Pattern::new(&["助詞"]))
                .matches(&schema, &bunsetsu, 1)
        );
        // After any morpheme: every morpheme but the first.
        let any = Pattern::default().after(Pattern::default());
        assert!(!any.matches(&schema, &bunsetsu, 0));
        assert!(any.matches(&schema, &bunsetsu, 1));
        assert!(any.matches(&schema, &bunsetsu, 2));
        // The morpheme before, not any earlier one: た is after し, not
        // after 利用.
        let auxiliary = Pattern::new(&["助動詞"]);
        assert!(
            !auxiliary
                .clone()
                .after(Pattern::new(&["名詞"]))
                .matches(&schema, &bunsetsu, 2)
        );
        // A pattern before may have one before it too.
        assert!(
            auxiliary
                .after(verb.after(Pattern::new(&["名詞"])))
                .matches(&schema, &bunsetsu, 2)
        );
    }

    #[test]
    fn a_morpheme_with_fewer_columns_matches_no_column_it_lacks() {
        let schema = Schema::ipadic();
        let bunsetsu = [
            LatticeMorpheme::new("x", ""),
            LatticeMorpheme::new("y", "動詞"),
        ];
        assert!(Pattern::default().matches(&schema, &bunsetsu, 0));
        assert!(!Pattern::new(&["動詞"]).matches(&schema, &bunsetsu, 0));
        assert!(Pattern::new(&["動詞"]).matches(&schema, &bunsetsu, 1));
        assert!(!Pattern::new(&["動詞", "自立"]).matches(&schema, &bunsetsu, 1));
        assert!(
            !Pattern::new(&["動詞"])
                .with_base_forms(&["する"])
                .matches(&schema, &bunsetsu, 1)
        );
    }

    #[test]
    fn a_pattern_comes_back_from_its_bytes() {
        let pattern = Pattern::new(&["動詞", "自立"])
            .with_base_forms(&["する", "できる"])
            .after(Pattern::default().after(Pattern::new(&["名詞"])));
        let mut bytes = Vec::new();
        pattern.write(&mut bytes);
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        assert_eq!(Pattern::read(&mut reader, 0).unwrap(), pattern);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn bytes_that_are_not_a_pattern_are_errors() {
        // No part of speech, no base form, and the flag 2.
        let bytes = [0, 0, 0, 0, 0, 0, 0, 0, 2];
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        let message = Pattern::read(&mut reader, 0).unwrap_err().to_string();
        assert!(message.contains("unknown pattern flag 2"), "{message}");

        // Patterns that each have one before them, without an end.
        let bytes = [0, 0, 0, 0, 0, 0, 0, 0, 1].repeat(100);
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        let message = Pattern::read(&mut reader, 0).unwrap_err().to_string();
        assert!(message.contains("nested deeper than 8"), "{message}");
    }
}
