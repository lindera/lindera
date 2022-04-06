use std::str::FromStr;

use lindera_core::viterbi::{Edge, Mode as LinderaCoreMode, Penalty as LinderaCorePenalty};
use serde::{Deserialize, Serialize};

use crate::error::{LinderaError, LinderaErrorKind};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Penalty {
    kanji_penalty_length_threshold: usize,
    kanji_penalty_length_penalty: i32,
    other_penalty_length_threshold: usize,
    other_penalty_length_penalty: i32,
}

impl From<Penalty> for LinderaCorePenalty {
    fn from(penalty: Penalty) -> Self {
        LinderaCorePenalty {
            kanji_penalty_length_threshold: penalty.kanji_penalty_length_threshold,
            kanji_penalty_length_penalty: penalty.kanji_penalty_length_penalty,
            other_penalty_length_threshold: penalty.other_penalty_length_threshold,
            other_penalty_length_penalty: penalty.other_penalty_length_penalty,
        }
    }
}

impl Default for Penalty {
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
    pub fn penalty(&self, edge: &Edge) -> i32 {
        let num_chars = edge.num_chars();
        if num_chars <= self.kanji_penalty_length_threshold {
            return 0;
        }
        if edge.kanji_only {
            ((num_chars - self.kanji_penalty_length_threshold) as i32)
                * self.kanji_penalty_length_penalty
        } else if num_chars > self.other_penalty_length_threshold {
            ((num_chars - self.other_penalty_length_threshold) as i32)
                * self.other_penalty_length_penalty
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum Mode {
    Normal,
    Decompose(Penalty),
}

impl Mode {
    pub fn is_search(&self) -> bool {
        match self {
            Mode::Normal => false,
            Mode::Decompose(_penalty) => true,
        }
    }
    pub fn penalty_cost(&self, edge: &Edge) -> i32 {
        match self {
            Mode::Normal => 0i32,
            Mode::Decompose(penalty) => penalty.penalty(edge),
        }
    }
}

impl FromStr for Mode {
    type Err = LinderaError;
    fn from_str(mode: &str) -> Result<Mode, Self::Err> {
        match mode {
            "normal" => Ok(Mode::Normal),
            "decompose" => Ok(Mode::Decompose(Penalty::default())),
            _ => {
                Err(LinderaErrorKind::ModeError
                    .with_error(anyhow::anyhow!("Invalid mode: {}", mode)))
            }
        }
    }
}

impl From<Mode> for LinderaCoreMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Normal => LinderaCoreMode::Normal,
            Mode::Decompose(penalty) => LinderaCoreMode::Decompose(penalty.into()),
        }
    }
}
