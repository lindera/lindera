//! #1245: a huge left-space penalty no longer takes the cost of a path to
//! the bound of the lattice's 32-bit costs. Past that bound the costs a path
//! paid after the penalty were lost, so the rest of the sentence was
//! segmented by the order of the edges and the transitions of negative cost
//! alone, and often split into shorter words.
//!
//! With every first part-of-speech tag of ko-dic penalized by the same cost,
//! every path of a sentence pays it once per run of whitespace (no word of
//! these sentences ends with or spans one), so with exact costs the sentence
//! is segmented as without the penalty. The lattice now bounds the rule
//! costs of each sentence, so that the costs of its paths stay exact.

#![cfg(feature = "embed-ko-dic")]

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};

/// Every first part-of-speech tag of the mecab-ko-dic 2.1.1 sources
/// (`*.csv` and `unk.def`).
const TAGS: [&str; 48] = [
    "NNP", "NNG", "VV", "MAG", "XR", "VA", "EC", "EF", "VX", "IC", "XSV", "NP", "VCP", "NNBC",
    "XSA", "NR", "MM", "EP", "XSN", "NNB", "MAJ", "JKB", "ETM", "JX", "XPN", "VCN", "JC", "JKS",
    "ETN", "JKV", "JKQ", "JKG", "UNA", "NA", "JKO", "SL", "SSO", "SSC", "JKC", "SF", "SC", "SY",
    "SH", "VSV", "UNKNOWN", "SP", "SN", "SE",
];

/// Builds a ko-dic segmenter in Normal mode with the left-space penalty
/// rules `space_penalty` (`None` turns the penalty off).
fn segmenter(space_penalty: Option<SpacePenaltyConfig>) -> Segmenter {
    Segmenter::new(
        Mode::Normal,
        load_dictionary("embedded://ko-dic").unwrap(),
        None,
    )
    .space_penalty(space_penalty)
    .unwrap()
}

/// Segments `text` with `segmenter`.
fn surfaces(segmenter: &Segmenter, text: &str) -> Vec<String> {
    segmenter
        .segment(Cow::Borrowed(text))
        .unwrap()
        .iter()
        .map(|token| token.surface.to_string())
        .collect()
}

/// A penalty of `i32::MAX` or `i32::MIN` on every tag leaves the
/// segmentation as it is without the penalty, in the 1-best path and the
/// first N-best path. Each sentence used to be split differently with at
/// least one of the two costs, e.g. `잔디 / 속 / 잎 / 이` with `i32::MAX` and
/// `하 / 고 / 나서 / ,` with `i32::MIN`.
#[test]
fn test_uniform_huge_space_penalty_keeps_the_segmentation() {
    let cases: &[(&str, &[&str])] = &[
        ("잔디 속잎이", &["잔디", "속잎", "이"]),
        ("그만 갤 것이지", &["그만", "갤", "것", "이", "지"]),
        ("놓치지 말아라", &["놓치", "지", "말", "아라"]),
        ("어젯밤 비만 해도", &["어젯밤", "비", "만", "해도"]),
        ("하고 나서,", &["하", "고", "나", "서", ","]),
    ];
    let unpenalized = segmenter(None);
    for &(text, expected) in cases {
        assert_eq!(surfaces(&unpenalized, text), expected, "{text}");
    }
    for cost in [i32::MAX, i32::MIN] {
        let rules = SpacePenaltyConfig::new(vec![SpacePenaltyRule::new(TAGS, cost)]);
        let penalized = segmenter(Some(rules));
        for &(text, expected) in cases {
            assert_eq!(surfaces(&penalized, text), expected, "{text} {cost}");
            let results = penalized
                .segment_nbest(Cow::Borrowed(text), 1, false, None)
                .unwrap();
            let nbest: Vec<_> = results[0]
                .0
                .iter()
                .map(|token| token.surface.as_ref())
                .collect();
            assert_eq!(nbest, expected, "{text} {cost}: N-best");
        }
    }
}
