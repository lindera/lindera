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
`lindera-segmenter` must match the baseline once the crate name is mapped back:

```sh
cargo +nightly public-api -p lindera-segmenter --simplified \
  | sed 's/\blindera_segmenter\b/lindera/g' \
  | diff - ci/public-api/lindera-6.2.0-default.txt
```

Expected result:

- Right after the rename commit: no difference.
- After the raw `lindera-dictionary` modules are re-exported under
  `lindera_segmenter::dictionary`: added lines only (`pub use` entries for
  `core`, `builder`, `error`, `loader`, `mode`, `nbest`, `space_penalty`,
  `util`, `viterbi` and the two macros).
- After the `pub type` aliases became `pub use` re-exports: the `pub type`
  lines of the aliases are replaced by `pub use` lines. The three
  `serde_json::Value` aliases (`DictionaryConfig`, `UserDictionaryConfig`,
  `SegmenterConfig`) stay as `pub type`.

No line may disappear from the baseline.

The facade re-exports the segmenter with `#[doc(inline)]`, which
`cargo public-api` and `cargo semver-checks` cannot see through, so those tools
are not run against the facade. The facade's own regression test is
`lindera/tests/public_paths.rs`, generated from these listings as
`use lindera::...::Item as _;` statements.
