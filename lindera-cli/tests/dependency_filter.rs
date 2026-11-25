//! The token filter `japanese_dependency` from the `lindera` command (#1438).
//!
//! The tests need the embedded IPADIC and the filter:
//!
//! ```sh
//! cargo test -p lindera-cli --features train,embed-ipadic,dependency
//! ```
//!
//! The model is the one of the tests of lindera-analysis, which check that
//! the file is that model: every bunsetsu depends on the next one.
#![cfg(all(feature = "embed-ipadic", feature = "dependency"))]

use assert_cmd::Command;
use serde_json::{Value, json};

/// The model file of the tests.
const MODEL_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../resources/dependency/test_model_ipadic.bin"
);

/// The standard output of `lindera tokenize` with IPADIC.
fn tokenize(arguments: &[&str], text: &str) -> String {
    let output = Command::cargo_bin("lindera")
        .expect("lindera binary should build")
        .args(["tokenize", "--dict", "embedded://ipadic"])
        .args(arguments)
        .write_stdin(text)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// The `--token-filter` argument of the filter.
fn filter() -> String {
    format!("japanese_dependency:{}", json!({ "model": MODEL_FILE }))
}

#[test]
fn the_json_output_has_the_dependency_of_each_token() {
    let filter = filter();
    let output = tokenize(
        &["--output", "json", "--token-filter", &filter],
        "彼は本を読んだ。\n",
    );
    let tokens: Value = serde_json::from_str(&output).unwrap();
    let written: Vec<(&str, &Value)> = tokens
        .as_array()
        .unwrap()
        .iter()
        .map(|token| (token["surface"].as_str().unwrap(), &token["dependency"]))
        .collect();
    let dependency = |bunsetsu: u32, head: Option<u32>, role: &str, score: f64| json!({"sentence": 0, "bunsetsu": bunsetsu, "head": head, "role": role, "score": score});
    assert_eq!(
        written,
        [
            ("彼", &dependency(0, Some(1), "head", 1.0)),
            ("は", &dependency(0, Some(1), "function", 1.0)),
            ("本", &dependency(1, Some(2), "head", 0.0)),
            ("を", &dependency(1, Some(2), "function", 0.0)),
            ("読ん", &dependency(2, None, "head", 0.0)),
            ("だ", &dependency(2, None, "function", 0.0)),
            ("。", &dependency(2, None, "other", 0.0)),
        ]
    );

    // Without the filter a token has no such key.
    let output = tokenize(&["--output", "json"], "彼は本を読んだ。\n");
    let tokens: Value = serde_json::from_str(&output).unwrap();
    assert!(
        tokens
            .as_array()
            .unwrap()
            .iter()
            .all(|token| token.get("dependency").is_none())
    );
}

#[test]
fn the_mecab_and_wakati_outputs_are_those_without_the_filter() {
    let filter = filter();
    let text = "彼は本を読んだ。「これは本です。」と太郎は言った！\n";
    for format in ["mecab", "wakati"] {
        let with = tokenize(&["--output", format, "--token-filter", &filter], text);
        let without = tokenize(&["--output", format], text);
        assert!(with.contains("読ん"), "{format}: {with}");
        assert_eq!(with, without, "{format}");
    }
}

#[test]
fn a_model_file_that_cannot_be_read_ends_the_command_with_its_path() {
    let output = Command::cargo_bin("lindera")
        .expect("lindera binary should build")
        .args(["tokenize", "--dict", "embedded://ipadic", "--token-filter"])
        .arg("japanese_dependency:{\"model\":\"no/such/model.bin\"}")
        .write_stdin("彼は本を読んだ。\n")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no/such/model.bin"), "{stderr}");
    assert!(output.stdout.is_empty());
}

/// The `--token-filter` argument of the filter with the embedded model.
fn embedded_filter() -> String {
    format!(
        "japanese_dependency:{}",
        json!({ "model": "embedded://ipadic" })
    )
}

// The builds of the tests embed the model file above: the test jobs name it
// in LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE.
#[cfg(feature = "embed-dependency-ipadic")]
#[test]
fn the_embedded_model_gives_what_its_file_gives() {
    let text = "彼は本を読んだ。今日は雨だ。\n";
    let from_the_file = tokenize(&["--output", "json", "--token-filter", &filter()], text);
    let from_the_build = tokenize(
        &["--output", "json", "--token-filter", &embedded_filter()],
        text,
    );
    assert!(from_the_file.contains("\"dependency\""), "{from_the_file}");
    assert_eq!(from_the_build, from_the_file);
}

#[cfg(not(feature = "embed-dependency-ipadic"))]
#[test]
fn a_build_without_the_embedded_model_names_the_feature() {
    let output = Command::cargo_bin("lindera")
        .expect("lindera binary should build")
        .args(["tokenize", "--dict", "embedded://ipadic", "--token-filter"])
        .arg(embedded_filter())
        .write_stdin("彼は本を読んだ。\n")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`embed-dependency-ipadic`"), "{stderr}");
    assert!(output.stdout.is_empty());
}

/// The model as `lindera download dependency-ipadic` leaves it, and the path
/// the command prints for it, given to the filter (#1440). The model is in
/// place and of this version, so nothing is downloaded.
#[test]
fn the_path_that_download_prints_is_what_the_filter_takes() {
    let dictionaries = tempfile::tempdir().unwrap();
    let directory = dictionaries.path().join("dependency-ipadic");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::copy(MODEL_FILE, directory.join("dependency.model")).unwrap();
    std::fs::write(directory.join("NOTICE.txt"), b"notice").unwrap();
    std::fs::write(
        directory.join("model.json"),
        json!({
            "name": "dependency-ipadic",
            "dictionary": "ipadic",
            "lindera_version": env!("CARGO_PKG_VERSION"),
        })
        .to_string(),
    )
    .unwrap();

    let output = Command::cargo_bin("lindera")
        .expect("lindera binary should build")
        .args(["download", "dependency-ipadic"])
        .env("LINDERA_DICTIONARIES_DIR", dictionaries.path())
        .env_remove("LINDERA_DATA_DIR")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed = String::from_utf8(output.stdout).unwrap();
    let model = printed.trim_end();

    let text = "彼は本を読んだ。\n";
    let with_the_printed_path = tokenize(
        &[
            "--output",
            "json",
            "--token-filter",
            &format!("japanese_dependency:{}", json!({ "model": model })),
        ],
        text,
    );
    let with_the_file = tokenize(&["--output", "json", "--token-filter", &filter()], text);
    assert!(with_the_file.contains("\"dependency\""), "{with_the_file}");
    assert_eq!(with_the_printed_path, with_the_file);
}
