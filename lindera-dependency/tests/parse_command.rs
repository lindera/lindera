//! The `lindera-dependency parse` command on plain text. It tokenizes with
//! the embedded IPADIC, and needs no training code: the models are made by
//! hand.

#![cfg(feature = "embed-ipadic")]

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use lindera_dependency::chunker::{ChunkerModel, Column, Template};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::ModelBuilder;
use lindera_dependency::parser::{FeatureSet, PairWeights, ParserModel, Quantization};
use lindera_dependency::{Morpheme, Schema, Sentence};

/// A chunker for which a particle, an auxiliary verb and a period continue
/// a bunsetsu, and anything else starts one.
fn chunker() -> ChunkerModel {
    let features: HashMap<String, [f64; 2]> = [
        "U12:助詞-係助詞",
        "U12:助詞-格助詞-一般",
        "U12:助動詞",
        "U12:記号-句点",
    ]
    .iter()
    .map(|feature| (feature.to_string(), [0.0, 1.0]))
    .collect();
    ChunkerModel::new(
        Schema::ipadic(),
        vec![Column::Surface, Column::PartOfSpeech],
        Template::cabocha(),
        [[0.0; 3]; 3],
        features,
    )
    .unwrap()
}

/// A parser for which a bunsetsu that ends in を depends on the next one,
/// and nothing else is attached before the end of the sentence.
fn parser() -> ParserModel {
    ParserModel::new(
        Schema::ipadic(),
        FeatureSet::cabocha(),
        -1.0,
        vec![("DIST:1".to_string(), 0.0), ("SFS:を".to_string(), 0.0)],
        PairWeights::Exact(vec![((0, 1), 2.0)]),
    )
    .unwrap()
}

/// A model file of a dictionary with the parts given.
fn model_bytes(dictionary: &str, chunker_part: bool, parser_part: bool) -> Vec<u8> {
    let mut builder = ModelBuilder::new(dictionary, Schema::ipadic()).unwrap();
    if chunker_part {
        builder = builder.chunker(&chunker()).unwrap();
    }
    if parser_part {
        builder = builder.parser(&parser(), Quantization::F32).unwrap();
    }
    builder.to_bytes()
}

/// A directory with `ipadic.model`, the model of IPADIC with both parts.
fn directory() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let model = directory.path().join("ipadic.model");
    fs::write(&model, model_bytes("ipadic", true, true)).unwrap();
    (directory, model)
}

/// Runs `parse` with arguments and paths, and text on its standard input.
fn run(arguments: &[&str], paths: &[(&str, &Path)], input: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"));
    command.arg("parse").args(arguments);
    for (option, path) in paths {
        command.arg(option).arg(path);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The command may end before it reads everything, when it refuses its
    // arguments: a pipe that is closed is no failure of the test.
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    child.wait_with_output().unwrap()
}

/// The standard output of a run that must succeed without a message.
fn success(output: Output) -> String {
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{error}");
    assert_eq!(error, "");
    String::from_utf8(output.stdout).unwrap()
}

/// The standard error of a run that must fail without output.
fn failure(output: Output) -> String {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    String::from_utf8(output.stderr).unwrap()
}

/// The arguments of the embedded IPADIC.
const IPADIC: [&str; 2] = ["--dict", "embedded://ipadic"];

/// The bunsetsu of a sentence as their surfaces, with their heads.
fn bunsetsu(sentence: &Sentence<LatticeMorpheme>) -> Vec<(String, Option<usize>)> {
    sentence
        .bunsetsu
        .iter()
        .map(|one| {
            let surface = sentence.morphemes[one.morphemes.clone()]
                .iter()
                .map(|morpheme| morpheme.surface())
                .collect();
            (surface, one.head)
        })
        .collect()
}

#[test]
fn parses_text_into_the_lattice_format() {
    let (_directory, model) = directory();
    let text = "彼は本を読んだ。\n彼は本を読んだ人を見た。\n";
    let output = success(run(&IPADIC, &[("--model", &model)], text));

    // One sentence per line, read back by the reader of the crate.
    let sentences = lattice::parse(&output).unwrap();
    assert_eq!(sentences.len(), 2);
    let named = |surface: &str, head| (surface.to_string(), head);
    assert_eq!(
        bunsetsu(&sentences[0]),
        [
            named("彼は", Some(2)),
            named("本を", Some(2)),
            named("読んだ。", None)
        ]
    );
    // 本を depends on the next bunsetsu, as the parser of the model has it.
    assert_eq!(
        bunsetsu(&sentences[1]),
        [
            named("彼は", Some(4)),
            named("本を", Some(2)),
            named("読んだ", Some(4)),
            named("人を", Some(4)),
            named("見た。", None)
        ]
    );
    for sentence in &sentences {
        assert!(sentence.violations().is_empty());
        assert!(sentence.comments.is_empty());
    }

    // What `cabocha -f1` writes: the head, the positions of the head word
    // and of the function word, the score with six decimals; then the
    // morphemes with the features of the dictionary.
    assert!(
        output.starts_with(
            "* 0 2D 0/1 0.000000\n\
             彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ\n\
             は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n\
             * 1 2D 0/1 0.000000\n\
             本\t名詞,一般,*,*,*,*,本,ホン,ホン\n"
        ),
        "{output}"
    );
    let second: Vec<&str> = output
        .lines()
        .skip_while(|line| *line != "EOS")
        .filter(|line| line.starts_with('*'))
        .collect();
    assert_eq!(
        second,
        [
            "* 0 4D 0/1 0.000000",
            "* 1 2D 0/1 1.000000",
            "* 2 4D 0/1 0.000000",
            "* 3 4D 0/1 0.000000",
            "* 4 -1D 0/1 0.000000"
        ]
    );
    assert_eq!(output.matches("\nEOS\n").count(), 2);
}

#[test]
fn every_line_gives_a_sentence() {
    let (_directory, model) = directory();
    // An empty line, one morpheme, symbols only, spaces, and a line that
    // ends in a carriage return.
    let text = "\n本\n。「」\n彼は 本を\n彼は本を読んだ。\r\n";
    let output = success(run(&IPADIC, &[("--model", &model)], text));
    let sentences = lattice::parse(&output).unwrap();
    assert_eq!(sentences.len(), 5);

    // An empty line: a sentence without a morpheme, `EOS` alone.
    assert!(output.starts_with("EOS\n"));
    assert!(sentences[0].morphemes.is_empty());
    assert!(sentences[0].bunsetsu.is_empty());

    assert_eq!(bunsetsu(&sentences[1]), [("本".to_string(), None)]);
    assert_eq!(sentences[2].morphemes.len(), 3);
    assert_eq!(sentences[2].bunsetsu.last().unwrap().head, None);
    // The tokenizer gives no morpheme for a space, as MeCab does not: the
    // space is not in the output.
    let surfaces: String = sentences[3]
        .morphemes
        .iter()
        .map(|morpheme| morpheme.surface())
        .collect();
    assert_eq!(surfaces, "彼は本を");
    // The carriage return is not part of the sentence.
    assert_eq!(bunsetsu(&sentences[4]).last().unwrap().0, "読んだ。");
    assert!(!output.contains('\r'));
    for sentence in &sentences {
        assert!(sentence.violations().is_empty(), "{sentence:?}");
    }

    // The last line without a line feed is a sentence as well.
    let output = success(run(&IPADIC, &[("--model", &model)], "彼は本を読んだ。"));
    assert_eq!(lattice::parse(&output).unwrap().len(), 1);
    // A line of spaces has no morpheme, as an empty line.
    assert_eq!(
        success(run(&IPADIC, &[("--model", &model)], " \n")),
        "EOS\n"
    );
    // No input: no output.
    assert_eq!(success(run(&IPADIC, &[("--model", &model)], "")), "");
}

#[test]
fn a_very_long_line_is_parsed() {
    let (_directory, model) = directory();
    // 3,000 bunsetsu of two morphemes each, and a verb.
    let text = format!("{}読む\n", "本を".repeat(3000));
    let output = success(run(&IPADIC, &[("--model", &model)], &text));
    let sentences = lattice::parse(&output).unwrap();
    assert_eq!(sentences.len(), 1);
    let sentence = &sentences[0];
    assert_eq!(sentence.morphemes.len(), 6001);
    assert_eq!(sentence.bunsetsu.len(), 3001);
    assert!(sentence.violations().is_empty());
    // Each 本を depends on the next bunsetsu.
    for (index, one) in sentence.bunsetsu.iter().enumerate().take(3000) {
        assert_eq!(one.head, Some(index + 1));
    }
}

#[test]
fn the_three_input_layers_give_the_same_dependencies() {
    let (directory, model) = directory();
    let text = "彼は本を読んだ人を見た。\n\n本を読む。\n";
    let from_text = success(run(&IPADIC, &[("--model", &model)], text));

    // The output as input that is tokenized: its bunsetsu are not read.
    let from_morphemes = success(run(
        &["--input-layer", "morphemes"],
        &[("--model", &model)],
        &from_text,
    ));
    assert_eq!(from_morphemes, from_text);
    // Morphemes alone, as a tokenizer writes them, with a comment.
    let morphemes: String = from_text
        .lines()
        .filter(|line| !line.starts_with('*'))
        .map(|line| format!("{line}\n"))
        .collect();
    let commented = format!("# the first sentence\n{morphemes}");
    let from_plain = success(run(
        &["-I", "morphemes"],
        &[("--model", &model)],
        &commented,
    ));
    assert_eq!(from_plain, format!("# the first sentence\n{from_text}"));

    // The output as input that is segmented: the bunsetsu of the input are
    // parsed, also other ones than the chunker gives.
    let from_bunsetsu = success(run(
        &["--input-layer", "bunsetsu"],
        &[("--model", &model)],
        &from_text,
    ));
    assert_eq!(from_bunsetsu, from_text);
    let merged = "* 0 -1D\n本\t名詞,一般,*,*,*,*,本,ホン,ホン\nを\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n\
                  読む\t動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム\nEOS\n";
    let output = success(run(
        &["--input-layer", "bunsetsu"],
        &[("--model", &model)],
        merged,
    ));
    assert_eq!(
        bunsetsu(&lattice::parse(&output).unwrap()[0]),
        [("本を読む".to_string(), None)]
    );
    // The chunker of the model is not needed for it.
    let parser_only = directory.path().join("parser.model");
    fs::write(&parser_only, model_bytes("ipadic", false, true)).unwrap();
    let output = success(run(
        &["--input-layer", "bunsetsu"],
        &[("--model", &parser_only)],
        &from_text,
    ));
    assert_eq!(output, from_text);
}

#[test]
fn reads_and_writes_files_as_well() {
    let (directory, model) = directory();
    let text = "彼は本を読んだ。\n本を読む。\n";
    let piped = success(run(&IPADIC, &[("--model", &model)], text));

    let input_path = directory.path().join("text.txt");
    let output_path = directory.path().join("output.lattice");
    fs::write(&input_path, text).unwrap();
    let printed = success(run(
        &IPADIC,
        &[
            ("--model", &model),
            ("--input", &input_path),
            ("--output", &output_path),
        ],
        "",
    ));
    assert_eq!(printed, "");
    assert_eq!(fs::read_to_string(&output_path).unwrap(), piped);
    // A file in, the standard output out, with the short options.
    let printed = success(run(
        &["-d", "embedded://ipadic"],
        &[("-m", &model), ("-i", &input_path)],
        "",
    ));
    assert_eq!(printed, piped);
}

#[test]
fn a_model_of_another_dictionary_is_refused() {
    let (directory, _model) = directory();
    let unidic = directory.path().join("unidic.model");
    fs::write(&unidic, model_bytes("unidic", true, true)).unwrap();
    let error = failure(run(&IPADIC, &[("--model", &unidic)], "彼は本を読んだ。\n"));
    assert_eq!(
        error,
        "Error: embedded://ipadic: the model is for the dictionary \"unidic\", and the morphemes are of \"ipadic\"\n"
    );
}

#[test]
fn fails_with_the_cause() {
    let (directory, model) = directory();
    let text = "彼は本を読んだ。\n";

    // Text needs a dictionary, and a lattice input has no use for one.
    let error = failure(run(&[], &[("--model", &model)], text));
    assert!(
        error.starts_with("Error: --dict is needed to tokenize text"),
        "{error}"
    );
    let mut with_layer = vec!["--input-layer", "morphemes"];
    with_layer.extend_from_slice(&IPADIC);
    let error = failure(run(&with_layer, &[("--model", &model)], text));
    assert!(
        error.starts_with("Error: --dict is for the input layer text"),
        "{error}"
    );
    let error = failure(run(
        &["--dict", "/no/such/dictionary"],
        &[("--model", &model)],
        text,
    ));
    assert!(
        error.starts_with("Error: cannot load the dictionary /no/such/dictionary: "),
        "{error}"
    );

    // A model that is missing, that is no model, or without the part that
    // is needed.
    let missing = directory.path().join("missing.model");
    let error = failure(run(&IPADIC, &[("--model", &missing)], text));
    assert!(error.starts_with("Error: cannot read "), "{error}");
    let broken = directory.path().join("broken.model");
    fs::write(&broken, b"not a model").unwrap();
    let error = failure(run(&IPADIC, &[("--model", &broken)], text));
    assert!(
        error.contains("broken.model: not a model file: the magic number is missing"),
        "{error}"
    );
    let chunker_only = directory.path().join("chunker.model");
    fs::write(&chunker_only, model_bytes("ipadic", true, false)).unwrap();
    let error = failure(run(&IPADIC, &[("--model", &chunker_only)], text));
    assert!(
        error.contains("chunker.model: the model has no parser"),
        "{error}"
    );
    let parser_only = directory.path().join("parser.model");
    fs::write(&parser_only, model_bytes("ipadic", false, true)).unwrap();
    for layer in [vec![], vec!["--input-layer", "morphemes"]] {
        let mut arguments = layer.clone();
        if layer.is_empty() {
            arguments.extend_from_slice(&IPADIC);
        }
        let error = failure(run(&arguments, &[("--model", &parser_only)], text));
        assert!(
            error.contains("parser.model: the model has no chunker"),
            "{error}"
        );
    }

    // An input that cannot be opened, and one that is not in the lattice
    // format: its line is named.
    let error = failure(run(
        &IPADIC,
        &[("--model", &model), ("--input", &missing)],
        "",
    ));
    assert!(error.starts_with("Error: cannot open "), "{error}");
    let error = failure(run(
        &["--input-layer", "morphemes"],
        &[("--model", &model)],
        "彼 名詞\nEOS\n",
    ));
    assert!(
        error.starts_with("Error: the standard input: line 1: "),
        "{error}"
    );

    // Morphemes that are not of the dictionary of the model: the sentence
    // is named, and the one before it was written.
    let narrow = "彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ\nEOS\n彼\t名詞,代名詞\nEOS\n";
    let output = run(
        &["--input-layer", "morphemes"],
        &[("--model", &model)],
        narrow,
    );
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.starts_with(
            "Error: the standard input: sentence 2: the morpheme 1 (\"彼\") has fewer than the 7 feature columns"
        ),
        "{error}"
    );
    let written = String::from_utf8(output.stdout).unwrap();
    assert_eq!(lattice::parse(&written).unwrap().len(), 1);
}

/// The build for a user has no training code: no subcommand that trains.
#[cfg(not(feature = "train"))]
#[test]
fn the_command_of_a_user_does_not_train() {
    for subcommand in [
        "train-chunker",
        "train-parser",
        "convert",
        "evaluate",
        "split",
        "instances",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"))
            .arg(subcommand)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains(&format!("unrecognized subcommand '{subcommand}'")),
            "{error}"
        );
    }
    let help = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("parse") && help.contains("chunk"), "{help}");
}

/// With the training code, the output of `parse` goes to `evaluate`.
#[cfg(feature = "train")]
#[test]
fn the_output_goes_to_evaluate() {
    let (directory, model) = directory();
    let text = "彼は本を読んだ人を見た。\n\n本を読む。\n";
    let output_path = directory.path().join("output.lattice");
    let input_path = directory.path().join("text.txt");
    fs::write(&input_path, text).unwrap();
    success(run(
        &IPADIC,
        &[
            ("--model", &model),
            ("--input", &input_path),
            ("--output", &output_path),
        ],
        "",
    ));
    // Against itself: every bunsetsu and every head is right.
    let evaluation = Command::new(env!("CARGO_BIN_EXE_lindera-dependency"))
        .arg("evaluate")
        .arg("--gold")
        .arg(&output_path)
        .arg("--system")
        .arg(&output_path)
        .output()
        .unwrap();
    let report = String::from_utf8(evaluation.stdout).unwrap();
    assert!(evaluation.status.success(), "{report}");
    assert!(report.contains("sentences: 3\n"), "{report}");
    assert!(report.contains("chunking F: 100.00%"), "{report}");
    assert!(
        report.contains("dependency level 1: 100.00% (5/5)"),
        "{report}"
    );
}

/// The model file that the builds of the tests embed: the test jobs name it
/// in `LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`. It is the model of the
/// tests of lindera-analysis, in which every bunsetsu depends on the next.
#[cfg(feature = "embed-model-ipadic")]
const EMBEDDED_MODEL_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../resources/dependency/test_model_ipadic.bin"
);

#[cfg(feature = "embed-model-ipadic")]
#[test]
fn the_embedded_model_parses_as_its_file_does() {
    let text = "彼は本を読んだ。\n今日は雨だ。\n";
    let mut embedded = vec!["--model", "embedded://ipadic"];
    embedded.extend_from_slice(&IPADIC);
    let from_the_build = success(run(&embedded, &[], text));
    let from_the_file = success(run(
        &IPADIC,
        &[("--model", Path::new(EMBEDDED_MODEL_FILE))],
        text,
    ));
    assert!(from_the_build.starts_with("* 0 1D "), "{from_the_build}");
    assert_eq!(from_the_build, from_the_file);
}

#[cfg(not(feature = "embed-model-ipadic"))]
#[test]
fn a_build_without_the_embedded_model_names_the_feature() {
    let mut embedded = vec!["--model", "embedded://ipadic"];
    embedded.extend_from_slice(&IPADIC);
    let error = failure(run(&embedded, &[], "彼は本を読んだ。\n"));
    assert!(
        error.starts_with(
            "Error: embedded://ipadic: the model for ipadic is not embedded in this build"
        ),
        "{error}"
    );
    assert!(error.contains("`embed-model-ipadic`"), "{error}");
}

#[test]
fn a_dictionary_without_an_embedded_model_is_refused() {
    let mut embedded = vec!["--model", "embedded://unidic"];
    embedded.extend_from_slice(&IPADIC);
    let error = failure(run(&embedded, &[], "彼は本を読んだ。\n"));
    assert!(
        error.starts_with(
            "Error: embedded://unidic: no model can be embedded for the dictionary unidic"
        ),
        "{error}"
    );
}
