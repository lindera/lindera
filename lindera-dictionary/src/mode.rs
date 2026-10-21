use std::borrow::Cow;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{LinderaError, LinderaErrorKind};
use crate::viterbi::Edge;

/// Bound, in absolute value, on the sum of the length penalties along any
/// path of one sentence: the lattice segments a sentence of `n` characters
/// with costs per character of at most `SENTENCE_PENALTY_BUDGET / n` (see
/// [`Penalty::for_sentence`]), and the characters beyond the thresholds of
/// the words of a path add up to at most `n`, as a word's excess is at most
/// its length and the words of a path do not overlap (#1223).
pub(crate) const SENTENCE_PENALTY_BUDGET: i32 = 1 << 29;

/// The bound leaves the default costs (3000 and 1700) as they are for every
/// sentence a lattice accepts, up to `u16::MAX - 1` characters.
const _: () = assert!(SENTENCE_PENALTY_BUDGET / (u16::MAX as i32 - 1) >= 3000);

/// The length penalty of [`Mode::Decompose`]: a cost that a long word pays on
/// every transition out of it, so that the lattice prefers to split it into
/// shorter words. A kanji-only word pays per character beyond
/// `kanji_penalty_length_threshold`, any other word per character beyond
/// `other_penalty_length_threshold`; the product saturates to the range of
/// `i32` (see [`Penalty::penalty`]).
///
/// The lattice bounds the costs per character of each sentence: for a
/// sentence of `n` characters, a cost beyond `±(2^29 / n)` counts as that
/// bound, which is at least 8192 for the longest sentence a lattice accepts.
/// The penalties of a path then add up to at most `2^29` in absolute value,
/// so that however large the costs, the penalties alone cannot take the cost
/// of a path to the bound of the lattice's 32-bit costs, past which costs
/// are no longer exact and the paths are no longer ranked by them (#1223).
/// A cost beyond the bound splits as much as the bound does.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Penalty {
    /// Length in characters up to which no word pays a penalty, and beyond
    /// which a kanji-only word pays `kanji_penalty_length_penalty` per
    /// character. A word up to this length pays nothing whatever its
    /// characters, so keep it at most `other_penalty_length_threshold`.
    pub kanji_penalty_length_threshold: usize,
    /// Cost per character beyond `kanji_penalty_length_threshold` that a
    /// kanji-only word pays. Any `i32`, bounded per sentence (see
    /// [`Penalty`]); a negative cost favors long words.
    pub kanji_penalty_length_penalty: i32,
    /// Length in characters beyond which a word that is not kanji-only pays
    /// `other_penalty_length_penalty` per character.
    pub other_penalty_length_threshold: usize,
    /// Cost per character beyond `other_penalty_length_threshold` that a
    /// word that is not kanji-only pays. Any `i32`, bounded per sentence
    /// (see [`Penalty`]); a negative cost favors long words.
    pub other_penalty_length_penalty: i32,
}

impl Default for Penalty {
    /// Returns the default penalty: 3000 per character beyond 2 for a
    /// kanji-only word, 1700 per character beyond 7 for any other word.
    ///
    /// # Returns
    ///
    /// The default [`Penalty`].
    fn default() -> Self {
        Penalty {
            kanji_penalty_length_threshold: 2,
            kanji_penalty_length_penalty: 3000,
            other_penalty_length_threshold: 7,
            other_penalty_length_penalty: 1700,
        }
    }
}

impl Penalty {
    /// Returns the Decompose-mode length penalty for an edge.
    ///
    /// # Arguments
    ///
    /// * `edge` - The edge to penalize (read for its kanji-only flag).
    /// * `num_chars` - The edge's exact span in characters: the whitespace
    ///   its own surface ends with included, whitespace skipped after it
    ///   excluded. The packed edge does not store its end position, so the
    ///   lattice derives this from the slot the edge lives in and the
    ///   whitespace carry-over (#943 — this also replaced the former
    ///   byte-length/3 approximation, which was wrong for 1-, 2-, and 4-byte
    ///   UTF-8 characters; #1108).
    ///
    /// # Returns
    ///
    /// The penalty cost to add to the transition: the cost per character
    /// times the characters beyond the threshold, saturated to the range of
    /// `i32`.
    #[inline]
    pub fn penalty(&self, edge: &Edge, num_chars: usize) -> i32 {
        self.length_penalty(edge.kanji_only(), num_chars)
    }

    /// The body of [`Penalty::penalty`], on the edge's kanji-only flag
    /// rather than the edge, so that the tests need no `Edge`.
    ///
    /// # Arguments
    ///
    /// * `kanji_only` - Whether the edge's surface is kanji only.
    /// * `num_chars` - The edge's span in characters (see
    ///   [`Penalty::penalty`]).
    ///
    /// # Returns
    ///
    /// The penalty cost, saturated to the range of `i32`.
    #[inline]
    fn length_penalty(&self, kanji_only: bool, num_chars: usize) -> i32 {
        if num_chars <= self.kanji_penalty_length_threshold {
            return 0;
        }
        if kanji_only {
            saturating_product(
                num_chars - self.kanji_penalty_length_threshold,
                self.kanji_penalty_length_penalty,
            )
        } else if num_chars > self.other_penalty_length_threshold {
            saturating_product(
                num_chars - self.other_penalty_length_threshold,
                self.other_penalty_length_penalty,
            )
        } else {
            0
        }
    }

    /// Returns the penalty the lattice segments a sentence of `n_chars`
    /// characters with, when it differs from this one: both costs per
    /// character clamped to `±(SENTENCE_PENALTY_BUDGET / n_chars)`, so that
    /// the penalties of any path of the sentence add up to at most
    /// [`SENTENCE_PENALTY_BUDGET`] in absolute value (see [`Penalty`]). The
    /// thresholds stay as they are.
    ///
    /// # Arguments
    ///
    /// * `n_chars` - The sentence's length in characters.
    ///
    /// # Returns
    ///
    /// The bounded penalty, or `None` when both costs are within the bound.
    pub(crate) fn for_sentence(&self, n_chars: usize) -> Option<Penalty> {
        let bound =
            i32::try_from(n_chars.max(1)).map_or(0, |n_chars| SENTENCE_PENALTY_BUDGET / n_chars);
        let kanji = self.kanji_penalty_length_penalty.clamp(-bound, bound);
        let other = self.other_penalty_length_penalty.clamp(-bound, bound);
        (kanji != self.kanji_penalty_length_penalty || other != self.other_penalty_length_penalty)
            .then(|| Penalty {
                kanji_penalty_length_penalty: kanji,
                other_penalty_length_penalty: other,
                ..self.clone()
            })
    }
}

/// Returns `excess * penalty` saturated to the range of `i32`. A plain `i32`
/// product overflowed for a large penalty: a panic in debug builds, and in
/// release builds a wrapped cost that could even favor the long word it was
/// meant to penalize (#1219).
///
/// # Arguments
///
/// * `excess` - The characters beyond the threshold.
/// * `penalty` - The cost per character.
///
/// # Returns
///
/// The exact product, clamped to `i32::MIN..=i32::MAX`.
#[inline]
fn saturating_product(excess: usize, penalty: i32) -> i32 {
    // Saturating the excess keeps the sign of the product, so the clamp
    // still gives the exact product saturated to `i32`.
    let product = i64::try_from(excess)
        .unwrap_or(i64::MAX)
        .saturating_mul(i64::from(penalty));
    product.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Mode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "decompose")]
    Decompose(Penalty),
}

impl Mode {
    #[inline]
    pub fn is_search(&self) -> bool {
        match self {
            Mode::Normal => false,
            Mode::Decompose(_penalty) => true,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Mode::Normal => "normal",
            Mode::Decompose(_penalty) => "decompose",
        }
    }

    /// Returns the mode the lattice segments a sentence of `n_chars`
    /// characters with: in Decompose mode, the penalty bounded for the
    /// sentence (see [`Penalty::for_sentence`]); otherwise this mode.
    ///
    /// # Arguments
    ///
    /// * `n_chars` - The sentence's length in characters.
    ///
    /// # Returns
    ///
    /// The mode, borrowed when it does not change.
    pub(crate) fn for_sentence(&self, n_chars: usize) -> Cow<'_, Mode> {
        if let Mode::Decompose(penalty) = self
            && let Some(bounded) = penalty.for_sentence(n_chars)
        {
            return Cow::Owned(Mode::Decompose(bounded));
        }
        Cow::Borrowed(self)
    }
}

impl FromStr for Mode {
    type Err = LinderaError;
    fn from_str(mode: &str) -> Result<Mode, Self::Err> {
        match mode {
            "normal" => Ok(Mode::Normal),
            "decompose" => Ok(Mode::Decompose(Penalty::default())),
            _ => Err(LinderaErrorKind::Mode.with_error(anyhow::anyhow!("Invalid mode: {mode}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kanji(cost: i32) -> Penalty {
        Penalty {
            kanji_penalty_length_penalty: cost,
            ..Penalty::default()
        }
    }

    fn other(cost: i32) -> Penalty {
        Penalty {
            other_penalty_length_penalty: cost,
            ..Penalty::default()
        }
    }

    /// Below the overflow, the penalty is the cost per character times the
    /// characters beyond the threshold, and nothing up to it.
    #[test]
    fn test_penalty_is_the_product() {
        let penalty = Penalty::default();
        assert_eq!(penalty.length_penalty(true, 5), 3 * 3000);
        assert_eq!(penalty.length_penalty(false, 9), 2 * 1700);
        assert_eq!(penalty.length_penalty(false, 7), 0);
        assert_eq!(penalty.length_penalty(true, 2), 0);
        assert_eq!(penalty.length_penalty(false, 2), 0);
    }

    /// #1219: the kanji-only product saturates instead of overflowing.
    #[test]
    fn test_kanji_penalty_saturates() {
        // 6 characters are 4 beyond the threshold of 2, 4 are 2 beyond it.
        assert_eq!(kanji(1_000_000_000).length_penalty(true, 6), i32::MAX);
        assert_eq!(kanji(i32::MAX).length_penalty(true, 4), i32::MAX);
        assert_eq!(kanji(-1_000_000_000).length_penalty(true, 6), i32::MIN);
        assert_eq!(kanji(i32::MIN).length_penalty(true, 4), i32::MIN);
        // The largest products that fit are exact.
        assert_eq!(kanji(i32::MAX).length_penalty(true, 3), i32::MAX);
        assert_eq!(kanji(1_073_741_823).length_penalty(true, 4), 2_147_483_646);
    }

    /// #1219: the product for a word that is not kanji-only saturates
    /// instead of overflowing.
    #[test]
    fn test_other_penalty_saturates() {
        // 11 characters are 4 beyond the threshold of 7, 9 are 2 beyond it.
        assert_eq!(other(1_000_000_000).length_penalty(false, 11), i32::MAX);
        assert_eq!(other(i32::MAX).length_penalty(false, 9), i32::MAX);
        assert_eq!(other(-1_000_000_000).length_penalty(false, 11), i32::MIN);
        assert_eq!(other(i32::MIN).length_penalty(false, 9), i32::MIN);
        assert_eq!(other(i32::MAX).length_penalty(false, 8), i32::MAX);
        assert_eq!(other(1_073_741_823).length_penalty(false, 9), 2_147_483_646);
    }

    /// #1219: a length beyond the range of `i32` saturates as well, and the
    /// result keeps the sign of the cost.
    #[test]
    fn test_penalty_saturates_a_huge_length() {
        for (cost, expected) in [
            (1, i32::MAX),
            (1_000_000_000, i32::MAX),
            (-1, i32::MIN),
            (-1_000_000_000, i32::MIN),
            (0, 0),
        ] {
            let penalty = Penalty {
                kanji_penalty_length_penalty: cost,
                other_penalty_length_penalty: cost,
                ..Penalty::default()
            };
            for kanji_only in [true, false] {
                assert_eq!(
                    penalty.length_penalty(kanji_only, usize::MAX),
                    expected,
                    "cost {cost}, kanji only {kanji_only}"
                );
            }
        }
    }

    /// #1223: the bound leaves the default penalty as it is for every
    /// sentence a lattice accepts, and Normal mode has nothing to bound.
    #[test]
    fn test_default_penalty_is_within_the_bound() {
        let default = Mode::Decompose(Penalty::default());
        for n_chars in [0, 1, 2, 1000, 32771, u16::MAX as usize - 1] {
            assert_eq!(Penalty::default().for_sentence(n_chars), None, "{n_chars}");
            assert!(
                matches!(default.for_sentence(n_chars), Cow::Borrowed(_)),
                "{n_chars}"
            );
        }
        assert!(matches!(Mode::Normal.for_sentence(3), Cow::Borrowed(_)));
    }

    /// #1223: both costs are clamped to the bound of the sentence, on
    /// either side, each only when it is beyond the bound; the thresholds
    /// stay as they are. A sentence without characters counts as one, and
    /// one longer than `i32` allows leaves no cost at all.
    #[test]
    fn test_for_sentence_clamps_the_costs() {
        let bound = SENTENCE_PENALTY_BUDGET / 10;
        let huge = Penalty {
            kanji_penalty_length_threshold: 3,
            kanji_penalty_length_penalty: i32::MAX,
            other_penalty_length_threshold: 5,
            other_penalty_length_penalty: i32::MIN,
        };
        assert_eq!(
            huge.for_sentence(10),
            Some(Penalty {
                kanji_penalty_length_penalty: bound,
                other_penalty_length_penalty: -bound,
                ..huge.clone()
            })
        );
        assert_eq!(other(-1_000_000_000).for_sentence(10), Some(other(-bound)));
        assert_eq!(kanji(bound).for_sentence(10), None);
        assert_eq!(kanji(bound + 1).for_sentence(10), Some(kanji(bound)));
        assert_eq!(
            kanji(i32::MAX).for_sentence(0),
            Some(kanji(SENTENCE_PENALTY_BUDGET))
        );
        assert_eq!(
            huge.for_sentence(usize::MAX),
            Some(Penalty {
                kanji_penalty_length_penalty: 0,
                other_penalty_length_penalty: 0,
                ..huge.clone()
            })
        );
        assert_eq!(
            Mode::Decompose(huge.clone()).for_sentence(10).into_owned(),
            Mode::Decompose(huge.for_sentence(10).unwrap())
        );
    }

    /// #1223: with the bounded penalty, the penalties of a path of a
    /// sentence add up to at most the budget, on either side: here the
    /// paths that pay the most, every character beyond a threshold of 0,
    /// as one word or as one-character words.
    #[test]
    fn test_bounded_penalties_of_a_path_fit_the_budget() {
        let budget = i64::from(SENTENCE_PENALTY_BUDGET);
        for n_chars in 1..=200 {
            for cost in [i32::MAX, i32::MIN] {
                let penalty = Penalty {
                    kanji_penalty_length_threshold: 0,
                    kanji_penalty_length_penalty: cost,
                    other_penalty_length_threshold: 0,
                    other_penalty_length_penalty: cost,
                }
                .for_sentence(n_chars)
                .unwrap();
                for kanji_only in [true, false] {
                    let whole = i64::from(penalty.length_penalty(kanji_only, n_chars));
                    let split = i64::from(penalty.length_penalty(kanji_only, 1)) * n_chars as i64;
                    for sum in [whole, split] {
                        assert!(
                            sum.abs() <= budget && sum.abs() > budget - n_chars as i64,
                            "{n_chars} characters, cost {cost}: {sum}"
                        );
                    }
                }
            }
        }
    }
}
