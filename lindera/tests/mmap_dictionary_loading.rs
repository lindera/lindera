//! `load_dictionary_with_options`/`load_fs_dictionary_with_options` with
//! `use_mmap = true` must produce byte-for-byte identical tokenization to
//! `use_mmap = false`. This is the only test that actually exercises the
//! mmap-backed loading path end to end (issue #808 -- the path existed but
//! was unreachable from any public API before this change).

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use lindera::dictionary::{load_dictionary_with_options, load_fs_dictionary_with_options};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;

const CHAR_DEF: &str = "\
DEFAULT 0 1 0
SPACE 0 1 0
KANJI 0 0 2
0x0020 SPACE
0x4E00..0x9FFF KANJI
";

const UNK_DEF: &str = "\
DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*
KANJI,0,0,10000,名詞,一般,*,*,*,*,*,*,*
";

const LEX_CSV: &str = "\
東京,0,0,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
都,0,0,0,名詞,接尾,地域,*,*,*,都,ト,ト
";

const MATRIX_DEF: &str = "\
1 1
0 0 0
";

/// Build a small fixture dictionary into a fresh directory.
fn build_dictionary() -> tempfile::TempDir {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("char.def"), CHAR_DEF).unwrap();
    fs::write(source.path().join("unk.def"), UNK_DEF).unwrap();
    fs::write(source.path().join("lex.csv"), LEX_CSV).unwrap();
    fs::write(source.path().join("matrix.def"), MATRIX_DEF).unwrap();

    let output = tempfile::tempdir().unwrap();
    DictionaryBuilder::new(Metadata::default())
        .build_dictionary(source.path(), output.path())
        .unwrap();
    output
}

/// Segment `text` with the dictionary at `dict_dir`, returning
/// `(surface, first POS detail)` pairs per token.
fn segment(dict_dir: &Path, text: &str, use_mmap: bool) -> Vec<(String, String)> {
    let dictionary =
        load_fs_dictionary_with_options(dict_dir, use_mmap).expect("dictionary should load");
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let mut tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
    tokens
        .iter_mut()
        .map(|t| {
            let pos = t.get_detail(0).unwrap_or("").to_string();
            (t.surface.to_string(), pos)
        })
        .collect()
}

#[test]
fn mmap_loading_matches_plain_loading_via_fs_path() {
    let dict = build_dictionary();

    let plain = segment(dict.path(), "東京都", false);
    let mmapped = segment(dict.path(), "東京都", true);

    let expected = vec![
        ("東京".to_string(), "名詞".to_string()),
        ("都".to_string(), "名詞".to_string()),
    ];
    assert_eq!(plain, expected);
    assert_eq!(mmapped, expected);
}

#[test]
fn mmap_loading_matches_plain_loading_via_uri() {
    let dict = build_dictionary();
    let uri = dict.path().to_str().unwrap();

    let plain = load_dictionary_with_options(uri, false).unwrap();
    let mmapped = load_dictionary_with_options(uri, true).unwrap();

    let plain_segmenter = Segmenter::new(Mode::Normal, plain, None);
    let mmapped_segmenter = Segmenter::new(Mode::Normal, mmapped, None);

    let text = "東京都";
    let plain_surfaces: Vec<_> = plain_segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .into_iter()
        .map(|t| t.surface.to_string())
        .collect();
    let mmapped_surfaces: Vec<_> = mmapped_segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .into_iter()
        .map(|t| t.surface.to_string())
        .collect();

    assert_eq!(mmapped_surfaces, plain_surfaces);
}

/// `use_mmap = true` on an `embedded://` URI must be silently ignored, not
/// an error.
#[test]
#[cfg(feature = "embed-ipadic")]
fn mmap_flag_is_ignored_for_embedded_dictionaries() {
    let result = load_dictionary_with_options("embedded://ipadic", true);
    assert!(result.is_ok());
}

/// Regression test for #879: the no-option loaders default to mmap when the
/// `mmap` feature is compiled in, and the default must produce output
/// identical to both explicit modes.
#[test]
fn default_load_matches_both_explicit_modes() {
    use lindera::dictionary::load_fs_dictionary;

    let dict = build_dictionary();

    let default_dictionary = load_fs_dictionary(dict.path()).expect("default load should succeed");
    let segmenter = Segmenter::new(Mode::Normal, default_dictionary, None);
    let mut tokens = segmenter.segment(Cow::Borrowed("東京都")).unwrap();
    let default_result: Vec<(String, String)> = tokens
        .iter_mut()
        .map(|t| {
            let pos = t.get_detail(0).unwrap_or("").to_string();
            (t.surface.to_string(), pos)
        })
        .collect();

    assert_eq!(default_result, segment(dict.path(), "東京都", false));
    assert_eq!(default_result, segment(dict.path(), "東京都", true));
}

/// Regression test for #879: `Segmenter::from_config` without a `use_mmap`
/// key must load a filesystem dictionary with the feature-dependent default.
#[test]
fn from_config_without_use_mmap_defaults_and_segments() {
    let dict = build_dictionary();

    let config = serde_json::json!({
        "dictionary": dict.path().to_str().unwrap(),
        "mode": "normal",
    });
    let segmenter = Segmenter::from_config(&config).expect("from_config should succeed");
    let tokens = segmenter.segment(Cow::Borrowed("東京都")).unwrap();
    let surfaces: Vec<_> = tokens.into_iter().map(|t| t.surface.to_string()).collect();
    assert_eq!(surfaces, vec!["東京".to_string(), "都".to_string()]);
}

/// Regression test for #879: `Dictionary::clone` must be fully O(1) --
/// every field is Arc-shared, not deep-copied.
#[test]
fn dictionary_clone_shares_all_fields() {
    use std::sync::Arc;

    let dict_dir = build_dictionary();
    let dictionary = load_fs_dictionary_with_options(dict_dir.path(), false).unwrap();
    let cloned = dictionary.clone();

    assert!(Arc::ptr_eq(
        &dictionary.prefix_dictionary,
        &cloned.prefix_dictionary
    ));
    assert!(Arc::ptr_eq(
        &dictionary.connection_cost_matrix,
        &cloned.connection_cost_matrix
    ));
    assert!(Arc::ptr_eq(
        &dictionary.character_definition,
        &cloned.character_definition
    ));
    assert!(Arc::ptr_eq(
        &dictionary.unknown_dictionary,
        &cloned.unknown_dictionary
    ));
    assert!(Arc::ptr_eq(&dictionary.metadata, &cloned.metadata));
}
