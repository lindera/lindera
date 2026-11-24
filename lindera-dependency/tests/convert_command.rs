//! The `lindera-dependency convert` command, with the embedded IPADIC.

#![cfg(all(feature = "train", feature = "embed-ipadic"))]

use std::fs;
use std::process::Command;

use lindera_dependency::{Morpheme, lattice};

/// A word line of CoNLL-U with the columns the conversion reads. A word
/// that starts a bunsetsu is its content head.
fn word(id: usize, form: &str, head: usize, deprel: &str, starts_bunsetsu: bool) -> String {
    let (label, position) = if starts_bunsetsu {
        ("B", "SEM_HEAD")
    } else {
        ("I", "CONT")
    };
    format!(
        "{id}\t{form}\t_\t_\t_\t_\t{head}\t{deprel}\t_\tBunsetuBILabel={label}|BunsetuPositionType={position}|SpaceAfter=No\n"
    )
}

/// Two sentences: その | 後 | 彼は | 本を | 読んだ。, whose first two bunsetsu
/// IPADIC has as the one morpheme その後, and a sentence whose root is not
/// its last bunsetsu.
fn corpus() -> String {
    let mut text = String::from("# sent_id = 1\n# text = その後彼は本を読んだ。\n");
    text.push_str(&word(1, "その", 2, "det", true));
    text.push_str(&word(2, "後", 7, "obl", true));
    text.push_str(&word(3, "彼", 7, "nsubj", true));
    text.push_str(&word(4, "は", 3, "case", false));
    text.push_str(&word(5, "本", 7, "obj", true));
    text.push_str(&word(6, "を", 5, "case", false));
    text.push_str(&word(7, "読ん", 0, "root", true));
    text.push_str(&word(8, "だ", 7, "aux", false));
    text.push_str(&word(9, "。", 7, "punct", false));
    text.push_str("\n# sent_id = 2\n# text = 読んだ彼\n");
    text.push_str(&word(1, "読ん", 0, "root", true));
    text.push_str(&word(2, "だ", 1, "aux", false));
    text.push_str(&word(3, "彼", 1, "nsubj", true));
    text.push('\n');
    text
}

/// The command.
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lindera-dependency"))
}

#[test]
fn converts_a_corpus_with_ipadic() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("corpus.conllu");
    let output = directory.path().join("corpus.lattice");
    fs::write(&input, corpus()).unwrap();

    let result = command()
        .args(["convert", "--dict", "embedded://ipadic", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    let report = String::from_utf8(result.stderr).unwrap();
    assert!(result.status.success(), "{report}");
    assert!(result.stdout.is_empty());

    // The output is the lattice format, a tree, with the dictionary's
    // features.
    let text = fs::read_to_string(&output).unwrap();
    let sentences = lattice::parse(&text).unwrap();
    assert_eq!(sentences.len(), 1);
    let sentence = &sentences[0];
    assert!(sentence.violations().is_empty());
    let bunsetsu: Vec<(String, Option<usize>)> = sentence
        .bunsetsu
        .iter()
        .map(|bunsetsu| {
            let surface = sentence.morphemes[bunsetsu.morphemes.clone()]
                .iter()
                .map(|morpheme| morpheme.surface())
                .collect();
            (surface, bunsetsu.head)
        })
        .collect();
    assert_eq!(
        bunsetsu,
        [
            ("その後".to_string(), Some(3)),
            ("彼は".to_string(), Some(3)),
            ("本を".to_string(), Some(3)),
            ("読んだ。".to_string(), None),
        ]
    );
    assert_eq!(sentence.morphemes[0].feature(0), Some("名詞"));
    assert!(text.starts_with("* 0 3D \nその後\t名詞,"), "{text}");
    assert!(sentence.comments.is_empty());
    // The bunsetsu lines hold the head only: no positions of the head word
    // and of the function word.
    assert!(
        sentence
            .bunsetsu
            .iter()
            .all(|bunsetsu| bunsetsu.positions.is_none())
    );

    // The report names the file and counts the merge and the drop.
    for line in [
        "corpus.conllu",
        "sentences kept: 1 of 2 (50.00%)",
        "  root: 1",
        "bunsetsu boundaries on a morpheme boundary: 3 of 4 (75.00%)",
        "bunsetsu merged: 1 of 5 (20.00%)",
        "bunsetsu written: 4",
        // その後, 彼, 本 and 読ん are the head words by the schema of IPADIC
        // and the content heads of the corpus.
        "head words on the content head of the corpus: 4 of 4 (100.00%)",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
}

#[test]
fn fails_with_the_cause_on_bad_arguments_and_input() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("corpus.conllu");
    let output = directory.path().join("corpus.lattice");

    // The error of a run of `convert`, which must fail.
    let run = |dict: &str| {
        let result = command()
            .args(["convert", "--dict", dict, "--input"])
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap();
        assert!(!result.status.success());
        String::from_utf8(result.stderr).unwrap()
    };

    // No input file.
    let error = run("embedded://ipadic");
    assert!(error.contains("Error: cannot open"), "{error}");

    // A file that is not CoNLL-U with bunsetsu labels: the line is named.
    fs::write(&input, "1\t彼\t_\t_\t_\t_\t0\troot\t_\t_\n").unwrap();
    let error = run("embedded://ipadic");
    assert!(
        error.contains("corpus.conllu: line 1: no BunsetuBILabel in the last column"),
        "{error}"
    );

    // No such dictionary.
    let error = run("/no/such/dictionary");
    assert!(
        error.contains("Error: cannot load the dictionary /no/such/dictionary"),
        "{error}"
    );
}
