//! End-to-end tests for `DictionaryBuilder::build_dictionary`.
//!
//! On non-wasm targets `build_dictionary` runs its independent stages
//! concurrently, so these tests exercise the parallel build path and assert it
//! produces the expected files deterministically.

use std::fs;
use std::path::Path;

use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;

/// Minimal `char.def`: the mandatory DEFAULT category plus KANJI, with the CJK
/// range mapped to KANJI so the sample surfaces are categorized.
const CHAR_DEF: &str = "\
DEFAULT 0 1 0
KANJI 0 0 2
0x4E00..0x9FFF KANJI
";

/// Minimal `unk.def`: one entry per category (category, left_id, right_id,
/// cost, then detail fields).
const UNK_DEF: &str = "\
DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
";

/// Minimal lexicon matching the default 13-field schema
/// (surface, left_context_id, right_context_id, cost, then 9 detail fields).
const LEX_CSV: &str = "\
日本,0,0,100,名詞,固有名詞,地域,国,*,*,日本,ニッポン,ニッポン
本,0,0,200,名詞,一般,*,*,*,*,本,ホン,ホン
";

/// Minimal 1x1 connection cost matrix.
const MATRIX_DEF: &str = "1 1\n0 0 0\n";

/// Write the minimal dictionary source files into `dir`.
fn write_source(dir: &Path) {
    fs::write(dir.join("char.def"), CHAR_DEF).unwrap();
    fs::write(dir.join("unk.def"), UNK_DEF).unwrap();
    fs::write(dir.join("lex.csv"), LEX_CSV).unwrap();
    fs::write(dir.join("matrix.def"), MATRIX_DEF).unwrap();
}

/// The artifacts a completed build must produce.
const EXPECTED_FILES: &[&str] = &[
    "metadata.json",
    "char_def.bin",
    "unk.bin",
    "dict.da",
    "dict.vals",
    "dict.words",
    "dict.wordsidx",
    "matrix.mtx",
];

#[test]
fn build_dictionary_produces_all_artifacts() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source(input.path());

    let builder = DictionaryBuilder::new(Metadata::default());
    builder
        .build_dictionary(input.path(), output.path())
        .expect("build_dictionary should succeed");

    for name in EXPECTED_FILES {
        let path = output.path().join(name);
        let meta =
            fs::metadata(&path).unwrap_or_else(|err| panic!("missing artifact {name}: {err}"));
        assert!(meta.len() > 0, "artifact {name} is empty");
    }
}

#[test]
fn build_dictionary_is_deterministic() {
    // Building the same source twice must yield byte-identical artifacts. On
    // non-wasm targets the stages run concurrently, so a data race would show
    // up as a byte difference between the two runs.
    let input = tempfile::tempdir().unwrap();
    write_source(input.path());
    let builder = DictionaryBuilder::new(Metadata::default());

    let out_a = tempfile::tempdir().unwrap();
    let out_b = tempfile::tempdir().unwrap();
    builder
        .build_dictionary(input.path(), out_a.path())
        .unwrap();
    builder
        .build_dictionary(input.path(), out_b.path())
        .unwrap();

    for name in EXPECTED_FILES {
        let a = fs::read(out_a.path().join(name)).unwrap();
        let b = fs::read(out_b.path().join(name)).unwrap();
        assert_eq!(a, b, "artifact {name} differs between builds");
    }
}

#[test]
fn build_dictionary_reports_missing_input() {
    // An empty input directory (no matrix.def / char.def) must return an error
    // rather than panicking, even though stages run concurrently.
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    let builder = DictionaryBuilder::new(Metadata::default());
    let result = builder.build_dictionary(input.path(), output.path());
    assert!(result.is_err(), "expected an error for missing input files");
}
