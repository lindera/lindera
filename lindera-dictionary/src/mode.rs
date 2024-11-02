use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{LinderaError, LinderaErrorKind};
use crate::viterbi::Edge;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Penalty {
    pub kanji_penalty_length_threshold: usize,
    pub kanji_penalty_length_penalty: i32,
    pub other_penalty_length_threshold: usize,
    pub other_penalty_length_penalty: i32,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Mode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "decompose")]
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

    pub fn as_str(&self) -> &str {
        match self {
            Mode::Normal => "normal",
            Mode::Decompose(_penalty) => "decompose",
        }
    }
}

impl FromStr for Mode {
    type Err = LinderaError;
    fn from_str(mode: &str) -> Result<Mode, Self::Err> {
        match mode {
            "normal" => Ok(Mode::Normal),
            "decompose" => Ok(Mode::Decompose(Penalty::default())),
            _ => Err(LinderaErrorKind::Mode.with_error(anyhow::anyhow!("Invalid mode: {}", mode))),
        }
    }
}
