//! The `lindera-dependency convert-cabocha` command, with `parse` on what it
//! writes. The two models are small ones written here in the formats of
//! CaboCha: no file of CaboCha is used.

#![cfg(feature = "train")]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use lindera_dependency::Morpheme;
use lindera_dependency::cabocha::{PartOfSpeechSet, schema};
use lindera_dependency::lattice;
use lindera_dependency::model::Model;
use lindera_dependency::parser::Quantization;

/// A chunker model: after `B` and after `I` it goes on with `I`, unless a
/// noun or a verb follows a case particle.
const CHUNKER: &str = "\
version: 100
cost-factor: 1
maxid: 8
xsize: 2

B
I

U00:%x[0,0]
U01:%x[-1,1]/%x[0,1]
B

0 B
4 U01:助詞-格助詞-一般/名詞-一般
6 U01:助詞-格助詞-一般/動詞-自立

-1
1
-1
1
5
-5
5
-5
";

/// A parser model: a bunsetsu whose head is ABC depends on the next one,
/// and any other waits. The kernel gives 2 for the features SHS:ABC,
/// DIST:1 and SFS:が, and -4.5 without the first.
const PARSER: &str = "\
C: 0.001500
bias: 0.000000
charset: UTF8
degree: 2
parsing_algorithm: 0
posset: IPA
type: dep

0 SHS:ABC
1 DIST:1
2 SFS:が

1 0 2
-1 1 2
0.5 0
";

/// Runs the command, for a minute at most.
fn run(arguments: &[&str], paths: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"));
    command.args(arguments);
    for (option, path) in paths {
        command.arg(option).arg(path);
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver is gone when the time is over: nothing to send to.
        let _ = sender.send(command.output());
    });
    receiver
        .recv_timeout(Duration::from_secs(60))
        .expect("the command ends within a minute")
        .unwrap()
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

/// The value of a line `key: value` of a summary.
fn value<'a>(summary: &'a str, key: &str) -> &'a str {
    summary
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
        .unwrap_or_else(|| panic!("no line {key} in {summary}"))
}

/// The morphemes of a sentence that starts with a word, in the lattice
/// format.
fn morphemes(first: &str) -> String {
    format!(
        "{first}\t名詞,一般,*,*,*,*,*\n\
         が\t助詞,格助詞,一般,*,*,*,が,ガ,ガ\n\
         本\t名詞,一般,*,*,*,*,本,ホン,ホン\n\
         を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n\
         読む\t動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム\n\
         EOS\n"
    )
}

/// The bunsetsu of the sentences of a file: the surfaces of each, and its
/// head.
fn bunsetsu(text: &str) -> Vec<Vec<(String, Option<usize>)>> {
    lattice::parse(text)
        .unwrap()
        .iter()
        .map(|sentence| {
            sentence
                .bunsetsu
                .iter()
                .map(|one| {
                    let surfaces: Vec<&str> = sentence.morphemes[one.morphemes.clone()]
                        .iter()
                        .map(|morpheme| morpheme.surface())
                        .collect();
                    (surfaces.join(" "), one.head)
                })
                .collect()
        })
        .collect()
}

#[test]
fn converts_the_two_models_and_parses_with_the_result() {
    let directory = tempfile::tempdir().unwrap();
    let chunker = directory.path().join("chunk.ipa.txt");
    let parser = directory.path().join("dep.ipa.txt");
    let model = directory.path().join("cabocha.model");
    fs::write(&chunker, CHUNKER).unwrap();
    fs::write(&parser, PARSER).unwrap();

    let summary = success(run(
        &["convert-cabocha", "--sigma", "0", "--min-support", "1"],
        &[
            ("--chunker", &chunker),
            ("--parser", &parser),
            ("--output", &model),
        ],
    ));
    assert_eq!(value(&summary, "dictionary"), "ipadic");
    assert_eq!(value(&summary, "chunker feature strings"), "2");
    assert_eq!(value(&summary, "support vectors"), "3 (2 positive)");
    assert_eq!(value(&summary, "parser feature strings"), "3");
    assert_eq!(value(&summary, "pairs"), "2 of 2 kept, 2 with a weight");
    // The terms of the models are said every time.
    assert!(
        summary.contains("limited to research and personal use")
            && summary.contains("do not distribute it"),
        "{summary}"
    );

    // The file is a whole model for IPADIC, with CaboCha's schema and the
    // weights as they are.
    let bytes = fs::read(&model).unwrap();
    assert_eq!(value(&summary, "bytes"), bytes.len().to_string());
    let read = Model::new(bytes).unwrap();
    assert_eq!(read.dictionary(), "ipadic");
    assert_eq!(read.schema(), &schema(PartOfSpeechSet::Ipa));
    assert!(read.chunker().is_some());
    let part = read.parser().unwrap();
    assert_eq!(part.classifier().quantization(), Quantization::F32);
    assert_eq!(part.classifier().feature_count(), 3);
    assert_eq!(part.classifier().pair_count(), 2);

    // `parse` reads it: ＡＢＣ is ABC for the model, and its bunsetsu
    // depends on the next one; another word waits for the last.
    let input = directory.path().join("input.lattice");
    let output = directory.path().join("output.lattice");
    fs::write(&input, morphemes("ＡＢＣ") + &morphemes("彼")).unwrap();
    success(run(
        &["parse", "--input-layer", "morphemes"],
        &[
            ("--model", &model),
            ("--input", &input),
            ("--output", &output),
        ],
    ));
    let parsed = bunsetsu(&fs::read_to_string(&output).unwrap());
    let sentence = |first: &str, head: usize| {
        vec![
            (format!("{first} が"), Some(head)),
            ("本 を".to_string(), Some(2)),
            ("読む".to_string(), None),
        ]
    };
    assert_eq!(parsed, [sentence("ＡＢＣ", 1), sentence("彼", 2)]);
}

#[test]
fn the_pruning_is_the_one_of_cabocha_unless_it_is_asked_for() {
    let directory = tempfile::tempdir().unwrap();
    let chunker = directory.path().join("chunk.ipa.txt");
    let parser = directory.path().join("dep.ipa.txt");
    let model = directory.path().join("cabocha.model");
    fs::write(&chunker, CHUNKER).unwrap();
    fs::write(&parser, PARSER).unwrap();
    let paths: [(&str, &Path); 3] = [
        ("--chunker", &chunker),
        ("--parser", &parser),
        ("--output", &model),
    ];
    // By default the three features are frequent ones, and their pairs
    // are kept whatever they are.
    let summary = success(run(&["convert-cabocha"], &paths));
    assert_eq!(value(&summary, "pairs"), "2 of 2 kept, 2 with a weight");
    // Without frequent features, a pair has to be in two support vectors.
    let summary = success(run(
        &["convert-cabocha", "--frequent-features", "0"],
        &paths,
    ));
    assert_eq!(value(&summary, "pairs"), "0 of 2 kept, 0 with a weight");
    let summary = success(run(
        &[
            "convert-cabocha",
            "--frequent-features",
            "0",
            "--min-support",
            "1",
        ],
        &paths,
    ));
    assert_eq!(value(&summary, "pairs"), "2 of 2 kept, 2 with a weight");
    // And to have the weight of sigma. Two of the three support vectors
    // are positive: 3.1 asks for 2.07 or more, which the pair of 2 is not,
    // and for -1.03 or less, which the pair of -2 is.
    let summary = success(run(
        &[
            "convert-cabocha",
            "--frequent-features",
            "0",
            "--min-support",
            "1",
            "--sigma",
            "3.1",
        ],
        &paths,
    ));
    assert_eq!(value(&summary, "pairs"), "1 of 2 kept, 1 with a weight");
}

#[test]
fn converts_a_model_for_juman_into_one_for_jumandic() {
    let directory = tempfile::tempdir().unwrap();
    let chunker = directory.path().join("chunk.juman.txt");
    let parser = directory.path().join("dep.juman.txt");
    let model = directory.path().join("cabocha.model");
    fs::write(&chunker, CHUNKER).unwrap();
    // The part-of-speech set is read in the header of the parser model.
    fs::write(&parser, PARSER.replace("posset: IPA", "posset: JUMAN")).unwrap();

    let summary = success(run(
        &["convert-cabocha"],
        &[
            ("--chunker", &chunker),
            ("--parser", &parser),
            ("--output", &model),
        ],
    ));
    assert_eq!(value(&summary, "dictionary"), "jumandic");
    let read = Model::new(fs::read(&model).unwrap()).unwrap();
    assert_eq!(read.dictionary(), "jumandic");
    assert_eq!(read.schema(), &schema(PartOfSpeechSet::Juman));
    assert_ne!(read.schema(), &schema(PartOfSpeechSet::Ipa));
}

#[test]
fn converts_a_model_for_unidic_into_one_for_the_dictionary_asked_for() {
    let directory = tempfile::tempdir().unwrap();
    let chunker = directory.path().join("chunk.unidic.txt");
    let parser = directory.path().join("dep.unidic.txt");
    let model = directory.path().join("cabocha.model");
    fs::write(&chunker, CHUNKER).unwrap();
    fs::write(&parser, PARSER.replace("posset: IPA", "posset: UNIDIC")).unwrap();
    let paths = [
        ("--chunker", chunker.as_path()),
        ("--parser", parser.as_path()),
        ("--output", model.as_path()),
    ];

    // Without the option the model is for `unidic`.
    let summary = success(run(&["convert-cabocha"], &paths));
    assert_eq!(value(&summary, "dictionary"), "unidic");
    let read = Model::new(fs::read(&model).unwrap()).unwrap();
    assert_eq!(read.dictionary(), "unidic");
    assert_eq!(read.schema(), &schema(PartOfSpeechSet::Unidic));
    // The parser reads two levels of the part of speech.
    assert_eq!(
        read.parser().unwrap().feature_set().to_string(),
        "distance,source,target,left,right,children,gap,pos-levels=2"
    );

    // With it, for another dictionary of UniDic.
    for dictionary in ["unidic-cwj", "unidic-csj", "unidic"] {
        let summary = success(run(
            &["convert-cabocha", "--dictionary", dictionary],
            &paths,
        ));
        assert_eq!(value(&summary, "dictionary"), dictionary);
        let read = Model::new(fs::read(&model).unwrap()).unwrap();
        assert_eq!(read.dictionary(), dictionary);
    }
    // A dictionary of another set is refused.
    assert!(
        failure(run(&["convert-cabocha", "--dictionary", "ipadic"], &paths)).contains(
            "the model is not one for the dictionary ipadic: its part-of-speech set is that of unidic, unidic-cwj, unidic-csj"
        )
    );
}

#[test]
fn fails_with_the_cause() {
    let directory = tempfile::tempdir().unwrap();
    let chunker = directory.path().join("chunk.ipa.txt");
    let parser = directory.path().join("dep.ipa.txt");
    let other = directory.path().join("dep.penn.txt");
    let model = directory.path().join("cabocha.model");
    let missing = directory.path().join("missing.txt");
    fs::write(&chunker, CHUNKER).unwrap();
    fs::write(&parser, PARSER).unwrap();
    fs::write(&other, PARSER.replace("posset: IPA", "posset: PENN")).unwrap();

    let convert = |chunker: &Path, parser: &Path, arguments: &[&str]| {
        let mut all = vec!["convert-cabocha"];
        all.extend_from_slice(arguments);
        failure(run(
            &all,
            &[
                ("--chunker", chunker),
                ("--parser", parser),
                ("--output", &model),
            ],
        ))
    };
    for (error, expected) in [
        (
            convert(&chunker, &other, &[]),
            "the model is for the part-of-speech set \"PENN\", which is not one of CaboCha",
        ),
        (
            convert(&chunker, &parser, &["--dictionary", "unidic"]),
            "the model is not one for the dictionary unidic: its part-of-speech set is that of ipadic",
        ),
        (
            convert(&parser, &chunker, &[]),
            "the parser model has the feature line \"B\"",
        ),
        (
            convert(&parser, &parser, &[]),
            "the chunker model has no `cost-factor` in its header",
        ),
        (convert(&missing, &parser, &[]), "cannot read "),
        (convert(&chunker, &missing, &[]), "cannot read "),
        (
            convert(&chunker, &parser, &["--sigma=-1"]),
            "sigma is -1, and has to be 0 or more",
        ),
    ] {
        assert!(error.contains(expected), "{expected}: {error}");
    }
    // Nothing is written when the conversion fails.
    assert!(!model.exists());
}
