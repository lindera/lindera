//! The normalization of a surface before it is used.

use std::borrow::Cow;

use crate::bytes::{Reader, push_string, push_u32};
use crate::error::{Error, Result};

/// The largest number of rules a table read from a model file may have.
const MAX_RULES: u32 = 1 << 20;

/// A table that replaces parts of a surface before the surface becomes a
/// feature of the chunker or of the parser, or is matched as punctuation
/// or as a bracket.
///
/// A rule replaces a string by another one. At each position of a surface
/// the longest rule that matches there is applied; where none matches, one
/// character is kept as it is. The result of a rule is not looked at
/// again. An empty table changes nothing, which is what the schemas of
/// this crate have: they recognize a mark in its full-width and its
/// half-width form instead.
///
/// The table is there for models made by another program that normalizes:
/// CaboCha writes the half-width katakana in full width, and the
/// full-width letters, digits and signs in ASCII, before it makes a
/// feature. A model converted from CaboCha carries CaboCha's table, so
/// that its features are the ones CaboCha makes.
///
/// # Examples
///
/// ```
/// use lindera_dependency::schema::Normalization;
///
/// let table = Normalization::new(&[("ｶ", "カ"), ("ｶﾞ", "ガ"), ("１", "1")]).unwrap();
/// // The longest rule first: ｶﾞ is ガ, not カ and a mark.
/// assert_eq!(table.apply("ｶﾞｶ１番"), "ガカ1番");
/// assert_eq!(Normalization::default().apply("ｶﾞ"), "ｶﾞ");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Normalization {
    /// The rules, sorted by the bytes of what they replace, each once.
    rules: Vec<(String, String)>,
    /// The number of bytes of the longest string a rule replaces.
    longest: usize,
}

impl Normalization {
    /// Makes a table.
    ///
    /// # Arguments
    ///
    /// * `rules` - The rules: what is replaced, and by what. Their order
    ///   does not matter. A rule given twice with the same result is one
    ///   rule.
    ///
    /// # Returns
    ///
    /// The table, or [`Error::Schema`] for a rule that replaces an empty
    /// string, or for two rules that replace the same string by different
    /// ones.
    pub fn new(rules: &[(&str, &str)]) -> Result<Self> {
        let mut sorted: Vec<(String, String)> = rules
            .iter()
            .map(|(from, to)| (from.to_string(), to.to_string()))
            .collect();
        sorted.sort();
        sorted.dedup();
        Self::from_sorted(sorted)
    }

    /// Makes a table from rules that are sorted.
    ///
    /// # Arguments
    ///
    /// * `rules` - The rules, sorted by what they replace.
    ///
    /// # Returns
    ///
    /// The table, or [`Error::Schema`] for a rule that replaces an empty
    /// string, or for rules that are not sorted with each string once.
    fn from_sorted(rules: Vec<(String, String)>) -> Result<Self> {
        if let Some((_, to)) = rules.iter().find(|(from, _)| from.is_empty()) {
            return Err(Error::Schema(format!(
                "a rule of the normalization replaces an empty string by {to:?}"
            )));
        }
        if let Some(pair) = rules.windows(2).find(|pair| pair[0].0 >= pair[1].0) {
            return Err(Error::Schema(format!(
                "the rules of the normalization replace {:?} twice, or are not sorted",
                pair[1].0
            )));
        }
        let longest = rules.iter().map(|(from, _)| from.len()).max().unwrap_or(0);
        Ok(Self { rules, longest })
    }

    /// Whether the table has no rule, and changes nothing.
    ///
    /// # Returns
    ///
    /// Whether it is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rules of the table: what is replaced, and by what, sorted by
    /// the bytes of what is replaced.
    ///
    /// # Returns
    ///
    /// The rules.
    pub fn rules(&self) -> impl Iterator<Item = (&str, &str)> {
        self.rules
            .iter()
            .map(|(from, to)| (from.as_str(), to.as_str()))
    }

    /// Normalizes a surface.
    ///
    /// # Arguments
    ///
    /// * `surface` - The surface.
    ///
    /// # Returns
    ///
    /// The surface with the rules applied; the surface itself, borrowed,
    /// when no rule matches in it.
    pub fn apply<'a>(&self, surface: &'a str) -> Cow<'a, str> {
        if self.rules.is_empty() {
            return Cow::Borrowed(surface);
        }
        let mut output: Option<String> = None;
        let mut position = 0;
        while position < surface.len() {
            let rest = &surface[position..];
            let step = match self.longest_rule(rest) {
                Some((from, to)) => {
                    output
                        .get_or_insert_with(|| surface[..position].to_string())
                        .push_str(to);
                    from.len()
                }
                None => {
                    let step = rest.chars().next().map_or(1, char::len_utf8);
                    if let Some(output) = output.as_mut() {
                        output.push_str(&rest[..step]);
                    }
                    step
                }
            };
            position += step;
        }
        output.map_or(Cow::Borrowed(surface), Cow::Owned)
    }

    /// The longest rule that matches at the start of a string.
    ///
    /// # Arguments
    ///
    /// * `rest` - The string.
    ///
    /// # Returns
    ///
    /// What the rule replaces and by what, or `None` when no rule matches.
    fn longest_rule(&self, rest: &str) -> Option<(&str, &str)> {
        let mut end = self.longest.min(rest.len());
        while end > 0 {
            if rest.is_char_boundary(end) {
                let prefix = &rest[..end];
                if let Ok(index) = self
                    .rules
                    .binary_search_by(|(from, _)| from.as_str().cmp(prefix))
                {
                    let (from, to) = &self.rules[index];
                    return Some((from.as_str(), to.as_str()));
                }
            }
            end -= 1;
        }
        None
    }

    /// Appends the table to the bytes of a schema: the number of rules,
    /// then for each what it replaces and by what.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    pub(super) fn write(&self, bytes: &mut Vec<u8>) {
        push_u32(bytes, self.rules.len() as u32);
        for (from, to) in &self.rules {
            push_string(bytes, from);
            push_string(bytes, to);
        }
    }

    /// Reads a table from the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the table.
    ///
    /// # Returns
    ///
    /// The table, or the error of bytes that are not one.
    pub(super) fn read(reader: &mut Reader) -> Result<Self> {
        let count = reader.u32()?;
        if count > MAX_RULES {
            return Err(reader.error(format_args!(
                "has {count} rules of normalization, more than {MAX_RULES}"
            )));
        }
        let mut rules = Vec::new();
        for _ in 0..count {
            let from = reader.string()?;
            let to = reader.string()?;
            rules.push((from, to));
        }
        Self::from_sorted(rules).map_err(|error| {
            reader.error(format_args!("has a normalization that is not one: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_longest_rule_at_a_position_is_applied() {
        let table = Normalization::new(&[("ｶ", "カ"), ("ｶﾞ", "ガ"), ("ﾞ", "゛")]).unwrap();
        assert_eq!(table.apply("ｶﾞ"), "ガ");
        assert_eq!(table.apply("ｶ"), "カ");
        assert_eq!(table.apply("ｶｶﾞﾞ"), "カガ゛");
    }

    #[test]
    fn what_no_rule_matches_is_kept() {
        let table = Normalization::new(&[("１", "1"), ("　", "___")]).unwrap();
        assert_eq!(table.apply("第１０回　大会"), "第1０回___大会");
        assert_eq!(table.apply(""), "");
    }

    #[test]
    fn the_result_of_a_rule_is_not_normalized_again() {
        let table = Normalization::new(&[("a", "b"), ("b", "c")]).unwrap();
        assert_eq!(table.apply("ab"), "bc");
    }

    #[test]
    fn a_surface_without_a_match_is_borrowed() {
        let table = Normalization::new(&[("１", "1")]).unwrap();
        assert!(matches!(table.apply("読ん"), Cow::Borrowed("読ん")));
        assert!(matches!(table.apply("１"), Cow::Owned(_)));
        assert!(matches!(
            Normalization::default().apply("１"),
            Cow::Borrowed("１")
        ));
    }

    #[test]
    fn an_empty_table_changes_nothing() {
        let table = Normalization::default();
        assert!(table.is_empty());
        assert_eq!(table.apply("ｶﾞ１　"), "ｶﾞ１　");
        assert_eq!(table, Normalization::new(&[]).unwrap());
    }

    #[test]
    fn the_rules_are_sorted_and_each_is_there_once() {
        let table = Normalization::new(&[("ｲ", "イ"), ("ｱ", "ア"), ("ｲ", "イ")]).unwrap();
        let rules: Vec<(&str, &str)> = table.rules().collect();
        assert_eq!(rules, [("ｱ", "ア"), ("ｲ", "イ")]);
        assert!(!table.is_empty());
    }

    #[test]
    fn refuses_rules_that_are_not_a_table() {
        let error = Normalization::new(&[("", "x")]).unwrap_err();
        assert!(error.to_string().contains("empty string"), "{error}");
        let error = Normalization::new(&[("a", "x"), ("a", "y")]).unwrap_err();
        assert!(error.to_string().contains("\"a\" twice"), "{error}");
    }

    #[test]
    fn a_table_is_read_as_it_was_written() {
        let table = Normalization::new(&[("ｶﾞ", "ガ"), ("１", "1"), ("　", "___")]).unwrap();
        let mut bytes = Vec::new();
        table.write(&mut bytes);
        let mut reader = Reader::new(&bytes, "model", Error::Model);
        assert_eq!(Normalization::read(&mut reader).unwrap(), table);

        let mut bytes = Vec::new();
        Normalization::default().write(&mut bytes);
        assert_eq!(bytes, [0, 0, 0, 0]);
    }

    #[test]
    fn refuses_bytes_that_are_not_a_table() {
        // Two rules that are not sorted.
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 2);
        for text in ["b", "x", "a", "y"] {
            push_string(&mut bytes, text);
        }
        let error =
            Normalization::read(&mut Reader::new(&bytes, "model", Error::Model)).unwrap_err();
        assert!(error.to_string().contains("normalization"), "{error}");

        // More rules than a table may have.
        let mut bytes = Vec::new();
        push_u32(&mut bytes, MAX_RULES + 1);
        let error =
            Normalization::read(&mut Reader::new(&bytes, "model", Error::Model)).unwrap_err();
        assert!(
            error.to_string().contains("rules of normalization"),
            "{error}"
        );

        // A table that ends before its rules.
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 1);
        assert!(Normalization::read(&mut Reader::new(&bytes, "model", Error::Model)).is_err());
    }
}
