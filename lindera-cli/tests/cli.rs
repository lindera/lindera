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

    #[test]
    fn tokenize_mecab_output_exact_format() {
        // Locks the exact MeCab output shape produced through the buffered
        // writer: one `surface\tdetails` line per token, a terminating `EOS`
        // line, and a trailing newline.
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.ends_with("EOS\n"), "got: {stdout}");
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 4, "3 tokens + EOS, got: {stdout}");
        assert!(lines[0].starts_with("関西国際空港\t"), "got: {stdout}");
        for line in &lines[..3] {
            let (_, details) = line
                .split_once('\t')
                .expect("token line must contain a tab");
            assert!(
                details.contains(','),
                "details must be comma-joined, got: {line}"
            );
        }
        assert_eq!(lines[3], "EOS");
    }

    #[test]
    fn tokenize_multiline_input_keeps_per_line_order() {
        // Output for multiple input lines must arrive complete and in order
        // through the buffered writer, with one EOS per input line.
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .write_stdin("すもももももももものうち\n関西国際空港\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.matches("EOS\n").count(), 2, "got: {stdout}");
        let first_segment = stdout.split("EOS\n").next().unwrap();
        assert!(first_segment.contains("すもも\t"), "got: {stdout}");
        assert!(!first_segment.contains("関西国際空港"), "got: {stdout}");
    }

    #[test]
    fn tokenize_nbest_headers() {
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic", "--nbest", "2"])
            .write_stdin("関西国際空港\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with("NBEST 1 (cost="), "got: {stdout}");
        assert!(stdout.contains("\nNBEST 2 (cost="), "got: {stdout}");
        assert!(stdout.ends_with("EOS\n"), "got: {stdout}");
    }

    #[test]
    fn tokenize_with_mmap_flag_parses_and_output_is_unchanged() {
        // --mmap has no effect on an embedded:// dictionary, but the flag
        // must still parse and produce identical output.
        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "wakati",
                "--mmap",
            ])
            .write_stdin("関西国際空港限定トートバッグ\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.trim(), "関西国際空港 限定 トートバッグ");
    }
}
