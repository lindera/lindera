//! The `lindera dependency train-parser` and `parse` commands. They need no
//! dictionary.

#![cfg(all(feature = "train", feature = "dependency"))]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use lindera_dependency::lattice;
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::parser::{FeatureSet, Quantization};
use lindera_dependency::{Morpheme, Schema};

/// The `lindera dependency` command.
///
/// # Returns
///
/// The command, with `dependency` as its first argument.
fn dependency() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lindera"));
    command.arg("dependency");
    command
}

/// The feature string of a word of the training data.
fn features_of(surface: &str) -> &'static str {
    match surface {
        "彼" => "名詞,代名詞,一般,*,*,*,彼,カレ,カレ",
        "犬" => "名詞,一般,*,*,*,*,犬,イヌ,イヌ",
        "本" => "名詞,一般,*,*,*,*,本,ホン,ホン",
        "水" => "名詞,一般,*,*,*,*,水,ミズ,ミズ",
        "花" => "名詞,一般,*,*,*,*,花,ハナ,ハナ",
        "は" => "助詞,係助詞,*,*,*,*,は,ハ,ワ",
        "が" => "助詞,格助詞,一般,*,*,*,が,ガ,ガ",
        "を" => "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
        "に" => "助詞,副詞化,*,*,*,*,に,ニ,ニ",
        "静か" => "名詞,形容動詞語幹,*,*,*,*,静か,シズカ,シズカ",
        "赤い" => "形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ",
        "読ん" => "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン",
        "飲ん" => "動詞,自立,*,*,五段・マ行,連用タ接続,飲む,ノン,ノン",
        "見" => "動詞,自立,*,*,一段,連用形,見る,ミ,ミ",
        "だ" => "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ",
        "た" => "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ",
        other => panic!("no features for {other}"),
    }
}

/// Sentences in the lattice format: each bunsetsu as its words with spaces,
/// and the number of its head, -1 for the root.
fn lattice_text(sentences: &[&[(&str, i32)]]) -> String {
    let mut text = String::new();
    for sentence in sentences {
        for (index, (words, head)) in sentence.iter().enumerate() {
            text.push_str(&format!("* {index} {head}D\n"));
            for word in words.split(' ') {
                text.push_str(&format!("{word}\t{}\n", features_of(word)));
            }
        }
        text.push_str("EOS\n");
    }
    text
}

/// Six sentences with the head of every bunsetsu.
fn training_data() -> String {
    lattice_text(&[
        &[("彼 は", 2), ("本 を", 2), ("読ん だ", -1)],
        &[("彼 は", 3), ("静か に", 3), ("本 を", 3), ("読ん だ", -1)],
        &[("犬 が", 2), ("水 を", 2), ("飲ん だ", -1)],
        &[("彼 は", 3), ("赤い", 2), ("花 を", 3), ("見 た", -1)],
        &[("犬 が", 3), ("赤い", 2), ("本 を", 3), ("読ん だ", -1)],
        &[
            ("彼 は", 4),
            ("犬 が", 2),
            ("飲ん だ", 3),
            ("水 を", 4),
            ("見 た", -1),
        ],
    ])
}

/// Runs the command, for a minute at most: a training that does not end is
/// a failure, not a test that hangs.
fn run(arguments: &[&str], paths: &[(&str, &Path)]) -> Output {
    let mut command = dependency();
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

/// The heads of the sentences of a file.
fn heads(text: &str) -> Vec<Vec<Option<usize>>> {
    lattice::parse(text)
        .unwrap()
        .iter()
        .map(|sentence| sentence.bunsetsu.iter().map(|one| one.head).collect())
        .collect()
}

/// The value of a line `key: value` of a summary.
fn value<'a>(summary: &'a str, key: &str) -> &'a str {
    summary
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
        .unwrap_or_else(|| panic!("no line {key} in {summary}"))
}

/// A cost that lets a few decisions be learned.
const COST: [&str; 2] = ["--cost", "1"];

#[test]
fn trains_a_parser_and_parses_with_it() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("parser.model");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();

    let summary = success(run(
        &["train-parser", "--cost", "1"],
        &[("--input", &train_path), ("--output", &model_path)],
    ));
    let bytes = fs::read(&model_path).unwrap();
    let lines: Vec<&str> = summary.lines().collect();
    assert_eq!(lines.len(), 9, "{summary}");
    assert!(
        lines[0].ends_with("train.lattice: 6 sentences"),
        "{summary}"
    );
    assert_eq!(
        lines[1],
        "features: distance,source,target,left,right,children,gap,after"
    );
    assert_eq!(lines[2], "instances: 15");
    assert!(value(&summary, "passes").parse::<u32>().unwrap() > 0);
    assert!(value(&summary, "updates").parse::<u32>().unwrap() > 0);
    assert_eq!(value(&summary, "bytes"), bytes.len().to_string());
    assert!(value(&summary, "seconds").parse::<f64>().is_ok());

    // The file: for IPADIC, with a parser and no chunker, the defaults of
    // the command.
    let model = Model::new(bytes.as_slice()).unwrap();
    assert_eq!(model.dictionary(), "ipadic");
    assert_eq!(model.schema(), &Schema::ipadic());
    assert!(model.chunker().is_none());
    let parser = model.parser().unwrap();
    assert_eq!(parser.feature_set(), &FeatureSet::default());
    let part = parser.classifier();
    assert_eq!(part.quantization(), Quantization::I8);
    // Hashed pairs: a table of 2^22 weights.
    assert_eq!(part.pair_count(), 1 << 22);
    assert_eq!(
        value(&summary, "feature strings"),
        part.feature_count().to_string()
    );

    assert_eq!(
        success(run(
            &["parse", "--input-layer", "bunsetsu"],
            &[
                ("--model", &model_path),
                ("--input", &train_path),
                ("--output", &output_path),
            ],
        )),
        ""
    );
    let output = fs::read_to_string(&output_path).unwrap();
    // The training data is parsed as its answer has it.
    assert_eq!(heads(&output), heads(&training_data()));

    // The command parses as the library does with the same file, and
    // writes the head word, the function word and the score.
    let answers = lattice::parse(&training_data()).unwrap();
    for (answer, written) in answers.iter().zip(lattice::parse(&output).unwrap()) {
        let ranges: Vec<_> = answer
            .bunsetsu
            .iter()
            .map(|one| one.morphemes.clone())
            .collect();
        let parsed = parser.parse(&answer.morphemes, &ranges).unwrap();
        for (parsed, written) in parsed.iter().zip(&written.bunsetsu) {
            assert_eq!(written.head, parsed.head);
            assert_eq!(written.positions, parsed.positions);
            let (score, expected) = (written.score.unwrap(), parsed.score.unwrap());
            // Six decimals are written.
            assert!((score - expected).abs() < 1e-6, "{score} {expected}");
        }
        assert_eq!(written.morphemes.len(), answer.morphemes.len());
        assert_eq!(
            written.morphemes[0].surface(),
            answer.morphemes[0].surface()
        );
    }
    assert!(output.starts_with("* 0 2D 0/1 "), "{output}");
}

#[test]
fn trains_the_whole_model_of_a_dictionary() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let chunker_path = directory.path().join("chunker.model");
    let model_path = directory.path().join("ipadic.model");
    let parser_path = directory.path().join("parser.model");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();
    let train: [(&str, &Path); 1] = [("--input", &train_path)];

    success(run(
        &[
            "train-chunker",
            "--regularization",
            "l2",
            "--lambda",
            "0.01",
            "--threads",
            "1",
        ],
        &[train[0], ("--output", &chunker_path)],
    ));
    success(run(
        &["train-parser", "--cost", "1"],
        &[
            train[0],
            ("--chunker", &chunker_path),
            ("--output", &model_path),
        ],
    ));
    success(run(
        &["train-parser", "--cost", "1"],
        &[train[0], ("--output", &parser_path)],
    ));

    // One file with the chunker of the first and the parser.
    let model = Model::new(fs::read(&model_path).unwrap()).unwrap();
    let chunker_file = Model::new(fs::read(&chunker_path).unwrap()).unwrap();
    assert!(model.chunker().is_some() && model.parser().is_some());
    assert_eq!(
        model.chunker().unwrap().model().feature_count(),
        chunker_file.chunker().unwrap().model().feature_count()
    );
    assert!(chunker_file.parser().is_none());

    // From the morphemes: the bunsetsu of the input are not read. The
    // input has every sentence as one bunsetsu.
    let flat: String = lattice::parse(&training_data())
        .unwrap()
        .iter()
        .map(|sentence| {
            let mut text = "* 0 -1D\n".to_string();
            for morpheme in &sentence.morphemes {
                text.push_str(&format!(
                    "{}\t{}\n",
                    morpheme.surface(),
                    morpheme.features()
                ));
            }
            text + "EOS\n"
        })
        .collect();
    let flat_path = directory.path().join("flat.lattice");
    fs::write(&flat_path, flat).unwrap();
    success(run(
        &["parse", "--input-layer", "morphemes"],
        &[
            ("--model", &model_path),
            ("--input", &flat_path),
            ("--output", &output_path),
        ],
    ));
    let output = fs::read_to_string(&output_path).unwrap();
    assert_eq!(heads(&output), heads(&training_data()));
    // Without --chunk the one bunsetsu of each sentence is its root.
    success(run(
        &["parse", "--input-layer", "bunsetsu"],
        &[
            ("--model", &model_path),
            ("--input", &flat_path),
            ("--output", &output_path),
        ],
    ));
    let output = fs::read_to_string(&output_path).unwrap();
    assert!(heads(&output).iter().all(|heads| heads == &[None]));

    // A file without the part that is needed.
    fs::remove_file(&output_path).unwrap();
    let error = failure(run(
        &["parse", "--input-layer", "morphemes"],
        &[
            ("--model", &parser_path),
            ("--input", &flat_path),
            ("--output", &output_path),
        ],
    ));
    assert!(
        error.contains("parser.model: the model has no chunker"),
        "{error}"
    );
    let error = failure(run(
        &["parse", "--input-layer", "bunsetsu"],
        &[
            ("--model", &chunker_path),
            ("--input", &flat_path),
            ("--output", &output_path),
        ],
    ));
    assert!(
        error.contains("chunker.model: the model has no parser"),
        "{error}"
    );
    assert!(!output_path.exists());
}

#[test]
fn trains_with_the_settings_asked_for() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("parser.model");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();
    let paths: [(&str, &Path); 2] = [("--input", &train_path), ("--output", &model_path)];
    let train = |arguments: &[&str]| {
        let mut all = vec!["train-parser"];
        all.extend_from_slice(arguments);
        let summary = success(run(&all, &paths));
        (summary, fs::read(&model_path).unwrap())
    };

    // Exact pairs, as they are and as one byte each.
    let (summary, exact) = train(&["--cost", "1", "--pairs", "exact", "--quantization", "f32"]);
    let pairs: usize = value(&summary, "pair weights").parse().unwrap();
    let model = Model::new(exact.as_slice()).unwrap();
    let part = *model.parser().unwrap().classifier();
    assert_eq!(part.quantization(), Quantization::F32);
    assert_eq!(part.pair_count(), pairs);
    assert!(pairs > 0 && pairs < 1 << 22);
    let (_, small) = train(&["--cost", "1", "--pairs", "exact", "--quantization", "i8"]);
    assert!(small.len() < exact.len());
    let (_, middle) = train(&["--cost", "1", "--pairs", "exact", "--quantization", "i16"]);
    assert!(small.len() < middle.len() && middle.len() < exact.len());

    // Fewer pairs with a support, with a threshold, and fewer features
    // with a count.
    let (summary, _) = train(&["--cost", "1", "--pairs", "exact", "--min-support", "3"]);
    assert!(value(&summary, "pair weights").parse::<usize>().unwrap() < pairs);
    let (summary, _) = train(&[
        "--cost",
        "1",
        "--pairs",
        "exact",
        "--pair-threshold",
        "0.001",
    ]);
    assert!(value(&summary, "pair weights").parse::<usize>().unwrap() < pairs);
    let features: usize = value(&summary, "feature strings").parse().unwrap();
    let (summary, _) = train(&["--cost", "1", "--pairs", "exact", "--min-count", "3"]);
    assert!(value(&summary, "feature strings").parse::<usize>().unwrap() < features);

    // A table of another size.
    let (_, hashed) = train(&["--cost", "1", "--bits", "10"]);
    let model = Model::new(hashed.as_slice()).unwrap();
    assert_eq!(model.parser().unwrap().classifier().pair_count(), 1 << 10);

    // Another feature set.
    let (summary, bytes) = train(&["--cost", "1", "--features", "source, target,distance"]);
    assert_eq!(value(&summary, "features"), "distance,source,target");
    let model = Model::new(bytes.as_slice()).unwrap();
    assert_eq!(
        model.parser().unwrap().feature_set().to_string(),
        "distance,source,target"
    );

    // Passive-aggressive, with its passes, and averaged.
    let (summary, last) = train(&["--algorithm", "pa", "--iterations", "50"]);
    assert_eq!(value(&summary, "passes"), "50");
    let (summary, averaged) = train(&["--algorithm", "pa", "--iterations", "50", "--averaged"]);
    assert_eq!(value(&summary, "passes"), "50");
    assert_ne!(averaged, last);
    fs::write(&model_path, &last).unwrap();
    success(run(
        &["parse", "--input-layer", "bunsetsu"],
        &[
            ("--model", &model_path),
            ("--input", &train_path),
            ("--output", &output_path),
        ],
    ));
    assert_eq!(
        heads(&fs::read_to_string(&output_path).unwrap()),
        heads(&training_data())
    );

    // The same settings give the same bytes; another seed is accepted.
    let (_, again) = train(&COST);
    let (_, same) = train(&COST);
    assert_eq!(again, same);
    train(&["--cost", "1", "--seed", "7"]);
}

#[test]
fn fails_with_the_cause() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("parser.model");
    let other_path = directory.path().join("other.model");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();
    let train: [(&str, &Path); 2] = [("--input", &train_path), ("--output", &model_path)];

    let error = failure(run(&["train-parser", "--features", "source,head"], &train));
    assert!(
        error.contains(
            "Error: LinderaError { kind: Content, source: \"head\" is not a group of features"
        ),
        "{error}"
    );
    let error = failure(run(&["train-parser", "--cost", "0"], &train));
    assert!(
        error.contains("train.lattice: the cost 0 is not a number above 0"),
        "{error}"
    );
    let error = failure(run(&["train-parser", "--bits", "31"], &train));
    assert!(
        error.contains("train.lattice: the table of the pairs has 2^31 weights"),
        "{error}"
    );
    let error = failure(run(
        &["train-parser", "--algorithm", "pa", "--iterations", "0"],
        &train,
    ));
    assert!(
        error.contains("train.lattice: passive-aggressive needs an iteration"),
        "{error}"
    );
    assert!(!model_path.exists());

    // The file the chunker is taken from: missing, no model, without a
    // chunker.
    let with_chunker: [(&str, &Path); 3] = [train[0], train[1], ("--chunker", &other_path)];
    let error = failure(run(&["train-parser"], &with_chunker));
    assert!(
        error.contains("Error: LinderaError { kind: Content, source: cannot read"),
        "{error}"
    );
    fs::write(&other_path, b"not a model").unwrap();
    let error = failure(run(&["train-parser"], &with_chunker));
    assert!(
        error.contains("other.model: not a model file: the magic number is missing"),
        "{error}"
    );
    success(run(
        &["train-parser", "--cost", "1"],
        &[train[0], ("--output", &other_path)],
    ));
    let error = failure(run(&["train-parser"], &with_chunker));
    assert!(
        error.contains("other.model: the model has no chunker"),
        "{error}"
    );
    // A model file of another dictionary.
    let unidic = ModelBuilder::new("unidic", Schema::ipadic())
        .unwrap()
        .to_bytes();
    fs::write(&other_path, unidic).unwrap();
    let error = failure(run(&["train-parser"], &with_chunker));
    assert!(
        error.contains(
            "other.model: the model is for the dictionary \"unidic\", and the morphemes are of \"ipadic\""
        ),
        "{error}"
    );
    assert!(!model_path.exists());
    success(run(
        &["train-parser", "--cost", "1"],
        &[train[0], ("--output", &other_path)],
    ));

    // Training data whose arcs cross: the sentence is named.
    let crossing_path = directory.path().join("crossing.lattice");
    let crossing = lattice_text(&[
        &[("彼 は", 2), ("本 を", 2), ("読ん だ", -1)],
        &[("彼 は", 2), ("赤い", 3), ("花 を", 3), ("見 た", -1)],
    ]);
    fs::write(&crossing_path, crossing).unwrap();
    let error = failure(run(
        &["train-parser"],
        &[("--input", &crossing_path), ("--output", &model_path)],
    ));
    assert!(
        error.contains(
            "crossing.lattice: sentence 2: invalid sentence: the parser cannot make the tree"
        ),
        "{error}"
    );
    assert!(!model_path.exists());

    // A model that is not one, and an input that is not of the dictionary
    // of the model: nothing is written.
    let parse: [(&str, &Path); 3] = [
        ("--model", &model_path),
        ("--input", &train_path),
        ("--output", &output_path),
    ];
    let error = failure(run(&["parse", "--input-layer", "bunsetsu"], &parse));
    assert!(
        error.contains("Error: LinderaError { kind: Content, source: cannot read"),
        "{error}"
    );
    fs::write(&model_path, b"not a model").unwrap();
    let error = failure(run(&["parse", "--input-layer", "bunsetsu"], &parse));
    assert!(
        error.contains("parser.model: not a model file: the magic number is missing"),
        "{error}"
    );
    let narrow_path = directory.path().join("narrow.lattice");
    let narrow = format!(
        "{}* 0 -1D\n彼\t名詞,代名詞\nEOS\n",
        lattice_text(&[&[("彼 は", 1), ("読ん だ", -1)]])
    );
    fs::write(&narrow_path, narrow).unwrap();
    let error = failure(run(
        &["parse", "--input-layer", "bunsetsu"],
        &[
            ("--model", &other_path),
            ("--input", &narrow_path),
            ("--output", &output_path),
        ],
    ));
    assert!(
        error.contains(
            "narrow.lattice: sentence 2: the morpheme 1 (\"彼\") has fewer than the 7 feature columns the schema reads"
        ),
        "{error}"
    );
    // The sentences are written as they are parsed: the one before the
    // sentence that failed is there, and nothing after it.
    let written = fs::read_to_string(&output_path).unwrap();
    assert_eq!(heads(&written), [[Some(1), None]]);
}

#[test]
fn trains_for_the_dictionary_named() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let chunker_path = directory.path().join("chunker.model");
    let model_path = directory.path().join("unidic-cwj.model");
    let default_path = directory.path().join("default.model");
    let instances_path = directory.path().join("instances.tsv");
    fs::write(&train_path, training_data()).unwrap();
    let train: [(&str, &Path); 1] = [("--input", &train_path)];
    let chunker_arguments = [
        "train-chunker",
        "--regularization",
        "l2",
        "--lambda",
        "0.01",
        "--threads",
        "1",
    ];
    let named = ["--dictionary-name", "unidic-cwj"];

    // The name and the schema of the dictionary go into the model, of the
    // chunker and of the parser.
    success(run(
        &[&chunker_arguments[..], &named[..]].concat(),
        &[train[0], ("--output", &chunker_path)],
    ));
    let model = Model::new(fs::read(&chunker_path).unwrap()).unwrap();
    assert_eq!(model.dictionary(), "unidic-cwj");
    assert_eq!(model.schema(), &Schema::unidic());
    success(run(
        &[&["train-parser", "--cost", "1"][..], &named[..]].concat(),
        &[
            train[0],
            ("--chunker", &chunker_path),
            ("--output", &model_path),
        ],
    ));
    let model = Model::new(fs::read(&model_path).unwrap()).unwrap();
    assert_eq!(model.dictionary(), "unidic-cwj");
    assert_eq!(model.schema(), &Schema::unidic());
    assert!(model.chunker().is_some() && model.parser().is_some());
    success(run(
        &[&["instances"][..], &named[..]].concat(),
        &[train[0], ("--output", &instances_path)],
    ));

    // Without the option the model is for IPADIC, as before.
    success(run(
        &chunker_arguments,
        &[train[0], ("--output", &default_path)],
    ));
    let model = Model::new(fs::read(&default_path).unwrap()).unwrap();
    assert_eq!(model.dictionary(), "ipadic");
    assert_eq!(model.schema(), &Schema::ipadic());

    // The chunker of another dictionary is refused: of UniDic CWJ for
    // IPADIC, and of UniDic CWJ for UniDic, which has the same schema.
    let unused_path = directory.path().join("unused.model");
    for (arguments, name) in [
        (&["train-parser"][..], "ipadic"),
        (
            &["train-parser", "--dictionary-name", "unidic"][..],
            "unidic",
        ),
    ] {
        let error = failure(run(
            arguments,
            &[
                train[0],
                ("--chunker", &chunker_path),
                ("--output", &unused_path),
            ],
        ));
        assert!(
            error.contains(&format!(
                "chunker.model: the model is for the dictionary \"unidic-cwj\", and the morphemes are of \"{name}\""
            )),
            "{error}"
        );
    }

    // A dictionary without a schema: the ones that have one are listed.
    for command in ["train-chunker", "train-parser", "instances"] {
        let error = failure(run(
            &[command, "--dictionary-name", "ko-dic"],
            &[train[0], ("--output", &unused_path)],
        ));
        assert!(
            error.contains(
                "Error: LinderaError { kind: Content, source: there is no schema for the dictionary ko-dic; there are schemas for ipadic, unidic, unidic-cwj, unidic-csj"
            ),
            "{command}: {error}"
        );
    }
    assert!(!unused_path.exists());
}
