//! Reads the files of `tests/data` in the lattice format and writes them
//! back: the bytes have to be the same (#1418).
//!
//! The files hold tabs and trailing spaces; see `tests/data/README.md`.

use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use lindera_dependency::lattice::{self, LatticeMorpheme, Reader};
use lindera_dependency::{ArcType, Morpheme, Sentence, Violation};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

/// Reads a file of `tests/data` with [`lattice::parse`], checks that the
/// [`Reader`] gives the same sentences from the file itself, and that
/// writing them gives the bytes of the file.
fn read_and_write_back(name: &str) -> Vec<Sentence<LatticeMorpheme>> {
    let path = data(name);
    let bytes = fs::read(&path).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let sentences = lattice::parse(text).unwrap();

    let from_file: Vec<Sentence<LatticeMorpheme>> =
        Reader::new(BufReader::new(File::open(&path).unwrap()))
            .collect::<Result<_, _>>()
            .unwrap();
    assert_eq!(from_file, sentences, "{name}");

    let mut written = Vec::new();
    for sentence in &sentences {
        lattice::write(&mut written, sentence).unwrap();
    }
    assert!(
        written == bytes,
        "{name} does not come back as it was read:\n{}",
        String::from_utf8_lossy(&written)
    );
    sentences
}

fn surfaces(sentence: &Sentence<LatticeMorpheme>) -> Vec<&str> {
    sentence.morphemes.iter().map(|m| m.surface()).collect()
}

#[test]
fn bunsetsu_with_positions_scores_and_comments() {
    let sentences = read_and_write_back("dependency.lattice");
    assert_eq!(sentences.len(), 2);

    let first = &sentences[0];
    assert_eq!(first.comments, ["# S-ID: 1"]);
    assert_eq!(first.bunsetsu.len(), 3);
    assert_eq!(first.bunsetsu[1].score, Some(1.25));
    assert!(first.violations().is_empty());

    let second = &sentences[1];
    assert_eq!(second.comments, ["# S-ID: 2", "# coordination"]);
    assert_eq!(second.bunsetsu.len(), 4);
    assert_eq!(second.bunsetsu[1].head, Some(2));
    assert_eq!(second.bunsetsu[1].arc_type, ArcType::Parallel);
    assert_eq!(second.bunsetsu[1].score, Some(-0.5));
    assert_eq!(second.bunsetsu[3].head, None);
    assert!(second.violations().is_empty());
}

#[test]
fn the_chunk_layer_with_the_head_only() {
    let sentences = read_and_write_back("chunks.lattice");
    assert_eq!(sentences.len(), 1);
    assert_eq!(sentences[0].bunsetsu.len(), 3);
    for bunsetsu in &sentences[0].bunsetsu {
        assert_eq!(bunsetsu.positions, None);
        assert_eq!(bunsetsu.score, None);
    }
}

#[test]
fn morphemes_without_bunsetsu() {
    let sentences = read_and_write_back("morphemes.lattice");
    assert_eq!(sentences.len(), 3);
    assert_eq!(
        surfaces(&sentences[0]),
        ["彼", "は", "本", "を", "読ん", "だ", "。"]
    );
    assert!(sentences[1].morphemes.is_empty());
    assert_eq!(surfaces(&sentences[2]), ["本"]);
    for sentence in &sentences {
        assert!(sentence.bunsetsu.is_empty());
        assert!(sentence.violations().is_empty());
    }
}

#[test]
fn surfaces_that_look_like_other_lines() {
    let sentences = read_and_write_back("surfaces.lattice");
    assert_eq!(sentences.len(), 1);
    let sentence = &sentences[0];

    assert_eq!(sentence.comments, ["#comment"]);
    assert_eq!(
        surfaces(sentence),
        ["*", "#", "#タグ", "* 3 -1D", "EOS", ",", "東京", ","]
    );
    let ranges: Vec<_> = sentence
        .bunsetsu
        .iter()
        .map(|b| b.morphemes.clone())
        .collect();
    assert_eq!(ranges, [0..1, 1..3, 3..4, 4..6, 6..8]);

    // The comma MeCab quotes is one column; unquoted, the row has more
    // columns, and the part of speech is still in place.
    let quoted = &sentence.morphemes[5];
    assert_eq!(quoted.feature(6), Some(","));
    assert_eq!(quoted.feature(9), None);
    let unquoted = &sentence.morphemes[7];
    assert_eq!(unquoted.feature(1), Some("読点"));
    assert_eq!(unquoted.feature(11), Some(""));

    assert_eq!(sentence.morphemes[6].named_entity(), Some("B-LOCATION"));
    assert_eq!(sentence.morphemes[7].named_entity(), Some("O"));
    assert_eq!(sentence.morphemes[0].named_entity(), None);
}

/// A morpheme of another tokenizer: the trait is all the writer needs.
struct Word {
    surface: &'static str,
    columns: Vec<&'static str>,
}

impl Morpheme for Word {
    fn surface(&self) -> &str {
        self.surface
    }

    fn feature(&self, index: usize) -> Option<&str> {
        self.columns.get(index).copied()
    }
}

#[test]
fn a_type_outside_the_crate_is_a_morpheme() {
    let words = vec![
        Word {
            surface: "a",
            columns: vec!["NOUN", "x,y"],
        },
        Word {
            surface: ",",
            columns: vec!["PUNCT", ","],
        },
    ];
    let sentence = Sentence::new(
        words,
        vec![
            lindera_dependency::Bunsetsu::new(0..1, None),
            lindera_dependency::Bunsetsu::new(1..2, Some(0)),
        ],
    )
    .unwrap();
    assert_eq!(
        sentence.violations(),
        [
            Violation::RootNotLast { root: 0 },
            Violation::HeadNotToTheRight {
                bunsetsu: 1,
                head: 0
            },
        ]
    );

    let mut written = Vec::new();
    lattice::write(&mut written, &sentence).unwrap();
    let written = String::from_utf8(written).unwrap();
    assert_eq!(
        written,
        "* 0 -1D \na\tNOUN,\"x,y\"\n* 1 0D \n,\tPUNCT,\",\"\nEOS\n"
    );

    // What the default feature string quotes, the reader splits back.
    let read = lattice::parse(&written).unwrap();
    assert_eq!(read[0].morphemes[0].feature(1), Some("x,y"));
    assert_eq!(read[0].morphemes[1].feature(1), Some(","));
}
