//! The `lindera dependency train-chunker` and `lindera dependency chunk`
//! commands. They need no dictionary.

#![cfg(all(feature = "train", feature = "dependency"))]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use lindera_dependency::chunker::{ChunkerData, Column, Template};
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::{Morpheme, Schema, lattice};

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

/// A sentence in the lattice format, of bunsetsu given as their morphemes,
/// each `surface/part of speech`, every bunsetsu depending on the last.
fn sentence(bunsetsu: &[&[&str]]) -> String {
    let mut text = String::new();
    let last = bunsetsu.len() as i32 - 1;
    for (index, morphemes) in bunsetsu.iter().enumerate() {
        let head = if index as i32 == last { -1 } else { last };
        text.push_str(&format!("* {index} {head}D \n"));
        for morpheme in *morphemes {
            let (surface, pos) = morpheme.split_once('/').unwrap();
            text.push_str(&format!("{surface}\t{pos},*,*,*,*,基本形,{surface},*,*\n"));
        }
    }
    text.push_str("EOS\n");
    text
}

/// Training data: nouns with particles and a verb.
fn training_data() -> String {
    [
        sentence(&[
            &["彼/名詞", "は/助詞"],
            &["本/名詞", "を/助詞"],
            &["読む/動詞"],
        ]),
        sentence(&[
            &["私/名詞", "が/助詞"],
            &["水/名詞", "を/助詞"],
            &["飲む/動詞"],
        ]),
        sentence(&[&["犬/名詞", "が/助詞"], &["走る/動詞"]]),
        sentence(&[
            &["猫/名詞", "は/助詞"],
            &["魚/名詞", "を/助詞"],
            &["食べる/動詞"],
        ]),
        sentence(&[&["本/名詞", "を/助詞"], &["読む/動詞"]]),
    ]
    .concat()
}

/// Runs the command.
fn run(arguments: &[&str], paths: &[(&str, &Path)]) -> Output {
    let mut command = dependency();
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
fn trains_a_model_and_chunks_with_it() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("chunker.model");
    let input_path = directory.path().join("input.lattice");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();

    let summary = success(run(
        &[
            "train-chunker",
            "--regularization",
            "l2",
            "--lambda",
            "0.01",
            "--threads",
            "1",
        ],
        &[("--input", &train_path), ("--output", &model_path)],
    ));
    let bytes = fs::read(&model_path).unwrap();
    let file = Model::new(bytes.as_slice()).unwrap();
    assert_eq!(file.dictionary(), "ipadic");
    assert_eq!(file.schema(), &Schema::ipadic());
    let chunker = file.chunker().unwrap();
    let model = chunker.model();
    assert_eq!(
        model.columns(),
        [
            Column::Surface,
            Column::PartOfSpeech,
            Column::ConjugationForm
        ]
    );
    assert_eq!(model.template().len(), 24);
    // After the progress of the optimizer, the summary names the input and
    // gives the size of the model.
    let lines: Vec<&str> = summary.lines().collect();
    assert!(lines.len() >= 5, "{summary}");
    let lines = &lines[lines.len() - 5..];
    assert!(
        lines[0].ends_with("train.lattice: 5 sentences"),
        "{summary}"
    );
    assert!(lines[1].starts_with("iterations: "), "{summary}");
    assert_eq!(
        lines[2..],
        [
            format!("feature strings: {}", model.feature_count()),
            format!("weights: {}", model.weight_count()),
            format!("bytes: {}", bytes.len()),
        ]
    );

    // An input with one bunsetsu per sentence, and a comment: the bunsetsu
    // of the input are not read.
    let input = "# a comment\n* 0 -1D \n犬\t名詞,*,*,*,*,基本形,犬,*,*\nは\t助詞,*,*,*,*,基本形,は,*,*\n\
                 水\t名詞,*,*,*,*,基本形,水,*,*\nを\t助詞,*,*,*,*,基本形,を,*,*\n\
                 飲む\t動詞,*,*,*,*,基本形,飲む,*,*\nEOS\n\
                 * 0 -1D \n読む\t動詞,*,*,*,*,基本形,読む,*,*\nEOS\n";
    fs::write(&input_path, input).unwrap();
    assert_eq!(
        success(run(
            &["chunk"],
            &[
                ("--model", &model_path),
                ("--input", &input_path),
                ("--output", &output_path),
            ],
        )),
        ""
    );
    let output = fs::read_to_string(&output_path).unwrap();
    assert_eq!(
        output,
        "# a comment\n* 0 1D \n犬\t名詞,*,*,*,*,基本形,犬,*,*\nは\t助詞,*,*,*,*,基本形,は,*,*\n\
         * 1 2D \n水\t名詞,*,*,*,*,基本形,水,*,*\nを\t助詞,*,*,*,*,基本形,を,*,*\n\
         * 2 -1D \n飲む\t動詞,*,*,*,*,基本形,飲む,*,*\nEOS\n\
         * 0 -1D \n読む\t動詞,*,*,*,*,基本形,読む,*,*\nEOS\n"
    );

    // The command chunks as the library does with the same model.
    for sentence in lattice::parse(&output).unwrap() {
        let expected: Vec<_> = sentence
            .bunsetsu
            .iter()
            .map(|bunsetsu| bunsetsu.morphemes.clone())
            .collect();
        assert_eq!(chunker.chunk(&sentence.morphemes).unwrap(), expected);
        assert!(sentence.violations().is_empty());
        assert!(!sentence.morphemes[0].surface().is_empty());
    }
}

#[test]
fn the_output_of_chunk_goes_to_evaluate() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("chunker.model");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();
    success(run(
        &[
            "train-chunker",
            "--regularization",
            "l2",
            "--lambda",
            "0.01",
        ],
        &[("--input", &train_path), ("--output", &model_path)],
    ));
    success(run(
        &["chunk"],
        &[
            ("--model", &model_path),
            ("--input", &train_path),
            ("--output", &output_path),
        ],
    ));

    // The training data is chunked as it was given.
    let result = run(
        &["evaluate"],
        &[("--gold", &train_path), ("--system", &output_path)],
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(result.status.success());
    assert!(
        text.contains("chunking precision: 100.00% (13/13)\nchunking recall: 100.00% (13/13)\n"),
        "{text}"
    );
}

#[test]
fn trains_with_the_settings_of_the_dictionary() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("chunker.model");
    fs::write(&train_path, training_data()).unwrap();
    let paths: [(&str, &Path); 2] = [("--input", &train_path), ("--output", &model_path)];
    // The regularization is given, for the few sentences; the columns and
    // the template are left to the dictionary.
    let arguments = [
        "train-chunker",
        "--regularization",
        "l2",
        "--lambda",
        "0.01",
    ];
    let trained = |more: &[&str]| {
        success(run(&[&arguments[..], more].concat(), &paths));
        let file = Model::new(fs::read(&model_path).unwrap()).unwrap();
        let chunker = file.chunker().unwrap();
        let model = chunker.model();
        (model.columns().to_vec(), model.template().clone())
    };

    // IPADIC, named or not: three columns and the default template.
    let ipadic = [
        Column::Surface,
        Column::PartOfSpeech,
        Column::ConjugationForm,
    ];
    assert_eq!(trained(&[]), (ipadic.to_vec(), Template::default()));
    assert_eq!(
        trained(&["--dictionary-name", "ipadic"]),
        (ipadic.to_vec(), Template::default())
    );

    // UniDic: the base form too, and its template.
    for name in ["unidic", "unidic-cwj", "unidic-csj"] {
        let (columns, template) = trained(&["--dictionary-name", name]);
        assert_eq!(
            columns,
            [
                Column::Surface,
                Column::PartOfSpeech,
                Column::ConjugationForm,
                Column::BaseForm
            ],
            "{name}"
        );
        assert_eq!(template, Template::unidic(), "{name}");
    }

    // An option still decides: the columns of the template of CaboCha.
    let template_path = directory.path().join("chunker.template");
    fs::write(&template_path, Template::cabocha().to_string()).unwrap();
    success(run(
        &[
            &arguments[..],
            &[
                "--dictionary-name",
                "unidic-cwj",
                "--columns",
                "surface,pos",
            ],
        ]
        .concat(),
        &[paths[0], paths[1], ("--template", &template_path)],
    ));
    let file = Model::new(fs::read(&model_path).unwrap()).unwrap();
    let chunker = file.chunker().unwrap();
    let model = chunker.model();
    assert_eq!(model.columns(), [Column::Surface, Column::PartOfSpeech]);
    assert_eq!(model.template(), &Template::cabocha());
}

#[test]
fn trains_with_other_columns_and_a_template_file() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("chunker.model");
    let template_path = directory.path().join("chunker.template");
    fs::write(&train_path, training_data()).unwrap();
    fs::write(&template_path, "U00:%x[0,1]\nU01:%x[0,2]/%x[-1,1]\nB\n").unwrap();

    success(run(
        &[
            "train-chunker",
            "--regularization",
            "l2",
            "--lambda",
            "0.01",
            "--columns",
            "surface,pos,cform",
        ],
        &[
            ("--input", &train_path),
            ("--output", &model_path),
            ("--template", &template_path),
        ],
    ));
    let file = Model::new(fs::read(&model_path).unwrap()).unwrap();
    let chunker = file.chunker().unwrap();
    let model = chunker.model();
    assert_eq!(
        model.columns(),
        [
            Column::Surface,
            Column::PartOfSpeech,
            Column::ConjugationForm
        ]
    );
    assert_eq!(
        model.template().to_string(),
        "U00:%x[0,1]\nU01:%x[0,2]/%x[-1,1]\nB\n"
    );
    // The feature strings are those of the template: the conjugation form
    // is read from the column the schema of IPADIC names.
    assert!(model.feature("U00:助詞").is_some());
    assert!(model.feature("U01:基本形/名詞").is_some());
    assert!(model.feature("U02:は").is_none());
}

#[test]
fn fails_with_the_cause() {
    let directory = tempfile::tempdir().unwrap();
    let train_path = directory.path().join("train.lattice");
    let model_path = directory.path().join("chunker.model");
    let template_path = directory.path().join("chunker.template");
    let output_path = directory.path().join("output.lattice");
    fs::write(&train_path, training_data()).unwrap();
    let train: [(&str, &Path); 2] = [("--input", &train_path), ("--output", &model_path)];

    // Columns and a template that cannot be read, and a template over more
    // columns than are given.
    let error = failure(run(
        &["train-chunker", "--columns", "surface,lemma"],
        &train,
    ));
    assert!(
        error.contains("Error: LinderaError { kind: Content, source: unknown column `lemma`"),
        "{error}"
    );
    fs::write(&template_path, "U00:%x[0,0\nB\n").unwrap();
    let error = failure(run(
        &["train-chunker"],
        &[
            ("--input", &train_path),
            ("--output", &model_path),
            ("--template", &template_path),
        ],
    ));
    assert!(
        error.contains("chunker.template: template line 1: no `]`"),
        "{error}"
    );
    let error = failure(run(&["train-chunker", "--columns", "surface"], &train));
    assert!(
        error.contains("train.lattice: the template reads 3 columns, and 1 are given"),
        "{error}"
    );
    // A coefficient lindera-crf refuses.
    let error = failure(run(&["train-chunker", "--lambda=-1"], &train));
    assert!(
        error.contains("train.lattice: training failed: "),
        "{error}"
    );
    assert!(!model_path.exists());

    // A file that is not a model, and no model.
    let chunk: [(&str, &Path); 3] = [
        ("--model", &model_path),
        ("--input", &train_path),
        ("--output", &output_path),
    ];
    let error = failure(run(&["chunk"], &chunk));
    assert!(
        error.contains("Error: LinderaError { kind: Content, source: cannot read"),
        "{error}"
    );
    fs::write(&model_path, b"not a model").unwrap();
    let error = failure(run(&["chunk"], &chunk));
    assert!(
        error.contains("chunker.model: not a model file: the magic number is missing"),
        "{error}"
    );
    // A model file without a chunker.
    let bytes = ModelBuilder::new("ipadic", Schema::ipadic())
        .unwrap()
        .to_bytes();
    fs::write(&model_path, bytes).unwrap();
    let error = failure(run(&["chunk"], &chunk));
    assert!(
        error.contains("chunker.model: the model has no chunker"),
        "{error}"
    );
    assert!(!output_path.exists());

    // Morphemes that are not of the dictionary of the model: the second
    // sentence has one with three feature columns. Nothing is written.
    success(run(&["train-chunker", "--threads", "1"], &train));
    let input_path = directory.path().join("input.lattice");
    let input = "* 0 -1D \n犬\t名詞,*,*,*,*,基本形,犬,*,*\nEOS\n\
                 * 0 -1D \n犬\t名詞,*,*,*,*,基本形,犬,*,*\nは\t助詞,*,*\nEOS\n";
    fs::write(&input_path, input).unwrap();
    let error = failure(run(
        &["chunk"],
        &[
            ("--model", &model_path),
            ("--input", &input_path),
            ("--output", &output_path),
        ],
    ));
    assert!(
        error.contains(
            "input.lattice: sentence 2: the morpheme 2 (\"は\") has fewer than the 7 feature columns the schema reads"
        ),
        "{error}"
    );
    assert!(!output_path.exists());
}
