//! A reused `Lattice` must not leak state between `Segmenter`s built from
//! different dictionaries.
//!
//! `Lattice::char_category_cache` used to memoize ASCII `CategoryId`s keyed
//! only by codepoint, with no identity of the `CharacterDefinition` that
//! populated it, and it was never cleared. `CategoryId`s are allocated in
//! char.def declaration order, which differs between dictionaries (IPADIC:
//! `DEFAULT, SPACE, KANJI, SYMBOL, NUMERIC, ALPHA, ...`; ko-dic:
//! `DEFAULT, SPACE, HANJA, KANJI, SYMBOL, NUMERIC, ALPHA, ...`), so reusing a
//! `Lattice` across dictionaries silently reinterpreted a cached ASCII
//! category under the second dictionary's category table.
//!
//! This test tokenizes the same ASCII-containing text with an IPADIC
//! `Segmenter` and then a ko-dic `Segmenter`, through one reused `Lattice`,
//! and asserts each result equals what a fresh `Lattice` produces.

#![cfg(all(feature = "embed-ipadic", feature = "embed-ko-dic"))]

use std::borrow::Cow;

use lindera::dictionary::{Lattice, load_dictionary};
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

const TEXT: &str = "Hello 世界";

/// Extracts `(surface, first POS detail)` per token, since the bug this
/// test guards against changes the unknown-word POS tag looked up for an
/// ASCII character (IPADIC's `ALPHA` = `CategoryId(5)`, misread as ko-dic's
/// `NUMERIC` = `CategoryId(5)` in ko-dic's own table) without necessarily
/// changing token boundaries -- ko-dic's `NUMERIC`/`ALPHA` share identical
/// `invoke`/`group`/`length` char.def flags, so only the POS tag (`SL` vs
/// `SN`) reveals the misattribution, not `surface` alone.
fn surfaces_and_pos(mut tokens: Vec<lindera::token::Token>) -> Vec<(String, String)> {
    tokens
        .iter_mut()
        .map(|t| {
            let pos = t.get_detail(0).unwrap_or("").to_string();
            (t.surface.to_string(), pos)
        })
        .collect()
}

#[test]
fn test_lattice_reuse_across_dictionaries_matches_fresh_lattice() {
    let ipadic_segmenter = Segmenter::new(
        Mode::Normal,
        load_dictionary("embedded://ipadic").unwrap(),
        None,
    );
    let ko_dic_segmenter = Segmenter::new(
        Mode::Normal,
        load_dictionary("embedded://ko-dic").unwrap(),
        None,
    );

    // Fresh-`Lattice` baseline for each dictionary.
    let mut fresh_ipadic_lattice = Lattice::default();
    let expected_ipadic = ipadic_segmenter
        .segment_with_lattice(Cow::Borrowed(TEXT), &mut fresh_ipadic_lattice)
        .unwrap();

    let mut fresh_ko_dic_lattice = Lattice::default();
    let expected_ko_dic = ko_dic_segmenter
        .segment_with_lattice(Cow::Borrowed(TEXT), &mut fresh_ko_dic_lattice)
        .unwrap();

    // Reused `Lattice` across both dictionaries, IPADIC first.
    let mut shared_lattice = Lattice::default();
    let actual_ipadic = ipadic_segmenter
        .segment_with_lattice(Cow::Borrowed(TEXT), &mut shared_lattice)
        .unwrap();
    let actual_ko_dic = ko_dic_segmenter
        .segment_with_lattice(Cow::Borrowed(TEXT), &mut shared_lattice)
        .unwrap();

    assert_eq!(
        surfaces_and_pos(actual_ipadic),
        surfaces_and_pos(expected_ipadic)
    );
    assert_eq!(
        surfaces_and_pos(actual_ko_dic),
        surfaces_and_pos(expected_ko_dic)
    );
}
