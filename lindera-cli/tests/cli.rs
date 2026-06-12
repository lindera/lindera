//! End-to-end smoke tests for the `lindera` CLI binary.
//!
//! Tests that do not require a dictionary always run. Tests that tokenize
//! text are gated behind the `embed-ipadic` feature:
//!
//! ```sh
//! cargo test -p lindera-cli --features train,embed-ipadic
//! ```

use assert_cmd::Command;

fn lindera() -> Command {
    Command::cargo_bin("lindera").expect("lindera binary should build")
}

#[test]
fn help_shows_subcommands() {
    let output = lindera().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("list"));
    assert!(stdout.contains("tokenize"));
    assert!(stdout.contains("build"));
}

#[test]
fn version_matches_crate() {
    let output = lindera().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn list_succeeds() {
    let output = lindera().arg("list").output().unwrap();
    assert!(output.status.success());

    #[cfg(feature = "embed-ipadic")]
    {
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("ipadic"),
            "embedded ipadic should be listed, got: {stdout}"
        );
    }
}

#[test]
fn tokenize_with_invalid_dictionary_fails() {
    let output = lindera()
        .args(["tokenize", "--dict", "/nonexistent/dictionary/path"])
        .write_stdin("テスト\n")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[cfg(feature = "embed-ipadic")]
mod with_ipadic {
    use super::*;

    #[test]
    fn tokenize_mecab_output() {
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("関西国際空港\t"), "got: {stdout}");
        assert!(stdout.contains("EOS"), "got: {stdout}");
    }

    #[test]
    fn tokenize_wakati_output() {
        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "wakati",
            ])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.trim(), "関西国際空港 限定 トートバッグ");
    }

    #[test]
    fn tokenize_json_output() {
        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "json",
            ])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("output should be valid JSON");
        let tokens = parsed.as_array().expect("output should be a JSON array");
        assert_eq!(tokens[0]["surface"], "関西国際空港");
    }

    #[test]
    fn tokenize_decompose_mode() {
        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "wakati",
                "--mode",
                "decompose",
            ])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.trim(), "関西 国際 空港 限定 トートバッグ");
    }
}
