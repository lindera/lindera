# Phase 6 Handoff — v4.0.0 Breaking Changes

> This is a **self-contained** handoff for **Phase 6** of the Lindera refactoring, intended to be
> picked up by a fresh session (e.g. Claude in the IDE) with **zero prior context**. The full plan
> lives in [`REFACTORING_PLAN.md`](REFACTORING_PLAN.md); its "進捗サマリ" (progress summary) section
> records the results of the non-breaking phases.

---

## 0. TL;DR

- **Phases 0–5 are complete and merged to `main`** (non-breaking, semver patch/minor; PRs #689–#706).
- **Phase 6 is the one remaining bucket: all the breaking changes (semver major = v4.0.0)** that the
  earlier phases deliberately deferred because they could not ship without a major bump.
- Five work items remain: full-facade bindings (5-1), viterbi encapsulation (5-2), API-parity
  fixes (5-3), removal of compatibility shims (5-4), and a v3→v4 migration guide (5-5).
- Because the blast radius reaches downstream projects (`lindera-tantivy`, `lindera-sqlite`, the
  language packages), **cut a `v4.0.0-alpha` and validate before the final release.**

All file paths, line numbers, and type claims in this document were verified against `main` at
handoff time. Treat line numbers as a starting hint, not gospel — re-`grep` before editing.

---

## 1. Repository map (the minimum you need)

Workspace (`Cargo.toml` → `[workspace]`, `edition = "2024"`, `version = "3.0.7"`), 16 members:

```text
lindera-crf            … CRF training (train feature only)
lindera-dictionary     … analysis engine core (viterbi, builder, assets, loader, trainer)
lindera-ipadic / -ipadic-neologd / -unidic / -ko-dic / -cc-cedict / -jieba
                       … dictionary crates (defined via the embedded_dictionary! macro)
lindera                … public facade (segmenter, tokenizer, token, token_filter, character_filter)
lindera-cli            … CLI (subcommands split under commands/)
lindera-binding-core   … ★ added in Phase 4. FFI-independent shared logic (TokenView, schema)
lindera-python / -php / -ruby / -nodejs / -wasm … language bindings
```

Phase 6's main battlefields: **`lindera-binding-core` / the bindings / `lindera-dictionary/src/viterbi.rs`**.

---

## 2. Development environment (read this first)

### 2.1 Dictionary downloads (the classic foot-gun)

When an `embed-*` feature is enabled, the dictionary crates **download a dictionary archive from
`https://lindera.dev/...` at build time.** In environments where a network policy blocks
`lindera.dev`, the build fails.

Workaround: each dictionary archive is **byte-for-byte (and MD5) identical to the tagged source
archive on the GitHub mirror.** Pre-populate the `LINDERA_DICTIONARIES_PATH` cache and the build
works offline:

```sh
# Cache layout: $LINDERA_DICTIONARIES_PATH/<workspace-version>/<archive>.tar.gz
# <workspace-version> tracks Cargo.toml's version (currently 3.0.7; on the v4 branch it becomes 4.0.0).
mkdir -p "$HOME/.lindera_cache/3.0.7"
cd "$HOME/.lindera_cache/3.0.7"

# IPADIC (other dictionaries follow the same pattern from their lindera/mecab-* repo tags)
curl -sL -o mecab-ipadic-2.7.0-20250920.tar.gz \
  https://github.com/lindera/mecab-ipadic/archive/refs/tags/2.7.0-20250920.tar.gz
curl -sL -o mecab-ko-dic-2.1.1-20180720.tar.gz \
  https://github.com/lindera/mecab-ko-dic/archive/refs/tags/2.1.1-20180720.tar.gz
curl -sL -o mecab-jieba-0.1.1.tar.gz \
  https://github.com/lindera/mecab-jieba/archive/refs/tags/0.1.1.tar.gz

# Confirm each file's md5sum matches the md5_hash in the corresponding build.rs
export LINDERA_DICTIONARIES_PATH="$HOME/.lindera_cache"
```

The authoritative URL and hash for each dictionary live in `lindera-<dict>/build.rs`'s `FetchParams`
(`file_name` / `download_urls` / `md5_hash`). In CI, `lindera.dev` is reachable, so this workaround
is unnecessary there.

### 2.2 Verification commands (required on every PR)

```sh
export LINDERA_DICTIONARIES_PATH="$HOME/.lindera_cache"   # offline environments only

# Golden tests (tokenization snapshots — the safety net that mechanically detects behavior changes)
cargo test -p lindera --features embed-ipadic,embed-ko-dic,embed-jieba,train --test golden_tokenization
# lindera core + CLI
cargo test -p lindera --features embed-ipadic,embed-ko-dic,train
cargo test -p lindera-cli --features train,embed-ipadic
# Bindings: the Rust side is testable without the FFI toolchain
cargo test -p lindera-python --lib   # nodejs / wasm / etc. also work with --lib
cargo check -p lindera-php -p lindera-ruby

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

> **Phase 6 is breaking, so there are legitimate cases to *update* the golden snapshots** (when you
> intentionally change the output spec). In that case run
> `INSTA_UPDATE=always cargo test ... --test golden_tokenization`, and **review the diff in the PR**.
> For changes that are *not* meant to alter behavior, never update the snapshots — a diff there is a bug.

### 2.3 Benchmarks (only when touching hot paths)

See [`BENCHMARKING.md`](BENCHMARKING.md). **Key lesson (Phase 3d):** `cargo bench`'s default profile
uses `lto = false`, which optimizes differently from production (`[profile.release] lto = true`). Any
performance judgment on the hot path (viterbi / segmenter) **must be made on a production-equivalent
build:**

```toml
# Add to Cargo.toml temporarily, for measurement only
[profile.bench]
lto = true
codegen-units = 1
```

Compare before (`--save-baseline`) and after (`--baseline`) **on the same machine**, and confirm the
delta stays within 3%.

---

## 3. Foundation Phase 6 builds on

- **Safety nets**:
  - `lindera/tests/golden_tokenization.rs` — IPADIC/ko-dic/Jieba × Normal/Decompose, plus user
    dictionary and N-best (8 snapshots total).
  - `lindera-cli/tests/cli.rs` — 8 smoke tests.
- **`lindera-binding-core` exists** (`src/lib.rs`, `src/token.rs`, `src/schema.rs`):
  - `TokenView::from_token(lindera::token::Token)` — consumed by all 5 bindings.
  - `schema::default_dictionary_fields()` / `schema::validate_record()` — consumed by
    Python / PHP / Ruby / Node.js.
- **The deferred breaking items** are exactly the contents of §5 below.

---

## 4. Goals and guiding principles

**Goal**: the "most beautiful end state" — reduce every binding to a **thin FFI translation layer**,
with all logic consolidated behind the `lindera-binding-core` facade. Along the way, fix the public
API inconsistencies and encapsulation leaks that were preserved for behavior compatibility.

**Principles**:

1. **Consolidate everything into a single major (v4.0.0).** Cut alpha/beta and validate with
   downstream projects.
2. **Semver**: bump the workspace `version` to `4.0.0`. Bump each language package
   (PyPI / npm / gem / Packagist) major in lockstep.
3. **Breaking changes are intentional but minimal.** Unlike Phases 0–5, updating golden snapshots is
   sometimes valid — but every break must be explicit and justified.
4. **Keep PRs small. Do not edit `REFACTORING_PLAN.md` (or this doc) inside code PRs** — that caused
   repeated merge conflicts across Phases 0–5. Batch doc/plan updates into a single PR at milestones.

---

## 5. Task details

Each task lists its **current state**, **what to do**, **why it is breaking**, and a
**definition of done (DoD)**.

### 5-1. Full facade — expand `lindera-binding-core`

**Current state**: each binding's `tokenizer.rs` / `schema.rs` / `metadata.rs` is tightly coupled to
its FFI class (`#[pyclass]` / `#[napi]` / magnus / wasm-bindgen); most methods either delegate to
`inner.method()` or expose public attributes. Phase 4 lifted only the "pure logic extractable without
a break" (token extraction, schema default/validate) into core.

**What to do**: add the following to `lindera-binding-core`, reducing each binding to a thin adapter:

- `CoreTokenizerBuilder` / `CoreTokenizer` — own the build-flow orchestration (`set_mode` /
  `set_dictionary` / `set_user_dictionary` / `append_*_filter` / `build` / `tokenize` /
  `tokenize_nbest`). Each binding keeps only "FFI type ⇔ `serde_json::Value`" conversion and the thin
  `#[pyclass]`-style wrapper.
- `CoreSchema` / `CoreFieldType` / `CoreFieldDefinition` — own schema field management, the index map,
  `get_field_by_name`, etc. (Phase 4 covered only default/validate.)
- `CoreMetadata` — own default values and schema wiring.
- `CoreError` + `ErrorKind` — a shared error type. Each binding keeps a single
  `From<CoreError>`-to-native-exception impl.

**Do NOT move value conversion into core**: `serde_json::Value` ⇔ FFI type
(`PyObject` / `Zval` / magnus `Value` / `JsValue`) is inherently FFI-dependent. Keep it in each
binding's `util.rs` / `convert.rs` (extracting a trait is fine; moving the impls is not).

**Why it is breaking**: e.g. Python's `schema.fields` is currently a `#[pyo3(get)]` **attribute**.
Wrapping it as `inner: CoreSchema` turns it into a getter, changing the consumer-facing API
(attribute → method/property). Each language's public surface shifts subtly, so it can only ship in a
major.

**Recommended approach**: settle the design on the largest, most complete binding (`lindera-python`)
first, then port the rest. Validate each port with `cargo test -p <binding> --lib` plus the language
test target (`make test-lindera-<binding>`).

**DoD**: every binding's tokenizer/schema/metadata logic lives in core; bindings contain only FFI
glue; `--lib` tests and language tests are green for all five bindings.

### 5-2. Encapsulate viterbi internal structs

The internal structs in `lindera-dictionary/src/viterbi.rs` expose all fields as `pub`, leaking
implementation details. Current locations on `main`:

| Struct | Line | Public fields → (accessor or private) |
| --- | --- | --- |
| `WordId` | 51 | `id`, `is_system`, `lex_type` |
| `WordEntry` | 104 | `word_id`, `word_cost`, `left_id`, `right_id`; plus `SERIALIZED_LEN` / `serialize` / `deserialize` |
| `Edge` | 161 | `edge_type`, `word_entry`, `path_cost`, `left_index`, `start_index`, `stop_index`, `kanji_only` |
| `PathEntry` | 184 | `edge_index`, `left_pos`, `left_index`, `cost` |
| `Lattice` | 196 | internal buffers |

**What to do**: make the fields non-`pub` and provide accessors (`#[inline] pub fn id(&self) -> u32`,
etc.). Demote the `WordEntry` serialization details (`SERIALIZED_LEN`, `serialize`, `deserialize`) to
`pub(crate)`.

**Caution (Phase 3d lesson)**: viterbi is **extremely optimization-sensitive.** When converting
fields to accessors, omitting `#[inline]` can drop inlining and regress performance. **Confirm within
3% on a production-equivalent LTO benchmark (§2.3).** Also, `lindera` / `lindera-cli` / benches / tests
access fields directly (`token.word_id.id`, etc.), so the accessor change requires call-site fixes —
flush them out with `grep` (`word_id.id`, `.is_unknown()`, `.word_cost`, …).

**DoD**: no `pub` fields remain on these structs (serialization helpers are `pub(crate)`); all call
sites compile via accessors; production-LTO bench delta ≤ 3%.

### 5-3. Resolve API inconsistencies

Inconsistencies found in Phases 0–5 and preserved to keep behavior stable:

1. **`Token.details` type differs across bindings** (verified):
   - Python / Ruby / Node.js: `Option<Vec<String>>`
   - PHP / WASM: `Vec<String>`
   - → Unify. **Recommended: drop `Option`, always `Vec` (empty → empty array; null-safe).**

2. **Default schema field names differ** (verified):
   - Python/PHP/Ruby/Node.js `Schema.create_default()` (via
     `lindera-binding-core/src/schema.rs::default_dictionary_fields()`): `middle_pos` / `small_pos` /
     `fine_pos`.
   - `lindera::dictionary::Schema::default()`
     (`lindera-dictionary/src/dictionary/schema.rs`, used by WASM): `pos_detail_1` / `pos_detail_2` /
     `pos_detail_3`.
   - → Unify. The doc comment in `lindera-binding-core/src/schema.rs` records the history. Converging
     on the core `pos_detail_*` is the natural choice, but it affects existing users' dictionary-schema
     expectations, so call it out explicitly in the migration guide.

3. **Builder return type / feature parity differ**:
   - Python is fluent (`PyRefMut<Self>`); the others mutate in place.
   - The presence of the `character_filter` / `token_filter` / `segmenter` modules varies by language
     (e.g. PHP and Node.js lack `segmenter`). → Unify as part of the full facade (5-1).

**DoD**: `details` has one type across all bindings; the default schema field names are identical
across core and bindings; builder ergonomics and module surface are consistent; the migration guide
documents each break.

### 5-4. Remove compatibility shims and legacy formats

- **`LINDERA_CACHE` env var (deprecated)** in `lindera-dictionary/src/assets.rs` — currently still
  honored with a deprecation warning (see `assets.rs:225`, `:234`, `:568`). Remove it and standardize
  on `LINDERA_DICTIONARIES_PATH`.
- **User-dictionary "5-bit variant-count encoding" legacy compatibility** (in `viterbi.rs`; `grep` for
  the current location) — decide its fate. Inspect the generation of dictionary binaries in the wild;
  remove if safe, or provide a migration tool if still needed.
- **Legacy name aliases** retained for compatibility in Phase 2 (e.g. `EmbeddedIPADICLoader`-style
  aliases, if any survive) — clean up.

**DoD**: `LINDERA_CACHE` no longer referenced anywhere; the legacy-format decision is documented and
implemented (removed or migration-tooled); no stale aliases remain.

### 5-5. Migration guide

- Add a **v3 → v4 migration guide** to `docs/` (mdBook; both languages: `docs/src/` and
  `docs/ja/src/`). Update `SUMMARY.md` for both.
- Use `cargo public-api` (or similar) to mechanically diff the v3 vs v4 public API, and keep the guide
  in sync with that diff.
- For each language package (Python / Node.js / Ruby / PHP / WASM), enumerate the breaking points for
  users: attribute → method, `details` type, schema names, builder ergonomics.
- Run `markdownlint-cli2 "docs/src/**/*.md"` to zero out lint warnings.

**DoD**: migration guide merged in both languages; the public-API diff is reconciled against the guide;
markdownlint passes.

---

## 6. How to proceed

1. **Branch**: cut a dedicated feature branch (e.g. `claude/phase6-v4`). Never push to `main` directly.
2. **One concern per PR**: keep changes small (e.g. "viterbi encapsulation", "CoreSchema port —
   Python"). Each PR green on `cargo fmt --check` / `clippy -D warnings` / the relevant tests.
3. **CI**: `regression.yml` runs on PRs. The bindings' multi-platform builds are slow, so split PRs and
   validate them individually. `release.yml` does not run on PRs (tag / dispatch only).
4. **Network-caused CI failures**: `curl` to crates.io occasionally fails with `SSL_ERROR_SYSCALL` /
   `schannel ... close_notify`. This is unrelated to the code — just **re-run the failed jobs**.
5. **Conflict avoidance**: keep doc/plan updates (`REFACTORING_PLAN.md`, this file) out of code PRs.
   Mixing them in caused repeated merge conflicts across Phases 0–5.
6. **Alpha release**: after the main tasks land, publish `v4.0.0-alpha` and verify with
   `lindera-tantivy` / `lindera-sqlite` before the official v4.0.0.

---

## 7. Pitfalls and proven knowledge (validated in Phases 0–5)

- **Splitting viterbi into modules alone regresses ~5% even on a production LTO build** (measured in
  Phase 3d — that's why it stayed a single file). Phase 6 encapsulation needs `#[inline]` plus a
  production-equivalent bench. A "just make it cleaner" change can break performance.
- **A local composite action (`uses: ./...`) only resolves after `actions/checkout`.** If you touch CI,
  keep `checkout` in each job.
- **Dictionary archives == GitHub mirror tagged sources (MD5-identical)** — this is what makes offline
  verification possible (§2.1).
- **Bindings pass `cargo check` / `cargo test --lib` without the FFI toolchain**, so pure-logic
  verification is fully doable with plain cargo (leave the full FFI build to CI).
- **`cargo bench`'s default is non-LTO.** Judging production performance requires the temporary
  `[profile.bench] lto = true` (§2.3).

---

## 8. Recommended order

1. **5-2 viterbi encapsulation** first — well-bounded, self-contained within `lindera-dictionary`,
   safe to confirm via bench.
2. Then **5-1 full facade**, starting with Python (settle the design on the largest binding, then fan
   out).
3. **5-3 API inconsistencies** are absorbed into 5-1 as you go (schema / metadata / details / builder).
4. **5-4 shim removal** is independent — do it whenever convenient.
5. **5-5 migration guide** last, driven by the `cargo public-api` diff.
6. Validate the whole thing as `v4.0.0-alpha` with downstream projects → official release.
