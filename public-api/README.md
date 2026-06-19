# Public API diff: v3.0.7 -> v4

Machine-generated public Rust API inventory and diff for the workspace crates that
expose a real public surface, captured to drive the v3 -> v4 migration guide (#724).

This is generated output, not hand-maintained docs. Regenerate it with the commands
below rather than editing the files by hand.

## Scope

| Crate | v3.0.7 baseline | v4 surface | Diff |
| --- | --- | --- | --- |
| `lindera` | `lindera.v3.0.7.txt` | `lindera.v4.txt` | `lindera.diff` |
| `lindera-dictionary` | `lindera-dictionary.v3.0.7.txt` | `lindera-dictionary.v4.txt` | `lindera-dictionary.diff` |
| `lindera-binding-core` | (new in v4 — no baseline) | `lindera-binding-core.v4.txt` | `lindera-binding-core.diff` |

`lindera-binding-core` did not exist at the `v3.0.7` tag, so its entire public surface
is new; its `.diff` is the full surface diffed against an empty baseline.

The FFI bindings (`lindera-python`, `lindera-nodejs`, `lindera-ruby`, `lindera-php`,
`lindera-wasm`) expose host-language APIs, not a Rust public API, so they are out of
scope here. Their breaking changes are enumerated directly in the migration guide
(#724).

## Key findings

- **`lindera`**: no public API differences between v3.0.7 and v4 (default features).
  The Phase 6 breaking changes live in the bindings and `lindera-binding-core`, not in
  the core `lindera` Rust API.
- **`lindera-dictionary`**: the `viterbi` internals were encapsulated (#709) —
  `EdgeType` was removed, and `Edge` / `PathEntry` / `WordEntry` / `WordId` /
  `ArchivedWordEntry` / `ArchivedWordId` no longer expose public fields; `WordEntry`
  dropped `SERIALIZED_LEN` / `serialize` / `deserialize` in favor of accessors
  (`new`, `word_cost`, `word_id`) and `WordId` gained an `id()` accessor. Added:
  `util::read_aligned_file` and the `embedded_dictionary!` macro.
- **`lindera-binding-core`**: new facade crate shared by the five bindings
  (`CoreError`/`ErrorKind`, `CoreSchema`/`CoreFieldType`/`CoreFieldDefinition`,
  `CoreMetadata`, `CoreTokenizerBuilder`/`CoreTokenizer`, `TokenView`).

## How this was generated

- Tool: `cargo public-api` 0.52.0 (`cargo install cargo-public-api --locked`).
- Toolchain: `nightly` (rustdoc JSON). Install with `rustup toolchain install nightly`.
- Simplification: `-ss` (omit blanket impls and auto-trait impls; auto-derived impls
  such as `Clone`/`Debug`/`Eq` are kept because losing one is a breaking change).
- Features: default features for each crate.
- Baseline ref: the `v3.0.7` git tag, captured in a detached `git worktree` so the
  working tree is never mutated.

### Regenerate

```sh
# v4 surfaces (run on the v4 branch)
cargo public-api -p lindera -ss               > public-api/lindera.v4.txt
cargo public-api -p lindera-dictionary -ss    > public-api/lindera-dictionary.v4.txt
cargo public-api -p lindera-binding-core -ss  > public-api/lindera-binding-core.v4.txt

# v3.0.7 baselines (isolated worktree so the main tree is untouched)
git worktree add --detach /tmp/lindera-v307 v3.0.7
( cd /tmp/lindera-v307 && cargo public-api -p lindera -ss            > /tmp/v307-lindera.txt )
( cd /tmp/lindera-v307 && cargo public-api -p lindera-dictionary -ss > /tmp/v307-lindera-dictionary.txt )
git worktree remove /tmp/lindera-v307
cp /tmp/v307-lindera.txt            public-api/lindera.v3.0.7.txt
cp /tmp/v307-lindera-dictionary.txt public-api/lindera-dictionary.v3.0.7.txt

# diffs (v3.0.7 -> v4)
diff -u public-api/lindera.v3.0.7.txt            public-api/lindera.v4.txt            > public-api/lindera.diff
diff -u public-api/lindera-dictionary.v3.0.7.txt public-api/lindera-dictionary.v4.txt > public-api/lindera-dictionary.diff
diff -u /dev/null                                public-api/lindera-binding-core.v4.txt > public-api/lindera-binding-core.diff
```

> Note: the workspace version is still `3.0.7` on the v4 branch; the bump to `4.0.0`
> happens at release time (#725). "v4" here means the Phase 6 integration branch.
