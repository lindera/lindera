//! #1219: a large Decompose length penalty saturates instead of overflowing,
//! so it splits a long word at least as much as a smaller penalty does.
//!
//! The product of the cost per character and the characters beyond the
//! threshold used to overflow `i32`: debug builds panicked, and release
//! builds wrapped the cost, which left `関西国際空港` whole with a kanji
//! penalty of 1000000000.
//!
//! #1223: a penalty large enough to saturate the cost of a path no longer
//! cuts the sentence short. A word that cannot be split paid it on every
//! transition out of it, every such transition was dropped, and the text
//! after the word was lost. The lattice now bounds the costs per character
//! of each sentence, so that the costs of its paths stay exact, and a
//! transition whose cost saturates anyway still connects.

#![cfg(feature = "embed-ipadic")]

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;

/// Builds an IPADIC segmenter in Decompose mode with `penalty`.
fn segmenter(penalty: Penalty) -> Segmenter {
    Segmenter::new(
        Mode::Decompose(penalty),
        load_dictionary("embedded://ipadic").unwrap(),
        None,
    )
}

/// Segments `text` with IPADIC in Decompose mode with `penalty`.
fn surfaces(penalty: Penalty, text: &str) -> Vec<String> {
    segmenter(penalty)
        .segment(Cow::Borrowed(text))
        .unwrap()
        .iter()
        .map(|token| token.surface.to_string())
        .collect()
}

/// A kanji penalty whose product overflowed splits the kanji compound as the
/// default penalty does. Since #1223 both costs count as the bound of the
/// sentence.
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
/// a smaller penalty that does not overflow does. Since #1223 all three
/// costs count as the bound of the sentence.
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

/// #1223: a word that cannot be split (a run of letters is one unknown word
/// in IPADIC, `ALPHA 1 1 0`) pays a penalty for words that are not
/// kanji-only large enough to saturate a path, and the rest of the text is
/// still segmented: what follows it in the sentence, the sentences after it
/// (`。` and `、` carry the context to the next sentence), and the kanji
/// compound after it, which the default kanji penalty still splits. The
/// first N-best result is the 1-best one, at a cost the bound keeps far
/// below the range of `i32`.
#[test]
fn test_unsplittable_word_keeps_the_rest_of_the_text() {
    let cases: &[(&str, i32, &[&str])] = &[
        (
            "今日はabcdefghiとabcdefghiです",
            1_000_000_000,
            &["今日", "は", "abcdefghi", "と", "abcdefghi", "です"],
        ),
        (
            "今日はabcdefghです",
            i32::MAX,
            &["今日", "は", "abcdefgh", "です"],
        ),
        (
            "今日はabcdefghです。明日も",
            i32::MAX,
            &["今日", "は", "abcdefgh", "です", "。", "明日", "も"],
        ),
        ("abcdefgh", i32::MAX, &["abcdefgh"]),
        (
            "今日はabcdefghです、明日も",
            i32::MAX,
            &["今日", "は", "abcdefgh", "です", "、", "明日", "も"],
        ),
        (
            "abcdefgh関西国際空港です",
            i32::MAX,
            &["abcdefgh", "関西", "国際", "空港", "です"],
        ),
        (
            "関西国際空港abcdefghです",
            i32::MAX,
            &["関西", "国際", "空港", "abcdefgh", "です"],
        ),
    ];
    for &(text, cost, expected) in cases {
        let penalty = Penalty {
            other_penalty_length_penalty: cost,
            ..Penalty::default()
        };
        assert_eq!(surfaces(penalty.clone(), text), expected, "{text} {cost}");

        let segmenter = segmenter(penalty);
        let results = segmenter
            .segment_nbest(Cow::Borrowed(text), 1, false, None)
            .unwrap();
        let (tokens, nbest_cost) = &results[0];
        let nbest: Vec<_> = tokens.iter().map(|token| token.surface.as_ref()).collect();
        assert_eq!(nbest, expected, "{text} {cost}: N-best");
        assert!(
            *nbest_cost < 1 << 30,
            "{text} {cost}: N-best cost {nbest_cost}"
        );
    }
}
