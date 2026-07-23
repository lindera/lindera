//! A user dictionary must produce the same segmentation against a system dictionary
//! built with `connection_id_mapping` as against one built without it.
//!
//! User dictionaries are always compiled in the original context-ID space. When the
//! system dictionary is built with the remap enabled, those IDs have to be relabeled or
//! every connection cost the user entry participates in addresses the wrong matrix cell
//! — silently, because the IDs stay in range.
//!
//! The fixture below is tuned so that failing to relabel actually changes the output:
//! the frequency histogram forces the permutation `1 <-> 3` on both axes, and the
//! connection matrix puts a cheap cost where the remapped user entry looks and a
//! prohibitive one where an un-remapped entry would look.

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use lindera::dictionary::{load_dictionary, load_user_dictionary};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;

/// DEFAULT plus KANJI, with the CJK range mapped to KANJI. `invoke = 0` keeps unknown
/// words out of the way whenever a lexicon entry matches.
const CHAR_DEF: &str = "\
DEFAULT 0 1 0
KANJI 0 0 2
0x4E00..0x9FFF KANJI
";

/// Unknown words are given a prohibitive cost so they never win over the entries below.
const UNK_DEF: &str = "\
DEFAULT,0,0,10000,補助記号,一般,*,*,*,*,*,*,*
KANJI,0,0,10000,名詞,一般,*,*,*,*,*,*,*
";

/// Both system entries use context ID 2; the user entry (below) uses 3.
const LEX_CSV: &str = "\
東京,2,2,0,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
都,2,2,0,名詞,接尾,地域,*,*,*,都,ト,ト
";

/// A *detailed* user dictionary row: 13 fields, so `row[1]`/`row[2]` are parsed as
/// explicit context IDs (a 3-field row would use the metadata defaults instead).
const USER_CSV: &str = "\
東京都,3,3,0,名詞,固有名詞,地域,一般,*,*,東京都,トウキョウト,トウキョウト
";

/// 4x4 matrix. Everything costs 100 except:
/// - `(0, 3) = 0`: BOS -> the user entry's left ID, making the single-token path cheap.
/// - `(0, 1) = 30000`: the trap. ID 1 is used by no entry, but the permutation maps
///   1 -> 3, so an un-remapped user entry (still left ID 3) lands exactly here.
const MATRIX_DEF: &str = "\
4 4
0 0 100
0 1 30000
0 2 100
0 3 0
1 0 100
1 1 100
1 2 100
1 3 100
2 0 100
2 1 100
2 2 100
2 3 100
3 0 100
3 1 100
3 2 100
3 3 100
";

/// Frequency histogram driving the permutation: ranking IDs by count descending gives
/// 3 -> 1, 2 -> 2, 1 -> 3 on both axes (ID 0 is always pinned to 0).
const CONTEXT_ID_FREQ: &str = "\
4 4
0 1 5 100
0 1 5 100
";

/// Write the shared dictionary source files into `dir`.
fn write_source(dir: &Path) {
    fs::write(dir.join("char.def"), CHAR_DEF).unwrap();
    fs::write(dir.join("unk.def"), UNK_DEF).unwrap();
    fs::write(dir.join("lex.csv"), LEX_CSV).unwrap();
    fs::write(dir.join("matrix.def"), MATRIX_DEF).unwrap();
}

/// Build the fixture dictionary into a fresh directory, with the context-ID remap either
/// enabled (and fed the histogram above) or disabled.
///
/// Returns the temporary directory holding the built artifacts.
fn build_dictionary(source: &Path, remap: bool) -> tempfile::TempDir {
    let output = tempfile::tempdir().unwrap();

    let metadata = Metadata {
        connection_id_mapping: remap,
        ..Default::default()
    };

    let mut builder = DictionaryBuilder::new(metadata);
    if remap {
        let freq = source.join("context_id_freq.txt");
        fs::write(&freq, CONTEXT_ID_FREQ).unwrap();
        builder = builder.with_context_id_freq(freq);
    }

    builder.build_dictionary(source, output.path()).unwrap();
    output
}

/// Segment `text` with the built dictionary at `dict_dir` plus the user dictionary CSV,
/// returning the token surfaces.
fn segment_with_user_dictionary(dict_dir: &Path, user_csv: &Path, text: &str) -> Vec<String> {
    let dictionary = load_dictionary(dict_dir.to_str().unwrap()).unwrap();
    let user_dictionary =
        load_user_dictionary(user_csv.to_str().unwrap(), &dictionary.metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

    segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .into_iter()
        .map(|token| token.surface.to_string())
        .collect()
}

/// The same detailed user dictionary must segment identically whether or not the system
/// dictionary was built with `connection_id_mapping`.
#[test]
fn detailed_user_dictionary_is_unaffected_by_context_id_remap() {
    let source = tempfile::tempdir().unwrap();
    write_source(source.path());

    // The user CSV must live outside the source directory: the system build globs
    // `*.csv` there, so a user dictionary placed alongside it would be compiled into the
    // system lexicon and the user-dictionary path would never be exercised.
    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = user_dir.path().join("user.csv");
    fs::write(&user_csv, USER_CSV).unwrap();

    let plain = build_dictionary(source.path(), false);
    let remapped = build_dictionary(source.path(), true);

    let text = "東京都";
    let from_plain = segment_with_user_dictionary(plain.path(), &user_csv, text);
    let from_remapped = segment_with_user_dictionary(remapped.path(), &user_csv, text);

    // Sanity: the fixture is only meaningful if the user entry wins on the plain build.
    assert_eq!(
        from_plain,
        vec!["東京都".to_string()],
        "fixture is mis-tuned: the user entry should win without remapping"
    );
    // The actual regression: without relabeling, the un-remapped left ID 3 would hit the
    // 30000 trap and the system entries would win instead.
    assert_eq!(
        from_remapped, from_plain,
        "remapped dictionary segmented differently; user-dictionary context IDs were not relabeled"
    );
}

/// The same must hold for a pre-built `.bin` user dictionary, which is compiled without
/// any knowledge of the system dictionary it will later be attached to.
#[test]
fn prebuilt_user_dictionary_bin_is_unaffected_by_context_id_remap() {
    let source = tempfile::tempdir().unwrap();
    write_source(source.path());

    let user_dir = tempfile::tempdir().unwrap();
    let user_csv = user_dir.path().join("user.csv");
    fs::write(&user_csv, USER_CSV).unwrap();

    // Compile the user dictionary the way `lindera build -u` does: metadata only, no
    // system dictionary in sight, so its context IDs stay in the original space.
    let user_bin = user_dir.path().join("user.bin");
    DictionaryBuilder::new(Metadata::default())
        .build_user_dictionary(&user_csv, &user_bin)
        .unwrap();

    let plain = build_dictionary(source.path(), false);
    let remapped = build_dictionary(source.path(), true);

    let text = "東京都";
    let from_plain = segment_with_user_dictionary(plain.path(), &user_bin, text);
    let from_remapped = segment_with_user_dictionary(remapped.path(), &user_bin, text);

    assert_eq!(from_plain, vec!["東京都".to_string()]);
    assert_eq!(
        from_remapped, from_plain,
        "pre-built user dictionary was not relabeled for the remapped system dictionary"
    );
}

/// The permutation must be persisted in the built metadata, since that is what the
/// segmenter uses to relabel user dictionaries.
#[test]
fn built_metadata_carries_the_context_id_map() {
    let source = tempfile::tempdir().unwrap();
    write_source(source.path());

    let remapped = build_dictionary(source.path(), true);
    let dictionary = load_dictionary(remapped.path().to_str().unwrap()).unwrap();
    let map = dictionary
        .metadata
        .context_id_map
        .expect("a remapped build must persist its context-ID map");

    // Ranked by the histogram: 3 -> 1, 2 -> 2, 1 -> 3, with 0 pinned.
    assert_eq!(map.left, vec![0, 3, 2, 1]);
    assert_eq!(map.right, vec![0, 3, 2, 1]);

    // A dictionary built without the remap must not carry one.
    let plain = build_dictionary(source.path(), false);
    let plain_dictionary = load_dictionary(plain.path().to_str().unwrap()).unwrap();
    assert!(plain_dictionary.metadata.context_id_map.is_none());
}
