//! #1219: a large Decompose length penalty saturates instead of overflowing,
//! so it splits a long word at least as much as a smaller penalty does.
//!
//! The product of the cost per character and the characters beyond the
//! threshold used to overflow `i32`: debug builds panicked, and release
//! builds wrapped the cost, which left `関西国際空港` whole with a kanji
//! penalty of 1000000000.

#![cfg(feature = "embed-ipadic")]

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;

/// Segments `text` with IPADIC in Decompose mode with `penalty`.
fn surfaces(penalty: Penalty, text: &str) -> Vec<String> {
    let segmenter = Segmenter::new(
        Mode::Decompose(penalty),
        load_dictionary("embedded://ipadic").unwrap(),
        None,
    );
    segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .iter()
        .map(|token| token.surface.to_string())
        .collect()
}

/// A kanji penalty whose product overflowed splits the kanji compound as the
/// default penalty does.
#[test]
fn test_large_kanji_penalty_splits_the_compound() {
    let text = "関西国際空港限定トートバッグ";
    let expected = surfaces(Penalty::default(), text);
    assert_eq!(expected, ["関西", "国際", "空港", "限定", "トートバッグ"]);
    for cost in [1_000_000_000, i32::MAX] {
        let penalty = Penalty {
            kanji_penalty_length_penalty: cost,
            ..Penalty::default()
        };
        assert_eq!(surfaces(penalty, text), expected, "kanji penalty {cost}");
    }
}

/// A penalty for words that are not kanji-only whose product overflowed
/// (13 characters, 6 beyond the threshold of 7) splits the katakana word as
/// a smaller penalty that does not overflow does.
#[test]
fn test_large_other_penalty_splits_the_katakana_word() {
    let text = "インターナショナルスクール";
    let with = |cost| Penalty {
        other_penalty_length_penalty: cost,
        ..Penalty::default()
    };
    let expected = surfaces(with(100_000_000), text);
    assert_eq!(expected, ["インター", "ナショナル", "スクール"]);
    for cost in [700_000_000, i32::MAX] {
        assert_eq!(surfaces(with(cost), text), expected, "other penalty {cost}");
    }
}
