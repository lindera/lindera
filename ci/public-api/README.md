# Public API baseline of the `lindera` crate

This directory holds the public API listing of the `lindera` crate as published
in 6.2.0, captured before the crate was split into the `lindera` facade and
`lindera-segmenter` (see issue #1078). It is not part of any package.

## How the listings were generated

Run on the 6.2.0 tree (`main` @ `0482efcc`) with
[cargo-public-api](https://github.com/cargo-public-api/cargo-public-api) 0.52.0
and a nightly toolchain:

```sh
cargo +nightly public-api -p lindera --simplified > ci/public-api/lindera-6.2.0-default.txt
cargo +nightly public-api -p lindera --simplified --features train > ci/public-api/lindera-6.2.0-train.txt
```

The two files differ only in `pub use lindera::dictionary::trainer`.

## How to compare after the split

The segmenter keeps the old surface under its new crate name, so the listing of
`lindera-segmenter` must match the baseline once the crate name is mapped back.

One more spelling difference has to be folded away. `cargo public-api` prints a
type reached through a `pub use` by its canonical path
(`lindera_dictionary::dictionary::Dictionary`), whereas the 6.2.0 listing
printed the same type through the `pub type` alias
(`lindera::dictionary::Dictionary`) wherever the source named the alias, and
canonically elsewhere (`Segmenter::new`, for example). The `normalize` function
below maps the canonical path of every re-exported name to its `lindera::` path
on both sides of the comparison, so that a signature matches regardless of how
it spells a type. The `pub type` lines of the baseline are left untouched so
that they show up verbatim in the diff. GNU sed is required (`\b`, `\|`).

```sh
normalize() {
  sed -e 's/\blindera_segmenter\b/lindera/g' -e '/^pub type /!{
    s/\blindera_dictionary::LinderaResult\b/lindera::LinderaResult/g
    s/\blindera_dictionary::builder::DictionaryBuilder\b/lindera::dictionary::DictionaryBuilder/g
    s/\blindera_dictionary::dictionary::metadata::Metadata\b/lindera::dictionary::Metadata/g
    s/\blindera_dictionary::dictionary::schema::\(FieldDefinition\|FieldType\|Schema\)\b/lindera::dictionary::\1/g
    s/\blindera_dictionary::dictionary::\(Dictionary\|UserDictionary\)\b/lindera::dictionary::\1/g
    s/\blindera_dictionary::viterbi::\(Lattice\|WordId\)\b/lindera::dictionary::\1/g
    s/\blindera_dictionary::error::\(LinderaError\|LinderaErrorKind\)\b/lindera::error::\1/g
    s/\blindera_dictionary::mode::\(Mode\|Penalty\)\b/lindera::mode::\1/g
    s/\blindera_dictionary::space_penalty::\(SpacePenaltyConfig\|SpacePenaltyRule\|SpacePenaltyTable\)\b/lindera::space_penalty::\1/g
  }'
}

cargo +nightly public-api -p lindera-segmenter --simplified | normalize \
  | diff - <(normalize < ci/public-api/lindera-6.2.0-default.txt)
```

In the output, `<` lines come from the current listing and `>` lines from the
baseline. Expected result:

- Right after the rename commit: no difference.
- After the raw `lindera-dictionary` modules are re-exported under
  `lindera_segmenter::dictionary`: added lines only (`pub use` entries for
  `core`, `builder`, `error`, `loader`, `mode`, `nbest`, `space_penalty`,
  `util`, `viterbi` and the two macros).
- After the `pub type` aliases became `pub use` re-exports: the 17 `pub type`
  lines of the aliases (`LinderaResult`, nine in `dictionary`, two in `error`,
  two in `mode`, three in `space_penalty`) are replaced by `pub use` lines
  with the same paths. The three `serde_json::Value` aliases
  (`DictionaryConfig`, `UserDictionaryConfig`, `SegmenterConfig`) stay as
  `pub type`. Without `normalize`, 26 more lines would differ on both sides
  (the `load_*` functions, the `Segmenter`, `SegmentWorker` and `Token`
  signatures that mention a converted name), because the new listing spells
  those names by their canonical `lindera_dictionary::…` path; the types are
  the same, so this is a rendering difference, not an API change.

Apart from the replaced alias lines, no line may disappear from the baseline.

The facade re-exports the segmenter with `#[doc(inline)]`, which
`cargo public-api` and `cargo semver-checks` cannot see through, so those tools
are not run against the facade. The facade's own regression test is
`lindera/tests/public_paths.rs`, generated from these listings as
`use lindera::...::Item as _;` statements.

## Regenerating `lindera/tests/public_paths.rs`

The generated part of `lindera/tests/public_paths.rs` is derived from the two
listings above by a small script (kept outside the repository; the logic is
short enough to reproduce):

1. Read both listings and keep the lines that declare a module-level item:
   `pub mod`, `pub struct`, `pub enum`, `pub trait`, `pub fn`, `pub type`,
   `pub use`, `pub const` and `pub static`. Drop `impl` lines, fields and
   enum variants (`pub lindera::...::Field: ...`).
2. Take the path that follows the keyword, cut it at the first `(`, strip
   generic arguments (`<'a>`, `<T>`) and a trailing `:`. Skip `lindera`
   itself, and skip associated items, i.e. paths in which a segment before the
   last one starts with an uppercase letter (`DictionaryKind::as_str`,
   `Token::new`, `DictionaryKindIter::Item`).
3. Emit `use <path> as _;` once per path, grouped by parent module (one blank
   line between groups) and sorted inside a group. Paths that appear only in
   the `train` listing get `#[cfg(feature = "train")]`.
4. Append the hand-written sections for the paths that only exist since the
   facade: `lindera::analysis::...` under `#[cfg(feature = "analysis")]`, and
   the raw `lindera::dictionary::{core, builder, viterbi}` paths and the two
   macros. Then run `cargo fmt --all`.

`use path as _;` binds nothing, so the file also compiles with
`--no-default-features`; its only test function asserts that
`lindera::get_version()` is non-empty so that the file is always run.
