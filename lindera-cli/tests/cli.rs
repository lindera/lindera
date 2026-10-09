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
    assert!(stdout.contains("download"));
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

#[test]
fn download_help_lists_dictionary_names() {
    let output = lindera().args(["download", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ipadic-neologd"), "got: {stdout}");
    assert!(stdout.contains("cc-cedict"), "got: {stdout}");
    assert!(stdout.contains("--force"), "got: {stdout}");
}

#[test]
fn download_with_invalid_name_fails() {
    let output = lindera()
        .args(["download", "no-such-dictionary"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ipadic"), "got: {stderr}");
}

#[test]
fn tokenize_with_undownloaded_name_suggests_download() {
    let data_dir = tempfile::tempdir().unwrap();
    let output = lindera()
        .env("LINDERA_DATA_DIR", data_dir.path())
        .args(["tokenize", "--dict", "ipadic"])
        .write_stdin("テスト\n")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("lindera download ipadic"), "got: {stderr}");
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

    /// Runs `lindera tokenize -o json` with IPADIC and `args` on `input`,
    /// and returns `(surface, byte_start, byte_end)` of the tokens of every
    /// output array: one per input line, or one per result with N-best
    /// (whose `NBEST k (cost=…)` header lines are skipped).
    fn spans(input: &str, args: &[&str]) -> Vec<Vec<(String, u64, u64)>> {
        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "json",
            ])
            .args(args)
            .write_stdin(input.to_string())
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let json = stdout
            .lines()
            .filter(|line| !line.starts_with("NBEST "))
            .collect::<Vec<_>>()
            .join("\n");
        serde_json::Deserializer::from_str(&json)
            .into_iter::<Vec<serde_json::Value>>()
            .map(|tokens| {
                tokens
                    .unwrap()
                    .iter()
                    .map(|token| {
                        (
                            token["surface"].as_str().unwrap().to_string(),
                            token["byte_start"].as_u64().unwrap(),
                            token["byte_end"].as_u64().unwrap(),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    fn span(surface: &str, start: u64, end: u64) -> (String, u64, u64) {
        (surface.to_string(), start, end)
    }

    #[test]
    fn tokenize_offsets_index_the_input_line() {
        // Only the line terminator is removed (#1115): the spaces and tabs
        // at the ends of the line are skipped by the segmenter, and the
        // offsets still index the input line.
        assert_eq!(spans("  東京 \n", &[]), vec![vec![span("東京", 2, 8)]]);
        assert_eq!(spans("\t東京\t\n", &[]), vec![vec![span("東京", 1, 7)]]);
    }

    #[test]
    fn tokenize_offsets_index_each_input_line() {
        assert_eq!(
            spans(" 東京\n 大阪 \n", &[]),
            vec![vec![span("東京", 1, 7)], vec![span("大阪", 1, 7)]]
        );
    }

    #[test]
    fn tokenize_keeps_ideographic_space_at_line_ends() {
        // U+3000 is the IPADIC entry `記号,空白`, not a `SPACE` character:
        // it is a token at the ends of a line as elsewhere in the line.
        let input = "\u{3000}東京\u{3000}\n";
        assert_eq!(
            spans(input, &[]),
            vec![vec![
                span("\u{3000}", 0, 3),
                span("東京", 3, 9),
                span("\u{3000}", 9, 12),
            ]]
        );

        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .write_stdin(input)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 4, "3 tokens + EOS, got: {stdout}");
        assert!(
            lines[0].starts_with("\u{3000}\t記号,空白,"),
            "got: {stdout}"
        );
        assert!(lines[1].starts_with("東京\t"), "got: {stdout}");
        assert!(
            lines[2].starts_with("\u{3000}\t記号,空白,"),
            "got: {stdout}"
        );
        assert_eq!(lines[3], "EOS");

        let output = lindera()
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "wakati",
            ])
            .write_stdin(input)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, "\u{3000} 東京 \u{3000}\n");
    }

    #[test]
    fn tokenize_removes_crlf() {
        assert_eq!(spans("東京\r\n", &[]), vec![vec![span("東京", 0, 6)]]);

        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .write_stdin("東京\r\n大阪\r\n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains('\r'), "got: {stdout:?}");
        assert_eq!(stdout.matches("EOS\n").count(), 2, "got: {stdout}");
    }

    #[test]
    fn tokenize_nbest_offsets_index_the_input_line() {
        let results = spans("  東京 \n", &["--nbest", "2"]);
        assert_eq!(results.len(), 2, "got: {results:?}");
        for tokens in &results {
            assert_eq!(tokens.first().map(|token| token.1), Some(2), "{results:?}");
            assert_eq!(tokens.last().map(|token| token.2), Some(8), "{results:?}");
        }
    }

    #[test]
    fn tokenize_nbest_line_of_spaces_gives_one_result() {
        // The spaces are skipped, so the line has one path without tokens
        // (from BOS to EOS), as in MeCab; trimming the line used to leave
        // an empty line, which has no N-best result.
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic", "--nbest", "2"])
            .write_stdin("   \n")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 2, "got: {stdout}");
        assert!(lines[0].starts_with("NBEST 1 (cost="), "got: {stdout}");
        assert_eq!(lines[1], "EOS");
    }

    #[test]
    fn tokenize_keep_whitespace_outputs_whitespace_at_line_ends() {
        assert_eq!(
            spans("  東京 \n", &["--keep-whitespace"]),
            vec![vec![span("  ", 0, 2), span("東京", 2, 8), span(" ", 8, 9)]]
        );
        assert_eq!(
            spans("  東京\r\n", &["--keep-whitespace"]),
            vec![vec![span("  ", 0, 2), span("東京", 2, 8)]]
        );
    }
}
