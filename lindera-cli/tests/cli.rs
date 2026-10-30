//! End-to-end smoke tests for the `lindera` CLI binary.
//!
//! Tests that do not require a dictionary always run. Tests that tokenize
//! text are gated behind the `embed-ipadic` feature:
//!
//! ```sh
//! cargo test -p lindera-cli --features train,embed-ipadic
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// Builds a hand-made system dictionary with a 4x4 connection matrix
/// (context IDs 0 to 3) in `dir`, so that no embedded dictionary is needed.
///
/// Returns the built dictionary directory and its `metadata.json`, the
/// default metadata, which has the IPADIC layout.
fn build_four_by_four_dictionary(dir: &Path) -> (PathBuf, PathBuf) {
    let source = dir.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("char.def"),
        "DEFAULT 0 1 0\nSPACE 0 1 0\nKANJI 0 0 2\n0x0020 SPACE\n0x4E00..0x9FFF KANJI\n",
    )
    .unwrap();
    std::fs::write(
        source.join("unk.def"),
        "DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*\n\
         SPACE,0,0,10000,空白,*,*,*,*,*,*,*,*\n\
         KANJI,0,0,10000,名詞,一般,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    std::fs::write(
        source.join("lex.csv"),
        "東京,2,2,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n",
    )
    .unwrap();
    let mut matrix_def = String::from("4 4\n");
    for right_id in 0..4 {
        for left_id in 0..4 {
            matrix_def.push_str(&format!("{right_id} {left_id} 100\n"));
        }
    }
    std::fs::write(source.join("matrix.def"), matrix_def).unwrap();
    let metadata = dir.join("metadata.json");
    std::fs::write(
        &metadata,
        serde_json::to_vec(&lindera::dictionary::Metadata::default()).unwrap(),
    )
    .unwrap();

    let dict = dir.join("dict");
    let output = lindera()
        .args(["build", "--src", source.to_str().unwrap()])
        .args(["--dest", dict.to_str().unwrap()])
        .args(["--metadata", metadata.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "system build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (dict, metadata)
}

/// Runs `lindera build --user` on `user_csv` into `dest`, with `extra_args`.
fn build_user_dictionary(
    user_csv: &Path,
    dest: &Path,
    metadata: &Path,
    extra_args: &[&str],
) -> std::process::Output {
    lindera()
        .args(["build", "--user", "--src", user_csv.to_str().unwrap()])
        .args(["--dest", dest.to_str().unwrap()])
        .args(["--metadata", metadata.to_str().unwrap()])
        .args(extra_args)
        .output()
        .unwrap()
}

/// A user dictionary whose context IDs lie outside the system dictionary's
/// connection matrix builds when `lindera build --user` is not given the
/// system dictionary, and `lindera tokenize` then reports it as an error
/// instead of panicking when the entry matches the input (#1171). The `.bin`
/// is checked when it is combined with the system dictionary, by word id;
/// the `.csv` is checked as it is built for the system dictionary, by CSV
/// line and surface (#1187).
#[test]
fn tokenize_rejects_user_dictionary_context_ids_outside_the_matrix() {
    let dir = tempfile::tempdir().unwrap();
    let (dict, metadata) = build_four_by_four_dictionary(dir.path());
    // The entry of the issue report: context IDs 60000 against a 4x4 matrix.
    let user_csv = dir.path().join("userdic.csv");
    std::fs::write(
        &user_csv,
        "東京都,60000,60000,-1000,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト\n",
    )
    .unwrap();
    let user_dest = dir.path().join("user");
    let output = build_user_dictionary(&user_csv, &user_dest, &metadata, &[]);
    assert!(
        output.status.success(),
        "user build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (user_dictionary, names_the_entry) in [
        (
            user_dest.join("userdic.bin"),
            "user dictionary entry 0 (名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト)",
        ),
        (user_csv, "user dictionary line 1 (surface: '東京都')"),
    ] {
        let output = lindera()
            .args(["tokenize", "--dict", dict.to_str().unwrap()])
            .args(["--user-dict", user_dictionary.to_str().unwrap()])
            .write_stdin("東京都\n")
            .output()
            .unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success(), "got: {stderr}");
        assert!(
            stderr.contains(&format!(
                "{names_the_entry}: left context id 60000 is out of range"
            )),
            "got: {stderr}"
        );
        assert!(!stderr.contains("panicked"), "got: {stderr}");
    }
}

/// With `--system-dict`, `lindera build --user` checks every row's context
/// IDs against the system dictionary's matrix and fails on a row outside
/// it, naming its CSV line and surface, without writing the output (#1182).
#[test]
fn build_user_with_system_dict_rejects_context_ids_outside_the_matrix() {
    let dir = tempfile::tempdir().unwrap();
    let (dict, metadata) = build_four_by_four_dictionary(dir.path());
    let user_csv = dir.path().join("userdic.csv");
    std::fs::write(
        &user_csv,
        "東京タワー,3,3,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー\n\
         東京都,60000,1,-1000,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト\n",
    )
    .unwrap();
    let user_dest = dir.path().join("user");

    let output = build_user_dictionary(
        &user_csv,
        &user_dest,
        &metadata,
        &["--system-dict", dict.to_str().unwrap()],
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success(), "got: {stderr}");
    assert!(
        stderr.contains(
            "user dictionary line 2 (surface: '東京都'): left context id 60000 is out of range \
             for the connection matrix (matrix.def size 4 4; left context ids must be below 4)"
        ),
        "got: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "got: {stderr}");
    assert!(!user_dest.join("userdic.bin").exists());
}

/// A user dictionary whose context IDs are all inside the matrix builds with
/// `--system-dict`, the last cell of each axis included.
#[test]
fn build_user_with_system_dict_accepts_context_ids_inside_the_matrix() {
    let dir = tempfile::tempdir().unwrap();
    let (dict, metadata) = build_four_by_four_dictionary(dir.path());
    let user_csv = dir.path().join("userdic.csv");
    std::fs::write(
        &user_csv,
        "東京タワー,3,3,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー\n",
    )
    .unwrap();
    let user_dest = dir.path().join("user");

    let output = build_user_dictionary(
        &user_csv,
        &user_dest,
        &metadata,
        &["--system-dict", dict.to_str().unwrap()],
    );
    assert!(
        output.status.success(),
        "got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(user_dest.join("userdic.bin").exists());
}

/// `--system-dict` names the system dictionary a user dictionary is for, so
/// it is rejected without `--user`.
#[test]
fn build_system_dict_requires_user() {
    let dir = tempfile::tempdir().unwrap();
    let output = lindera()
        .args(["build", "--src", dir.path().to_str().unwrap()])
        .args(["--dest", dir.path().join("dict").to_str().unwrap()])
        .args([
            "--metadata",
            dir.path().join("metadata.json").to_str().unwrap(),
        ])
        .args(["--system-dict", "embedded://ipadic"])
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success(), "got: {stderr}");
    assert!(stderr.contains("--user"), "got: {stderr}");
}

/// `lindera build` rejects a `char.def` category without an `unk.def` line,
/// as `mecab-dict-index` does. Such a dictionary used to build, and its
/// output then stopped before a character of the category that no word
/// covers (#1195).
#[test]
fn build_rejects_a_char_def_category_without_an_unk_def_line() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("char.def"),
        "DEFAULT 0 1 0\nSPACE 0 1 0\n0x0020 SPACE\n",
    )
    .unwrap();
    std::fs::write(
        source.join("unk.def"),
        "DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    std::fs::write(
        source.join("lex.csv"),
        "東京,0,0,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n",
    )
    .unwrap();
    std::fs::write(source.join("matrix.def"), "1 1\n0 0 0\n").unwrap();
    let metadata = dir.path().join("metadata.json");
    std::fs::write(
        &metadata,
        serde_json::to_vec(&lindera::dictionary::Metadata::default()).unwrap(),
    )
    .unwrap();

    let output = lindera()
        .args(["build", "--src", source.to_str().unwrap()])
        .args(["--dest", dir.path().join("dict").to_str().unwrap()])
        .args(["--metadata", metadata.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success(), "got: {stderr}");
    assert!(
        stderr.contains(&format!(
            "{}: no line for category SPACE, which char.def defines",
            source.join("unk.def").display()
        )),
        "got: {stderr}"
    );
}

/// Builds a dictionary of the words `a`, `b` (context IDs 1, cost 100 each)
/// and `ab` (context IDs 2, cost 150) with the given `matrix.def`, and
/// returns the wakati output of `lindera tokenize` for `ab`.
fn tokenize_ab_with_matrix_def(dir: &Path, matrix_def: &str) -> String {
    let source = dir.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("char.def"),
        "DEFAULT 0 1 0\nSPACE 0 1 0\n0x0020 SPACE\n",
    )
    .unwrap();
    std::fs::write(
        source.join("unk.def"),
        "DEFAULT,1,1,10000,名詞,一般,*,*,*,*,*,*,*\n\
         SPACE,1,1,10000,記号,空白,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    std::fs::write(
        source.join("lex.csv"),
        "a,1,1,100,名詞,一般,*,*,*,*,a,エー,エー\n\
         b,1,1,100,名詞,一般,*,*,*,*,b,ビー,ビー\n\
         ab,2,2,150,名詞,一般,*,*,*,*,ab,エービー,エービー\n",
    )
    .unwrap();
    std::fs::write(source.join("matrix.def"), matrix_def).unwrap();
    let metadata = dir.join("metadata.json");
    std::fs::write(
        &metadata,
        serde_json::to_vec(&lindera::dictionary::Metadata::default()).unwrap(),
    )
    .unwrap();
    let dict = dir.join("dict");
    lindera()
        .args(["build", "--src", source.to_str().unwrap()])
        .args(["--dest", dict.to_str().unwrap()])
        .args(["--metadata", metadata.to_str().unwrap()])
        .assert()
        .success();

    let output = lindera()
        .args(["tokenize", "--dict", dict.to_str().unwrap()])
        .args(["--output", "wakati"])
        .write_stdin("ab\n")
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

/// `lindera build` gives a cell that `matrix.def` leaves out the cost 0, as
/// `mecab-dict-index` does. Such a cell cost 32767 before (#1211).
#[test]
fn build_gives_a_cell_missing_from_matrix_def_the_cost_0() {
    // The matrix lists only the cells on the path `a / b`, which costs 200.
    // The path through `ab` costs 150 plus the two cells (0, 2) and (2, 0)
    // that it leaves out, so it wins when they cost 0. MeCab 0.996 outputs
    // `ab` for this dictionary.
    let sparse = "3 3\n0 1 0\n1 1 0\n1 0 0\n";
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(tokenize_ab_with_matrix_def(dir.path(), sparse), "ab\n");

    // With the two cells listed at 32767, `a / b` wins, as in MeCab: the
    // missing cells decide the result above.
    let listed = format!("{sparse}0 2 32767\n2 0 32767\n");
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(tokenize_ab_with_matrix_def(dir.path(), &listed), "a b\n");
}

/// Runs `lindera tokenize --dict <dict>` on `input`, with `RUST_LOG` set to
/// `rust_log`, or unset when it is `None`.
fn tokenize_with_rust_log(
    dict: &Path,
    input: &str,
    rust_log: Option<&str>,
) -> std::process::Output {
    let mut command = lindera();
    command
        .args(["tokenize", "--dict", dict.to_str().unwrap()])
        .write_stdin(input);
    match rust_log {
        Some(filter) => command.env("RUST_LOG", filter),
        None => command.env_remove("RUST_LOG"),
    };
    command.output().unwrap()
}

/// A line without a sentence delimiter for more than 32 KiB makes the
/// segmenter force a sentence boundary with a warning, which the CLI writes
/// to stderr by default and hides with `RUST_LOG=error`. stdout is the same
/// either way (#1107).
#[test]
fn tokenize_writes_the_forced_cut_warning_to_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let (dict, _) = build_four_by_four_dictionary(dir.path());
    // One forced cut, at 32 KiB.
    let input = format!("{}\n", "a".repeat(40_000));
    let warning = "forcing a sentence boundary";

    let default = tokenize_with_rust_log(&dict, &input, None);
    let default_stderr = String::from_utf8(default.stderr).unwrap();
    assert!(default.status.success(), "got: {default_stderr}");
    assert_eq!(
        default_stderr.matches(warning).count(),
        1,
        "got: {default_stderr}"
    );

    let quiet = tokenize_with_rust_log(&dict, &input, Some("error"));
    let quiet_stderr = String::from_utf8(quiet.stderr).unwrap();
    assert!(quiet.status.success(), "got: {quiet_stderr}");
    assert!(quiet_stderr.is_empty(), "got: {quiet_stderr}");

    let stdout = String::from_utf8(default.stdout).unwrap();
    assert!(stdout.contains("aaaa"), "got: {stdout}");
    assert!(!stdout.contains(warning), "got: {stdout}");
    assert_eq!(stdout.as_bytes(), quiet.stdout.as_slice());
}

/// `lindera build` accepts an `unk.def` line whose left and right context
/// IDs differ without a warning, as `mecab-dict-index` does: every line of
/// mecab-ko-dic's `unk.def` is such a line (#1107).
#[test]
fn build_accepts_unk_def_lines_with_different_left_and_right_ids_silently() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("char.def"),
        "DEFAULT 0 1 0\nSPACE 0 1 0\n0x0020 SPACE\n",
    )
    .unwrap();
    std::fs::write(
        source.join("unk.def"),
        "DEFAULT,0,1,10000,補助記号,一般,*,*,*,*,*,*,*\n\
         SPACE,1,0,10000,空白,*,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    std::fs::write(
        source.join("lex.csv"),
        "東京,0,0,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ\n",
    )
    .unwrap();
    std::fs::write(
        source.join("matrix.def"),
        "2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n",
    )
    .unwrap();
    let metadata = dir.path().join("metadata.json");
    std::fs::write(
        &metadata,
        serde_json::to_vec(&lindera::dictionary::Metadata::default()).unwrap(),
    )
    .unwrap();

    let output = lindera()
        .args(["build", "--src", source.to_str().unwrap()])
        .args(["--dest", dir.path().join("dict").to_str().unwrap()])
        .args(["--metadata", metadata.to_str().unwrap()])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "got: {stderr}");
    assert!(stderr.is_empty(), "got: {stderr}");
}

#[cfg(feature = "embed-ipadic")]
mod with_ipadic {
    use super::*;

    /// `--system-dict` takes a dictionary URI as `lindera tokenize --dict`
    /// does; against IPADIC's 1316 x 1316 matrix, context ID 60000 fails
    /// the build (#1182).
    #[test]
    fn build_user_with_embedded_system_dict_rejects_context_ids_outside_the_matrix() {
        let dir = tempfile::tempdir().unwrap();
        let user_csv = dir.path().join("userdic.csv");
        std::fs::write(
            &user_csv,
            "東京スカイツリー,60000,1,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n",
        )
        .unwrap();
        let metadata =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../lindera-ipadic/metadata.json");

        let output = build_user_dictionary(
            &user_csv,
            &dir.path().join("user"),
            &metadata,
            &["--system-dict", "embedded://ipadic"],
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success(), "got: {stderr}");
        assert!(
            stderr.contains(
                "user dictionary line 1 (surface: '東京スカイツリー'): left context id 60000 is \
                 out of range for the connection matrix (matrix.def size 1316 1316; left context \
                 ids must be below 1316)"
            ),
            "got: {stderr}"
        );
    }

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

    /// Writes a configuration file named `name` in `dir` whose only token
    /// filter is `kind`, and returns its path.
    fn write_config_with_token_filter(dir: &Path, name: &str, kind: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("token_filters:\n  - kind: \"{kind}\"\n    args: {{}}\n"),
        )
        .unwrap();
        path
    }

    /// Returns the wakati output and the stderr of `lindera tokenize` for
    /// `Lindera`, with the configuration variables set as in `vars` and
    /// unset otherwise.
    fn tokenize_with_config_variables(vars: &[(&str, &Path)]) -> (String, String) {
        let mut command = lindera();
        command
            .env_remove("LINDERA_CONFIG_FILE")
            .env_remove("LINDERA_CONFIG_PATH")
            .env_remove("RUST_LOG");
        for (name, value) in vars {
            command.env(name, value);
        }
        let output = command
            .args([
                "tokenize",
                "--dict",
                "embedded://ipadic",
                "--output",
                "wakati",
            ])
            .write_stdin("Lindera\n")
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "got: {stderr}");
        (stdout.trim().to_string(), stderr)
    }

    /// `LINDERA_CONFIG_FILE` names the configuration file, without a
    /// warning (#1304).
    #[test]
    fn tokenize_reads_the_config_file_variable() {
        let dir = tempfile::tempdir().unwrap();
        let lower = write_config_with_token_filter(dir.path(), "lower.yml", "lowercase");

        let (stdout, _) = tokenize_with_config_variables(&[]);
        assert_eq!(stdout, "Lindera");

        let (stdout, stderr) = tokenize_with_config_variables(&[("LINDERA_CONFIG_FILE", &lower)]);
        assert_eq!(stdout, "lindera");
        assert!(!stderr.contains("LINDERA_CONFIG"), "got: {stderr}");
    }

    /// The deprecated `LINDERA_CONFIG_PATH` alone still names the
    /// configuration file, with a deprecation warning on stderr (#1304).
    #[test]
    fn tokenize_reads_the_deprecated_config_path_variable_with_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        let lower = write_config_with_token_filter(dir.path(), "lower.yml", "lowercase");

        let (stdout, stderr) = tokenize_with_config_variables(&[("LINDERA_CONFIG_PATH", &lower)]);
        assert_eq!(stdout, "lindera");
        assert!(
            stderr.contains(
                "Warning: LINDERA_CONFIG_PATH is deprecated and will be removed in v8.0.0; set \
                 LINDERA_CONFIG_FILE instead"
            ),
            "got: {stderr}"
        );
    }

    /// With both variables set, the file of `LINDERA_CONFIG_FILE` is read,
    /// and stderr says that `LINDERA_CONFIG_PATH` is ignored (#1304).
    #[test]
    fn tokenize_prefers_the_config_file_variable_over_the_deprecated_one() {
        let dir = tempfile::tempdir().unwrap();
        let lower = write_config_with_token_filter(dir.path(), "lower.yml", "lowercase");
        let upper = write_config_with_token_filter(dir.path(), "upper.yml", "uppercase");

        let (stdout, stderr) = tokenize_with_config_variables(&[
            ("LINDERA_CONFIG_FILE", &upper),
            ("LINDERA_CONFIG_PATH", &lower),
        ]);
        assert_eq!(stdout, "LINDERA");
        assert!(
            stderr.contains(
                "Warning: LINDERA_CONFIG_PATH is ignored because LINDERA_CONFIG_FILE is set"
            ),
            "got: {stderr}"
        );
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

    /// Two lines with tokens around an empty line and a line of spaces,
    /// which have no tokens.
    const LINES_WITHOUT_TOKENS: &str = "東京\n\n   \n大阪\n";

    fn tokenize_stdout(input: &str, args: &[&str]) -> String {
        let output = lindera()
            .args(["tokenize", "--dict", "embedded://ipadic"])
            .args(args)
            .write_stdin(input.to_string())
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn tokenize_wakati_writes_a_line_for_every_input_line() {
        // A line without tokens is an empty line, as in MeCab (#1147), so
        // the output lines match the input lines.
        assert_eq!(
            tokenize_stdout(LINES_WITHOUT_TOKENS, &["--output", "wakati"]),
            "東京\n\n\n大阪\n"
        );
    }

    #[test]
    fn tokenize_nbest_gives_every_line_a_result() {
        // Every input line, the empty one included, gets at least one
        // result (#1147), and every result has its line: `EOS` (mecab),
        // an array (json) or a line of surfaces, empty without tokens
        // (wakati).
        for format in ["mecab", "json", "wakati"] {
            let stdout =
                tokenize_stdout(LINES_WITHOUT_TOKENS, &["--nbest", "2", "--output", format]);
            let lines: Vec<&str> = stdout.lines().collect();
            let headers: Vec<usize> = (0..lines.len())
                .filter(|&i| lines[i].starts_with("NBEST "))
                .collect();
            let firsts = lines
                .iter()
                .filter(|line| line.starts_with("NBEST 1 "))
                .count();
            assert_eq!(
                firsts, 4,
                "one first result per input line ({format}): {stdout}"
            );
            if format == "wakati" {
                // One line of surfaces after each header.
                assert_eq!(lines.len(), 2 * headers.len(), "{stdout}");
                for &i in &headers {
                    assert!(!lines[i + 1].starts_with("NBEST "), "{stdout}");
                }
            } else {
                // The last line of each result: `EOS`, or the closing
                // bracket of a pretty-printed array (`[]` without tokens).
                let ends = lines
                    .iter()
                    .filter(|line| match format {
                        "mecab" => **line == "EOS",
                        _ => **line == "]" || **line == "[]",
                    })
                    .count();
                assert_eq!(ends, headers.len(), "({format}): {stdout}");
            }
        }

        // The empty line and the line of spaces get one result without
        // tokens each, with the same cost.
        let results = spans(LINES_WITHOUT_TOKENS, &["--nbest", "2"]);
        let empty: Vec<usize> = (0..results.len())
            .filter(|&i| results[i].is_empty())
            .collect();
        assert_eq!(empty.len(), 2, "{results:?}");
        let stdout = tokenize_stdout(LINES_WITHOUT_TOKENS, &["--nbest", "2"]);
        let blank_costs: Vec<&str> = stdout
            .lines()
            .zip(stdout.lines().skip(1))
            .filter(|(_, next)| *next == "EOS")
            .map(|(header, _)| header)
            .filter(|header| header.starts_with("NBEST "))
            .collect();
        assert_eq!(blank_costs.len(), 2, "{stdout}");
        assert_eq!(blank_costs[0], blank_costs[1], "{stdout}");
    }

    // The three examples of "N-Best tokenization" in
    // docs/src/lindera-cli/commands.md and docs/ja/src/lindera-cli/commands.md
    // (#1113), run with the same flags. When one of these outputs changes,
    // update both pages too; they align the details with spaces where the
    // CLI writes a tab.

    #[test]
    fn tokenize_nbest_documented_example() {
        // Results 2 and 3 are different segmentations with the same cost.
        assert_eq!(
            tokenize_stdout("すもももももももものうち\n", &["-N", "3"]),
            "NBEST 1 (cost=21245)\n\
             すもも\t名詞,一般,*,*,*,*,すもも,スモモ,スモモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             の\t助詞,連体化,*,*,*,*,の,ノ,ノ\n\
             うち\t名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ\n\
             EOS\n\
             NBEST 2 (cost=24541)\n\
             すもも\t名詞,一般,*,*,*,*,すもも,スモモ,スモモ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             の\t助詞,連体化,*,*,*,*,の,ノ,ノ\n\
             うち\t名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ\n\
             EOS\n\
             NBEST 3 (cost=24541)\n\
             すもも\t名詞,一般,*,*,*,*,すもも,スモモ,スモモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             もも\t名詞,一般,*,*,*,*,もも,モモ,モモ\n\
             も\t助詞,係助詞,*,*,*,*,も,モ,モ\n\
             の\t助詞,連体化,*,*,*,*,の,ノ,ノ\n\
             うち\t名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ\n\
             EOS\n"
        );
    }

    #[test]
    fn tokenize_nbest_unique_documented_example() {
        assert_eq!(
            tokenize_stdout(
                "営業部長谷川です\n",
                &["-N", "5", "--nbest-unique", "-o", "wakati"]
            ),
            "NBEST 1 (cost=15760)\n\
             営業 部長 谷川 です\n\
             NBEST 2 (cost=17747)\n\
             営業 部 長谷川 です\n\
             NBEST 3 (cost=18365)\n\
             営業 部 長谷 川 です\n\
             NBEST 4 (cost=21125)\n\
             営業 部 長 谷川 です\n\
             NBEST 5 (cost=22259)\n\
             営業 部長 谷 川 です\n"
        );
    }

    #[test]
    fn tokenize_nbest_cost_threshold_documented_example() {
        // The 4th result of the example above (21125) exceeds 15760 + 5000.
        assert_eq!(
            tokenize_stdout(
                "営業部長谷川です\n",
                &[
                    "-N",
                    "10",
                    "--nbest-unique",
                    "--nbest-cost-threshold",
                    "5000",
                    "-o",
                    "wakati",
                ]
            ),
            "NBEST 1 (cost=15760)\n\
             営業 部長 谷川 です\n\
             NBEST 2 (cost=17747)\n\
             営業 部 長谷川 です\n\
             NBEST 3 (cost=18365)\n\
             営業 部 長谷 川 です\n"
        );
    }
}

// The training pipeline of docs/src/development/training_pipeline.md and of
// the complete workflow example in docs/src/lindera-cli/commands.md (and
// their docs/ja pages), run on the files in resources/training/ with the same
// flags. When the tokenize output changes, update those pages too.
#[cfg(feature = "train")]
mod with_trained_dictionary {
    use super::*;

    /// The documented tokenize output for "これは外国人参政権です。".
    /// `これ` and `は` are the seed words of the corpus, not one unknown
    /// word (#1168). The default L2 regularization keeps bigram weights
    /// (#1261); v6's default, L1 with lambda 0.01, left none and gives the
    /// same output.
    const DOCUMENTED_OUTPUT: &str = "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n\
                                     は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n\
                                     外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n\
                                     人\t名詞,接尾,一般,*,*,*,人,ジン,ジン\n\
                                     参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ\n\
                                     権\t名詞,接尾,一般,*,*,*,権,ケン,ケン\n\
                                     です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス\n\
                                     。\t記号,句点,*,*,*,*,。,。,。\n\
                                     EOS\n";

    /// Runs `command`, asserts that it succeeds and returns its stdout.
    fn stdout_of(command: &mut Command) -> String {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// The directory of the training files, `resources/training/`.
    fn training_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/training")
    }

    /// The documented `lindera train` command: the files in
    /// `resources/training/` with `corpus` as the corpus, writing the model
    /// to `model`.
    fn train_command(corpus: &Path, model: &Path) -> Command {
        let mut command = train_inputs_command(corpus, model);
        command.args(["--max-iterations", "100"]);
        command
    }

    /// `lindera train` with the files in `resources/training/` and `corpus`
    /// as the corpus, writing the model to `model`, without any other flag.
    fn train_inputs_command(corpus: &Path, model: &Path) -> Command {
        let training = training_dir();
        let input = |name: &str| training.join(name).to_str().unwrap().to_string();
        let mut command = lindera();
        command
            .arg("train")
            .args(["--seed", &input("seed.csv")])
            .args(["--corpus", corpus.to_str().unwrap()])
            .args(["--unk-def", &input("unk.def")])
            .args(["--char-def", &input("char.def")])
            .args(["--feature-def", &input("feature.def")])
            .args(["--rewrite-def", &input("rewrite.def")])
            .args(["--output", model.to_str().unwrap()]);
        command
    }

    /// The CLI accepts zero L1 penalties without passing them to OWL-QN.
    #[test]
    fn train_with_zero_l1_coefficient() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = training_dir().join("corpus.txt");
        let cases: [&[&str]; 3] = [
            &["--regularization", "l1", "--lambda", "0"],
            &["--regularization", "elasticnet", "--lambda", "0"],
            &[
                "--regularization",
                "elasticnet",
                "--lambda",
                "1",
                "--elastic-net-l1-ratio",
                "0",
            ],
        ];
        for (i, args) in cases.iter().enumerate() {
            let model = dir.path().join(format!("zero-l1-{i}.dat"));
            train_inputs_command(&corpus, &model)
                .args(["--max-iterations", "5", "--max-threads", "1"])
                .args(*args)
                .assert()
                .success();
            assert!(std::fs::metadata(model).unwrap().len() > 0);
        }
    }

    /// `lindera train` regularizes with L2 and lambda 1.0 by default, and
    /// `--regularization` chooses the type with the same coefficient
    /// (#1261). The parameters are printed before the optimizer starts, so
    /// one iteration is enough.
    #[test]
    fn train_regularizes_with_l2_and_lambda_1_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = training_dir().join("corpus.txt");
        let model = dir.path().join("model.dat");
        let cases: [(&[&str], &str); 3] = [
            (&[], "Training parameters: regularization=1 (L2),"),
            (
                &["--regularization", "l1"],
                "Training parameters: regularization=1 (L1),",
            ),
            (
                &["--regularization", "elasticnet"],
                "Training parameters: regularization=1 (ElasticNet, l1_ratio=0.5),",
            ),
        ];
        for (args, expected) in cases {
            let stdout = stdout_of(
                train_inputs_command(&corpus, &model)
                    .args(["--max-iterations", "1"])
                    .args(args),
            );
            assert!(stdout.contains(expected), "{args:?}: {stdout}");
        }
    }

    /// `--regularization` takes the names of `RegularizationType`, which the
    /// bindings share, in any case; another name fails before any file is
    /// read, and so does a NaN `--lambda` once training starts (#1262).
    #[test]
    fn train_rejects_an_unknown_regularization_and_a_nan_lambda() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = training_dir().join("corpus.txt");
        let model = dir.path().join("model.dat");
        let stdout = stdout_of(
            train_inputs_command(&corpus, &model)
                .args(["--max-iterations", "1"])
                .args(["--regularization", "Elastic-Net"]),
        );
        assert!(
            stdout.contains("Training parameters: regularization=1 (ElasticNet, l1_ratio=0.5),"),
            "{stdout}"
        );

        let missing = dir.path().join("missing.txt");
        for (args, expected, corpus) in [
            (
                ["--regularization", "l3"],
                "regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'",
                &missing,
            ),
            (
                ["--lambda", "NaN"],
                "lambda must be finite and >= 0",
                &corpus,
            ),
        ] {
            // Without the check, training with a NaN lambda never ends, even
            // with one iteration; the timeout kills it, so that the message
            // check fails instead of the test hanging.
            let output = train_inputs_command(corpus, &model)
                .args(["--max-iterations", "1"])
                .args(args)
                .timeout(Duration::from_secs(120))
                .output()
                .unwrap_or_else(|err| panic!("{args:?}: {err}"));
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!output.status.success(), "{args:?}: {stderr}");
            assert!(stderr.contains(expected), "{args:?}: {stderr}");
        }
    }

    /// `--max-iterations` and `--max-threads` take 1 or more: clap rejects
    /// 0 while it parses the arguments, so a missing corpus is never read,
    /// where the trainer rejected 0 only after building every lattice
    /// (#1275).
    #[test]
    fn train_rejects_zero_iterations_and_zero_threads() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.txt");
        let model = dir.path().join("model.dat");
        for flag in ["--max-iterations", "--max-threads"] {
            let output = train_inputs_command(&missing, &model)
                .args([flag, "0"])
                .output()
                .unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!output.status.success(), "{flag}: {stderr}");
            // clap's message, such as "invalid value '0' for
            // '--max-iterations <ITER>': 0 is not in 1..18446744073709551615".
            assert!(
                stderr.contains(&format!("invalid value '0' for '{flag} ")),
                "{flag}: {stderr}"
            );
            assert!(stderr.contains("0 is not in 1.."), "{flag}: {stderr}");
        }
    }

    /// `lindera export --metadata` records the regularization type next to
    /// the coefficient in `model_info`, and the Elastic Net ratio only for
    /// Elastic Net (#1272).
    #[test]
    fn export_records_the_regularization_type_in_model_info() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = training_dir().join("corpus.txt");
        let metadata = training_dir().join("metadata.json");
        let cases: [(&str, &[&str], &str, Option<f64>); 3] = [
            ("default", &[], "l2", None),
            ("l1", &["--regularization", "l1"], "l1", None),
            (
                "elasticnet",
                &[
                    "--regularization",
                    "elasticnet",
                    "--elastic-net-l1-ratio",
                    "0.3",
                ],
                "elasticnet",
                Some(0.3),
            ),
        ];
        for (label, args, expected, ratio) in cases {
            let model = dir.path().join(format!("{label}.dat"));
            let output = dir.path().join(label);
            stdout_of(
                train_inputs_command(&corpus, &model)
                    .args(["--max-iterations", "3"])
                    .args(args),
            );
            stdout_of(
                lindera()
                    .arg("export")
                    .args(["--model", model.to_str().unwrap()])
                    .args(["--metadata", metadata.to_str().unwrap()])
                    .args(["--output", output.to_str().unwrap()]),
            );
            let exported: serde_json::Value =
                serde_json::from_slice(&std::fs::read(output.join("metadata.json")).unwrap())
                    .unwrap();
            let info = &exported["model_info"];
            assert_eq!(info["regularization_type"], expected, "{label}: {info}");
            assert_eq!(info["regularization"], 1.0, "{label}: {info}");
            assert_eq!(
                info.get("elastic_net_l1_ratio")
                    .and_then(serde_json::Value::as_f64),
                ratio,
                "{label}: {info}"
            );
            assert_eq!(info["version"], "3.0.0", "{label}: {info}");
        }
    }

    /// Trains, exports and builds a dictionary from `resources/training/` in
    /// `dir`, and returns the built dictionary directory.
    fn build_trained_dictionary(dir: &Path) -> PathBuf {
        let training = training_dir();
        let input = |name: &str| training.join(name).to_str().unwrap().to_string();
        let model = dir.join("model.dat");
        let source = dir.join("dictionary");
        let dict = dir.join("compiled_dictionary");

        stdout_of(&mut train_command(&training.join("corpus.txt"), &model));
        stdout_of(
            lindera()
                .arg("export")
                .args(["--model", model.to_str().unwrap()])
                .args(["--metadata", &input("metadata.json")])
                .args(["--output", source.to_str().unwrap()]),
        );
        stdout_of(
            lindera()
                .arg("build")
                .args(["--src", source.to_str().unwrap()])
                .args(["--dest", dict.to_str().unwrap()])
                .args(["--metadata", source.join("metadata.json").to_str().unwrap()]),
        );
        dict
    }

    /// A dictionary trained from `resources/training/` tokenizes with the
    /// default settings, which skip whitespace by the `SPACE` category of its
    /// `char.def`, and outputs a kept space as the `SPACE` unknown word
    /// (#1166).
    #[test]
    fn tokenize_with_dictionary_trained_from_resources() {
        let dir = tempfile::tempdir().unwrap();
        let dict = build_trained_dictionary(dir.path());
        let tokenize = |input: &str, args: &[&str]| {
            stdout_of(
                lindera()
                    .args(["tokenize", "--dict", dict.to_str().unwrap()])
                    .args(args)
                    .write_stdin(input.to_string()),
            )
        };

        assert_eq!(
            tokenize("これは外国人参政権です。\n", &[]),
            DOCUMENTED_OUTPUT
        );
        assert_eq!(
            tokenize("これは 外国人参政権です。\n", &[]),
            DOCUMENTED_OUTPUT
        );
        assert_eq!(
            tokenize("これは 外国人参政権です。\n", &["--keep-whitespace"]),
            DOCUMENTED_OUTPUT.replacen("\n外国", "\n \t記号,空白,*,*,*,*,*,*,*\n外国", 1)
        );
    }

    /// Writes `resources/training/corpus.txt` with every feature cut to its
    /// first six columns, without the base form and the readings, into
    /// `dir`. No seed row has the whole feature string of one of its gold
    /// tokens, though each gold token has the surface of one.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory to write `corpus.txt` into.
    ///
    /// # Returns
    ///
    /// The path of the corpus.
    fn cut_corpus(dir: &Path) -> PathBuf {
        let cut: String = std::fs::read_to_string(training_dir().join("corpus.txt"))
            .unwrap()
            .lines()
            .map(|line| match line.split_once('\t') {
                Some((surface, feature)) => {
                    let columns: Vec<&str> = feature.split(',').take(6).collect();
                    format!("{surface}\t{}\n", columns.join(","))
                }
                None => format!("{line}\n"),
            })
            .collect();
        let path = dir.join("corpus.txt");
        std::fs::write(&path, cut).unwrap();
        path
    }

    /// `lindera train` warns on stderr, not on stdout, when no gold token
    /// is a seed word, and still trains (#1259), as on the corpus of
    /// `cut_corpus`. The files as they are give no warning.
    #[test]
    fn train_warns_on_stderr_when_no_gold_token_is_a_seed_word() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = training_dir().join("corpus.txt");

        let output = train_command(&corpus, &dir.path().join("model.dat"))
            .output()
            .unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "got: {stderr}");
        assert!(!stderr.contains("seed word"), "got: {stderr}");

        let cut_corpus = cut_corpus(dir.path());
        let model = dir.path().join("cut.dat");

        let output = train_command(&cut_corpus, &model).output().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(output.status.success(), "got: {stderr}");
        assert!(
            stderr.contains(
                "Warning: 0 of the 14 gold tokens are seed words, and 14 are not although seed \
                 rows have their surface."
            ),
            "got: {stderr}"
        );
        assert!(
            stderr.contains(
                "\n  gold token: 外国\t名詞,一般,*,*,*,*\n  \
                 seed row:   外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n"
            ),
            "got: {stderr}"
        );
        assert!(!stdout.contains("seed word"), "got: {stdout}");
        assert!(model.is_file());
    }

    /// `lindera train` warns on stderr, not on stdout, when the optimizer
    /// stops because an iteration left the objective unchanged, and still
    /// writes the model (#1268). On the corpus of `cut_corpus`, L1 with
    /// `--lambda 1` and the convergence test turned off (`--eta 0`) stops
    /// so, whether the first line search starts at a step of 1 or at a unit
    /// step (#1290). The line that says why stays on stdout. The warning of
    /// a failed line search is written by the same call.
    #[test]
    fn train_warns_on_stderr_when_the_optimizer_makes_no_progress() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = cut_corpus(dir.path());
        let model = dir.path().join("model.dat");

        let output = train_command(&corpus, &model)
            .args(["--regularization", "l1", "--lambda", "1", "--eta", "0"])
            .output()
            .unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(output.status.success(), "got: {stderr}");
        assert!(
            stdout.lines().any(|line| {
                line.starts_with("L-BFGS stopped after ")
                    && line.ends_with(": no progress: an iteration left the objective unchanged")
            }),
            "got: {stdout}"
        );
        let warning = "Warning: the optimizer could not make progress; after few iterations the \
                       model is barely trained";
        assert_eq!(
            stderr.lines().filter(|line| *line == warning).count(),
            1,
            "got: {stderr}"
        );
        assert!(!stdout.contains("could not make progress"), "got: {stdout}");
        assert!(model.is_file());
    }
}
