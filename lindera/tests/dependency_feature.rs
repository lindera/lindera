//! The token filter `japanese_dependency` and the feature `dependency`
//! (#1438).
//!
//! The filter is built with the feature only. `make test-lindera` and the CI
//! job of this crate run without it, and check that the build says so; with
//! the feature, the filter is there under the paths of this crate.
#![cfg(feature = "analysis")]

use lindera::analysis::token_filter::TokenFilterLoader;
use lindera::error::LinderaErrorKind;
use serde_json::json;

#[test]
#[cfg(not(feature = "dependency"))]
fn without_the_feature_the_filter_is_refused_with_the_name_of_the_feature() {
    let args = json!({"model": "model.bin"});
    let error = TokenFilterLoader::load_from_value("japanese_dependency", &args)
        .err()
        .unwrap();
    assert_eq!(error.kind(), LinderaErrorKind::FeatureDisabled);
    let message = error.to_string();
    assert!(message.contains("japanese_dependency"), "{message}");
    assert!(message.contains("the feature `dependency`"), "{message}");
}

#[test]
#[cfg(feature = "dependency")]
fn with_the_feature_the_filter_is_loaded_by_its_kind() {
    use lindera::analysis::token_filter::japanese_dependency::JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME;

    let model = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../resources/dependency/test_model_ipadic.bin"
    );
    let filter = TokenFilterLoader::load_from_value(
        JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME,
        &json!({ "model": model }),
    )
    .unwrap();
    assert_eq!(filter.name(), "japanese_dependency");

    // Without a model the error is the filter's, not the one of a build
    // without it.
    let error = TokenFilterLoader::load_from_value("japanese_dependency", &json!({}))
        .err()
        .unwrap();
    assert_eq!(error.kind(), LinderaErrorKind::Deserialize);
}
