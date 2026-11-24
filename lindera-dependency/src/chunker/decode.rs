//! Decoding: the best labels of a sentence, by Viterbi over `B` and `I`.

use super::model::{BOUNDARY, ChunkerData, Label};

/// The best labels of a sentence.
///
/// The score of a sequence of labels is the sum of the weights of the
/// feature strings of each position for its label, of the transitions
/// between neighbors, and of the transitions from the start of the sentence
/// and to its end. The first morpheme is always [`Label::Begin`].
///
/// The sums are taken from the end of the sentence, in the order
/// `search_best_path` of `lindera-crf` takes them, and a tie goes to
/// [`Label::Begin`], so that the labels are the ones it gives for the same
/// weights.
///
/// # Arguments
///
/// * `model` - The model.
/// * `features` - The feature strings of each position, in the order of the
///   template.
///
/// # Returns
///
/// One label per position; none for an empty sentence.
pub(crate) fn decode<D: ChunkerData>(model: &D, features: &[Vec<String>]) -> Vec<Label> {
    let count = features.len();
    if count == 0 {
        return Vec::new();
    }

    // The score of each label at each position, from its features alone.
    let unigram: Vec<[f64; 2]> = features
        .iter()
        .map(|features| {
            let mut scores = [0.0; 2];
            for feature in features {
                if let Some(weights) = model.feature(feature) {
                    scores[0] += weights[0];
                    scores[1] += weights[1];
                }
            }
            scores
        })
        .collect();

    // best[position][label]: the best score from this position to the end
    // with this label here; next[position][label]: the label after it on
    // that path.
    let mut best = vec![[0.0; 2]; count];
    let mut next = vec![[Label::Begin; 2]; count];
    for position in (0..count).rev() {
        for label in Label::ALL {
            let from = label.index() + 1;
            let rest = if position + 1 == count {
                0.0 + model.transition(from, BOUNDARY)
            } else {
                let mut rest = f64::NEG_INFINITY;
                for candidate in Label::ALL {
                    let score = best[position + 1][candidate.index()]
                        + model.transition(from, candidate.index() + 1);
                    if score > rest {
                        rest = score;
                        next[position][label.index()] = candidate;
                    }
                }
                rest
            };
            best[position][label.index()] = unigram[position][label.index()] + rest;
        }
    }

    // The first morpheme starts a bunsetsu, whatever the weights say.
    let mut labels = Vec::with_capacity(count);
    let mut label = Label::Begin;
    for links in &next {
        labels.push(label);
        label = links[label.index()];
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::model::tests::model;

    /// Feature strings: one per position, named by the position.
    fn positions(count: usize) -> Vec<Vec<String>> {
        (0..count).map(|index| vec![format!("p{index}")]).collect()
    }

    /// The labels as a string of `B` and `I`.
    fn letters(labels: &[Label]) -> String {
        labels
            .iter()
            .map(|label| match label {
                Label::Begin => 'B',
                Label::Inside => 'I',
            })
            .collect()
    }

    /// No transition has a weight.
    const FLAT: [[f64; 3]; 3] = [[0.0; 3]; 3];

    #[test]
    fn an_empty_sentence_has_no_label() {
        let model = model(&[], FLAT);
        assert!(decode(&model, &[]).is_empty());
    }

    #[test]
    fn each_position_takes_the_label_with_the_larger_weight() {
        let model = model(
            &[
                ("p0", [1.0, 0.0]),
                ("p1", [0.0, 1.0]),
                ("p2", [2.0, 0.5]),
                ("p3", [-1.0, -0.5]),
            ],
            FLAT,
        );
        assert_eq!(letters(&decode(&model, &positions(4))), "BIBI");
    }

    #[test]
    fn the_first_morpheme_is_b_whatever_the_weights() {
        let model = model(
            &[("p0", [-100.0, 100.0]), ("p1", [-100.0, 100.0])],
            [[0.0, -50.0, 50.0], [0.0; 3], [0.0; 3]],
        );
        assert_eq!(letters(&decode(&model, &positions(2))), "BI");
        assert_eq!(letters(&decode(&model, &positions(1))), "B");
    }

    #[test]
    fn several_features_of_a_position_add_up() {
        let model = model(
            &[("a", [1.0, 0.0]), ("b", [1.0, 0.0]), ("c", [0.0, 1.5])],
            FLAT,
        );
        let features = vec![
            vec!["x".to_string()],
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "unknown".to_string(),
            ],
            vec!["a".to_string(), "c".to_string()],
        ];
        // Position 1: B has 2.0 and I 1.5. Position 2: B has 1.0 and I 1.5.
        assert_eq!(letters(&decode(&model, &features)), "BBI");
    }

    #[test]
    fn a_transition_changes_the_labels() {
        // Position 1 prefers I by 1.5 and position 2 by 1.0.
        let features = [("p0", [0.0, 0.0]), ("p1", [0.0, 1.5]), ("p2", [0.0, 1.0])];
        assert_eq!(
            letters(&decode(&model(&features, FLAT), &positions(3))),
            "BII"
        );
        // I after I costs 3.0: the position that prefers I less becomes B.
        let transitions = [[0.0; 3], [0.0; 3], [0.0, 0.0, -3.0]];
        assert_eq!(
            letters(&decode(&model(&features, transitions), &positions(3))),
            "BIB"
        );
        // B after I costs 3.0 too: then B, B, I, by B after B being free.
        let transitions = [[0.0; 3], [0.0; 3], [0.0, -3.0, -3.0]];
        assert_eq!(
            letters(&decode(&model(&features, transitions), &positions(3))),
            "BBI"
        );
    }

    #[test]
    fn the_transition_to_the_end_changes_the_last_label() {
        // The last position prefers I by 1.0, and ending on I costs 2.0.
        let features = [("p0", [0.0, 0.0]), ("p1", [0.0, 1.0])];
        assert_eq!(
            letters(&decode(&model(&features, FLAT), &positions(2))),
            "BI"
        );
        let transitions = [[0.0; 3], [0.0; 3], [-2.0, 0.0, 0.0]];
        assert_eq!(
            letters(&decode(&model(&features, transitions), &positions(2))),
            "BB"
        );
    }

    #[test]
    fn a_tie_goes_to_b() {
        let model = model(&[], FLAT);
        assert_eq!(letters(&decode(&model, &positions(3))), "BBB");
    }

    #[test]
    fn the_best_path_is_not_the_best_label_at_each_position() {
        // Position 1 alone prefers B by 1.0. But I at position 2 is worth
        // 5.0 only after I: the best path gives up position 1.
        let features = [("p0", [0.0, 0.0]), ("p1", [1.0, 0.0]), ("p2", [0.0, 0.0])];
        let transitions = [[0.0; 3], [0.0; 3], [0.0, 0.0, 5.0]];
        assert_eq!(
            letters(&decode(&model(&features, transitions), &positions(3))),
            "BII"
        );
    }
}
