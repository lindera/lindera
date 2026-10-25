//! Regularization types of training, by the names `lindera train
//! --regularization` and the bindings' training functions take.

use std::fmt;
use std::str::FromStr;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// The default L1 ratio of Elastic Net regularization, which
/// `lindera train --elastic-net-l1-ratio` and the bindings use: `0.5`, an
/// even mix of L1 and L2.
pub const DEFAULT_ELASTIC_NET_L1_RATIO: f64 = 0.5;

/// A regularization type of training, set with
/// [`Trainer::regularization_type`](crate::Trainer::regularization_type).
/// The default, [`Self::L2`], is the type a new trainer uses.
///
/// A trained model records it in `model.dat` and in the `model_info` of the
/// `metadata.json` that `lindera export` writes (#1272), where serde writes
/// it by the name [`FromStr`] takes first: `l1`, `l2` or `elasticnet`.
#[derive(
    Clone,
    Copy,
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
#[serde(rename_all = "lowercase")]
pub enum RegularizationType {
    /// L1 regularization (`l1`).
    L1,
    /// L2 regularization (`l2`), the default.
    #[default]
    L2,
    /// Elastic Net regularization (`elasticnet`): L1 with a coefficient of
    /// `lambda * l1_ratio` and L2 with `lambda * (1 - l1_ratio)`.
    ElasticNet,
}

impl RegularizationType {
    /// Splits the regularization lindera-crf trained with into the type and,
    /// for Elastic Net, the L1 ratio, as a model records them (#1272).
    ///
    /// # Arguments
    ///
    /// * `regularization` - The regularization.
    ///
    /// # Returns
    ///
    /// The type, and the ratio for [`Self::ElasticNet`] or `None`.
    pub(crate) fn split(regularization: lindera_crf::Regularization) -> (Self, Option<f64>) {
        match regularization {
            lindera_crf::Regularization::L1 => (Self::L1, None),
            lindera_crf::Regularization::L2 => (Self::L2, None),
            lindera_crf::Regularization::ElasticNet { l1_ratio } => {
                (Self::ElasticNet, Some(l1_ratio))
            }
        }
    }
}

impl FromStr for RegularizationType {
    type Err = ParseRegularizationTypeError;

    /// Parses `l1`, `l2` or `elasticnet` (also spelled `elastic_net` and
    /// `elastic-net`), in any ASCII case.
    ///
    /// # Arguments
    ///
    /// * `name` - The name.
    ///
    /// # Returns
    ///
    /// The type, or a [`ParseRegularizationTypeError`] for any other name.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.to_ascii_lowercase().as_str() {
            "l1" => Ok(Self::L1),
            "l2" => Ok(Self::L2),
            "elasticnet" | "elastic_net" | "elastic-net" => Ok(Self::ElasticNet),
            _ => Err(ParseRegularizationTypeError {
                name: name.to_string(),
            }),
        }
    }
}

/// The error of parsing a [`RegularizationType`] from a name it does not
/// know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseRegularizationTypeError {
    /// The name that was given.
    name: String,
}

impl fmt::Display for ParseRegularizationTypeError {
    /// Writes the message the CLI and every binding raise, such as
    /// `regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "regularization must be 'l1', 'l2' or 'elasticnet', got '{}'",
            self.name
        )
    }
}

impl std::error::Error for ParseRegularizationTypeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_names_of_the_cli_in_any_ascii_case() {
        for (name, expected) in [
            ("l1", RegularizationType::L1),
            ("L1", RegularizationType::L1),
            ("l2", RegularizationType::L2),
            ("L2", RegularizationType::L2),
            ("elasticnet", RegularizationType::ElasticNet),
            ("ElasticNet", RegularizationType::ElasticNet),
            ("elastic_net", RegularizationType::ElasticNet),
            ("elastic-net", RegularizationType::ElasticNet),
            ("ELASTIC-NET", RegularizationType::ElasticNet),
        ] {
            assert_eq!(name.parse::<RegularizationType>(), Ok(expected), "{name}");
        }
    }

    // The bindings use the default when the caller leaves the type out, so
    // it must be the type of `Trainer::new` and of `lindera train`'s
    // `--regularization l2`.
    // serde writes a type by the name `FromStr` takes first, which is what
    // `model_info.regularization_type` in `metadata.json` holds (#1272).
    #[test]
    fn serde_writes_the_name_that_from_str_reads() {
        for (kind, name) in [
            (RegularizationType::L1, "l1"),
            (RegularizationType::L2, "l2"),
            (RegularizationType::ElasticNet, "elasticnet"),
        ] {
            let json = serde_json::to_value(kind).unwrap();
            assert_eq!(json, serde_json::json!(name));
            assert_eq!(name.parse::<RegularizationType>(), Ok(kind));
            assert_eq!(
                serde_json::from_value::<RegularizationType>(json).unwrap(),
                kind
            );
        }
    }

    #[test]
    fn split_gives_the_ratio_only_for_elastic_net() {
        assert_eq!(
            RegularizationType::split(lindera_crf::Regularization::L1),
            (RegularizationType::L1, None)
        );
        assert_eq!(
            RegularizationType::split(lindera_crf::Regularization::L2),
            (RegularizationType::L2, None)
        );
        assert_eq!(
            RegularizationType::split(lindera_crf::Regularization::ElasticNet { l1_ratio: 0.3 }),
            (RegularizationType::ElasticNet, Some(0.3))
        );
    }

    #[test]
    fn the_default_is_l2() {
        assert_eq!(RegularizationType::default(), RegularizationType::L2);
    }

    #[test]
    fn rejects_other_names_with_the_accepted_ones() {
        for name in ["", "l3", " l1", "l1 ", "elastic net", "lasso", "ridge"] {
            let error = name.parse::<RegularizationType>().unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("regularization must be 'l1', 'l2' or 'elasticnet', got '{name}'")
            );
        }
    }
}
