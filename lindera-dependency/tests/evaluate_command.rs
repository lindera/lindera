//! The `lindera-dependency evaluate` and `lindera-dependency split`
//! commands. They need no dictionary.

#![cfg(feature = "train")]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use lindera_dependency::{Morpheme, lattice};

/// A sentence in the lattice format, of bunsetsu given as
/// `(surfaces of the morphemes, head)`, with `-1` for the root.
fn sentence(bunsetsu: &[(&[&str], i32)]) -> String {
    let mut text = String::new();
    for (index, (surfaces, head)) in bunsetsu.iter().enumerate() {
        text.push_str(&format!("* {index} {head}D \n"));
        for surface in *surfaces {
            text.push_str(&format!("{surface}\t*\n"));
        }
    }
    text.push_str("EOS\n");
    text
}

/// The answer: a | b | c | d as a chain, and x | y.
fn gold() -> String {
    sentence(&[(&["a"], 1), (&["b"], 2), (&["c"], 3), (&["d"], -1)])
        + &sentence(&[(&["x"], 1), (&["y"], -1)])
}

/// An output with the segmentation of the answer, and b → d instead of
/// b → c.
fn wrong_head() -> String {
    sentence(&[(&["a"], 1), (&["b"], 3), (&["c"], 3), (&["d"], -1)])
        + &sentence(&[(&["x"], 1), (&["y"], -1)])
}

/// An output that merges b and c, and gets the second sentence right.
fn merged() -> String {
    sentence(&[(&["a"], 1), (&["b", "c"], 2), (&["d"], -1)])
        + &sentence(&[(&["x"], 1), (&["y"], -1)])
}

/// Runs the command.
fn run(arguments: &[&str], paths: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"));
    command.args(arguments);
    for (option, path) in paths {
        command.arg(option).arg(path);
    }
    command.output().unwrap()
}

/// The standard output of a run that must succeed.
fn stdout(output: Output) -> String {
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{error}");
    String::from_utf8(output.stdout).unwrap()
}

/// The standard error of a run that must fail.
fn failure(output: Output) -> String {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn evaluates_an_output_as_text_and_as_json() {
    let directory = tempfile::tempdir().unwrap();
    let gold_path = directory.path().join("gold.lattice");
    let system_path = directory.path().join("system.lattice");
    fs::write(&gold_path, gold()).unwrap();
    fs::write(&system_path, wrong_head()).unwrap();
    let paths: [(&str, &Path); 2] = [("--gold", &gold_path), ("--system", &system_path)];

    let text = stdout(run(&["evaluate"], &paths));
    assert_eq!(
        text,
        "\
sentences: 2
left out, different morphemes: 0
not counted, no bunsetsu: 0
chunking precision: 100.00% (6/6)
chunking recall: 100.00% (6/6)
chunking F: 100.00%
chunking sentences: 100.00% (2/2)
dependency level 0: 83.33% (5/6)
dependency level 1: 75.00% (3/4)
dependency level 2: 50.00% (1/2)
dependency sentences: 50.00% (1/2)
end to end: 75.00% (3/4)
end to end sentences: 50.00% (1/2)
"
    );

    let json = stdout(run(&["evaluate", "--json"], &paths));
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "sentences": 2,
            "different_morphemes": [],
            "without_bunsetsu": 0,
            "chunking": {
                "correct": 6,
                "system": 6,
                "gold": 6,
                "precision": 1.0,
                "recall": 1.0,
                "f_score": 1.0,
                "sentences": { "correct": 2, "total": 2, "ratio": 1.0 },
            },
            "dependency": {
                "level0": { "correct": 5, "total": 6, "ratio": 5.0 / 6.0 },
                "level1": { "correct": 3, "total": 4, "ratio": 0.75 },
                "level2": { "correct": 1, "total": 2, "ratio": 0.5 },
                "sentences": { "correct": 1, "total": 2, "ratio": 0.5 },
            },
            "end_to_end": {
                "arcs": { "correct": 3, "total": 4, "ratio": 0.75 },
                "sentences": { "correct": 1, "total": 2, "ratio": 0.5 },
            },
        })
    );
}

#[test]
fn compares_two_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let gold_path = directory.path().join("gold.lattice");
    let first_path = directory.path().join("first.lattice");
    let second_path = directory.path().join("second.lattice");
    fs::write(&gold_path, gold()).unwrap();
    fs::write(&first_path, wrong_head()).unwrap();
    fs::write(&second_path, merged()).unwrap();
    let paths: [(&str, &Path); 3] = [
        ("--gold", &gold_path),
        ("--system", &first_path),
        ("--compare", &second_path),
    ];

    // The first output has a and c right in the first sentence; the second
    // has no arc of it right (the head of a is `b c`, and b and c are not
    // bunsetsu of their own). Both have x right.
    let text = stdout(run(&["evaluate"], &paths));
    assert!(text.starts_with("first output\nsentences: 2\n"), "{text}");
    assert!(text.contains("\n\nsecond output\nsentences: 2\n"), "{text}");
    assert!(
        text.contains("chunking precision: 80.00% (4/5)\nchunking recall: 66.67% (4/6)\n"),
        "{text}"
    );
    assert!(
        text.ends_with(
            "paired segmentations: both 1, first only 1, second only 0, neither 0, McNemar p = 1.0000\n\
             paired arcs: both 1, first only 2, second only 0, neither 1, McNemar p = 0.5000\n\
             paired sentences: both 1, first only 0, second only 0, neither 1, McNemar p = 1.0000\n"
        ),
        "{text}"
    );

    let json = stdout(run(&["evaluate", "--json"], &paths));
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["first"]["end_to_end"]["arcs"]["correct"], 3);
    assert_eq!(json["second"]["end_to_end"]["arcs"]["correct"], 1);
    assert_eq!(json["second"]["dependency"]["level0"]["total"], 2);
    let arcs = &json["paired_arcs"];
    assert_eq!(
        (
            &arcs["both"],
            &arcs["first_only"],
            &arcs["second_only"],
            &arcs["neither"]
        ),
        (&1.into(), &2.into(), &0.into(), &1.into())
    );
    // 2 and 0 discordant arcs: 2 × 1 / 4.
    let p_value = arcs["p_value"].as_f64().unwrap();
    assert!((p_value - 0.5).abs() < 1e-12, "{p_value}");
    assert_eq!(json["paired_sentences"]["neither"], 1);
    assert_eq!(json["paired_segmentations"]["first_only"], 1);
    assert_eq!(json["paired_segmentations"]["both"], 1);
}

#[test]
fn reports_a_sentence_with_different_morphemes() {
    let directory = tempfile::tempdir().unwrap();
    let gold_path = directory.path().join("gold.lattice");
    let system_path = directory.path().join("system.lattice");
    fs::write(&gold_path, gold()).unwrap();
    // The second sentence has another morpheme.
    let system = sentence(&[(&["a"], 1), (&["b"], 2), (&["c"], 3), (&["d"], -1)])
        + &sentence(&[(&["x"], 1), (&["z"], -1)]);
    fs::write(&system_path, system).unwrap();

    let text = stdout(run(
        &["evaluate"],
        &[("--gold", &gold_path), ("--system", &system_path)],
    ));
    assert!(
        text.starts_with(
            "sentences: 2\nleft out, different morphemes: 1 (sentences 2)\n\
             not counted, no bunsetsu: 0\nchunking precision: 100.00% (4/4)\n"
        ),
        "{text}"
    );
    assert!(
        text.ends_with("end to end sentences: 100.00% (1/1)\n"),
        "{text}"
    );
}

#[test]
fn evaluate_fails_with_the_cause() {
    let directory = tempfile::tempdir().unwrap();
    let gold_path = directory.path().join("gold.lattice");
    let system_path = directory.path().join("system.lattice");
    let paths: [(&str, &Path); 2] = [("--gold", &gold_path), ("--system", &system_path)];

    // No file.
    let error = failure(run(&["evaluate"], &paths));
    assert!(error.contains("Error: cannot open"), "{error}");

    // Files with different numbers of sentences.
    fs::write(&gold_path, gold()).unwrap();
    fs::write(&system_path, sentence(&[(&["a"], -1)])).unwrap();
    let error = failure(run(&["evaluate"], &paths));
    assert!(
        error.contains("Error: the answer has 2 sentences and the output 1"),
        "{error}"
    );

    // A file that is not in the lattice format: the file and the line.
    fs::write(&system_path, "not a lattice\n").unwrap();
    let error = failure(run(&["evaluate"], &paths));
    assert!(error.contains("system.lattice: line 1:"), "{error}");
}

#[test]
fn splits_a_file_into_folds() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("input.lattice");
    let folds_path = directory.path().join("folds");
    let surfaces = ["s0", "s1", "s2", "s3", "s4", "s5", "s6"];
    let sentences: Vec<String> = surfaces
        .iter()
        .map(|surface| sentence(&[(&[*surface], -1)]))
        .collect();
    fs::write(&input_path, sentences.concat()).unwrap();

    let output = run(
        &["split", "--folds", "3"],
        &[("--input", &input_path), ("--output-dir", &folds_path)],
    );
    let summary = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{summary}");
    assert!(output.stdout.is_empty());
    assert!(
        summary.ends_with(
            "input.lattice: 7 sentences\nfold 0: test 3, train 4\n\
             fold 1: test 2, train 5\nfold 2: test 2, train 5\n"
        ),
        "{summary}"
    );

    // The first surface of every sentence of a file of a fold.
    let read = |name: &str| -> Vec<String> {
        let text = fs::read_to_string(folds_path.join(name)).unwrap();
        lattice::parse(&text)
            .unwrap()
            .iter()
            .map(|sentence| sentence.morphemes[0].surface().to_string())
            .collect()
    };
    assert_eq!(read("fold0.test.lattice"), ["s0", "s3", "s6"]);
    assert_eq!(read("fold0.train.lattice"), ["s1", "s2", "s4", "s5"]);
    assert_eq!(read("fold1.test.lattice"), ["s1", "s4"]);
    assert_eq!(read("fold1.train.lattice"), ["s0", "s2", "s3", "s5", "s6"]);
    assert_eq!(read("fold2.test.lattice"), ["s2", "s5"]);
    assert_eq!(read("fold2.train.lattice"), ["s0", "s1", "s3", "s4", "s6"]);
    assert_eq!(fs::read_dir(&folds_path).unwrap().count(), 6);

    // A sentence is written with the bytes it was read from.
    assert_eq!(
        fs::read_to_string(folds_path.join("fold1.test.lattice")).unwrap(),
        format!("{}{}", sentences[1], sentences[4])
    );
}

#[test]
fn split_refuses_fewer_than_two_folds() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("input.lattice");
    let folds_path = directory.path().join("folds");
    fs::write(&input_path, sentence(&[(&["a"], -1)])).unwrap();
    let paths: [(&str, &Path); 2] = [("--input", &input_path), ("--output-dir", &folds_path)];

    let error = failure(run(&["split", "--folds", "1"], &paths));
    assert!(
        error.contains("Error: a cross-validation needs at least 2 folds"),
        "{error}"
    );
    let error = failure(run(&["split", "--folds", "0"], &paths));
    assert!(error.contains("--folds"), "{error}");
    assert!(!folds_path.exists());
}
