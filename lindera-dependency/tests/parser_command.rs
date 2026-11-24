//! The `lindera-dependency instances` command. It needs no dictionary.

#![cfg(feature = "train")]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Three sentences in the lattice format: 彼は / 本を / 静かに / 読んだ。
/// with 彼は depending on the last bunsetsu, 本を / 読む, and 読む.
const ANSWERS: &str = "\
* 0 3D 0/1 0.000000
彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 3D 0/1 0.000000
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 3D 0/1 0.000000
静か\t名詞,形容動詞語幹,*,*,*,*,静か,シズカ,シズカ
に\t助詞,副詞化,*,*,*,*,に,ニ,ニ
* 3 -1D 0/1 0.000000
読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。\t記号,句点,*,*,*,*,。,。,。
EOS
* 0 1D 0/1 0.000000
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 1 -1D 0/0 0.000000
読む\t動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム
EOS
* 0 -1D 0/0 0.000000
読む\t動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム
EOS
";

/// Runs the command.
fn run(arguments: &[&str], paths: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"));
    command.args(arguments);
    for (option, path) in paths {
        command.arg(option).arg(path);
    }
    command.output().unwrap()
}

/// The standard error of a run that must succeed without writing to the
/// standard output.
fn success(output: Output) -> String {
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{error}");
    assert!(output.stdout.is_empty());
    error
}

/// The standard error of a run that must fail.
fn failure(output: Output) -> String {
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn writes_the_decisions_of_each_sentence() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("answers.lattice");
    let output_path = directory.path().join("answers.instances");
    fs::write(&input_path, ANSWERS).unwrap();

    let report = success(run(
        &["instances"],
        &[("--input", &input_path), ("--output", &output_path)],
    ));
    assert_eq!(
        report,
        format!(
            "{}\n\
             features: distance,source,target,left,right,children,gap,after\n\
             sentences: 3\n\
             bunsetsu: 7\n\
             instances: 2\n\
             positive: 0\n\
             negative: 2\n",
            input_path.display()
        )
    );

    let written = fs::read_to_string(&output_path).unwrap();
    // The first sentence asks 彼は -> 本を and 本を -> 静かに, and both are
    // refused; the other two ask nothing. An empty line ends each.
    let sentences: Vec<&str> = written.split("\n\n").collect();
    assert_eq!(sentences.len(), 3, "{written:?}");
    assert_eq!(sentences[1..], ["", ""]);
    let lines: Vec<Vec<&str>> = sentences[0]
        .lines()
        .map(|line| line.split('\t').collect())
        .collect();
    assert_eq!(lines.len(), 2);
    for (line, source, target) in [(&lines[0], "彼", "本"), (&lines[1], "本", "静か")] {
        assert_eq!(line[0], "-1");
        assert!(line.contains(&format!("SHS:{source}").as_str()), "{line:?}");
        assert!(line.contains(&format!("DHS:{target}").as_str()), "{line:?}");
        assert!(line.contains(&"DIST:1"), "{line:?}");
        // The particle after the head of the source, which the default
        // feature set has.
        assert!(line.contains(&"SNP0:助詞"), "{line:?}");
        // The features are sorted.
        let mut sorted = line[1..].to_vec();
        sorted.sort_unstable();
        assert_eq!(line[1..], sorted);
    }
}

#[test]
fn makes_the_groups_of_features_asked_for() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("answers.lattice");
    let output_path = directory.path().join("answers.instances");
    fs::write(&input_path, ANSWERS).unwrap();

    let report = success(run(
        &["instances", "--features", "gap,distance"],
        &[("--input", &input_path), ("--output", &output_path)],
    ));
    assert!(report.contains("features: distance,gap\n"), "{report}");
    assert_eq!(
        fs::read_to_string(&output_path).unwrap(),
        "-1\tDIST:1\tGNB:1\n-1\tDIST:1\tGNB:1\n\n\n\n"
    );
}

#[test]
fn refuses_what_it_cannot_make_instances_of() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("answers.lattice");
    let output_path = directory.path().join("answers.instances");
    fs::write(&input_path, ANSWERS).unwrap();

    let error = failure(run(
        &["instances", "--features", "distance,head"],
        &[("--input", &input_path), ("--output", &output_path)],
    ));
    assert!(
        error.contains("\"head\" is not a group of features"),
        "{error}"
    );

    // The second sentence has its root first.
    let crossing =
        ANSWERS
            .replacen("* 0 1D 0/1", "* 0 -1D 0/1", 1)
            .replacen("* 1 -1D 0/0", "* 1 0D 0/0", 1);
    fs::write(&input_path, crossing).unwrap();
    let error = failure(run(
        &["instances"],
        &[("--input", &input_path), ("--output", &output_path)],
    ));
    assert!(error.contains("sentence 2"), "{error}");
    assert!(
        error.contains("the parser cannot make the tree of the sentence"),
        "{error}"
    );

    let error = failure(run(
        &["instances"],
        &[
            ("--input", &directory.path().join("missing.lattice")),
            ("--output", &output_path),
        ],
    ));
    assert!(error.contains("cannot open"), "{error}");
}
