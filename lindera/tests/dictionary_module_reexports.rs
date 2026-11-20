//! The modules of `lindera-dictionary` added in v7.0.0 whose types appear in
//! public items of `lindera` are reachable through `lindera::dictionary`, as
//! the older modules are: `LatticeOptions` has fields of the types
//! `BreakCharacters` and `WhitespaceClassifier`, and `Segmenter` a field of
//! the type `BreakCharacters`. Without the re-exports, this test does not
//! compile.

use lindera::dictionary::break_characters::{BreakCharacters, DEFAULT_BREAK_CHARACTERS};
use lindera::dictionary::viterbi::LatticeOptions;
use lindera::dictionary::whitespace::WhitespaceClassifier;

/// Names the types of the two fields of `LatticeOptions` that come from the
/// modules added in v7.0.0.
fn new_field_types<'a>(
    options: &LatticeOptions<'a>,
) -> (
    Option<&'a BreakCharacters>,
    Option<&'a WhitespaceClassifier>,
) {
    (options.break_characters, options.skip_whitespace)
}

#[test]
fn break_characters_and_whitespace_are_reachable_through_lindera_dictionary() {
    let defaults = BreakCharacters::new(DEFAULT_BREAK_CHARACTERS);
    assert!(defaults.contains('・'));
    assert!(!BreakCharacters::new("").contains('・'));
    // `new_field_types` compiles only when both field types can be named
    // through `lindera::dictionary`.
    let _ = new_field_types;
}
