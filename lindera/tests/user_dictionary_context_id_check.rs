//! A user dictionary whose context IDs lie outside the system dictionary's connection
//! matrix must be rejected when the two are combined, not panic at tokenization (#1171).
//!
//! A user dictionary can be built without the system dictionary (`lindera build --user`
//! without `--system-dict` reads only `metadata.json`), so nothing stops an entry from
//! naming a context ID the matrix does not have. `Segmenter::new` accepts such a
//! dictionary, and the lookup then indexes past the matrix when the entry matches the
//! input. `Segmenter::try_new`, and the configuration path that the CLI and the bindings
//! use, check every entry first. The configuration path builds a `.csv` against the
//! system dictionary's matrix, so its error names the CSV line and surface (#1187).
//!
//! The fixture is a 4x4 dictionary, so context IDs 0..=3 are valid on both axes.

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use lindera::dictionary::{
    DictionaryBuilder, Metadata, UserDictionary, load_dictionary, load_user_dictionary,
};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

/// DEFAULT plus SPACE (so the default `keep_whitespace: false` of the configuration path
/// is accepted) and KANJI.
const CHAR_DEF: &str = "\
DEFAULT 0 1 0
SPACE 0 1 0
KANJI 0 0 2
0x0020 SPACE
0x4E00..0x9FFF KANJI
";

/// One unknown-word entry per category, with a prohibitive cost.
const UNK_DEF: &str = "\
DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*
SPACE,0,0,10000,空白,*,*,*,*,*,*,*,*
KANJI,0,0,10000,名詞,一般,*,*,*,*,*,*,*
";

/// Two system entries that together spell the user entry's surface.
const LEX_CSV: &str = "\
東京,2,2,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
都,2,2,0,名詞,接尾,地域,*,*,*,都,ト,ト
";

/// A user entry whose context IDs are both inside the 4x4 matrix.
const USER_CSV_IN_RANGE: &str = "\
東京都,3,3,0,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト
";

/// The user entry of the issue report: a left context ID far outside the matrix.
const USER_CSV_LEFT_OUT_OF_RANGE: &str = "\
東京都,60000,3,0,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト
";

/// A user entry whose right context ID is the first one outside the matrix.
const USER_CSV_RIGHT_OUT_OF_RANGE: &str = "\
東京都,3,4,0,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト
";

/// The detail fields of the user entries above, as the error names them.
const USER_DETAILS: &str = "名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト";

/// Writes the system dictionary source files into `dir`: the files above and a 4x4
/// `matrix.def` in which every connection costs 100.
fn write_source(dir: &Path) {
    let mut matrix_def = String::from("4 4\n");
    for right_id in 0..4 {
        for left_id in 0..4 {
            matrix_def.push_str(&format!("{right_id} {left_id} 100\n"));
        }
    }
    fs::write(dir.join("char.def"), CHAR_DEF).unwrap();
    fs::write(dir.join("unk.def"), UNK_DEF).unwrap();
    fs::write(dir.join("lex.csv"), LEX_CSV).unwrap();
    fs::write(dir.join("matrix.def"), matrix_def).unwrap();
}

/// Builds the fixture system dictionary into a fresh directory, with or without the
/// context-ID remap.
fn build_system_dictionary(remap: bool) -> tempfile::TempDir {
    let source = tempfile::tempdir().unwrap();
    write_source(source.path());

    let output = tempfile::tempdir().unwrap();
    let metadata = Metadata {
        connection_id_mapping: remap,
        ..Default::default()
    };
    DictionaryBuilder::new(metadata)
        .build_dictionary(source.path(), output.path())
        .unwrap();
    output
}

/// Writes `csv` as `user.csv` into `dir` (outside the system source directory, whose
/// `*.csv` files the system build reads) and returns its path.
fn write_user_csv(dir: &Path, csv: &str) -> String {
    let path = dir.join("user.csv");
    fs::write(&path, csv).unwrap();
    path.to_str().unwrap().to_string()
}

/// Loads the user dictionary at `uri` with the metadata of the system dictionary at
/// `dict_dir`, as the configuration path does.
fn load_user(dict_dir: &Path, uri: &str) -> UserDictionary {
    let dictionary = load_dictionary(dict_dir.to_str().unwrap()).unwrap();
    load_user_dictionary(uri, &dictionary.metadata).unwrap()
}

/// Returns the message of the error `Segmenter::try_new` gives for the user dictionary at
/// `uri` combined with the system dictionary at `dict_dir`.
fn try_new_error(dict_dir: &Path, uri: &str) -> String {
    let user_dictionary = load_user(dict_dir, uri);
    let dictionary = load_dictionary(dict_dir.to_str().unwrap()).unwrap();
    match Segmenter::try_new(Mode::Normal, dictionary, Some(user_dictionary)) {
        Ok(_) => panic!("try_new accepted a user dictionary outside the matrix"),
        Err(err) => err.to_string(),
    }
}

/// Segments `text` with `segmenter` and returns the surfaces.
fn surfaces(segmenter: &Segmenter, text: &str) -> Vec<String> {
    segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .into_iter()
        .map(|token| token.surface.to_string())
        .collect()
}

/// The entry of the issue report: a left context ID of 60000 against a 4x4 matrix. The
/// error names the entry, the ID and the matrix size.
#[test]
fn try_new_rejects_a_left_id_outside_the_matrix() {
    let dict_dir = build_system_dictionary(false);
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_LEFT_OUT_OF_RANGE);

    let err = try_new_error(dict_dir.path(), &user_csv);
    assert!(
        err.contains(&format!(
            "user dictionary entry 0 ({USER_DETAILS}): left context id 60000 is out of range \
             for the connection matrix (matrix.def size 4 4; left context ids must be below 4)"
        )),
        "{err}"
    );
}

/// A right context ID equal to the matrix's forward size is the first one outside it.
#[test]
fn try_new_rejects_a_right_id_outside_the_matrix() {
    let dict_dir = build_system_dictionary(false);
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_RIGHT_OUT_OF_RANGE);

    let err = try_new_error(dict_dir.path(), &user_csv);
    assert!(err.contains("right context id 4 is out of range"), "{err}");
}

/// A pre-built `.bin` user dictionary is checked the same way: `lindera build --user`
/// compiles it with no system dictionary in sight.
#[test]
fn try_new_rejects_a_prebuilt_bin_user_dictionary() {
    let dict_dir = build_system_dictionary(false);
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_LEFT_OUT_OF_RANGE);
    let user_bin = user_dir.path().join("user.bin");
    DictionaryBuilder::new(Metadata::default())
        .build_user_dictionary(Path::new(&user_csv), &user_bin)
        .unwrap();

    let err = try_new_error(dict_dir.path(), user_bin.to_str().unwrap());
    assert!(
        err.contains("left context id 60000 is out of range"),
        "{err}"
    );
}

/// With a system dictionary built with `connection_id_mapping`, the IDs are checked
/// before they are relabeled: the error still shows the ID the user wrote.
#[test]
fn try_new_rejects_out_of_range_ids_with_a_remapped_dictionary() {
    let dict_dir = build_system_dictionary(true);
    let dictionary = load_dictionary(dict_dir.path().to_str().unwrap()).unwrap();
    assert!(
        dictionary.metadata.context_id_map.is_some(),
        "fixture is mis-built: the system dictionary should carry a context-ID map"
    );
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_LEFT_OUT_OF_RANGE);

    let err = try_new_error(dict_dir.path(), &user_csv);
    assert!(
        err.contains("left context id 60000 is out of range"),
        "{err}"
    );
}

/// A user dictionary inside the matrix passes the check, with and without the remap, and
/// segments as with `Segmenter::new`.
#[test]
fn try_new_accepts_a_user_dictionary_inside_the_matrix() {
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_IN_RANGE);

    for remap in [false, true] {
        let dict_dir = build_system_dictionary(remap);
        let dictionary = load_dictionary(dict_dir.path().to_str().unwrap()).unwrap();

        let checked = Segmenter::try_new(
            Mode::Normal,
            dictionary.clone(),
            Some(load_user(dict_dir.path(), &user_csv)),
        )
        .unwrap();
        let unchecked = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(load_user(dict_dir.path(), &user_csv)),
        );

        assert_eq!(
            surfaces(&checked, "東京都"),
            vec!["東京都".to_string()],
            "remap {remap}"
        );
        assert_eq!(
            surfaces(&checked, "東京都"),
            surfaces(&unchecked, "東京都"),
            "remap {remap}"
        );
    }
}

/// Returns the message of the error `Segmenter::from_config` gives for the user
/// dictionary at `uri` with the system dictionary at `dict_dir`.
fn from_config_error(dict_dir: &Path, uri: &str) -> String {
    let config = serde_json::json!({
        "dictionary": dict_dir.to_str().unwrap(),
        "user_dictionary": uri,
    });
    match Segmenter::from_config(&config) {
        Ok(_) => panic!("from_config accepted a user dictionary outside the matrix"),
        Err(err) => err.to_string(),
    }
}

/// The configuration path, which `TokenizerBuilder`, the CLI and the bindings' builders
/// use, checks the user dictionary that the `user_dictionary` key names. A `.csv` is
/// built against the system dictionary's matrix, so the error names its CSV line and
/// surface, with and without the context-ID remap (#1187).
#[test]
fn from_config_rejects_a_user_dictionary_outside_the_matrix() {
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_LEFT_OUT_OF_RANGE);

    for remap in [false, true] {
        let dict_dir = build_system_dictionary(remap);
        let err = from_config_error(dict_dir.path(), &user_csv);
        assert!(
            err.contains(
                "user dictionary line 1 (surface: '東京都'): left context id 60000 is out of \
                 range for the connection matrix (matrix.def size 4 4; left context ids must be \
                 below 4)"
            ),
            "remap {remap}: {err}"
        );
    }
}

/// A pre-built `.bin` that the `user_dictionary` key names is still checked when it is
/// combined with the system dictionary, and the error names the entry by word id.
#[test]
fn from_config_rejects_a_prebuilt_bin_user_dictionary_by_word_id() {
    let dict_dir = build_system_dictionary(false);
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = write_user_csv(user_dir.path(), USER_CSV_LEFT_OUT_OF_RANGE);
    let user_bin = user_dir.path().join("user.bin");
    DictionaryBuilder::new(Metadata::default())
        .build_user_dictionary(Path::new(&user_csv), &user_bin)
        .unwrap();

    let err = from_config_error(dict_dir.path(), user_bin.to_str().unwrap());
    assert!(
        err.contains(&format!(
            "user dictionary entry 0 ({USER_DETAILS}): left context id 60000 is out of range"
        )),
        "{err}"
    );
}
