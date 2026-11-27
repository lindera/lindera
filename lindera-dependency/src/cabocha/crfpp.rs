//! Reading the chunker model of CaboCha: a CRF++ model in text.
//!
//! The file has five blocks, with an empty line between them:
//!
//! ```text
//! version: 100            the header: `key: value` lines
//! cost-factor: 1
//! maxid: 679502           the number of weights
//! xsize: 2                the number of columns
//!
//! B                       the labels
//! I
//!
//! U00:%x[-2,0]            the template
//! B
//!
//! 0 B                     the features: the index of the first weight of
//! 4 U00:!                 each, and its string
//!
//! -2.0282322282844056     the weights, one a line
//! ```
//!
//! A unigram feature has a weight for each label, in the order of the
//! labels. The bigram feature `B` has one for each pair of labels: the
//! label before, then the label after.

use std::collections::HashMap;

use crate::chunker::Label;
use crate::error::{Error, Result};

/// What the file says, as [`ChunkerModel::new`](crate::chunker::ChunkerModel::new)
/// takes it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CrfModel {
    /// The number of columns the template reads.
    pub(super) columns: usize,
    /// The template, as it is in the file.
    pub(super) template: String,
    /// The weights of the transitions between the labels, by the label
    /// before and the label after: 0 the boundary of a sentence, 1 `B`,
    /// 2 `I`. CRF++ has no weight at the boundaries.
    pub(super) transitions: [[f64; 3]; 3],
    /// The weights of each unigram feature: for `B`, then for `I`.
    pub(super) features: HashMap<String, [f64; 2]>,
}

/// Makes the error of a file that is not a chunker model.
fn error(problem: impl std::fmt::Display) -> Error {
    Error::Cabocha(format!("the chunker model {problem}"))
}

/// The name of a label in a CRF++ model.
fn name_of(label: Label) -> &'static str {
    match label {
        Label::Begin => "B",
        Label::Inside => "I",
    }
}

/// Reads a value of the header.
fn header<T: std::str::FromStr>(values: &HashMap<&str, &str>, key: &str) -> Result<T> {
    let value = values
        .get(key)
        .ok_or_else(|| error(format_args!("has no `{key}` in its header")))?;
    value
        .parse()
        .map_err(|_| error(format_args!("has `{key}: {value}` in its header")))
}

/// Reads a chunker model of CaboCha.
///
/// The weights are rounded to 32-bit floats, which is what CRF++ keeps of
/// them when CaboCha compiles the model, and multiplied by the cost factor
/// of the header.
///
/// # Arguments
///
/// * `text` - The content of the file, as `chunk.ipa.txt`.
///
/// # Returns
///
/// The model, or [`Error::Cabocha`] for a file that is not a CRF++ text
/// model, whose labels are not `B` and `I`, or whose template has a bigram
/// line with features.
pub(super) fn read(text: &str) -> Result<CrfModel> {
    let mut lines = text.lines();
    let mut block = |what: &str| -> Result<Vec<&str>> {
        let block: Vec<&str> = lines
            .by_ref()
            .map(|line| line.trim_end_matches('\r'))
            .take_while(|line| !line.is_empty())
            .collect();
        if block.is_empty() {
            return Err(error(format_args!(
                "ends before {what}: it is not a CRF++ model in text"
            )));
        }
        Ok(block)
    };

    let mut values = HashMap::new();
    for line in block("its header")? {
        let (key, value) = line
            .split_once(": ")
            .ok_or_else(|| error(format_args!("has the line {line:?} in its header")))?;
        values.insert(key, value);
    }
    let cost_factor: f64 = header(&values, "cost-factor")?;
    let count: usize = header(&values, "maxid")?;
    let columns: usize = header(&values, "xsize")?;
    if !cost_factor.is_finite() {
        return Err(error(format_args!("has the cost factor {cost_factor}")));
    }

    let labels = block("its labels")?;
    let position = |label: Label| labels.iter().position(|name| *name == name_of(label));
    let (Some(begin), Some(inside), 2) = (
        position(Label::Begin),
        position(Label::Inside),
        labels.len(),
    ) else {
        return Err(error(format_args!(
            "has the labels {}, and a chunker has `B` and `I`",
            labels.join(", ")
        )));
    };
    let size = labels.len();

    let template = block("its template")?;
    if let Some(line) = template
        .iter()
        .find(|line| line.starts_with('B') && **line != "B")
    {
        return Err(error(format_args!(
            "has the bigram line {line:?} in its template, and only `B` is supported"
        )));
    }
    let template = template.join("\n") + "\n";

    let names = block("its features")?;
    let weights: Vec<&str> = lines
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .collect();
    if weights.len() != count {
        return Err(error(format_args!(
            "has {} weights, and its header says {count}",
            weights.len()
        )));
    }
    let weight = |index: usize| -> Result<f64> {
        let text = weights
            .get(index)
            .ok_or_else(|| error(format_args!("has a feature at {index}, past its weights")))?;
        let value: f64 = text
            .parse()
            .map_err(|_| error(format_args!("has the weight {text:?}")))?;
        // What CRF++ keeps of a weight in a compiled model.
        let value = f64::from(value as f32) * cost_factor;
        if !value.is_finite() {
            return Err(error(format_args!("has the weight {text:?}")));
        }
        Ok(value)
    };

    let mut transitions = [[0.0; 3]; 3];
    let mut features = HashMap::with_capacity(names.len());
    for line in names {
        let (index, name) = line
            .split_once(' ')
            .ok_or_else(|| error(format_args!("has the feature line {line:?}")))?;
        let index: usize = index
            .parse()
            .map_err(|_| error(format_args!("has the feature line {line:?}")))?;
        if name == "B" {
            for (before, from) in [(Label::Begin, begin), (Label::Inside, inside)] {
                for (after, to) in [(Label::Begin, begin), (Label::Inside, inside)] {
                    transitions[before.index() + 1][after.index() + 1] =
                        weight(index + from * size + to)?;
                }
            }
        } else if name.starts_with('B') {
            return Err(error(format_args!(
                "has the bigram feature {name:?}, and only `B` is supported"
            )));
        } else if features
            .insert(
                name.to_string(),
                [weight(index + begin)?, weight(index + inside)?],
            )
            .is_some()
        {
            return Err(error(format_args!("has the feature {name:?} twice")));
        }
    }
    Ok(CrfModel {
        columns,
        template,
        transitions,
        features,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A small model: two columns, two unigram lines and `B`.
    pub(in crate::cabocha) const MODEL: &str = "\
version: 100
cost-factor: 1
maxid: 10
xsize: 2

B
I

U00:%x[0,0]
U01:%x[-1,1]/%x[0,1]
B

0 B
4 U00:本
6 U01:名詞/助詞
8 U00:を

0.5
-0.25
1.5
-2
3
-3
0.125
4
-1
1
";

    #[test]
    fn reads_the_blocks_of_a_model() {
        let model = read(MODEL).unwrap();
        assert_eq!(model.columns, 2);
        assert_eq!(model.template, "U00:%x[0,0]\nU01:%x[-1,1]/%x[0,1]\nB\n");
        assert_eq!(model.features.len(), 3);
        assert_eq!(model.features["U00:本"], [3.0, -3.0]);
        assert_eq!(model.features["U01:名詞/助詞"], [0.125, 4.0]);
        assert_eq!(model.features["U00:を"], [-1.0, 1.0]);
        // B to B, B to I, I to B, I to I; nothing at the boundaries.
        assert_eq!(
            model.transitions,
            [[0.0, 0.0, 0.0], [0.0, 0.5, -0.25], [0.0, 1.5, -2.0]]
        );
    }

    #[test]
    fn the_weights_follow_the_order_of_the_labels() {
        // The same model with `I` first: the first weight of a feature is
        // the one of `I`, and the first of the bigram is I to I.
        let swapped = MODEL.replace("B\nI\n\n", "I\nB\n\n");
        let model = read(&swapped).unwrap();
        assert_eq!(model.features["U00:本"], [-3.0, 3.0]);
        assert_eq!(
            model.transitions,
            [[0.0, 0.0, 0.0], [0.0, -2.0, 1.5], [0.0, -0.25, 0.5]]
        );
    }

    #[test]
    fn the_weights_are_those_of_a_compiled_model() {
        // 0.1 as a 32-bit float, and the cost factor.
        let model = read(&MODEL.replace("\n3\n", "\n0.1\n")).unwrap();
        assert_eq!(model.features["U00:本"][0], f64::from(0.1f32));
        let model = read(&MODEL.replace("cost-factor: 1", "cost-factor: 2")).unwrap();
        assert_eq!(model.features["U00:本"], [6.0, -6.0]);
        assert_eq!(model.transitions[1][1], 1.0);
    }

    #[test]
    fn a_model_without_the_bigram_feature_has_no_transitions() {
        let model = read(&MODEL.replace("0 B\n", "")).unwrap();
        assert_eq!(model.transitions, [[0.0; 3]; 3]);
        assert_eq!(model.features.len(), 3);
    }

    #[test]
    fn reads_a_file_with_windows_line_ends() {
        let model = read(&MODEL.replace('\n', "\r\n")).unwrap();
        assert_eq!(model, read(MODEL).unwrap());
    }

    #[test]
    fn refuses_a_file_that_is_not_a_chunker_model() {
        let message = |text: &str| read(text).unwrap_err().to_string();
        for (text, expected) in [
            (String::new(), "ends before its header"),
            ("C: 0.0015\n".to_string(), "has no `cost-factor`"),
            (
                "not a header\n".to_string(),
                "has the line \"not a header\"",
            ),
            (MODEL.replace("maxid: 10", "maxid: x"), "has `maxid: x`"),
            (MODEL.replace("xsize: 2\n", ""), "has no `xsize`"),
            (
                MODEL.replace("cost-factor: 1", "cost-factor: inf"),
                "has the cost factor inf",
            ),
            (
                MODEL.replace("B\nI\n\n", "B\nI\nO\n\n"),
                "has the labels B, I, O",
            ),
            (MODEL.replace("B\nI\n\n", "B\nO\n\n"), "has the labels B, O"),
            (
                MODEL.replace("\nB\n\n0 B", "\nB01:%x[0,0]\n\n0 B"),
                "has the bigram line \"B01:%x[0,0]\"",
            ),
            (
                MODEL.replace("0 B\n", "0 B01:本\n"),
                "has the bigram feature \"B01:本\"",
            ),
            (MODEL.replace("maxid: 10", "maxid: 9"), "has 10 weights"),
            (MODEL.replace("8 U00:を", "9 U00:を"), "past its weights"),
            (
                MODEL.replace("8 U00:を", "x U00:を"),
                "has the feature line",
            ),
            (MODEL.replace("8 U00:を", "8"), "has the feature line"),
            (
                MODEL.replace("8 U00:を", "8 U00:本"),
                "has the feature \"U00:本\" twice",
            ),
            (MODEL.replace("\n3\n", "\nx\n"), "has the weight \"x\""),
            (
                MODEL.replace("\n3\n", "\n1e99\n"),
                "has the weight \"1e99\"",
            ),
            (
                "version: 100\ncost-factor: 1\nmaxid: 0\nxsize: 2\n\nB\nI\n".to_string(),
                "ends before its template",
            ),
        ] {
            let message = message(&text);
            assert!(
                message.starts_with("the chunker model ") && message.contains(expected),
                "{expected}: {message}"
            );
        }
    }
}
