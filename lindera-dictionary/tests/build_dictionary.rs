//! End-to-end tests for `DictionaryBuilder::build_dictionary`.
//!
//! On non-wasm targets `build_dictionary` runs its independent stages
//! concurrently, so these tests exercise the parallel build path and assert it
//! produces the expected files deterministically.

use std::fs;
use std::path::Path;

use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::metadata::{DICTIONARY_FORMAT_VERSION, Metadata};

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

/// Lexicon rows whose surface is or ends with whitespace: IPADIC's U+3000
/// entry, SudachiDict's U+0020 entry, and a trailing-space variant of `日本`
/// that is cheaper than the plain one (#1094).
const WHITESPACE_LEX_CSV: &str = "\
\u{3000},0,0,1287,記号,空白,*,*,*,*,\u{3000},\u{3000},\u{3000}
\u{20},0,0,10,空白,*,*,*,*,*,*,\u{20},\u{20}
日本\u{20},0,0,50,名詞,固有名詞,*,*,*,*,日本\u{20},ニッポン,ニッポン
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

/// The artifacts a completed build must produce (dictionary format
/// version 2: the system automaton is `dict.trie` + `dict.valsidx`).
const EXPECTED_FILES: &[&str] = &[
    "metadata.json",
    "char_def.bin",
    "unk.bin",
    "dict.trie",
    "dict.valsidx",
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

/// Whitespace surfaces are kept verbatim: a surface made of whitespace is
/// looked up as is, and an entry ending with whitespace gets its own key
/// instead of being merged into the trimmed one (#1094).
#[test]
fn build_dictionary_keeps_whitespace_surfaces() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source(input.path());
    fs::write(input.path().join("whitespace.csv"), WHITESPACE_LEX_CSV).unwrap();

    DictionaryBuilder::new(Metadata::default())
        .build_dictionary(input.path(), output.path())
        .expect("build_dictionary should succeed");
    let dictionary =
        Dictionary::load_from_path(output.path()).expect("a freshly built dictionary should load");
    let prefix = &dictionary.prefix_dictionary;

    let costs = |surface: &str| -> Vec<i16> {
        prefix
            .find_surface(surface)
            .iter()
            .map(|entry| entry.word_cost())
            .collect()
    };
    assert_eq!(costs("\u{3000}"), vec![1287]);
    assert_eq!(costs(" "), vec![10]);
    assert_eq!(costs("日本"), vec![100]);
    assert_eq!(costs("日本 "), vec![50]);
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

/// The builder stamps the format version it actually produced, not whatever
/// the source metadata happened to say. A source `metadata.json` describes
/// build inputs and has no business labelling the output.
#[test]
fn build_dictionary_stamps_the_current_format_version() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source(input.path());

    let metadata = Metadata {
        format_version: DICTIONARY_FORMAT_VERSION + 41,
        ..Metadata::default()
    };

    DictionaryBuilder::new(metadata)
        .build_dictionary(input.path(), output.path())
        .unwrap();

    let written = Metadata::load(&fs::read(output.path().join("metadata.json")).unwrap()).unwrap();
    assert_eq!(written.format_version, DICTIONARY_FORMAT_VERSION);

    // And what the builder wrote must be loadable.
    Dictionary::load_from_path(output.path()).expect("a freshly built dictionary should load");
}

/// A dictionary from an incompatible format must be refused at load. Every
/// other artifact is a headerless raw array, so without this check a stale
/// directory of a plausible size decodes into garbage costs instead of
/// failing.
#[test]
fn loading_rejects_a_dictionary_from_another_format_version() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source(input.path());

    DictionaryBuilder::new(Metadata::default())
        .build_dictionary(input.path(), output.path())
        .unwrap();

    // Rewrite only metadata.json, exactly as an older Lindera would have left
    // the directory.
    let path = output.path().join("metadata.json");
    let mut metadata = Metadata::load(&fs::read(&path).unwrap()).unwrap();
    metadata.format_version = DICTIONARY_FORMAT_VERSION + 1;
    fs::write(&path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();

    let err = match Dictionary::load_from_path(output.path()) {
        Ok(_) => panic!("a dictionary from another format version must not load"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains("format version"), "{err}");
}

/// An asymmetric connection matrix: 3 forward ids (the right context ids of
/// the left word) and 2 backward ids (the left context ids of the right word).
/// An id of 2 is valid as a right id but not as a left id, so the two bounds
/// are told apart.
const ASYMMETRIC_MATRIX_DEF: &str = "\
3 2
0 0 0
1 0 0
2 0 0
0 1 0
1 1 0
2 1 0
";

/// Writes a source whose lexicon, `unk.def` and `matrix.def` are given, with
/// the minimal `char.def`.
fn write_source_with(dir: &Path, lex_csv: &str, unk_def: &str, matrix_def: &str) {
    fs::write(dir.join("char.def"), CHAR_DEF).unwrap();
    fs::write(dir.join("unk.def"), unk_def).unwrap();
    fs::write(dir.join("lex.csv"), lex_csv).unwrap();
    fs::write(dir.join("matrix.def"), matrix_def).unwrap();
}

/// Builds a source made of the given files with `metadata`.
///
/// Returns the message of the build error, or `None` when the build
/// succeeded.
fn build_error(
    metadata: Metadata,
    lex_csv: &str,
    unk_def: &str,
    matrix_def: &str,
) -> Option<String> {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source_with(input.path(), lex_csv, unk_def, matrix_def);
    DictionaryBuilder::new(metadata)
        .build_dictionary(input.path(), output.path())
        .err()
        .map(|err| err.to_string())
}

/// A lexicon row whose context ids are `left` and `right`, after an in-range
/// first row, so that the row checked is on line 2.
fn lex_with_ids(left: u32, right: u32) -> String {
    format!(
        "本,0,0,200,名詞,一般,*,*,*,*,本,ホン,ホン\n\
         日本,{left},{right},100,名詞,固有名詞,地域,国,*,*,日本,ニッポン,ニッポン\n"
    )
}

/// The lexicon's context ids are checked against the matrix as
/// `mecab-dict-index` does: each id against its own axis (left ids against
/// the backward size, right ids against the forward size), naming the file,
/// the line, the id and the matrix size.
#[test]
fn build_dictionary_rejects_lexicon_context_ids_outside_the_matrix() {
    // Left id 2 is valid for the forward axis (3) but not for the backward
    // axis (2).
    let err = build_error(
        Metadata::default(),
        &lex_with_ids(2, 0),
        UNK_DEF,
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("left id 2 is out of the backward axis");
    assert!(err.contains("lex.csv: line 2: "), "{err}");
    assert!(err.contains("left context id 2 is out of range"), "{err}");
    assert!(err.contains("matrix.def size 3 2"), "{err}");
    assert!(err.contains("must be below 2"), "{err}");

    let err = build_error(
        Metadata::default(),
        &lex_with_ids(0, 3),
        UNK_DEF,
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("right id 3 is out of the forward axis");
    assert!(err.contains("lex.csv: line 2: "), "{err}");
    assert!(err.contains("right context id 3 is out of range"), "{err}");
    assert!(err.contains("must be below 3"), "{err}");

    // The largest id of each axis: right id 2 would fail a check against the
    // backward size (2), as a swapped check would do.
    assert_eq!(
        build_error(
            Metadata::default(),
            &lex_with_ids(1, 2),
            UNK_DEF,
            ASYMMETRIC_MATRIX_DEF
        ),
        None
    );
}

/// The check is made on the ids as written, before the context-id remap
/// relabels them, so it holds when the remap is enabled. Without it the remap
/// would leave the out-of-range id as is and the build would succeed.
#[test]
fn build_dictionary_rejects_out_of_range_context_ids_with_the_remap() {
    let metadata = Metadata {
        connection_id_mapping: true,
        ..Metadata::default()
    };

    let err = build_error(
        metadata.clone(),
        &lex_with_ids(2, 0),
        UNK_DEF,
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("left id 2 is out of the backward axis");
    assert!(err.contains("lex.csv: line 2: "), "{err}");
    assert!(err.contains("left context id 2 is out of range"), "{err}");

    let err = build_error(
        metadata.clone(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\nKANJI,1,3,0,名詞,一般,*,*,*,*,*,*,*\n",
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("right id 3 is out of the forward axis");
    assert!(err.contains("unk.def: line 2: "), "{err}");
    assert!(err.contains("right context id 3 is out of range"), "{err}");

    assert_eq!(
        build_error(
            metadata,
            &lex_with_ids(1, 2),
            UNK_DEF,
            ASYMMETRIC_MATRIX_DEF
        ),
        None
    );
}

/// `unk.def` ids are checked as written: 65537 would be truncated to 1 (in
/// range) by the conversion to the `u16` the dictionary stores.
#[test]
fn build_dictionary_rejects_unk_def_context_ids_outside_the_matrix() {
    let err = build_error(
        Metadata::default(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\nKANJI,65537,65537,0,名詞,一般,*,*,*,*,*,*,*\n",
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("id 65537 is out of range");
    assert!(err.contains("unk.def: line 2: "), "{err}");
    assert!(
        err.contains("left context id 65537 is out of range"),
        "{err}"
    );

    let err = build_error(
        Metadata::default(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\nKANJI,1,3,0,名詞,一般,*,*,*,*,*,*,*\n",
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("right id 3 is out of the forward axis");
    assert!(err.contains("right context id 3 is out of range"), "{err}");

    // A line too short to hold the ids is an error, not a panic.
    let err = build_error(
        Metadata::default(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\nKANJI,0\n",
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("a short line is rejected");
    assert!(err.contains("unk.def: line 2: "), "{err}");
    assert!(err.contains("expected at least 4 fields"), "{err}");
}

/// The context-id check leaves an id that does not parse to the `unk.def`
/// parser, which names the file and the line (#1175).
#[test]
fn build_dictionary_names_the_unk_def_line_whose_id_does_not_parse() {
    let err = build_error(
        Metadata::default(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\nKANJI,x,0,0,名詞,一般,*,*,*,*,*,*,*\n",
        ASYMMETRIC_MATRIX_DEF,
    )
    .expect("id x does not parse");
    assert!(
        err.contains("unk.def: line 2: invalid left context id `x`"),
        "{err}"
    );
}

/// Run on its own, the unknown-word stage does not check the context ids, so
/// a comment line reached the `unk.def` parser and panicked there (#1175). It
/// is an error naming the file and the line.
#[test]
fn build_unknown_dictionary_rejects_a_comment_line() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source_with(
        input.path(),
        LEX_CSV,
        &format!("# a comment\n{UNK_DEF}"),
        ASYMMETRIC_MATRIX_DEF,
    );
    let builder = DictionaryBuilder::new(Metadata::default());
    let chardef = builder
        .build_character_definition(input.path(), output.path())
        .unwrap();

    let err = builder
        .build_unknown_dictionary(input.path(), output.path(), &chardef, None)
        .expect_err("a comment line is rejected")
        .to_string();
    let expected = format!(
        "{}: line 1: expected at least 4 fields",
        input.path().join("unk.def").display()
    );
    assert!(err.contains(&expected), "{err}");
}

/// Each `matrix.def` id is checked against its own axis. A forward id equal to
/// the forward size used to pass the flat index check and land in the first
/// cell of the next row.
#[test]
fn build_dictionary_rejects_matrix_entries_outside_their_axis() {
    let err = build_error(Metadata::default(), LEX_CSV, UNK_DEF, "3 2\n0 0 0\n3 0 0\n")
        .expect("forward id 3 is out of the forward axis");
    assert!(err.contains("matrix.def line 3: "), "{err}");
    assert!(err.contains("entry (3, 0) is out of range"), "{err}");
    assert!(err.contains("forward ids must be below 3"), "{err}");

    let err = build_error(
        Metadata::default(),
        LEX_CSV,
        UNK_DEF,
        "3 2\n0 0 0\n1 1 0\n0 2 0\n",
    )
    .expect("backward id 2 is out of the backward axis");
    assert!(err.contains("matrix.def line 4: "), "{err}");
    assert!(err.contains("entry (0, 2) is out of range"), "{err}");
    assert!(err.contains("backward ids below 2"), "{err}");
}

/// With `skip_invalid_cost_or_id` (CC-CEDICT, Jieba, SudachiDict) a lexicon
/// row whose ids are out of range is dropped, like the other invalid rows,
/// instead of failing the build.
#[test]
fn build_dictionary_skips_out_of_range_rows_when_skipping_invalid_ids() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source_with(
        input.path(),
        &lex_with_ids(2, 0),
        UNK_DEF,
        ASYMMETRIC_MATRIX_DEF,
    );
    let metadata = Metadata {
        skip_invalid_cost_or_id: true,
        ..Metadata::default()
    };

    DictionaryBuilder::new(metadata)
        .build_dictionary(input.path(), output.path())
        .expect("the out-of-range row is skipped");

    let dictionary =
        Dictionary::load_from_path(output.path()).expect("a freshly built dictionary should load");
    let prefix = &dictionary.prefix_dictionary;
    assert!(prefix.find_surface("日本").is_empty());
    assert_eq!(prefix.find_surface("本").len(), 1);
}

/// The header is read for the checks with the decoding the matrix stage uses,
/// so a UTF-16 `matrix.def` (here with a BOM, under the default UTF-8 label)
/// still builds, and its sizes are the ones checked against.
#[test]
fn build_dictionary_reads_the_header_of_a_utf16_matrix() {
    let utf16 = |text: &str| -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes
    };

    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source_with(input.path(), &lex_with_ids(1, 2), UNK_DEF, "");
    fs::write(
        input.path().join("matrix.def"),
        utf16(ASYMMETRIC_MATRIX_DEF),
    )
    .unwrap();
    DictionaryBuilder::new(Metadata::default())
        .build_dictionary(input.path(), output.path())
        .expect("a UTF-16 matrix.def builds");
    let dictionary =
        Dictionary::load_from_path(output.path()).expect("a freshly built dictionary should load");
    assert_eq!(dictionary.prefix_dictionary.find_surface("日本").len(), 1);

    fs::write(input.path().join("lex.csv"), lex_with_ids(2, 0)).unwrap();
    let err = DictionaryBuilder::new(Metadata::default())
        .build_dictionary(input.path(), output.path())
        .expect_err("left id 2 is out of the backward axis")
        .to_string();
    assert!(err.contains("matrix.def size 3 2"), "{err}");
}

/// Builds a source made of the given `char.def` and `unk.def`, the minimal
/// lexicon and the 1x1 matrix.
///
/// Returns the input directory, whose paths the messages name, and the
/// message of the build error, or `None` when the build succeeded.
fn char_def_build_error(char_def: &str, unk_def: &str) -> (tempfile::TempDir, Option<String>) {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(input.path().join("char.def"), char_def).unwrap();
    fs::write(input.path().join("unk.def"), unk_def).unwrap();
    fs::write(input.path().join("lex.csv"), LEX_CSV).unwrap();
    fs::write(input.path().join("matrix.def"), MATRIX_DEF).unwrap();
    let err = DictionaryBuilder::new(Metadata::default())
        .build_dictionary(input.path(), output.path())
        .err()
        .map(|err| err.to_string());
    (input, err)
}

/// #1195: `char.def` and `unk.def` are checked against each other as
/// `mecab-dict-index` does. Each pair used to build, and the dictionary then
/// panicked, shifted the categories' settings, cut the output short or
/// ignored a line. The error names the file, the line where there is one,
/// and the category.
#[test]
fn build_dictionary_rejects_inconsistent_char_def_and_unk_def() {
    let unk_def_with_hiragana = format!("{UNK_DEF}HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n");
    for (char_def, unk_def, file, message) in [
        // A range line names a category that no line defines.
        (
            format!("{CHAR_DEF}0x3042 FOO\n"),
            UNK_DEF.to_string(),
            "char.def",
            "line 4: category FOO is not defined; a range line can only name a category \
             defined on an earlier line",
        ),
        // A range line comes before the definition of its category.
        (
            "DEFAULT 0 1 0\n0x3041..0x3096 HIRAGANA\nKANJI 0 0 2\nHIRAGANA 1 1 0\n\
             0x4E00..0x9FFF KANJI\n"
                .to_string(),
            unk_def_with_hiragana.clone(),
            "char.def",
            "line 2: category HIRAGANA is not defined",
        ),
        // A category is defined twice.
        (
            "DEFAULT 0 1 0\nKANJI 0 0 2\nKANJI 1 1 0\n0x4E00..0x9FFF KANJI\n".to_string(),
            UNK_DEF.to_string(),
            "char.def",
            "line 3: category KANJI is already defined on line 2",
        ),
        // A char.def category has no unk.def line.
        (
            format!("{CHAR_DEF}HIRAGANA 1 1 0\n0x3041..0x3096 HIRAGANA\n"),
            UNK_DEF.to_string(),
            "unk.def",
            "no line for category HIRAGANA, which char.def defines",
        ),
        // An unk.def line names a category that char.def does not define.
        (
            CHAR_DEF.to_string(),
            unk_def_with_hiragana,
            "unk.def",
            "line 3: category HIRAGANA is not defined in char.def",
        ),
    ] {
        let (input, err) = char_def_build_error(&char_def, &unk_def);
        let err = err.unwrap_or_else(|| panic!("accepted: {char_def:?}, {unk_def:?}"));
        let expected = format!("{}: {message}", input.path().join(file).display());
        assert!(err.contains(&expected), "{err}");
    }
}

/// Run on its own, the unknown-word stage checks the categories against the
/// character definition it is given, and writes no `unk.bin`.
#[test]
fn build_unknown_dictionary_checks_the_categories() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    write_source_with(
        input.path(),
        LEX_CSV,
        "DEFAULT,0,0,0,補助記号,一般,*,*,*,*,*,*,*\n",
        MATRIX_DEF,
    );
    let builder = DictionaryBuilder::new(Metadata::default());
    let chardef = builder
        .build_character_definition(input.path(), output.path())
        .unwrap();

    let err = builder
        .build_unknown_dictionary(input.path(), output.path(), &chardef, None)
        .expect_err("KANJI has no unk.def line")
        .to_string();
    let expected = format!(
        "{}: no line for category KANJI",
        input.path().join("unk.def").display()
    );
    assert!(err.contains(&expected), "{err}");
    assert!(!output.path().join("unk.bin").exists());
}
