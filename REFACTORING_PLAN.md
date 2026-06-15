# Lindera Codebase-Wide Refactoring Plan

This document is a phased refactoring plan based on a survey of the entire workspace (18 crates,
~177 Rust source files). Each phase is split into an independently mergeable unit, and the overriding
principle is to **proceed incrementally while always keeping CI green**.

---

## Progress summary (non-breaking phases complete)

The semver non-breaking (patch/minor) Phases 0–5 are merged. Breaking changes are consolidated into
Phase 6 (v4.0.0).

| Phase | Content | Result |
| --- | --- | --- |
| 0 | Safety net (8 golden + 8 CLI smoke + bench baseline procedure) | ✅ |
| 1 | Immediate cleanup (exclude ~14 MB of artifacts, `VERSION` typo, unused deps, `bocchan` duplication) | ✅ |
| 2 | De-duplicate the 6 dictionary crates (`embedded_dictionary!` macro + build helper, 786 → 160 lines) | ✅ |
| 3a | Unify the 4 tag filters (`token_filter/tags.rs`) | ✅ |
| 3b | Consolidate the loader's `read_aligned_file` | ✅ |
| 3c | Eliminate `unwrap` on the dictionary-build path + move feature_extractor regexes to `LazyLock` | ✅ (3c-1/3c-6 deferred) |
| 3d | Split oversized files | **viterbi deferred — ~5% regression on a production-equivalent LTO bench** (verified) |
| 4 | Shared binding layer (`lindera-binding-core`: `TokenView`, schema default/validate) | ✅ (4-1/4-2). Full facade → Phase 6 |
| 5 | Build infra (Makefile loops, setup-rust composite, reusable test-crate, CLI split) | ✅ (5-1/5-2/5-3/5-5). 5-4 deferred |
| 6 | Public-API redesign / breaking changes (v4.0.0) | Not started |

**Key engineering findings**: (1) viterbi regresses in production builds because module splitting alone
changes code generation → keep it a single file. (2) The bindings are tightly FFI-coupled, so the pure
logic extractable without a break is limited → the full facade is a breaking change deferred to
Phase 6. (3) A local composite action only resolves after checkout → keep checkout in each job.

---

## Overview of the technical debt found in the survey

### A. The 6 dictionary crates are near-complete copy-paste clones

The 6 crates `lindera-ipadic` / `lindera-ipadic-neologd` / `lindera-unidic` / `lindera-ko-dic` /
`lindera-cc-cedict` / `lindera-jieba` have `build.rs` (41 lines), `src/lib.rs` (9 lines), and
`src/embedded.rs` (88–96 lines) that are **>97% identical**.

- What actually differs: only the dictionary URL, MD5 hash, archive name, dummy input, and feature
  flag name (the only logic difference is jieba's single `src_subdir: Some("dict-src")`).
- ~590 lines of duplication in total. The varying macro/struct names (`EmbeddedIPADICLoader`, etc.) are
  unnecessary variation.
- A `VERERSION` typo (should be `VERSION`) has propagated by copy-paste across all 6 crates plus
  `lindera-dictionary/src/lib.rs:17` and `lindera/src/lib.rs:16`.
- All 6 crates declare `anyhow` / `byteorder` / `csv` as regular dependencies, but they are unused
  (needed only as build-dependencies).

### B. The 5 language bindings have zero shared layer

`lindera-python` / `lindera-php` / `lindera-ruby` / `lindera-nodejs` / `lindera-wasm` (~11,750 lines
total) each independently re-implement the same wrappers.

| Module | Python | PHP | Ruby | Node.js | WASM | Logic similarity |
| --- | --- | --- | --- | --- | --- | --- |
| tokenizer.rs | 361 | 310 | 313 | 239 | 548 | 80–90% |
| schema.rs | 579 | 509 | 641 | 430 | 452 | 85%+ |
| metadata.rs | 447 | 389 | 433 | 425 | 197 | 80%+ |
| util/convert (value conversion) | 115 | 87 | 159 | 53 | 0 | 70–75% |

- Estimated **2,000+ lines** of duplication. Any core API change requires following up in 5 places.
- API inconsistencies have already appeared:
  - `Token.details` is `Vec<String>` in PHP but `Option<Vec<String>>` elsewhere.
  - The builder return type is fluent in Python (`PyRefMut<Self>`) but in-place mutation elsewhere.
  - Error handling follows 5 different patterns (class / function helper / JsValue wrap).
  - The presence of the `character_filter` / `token_filter` / `segmenter` modules varies per binding
    (PHP and Node.js lack `segmenter`).

### C. Core-crate bloat and duplication

- **7 files exceed 500 lines**:
  - `lindera/src/segmenter.rs` — 1,882 lines
  - `lindera-dictionary/src/trainer/model.rs` — 1,248 lines
  - `lindera-dictionary/src/viterbi.rs` — 1,147 lines (`Lattice::set_text()` is a ~220-line giant)
  - `lindera-dictionary/src/trainer.rs` — 971 lines
  - `lindera-dictionary/src/trainer/config.rs` — 793 lines
  - `lindera-dictionary/src/builder/prefix_dictionary.rs` — 706 lines
  - `lindera/src/token_filter/japanese_number.rs` — 1,257 lines
- **The 4 tag filters duplicate each other**: `japanese_keep_tags.rs` (479) / `japanese_stop_tags.rs`
  (437) / `korean_keep_tags.rs` (384) / `korean_stop_tags.rs` (446) are the same logic with the
  keep/stop boolean inverted. ~800 lines → consolidatable to ~200.
- **Boilerplate duplication in the loaders**: the 5 files under `lindera-dictionary/src/loader/`
  repeat the same `load()` / `load_mmap()` pattern (~200 lines → 80). The `DictionaryLoader` trait in
  `loader.rs` is defined but almost unused.
- **`lindera/src/dictionary.rs`**: 12 conditional imports for 6 dictionaries × `#[cfg(feature)]`, and
  the same 6-way `#[cfg(any(...))]` condition is repeated 3+ times. Lines 217–245 leave commented-out
  error handling (for all 6 dictionaries) lying around.

### D. Inconsistent error handling and scattered `unwrap()`

- `unwrap`/`expect` totals: lindera 384 / lindera-dictionary 136 / lindera-crf 37+ (many in tests, but
  a fair number in non-test code too).
- Non-test problem sites:
  - `lindera-dictionary/src/builder.rs:60,73,89,101,148` — chained `.unwrap()` on builder calls.
  - `lindera-dictionary/src/builder/prefix_dictionary.rs` — 13 `unwrap()` on CSV field parsing.
  - `lindera-dictionary/src/trainer/feature_extractor.rs` — 3 `Regex::new().unwrap()` plus 9+ `unwrap()`
    on captures.
  - `lindera-dictionary/src/assets.rs:257,262` — `unwrap()` on environment variables.
- Mixing `LinderaError` (`LinderaErrorKind`, 41 variants) with `anyhow` gives an inconsistent strategy.
- The CLI repeats the `.map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?` pattern
  15+ times.

### E. Insufficient public-API encapsulation

- The internal structs in `lindera-dictionary/src/viterbi.rs` have all fields `pub`:
  `WordId { id, is_system, lex_type }`, `WordEntry`, `Edge { path_cost, ... }`, `PathEntry`, and even
  `Lattice`'s internal buffers are exposed. Consumers can depend on implementation details.
- `WordEntry`'s serialization details (`SERIALIZED_LEN`, etc.) are also public.

### F. Build infra / CI / repository hygiene

- **Makefile (458 lines)**: clean/format/lint/test/build targets for 13 crates are written out by hand,
  nearly identical.
- **GitHub Actions (2,028 lines total)**:
  - `release.yml` (1,103 lines) and `regression.yml` (495 lines) each duplicate ~13 nearly identical
    test jobs.
  - The Ruby/PHP/Node.js release jobs are >90% clones including the platform matrix.
  - Version detection via `cargo metadata | jq` is repeated in 5+ places.
- **Committed artifacts**: ~21 MB of mdBook-generated HTML/JS in `docs/book/` and `docs/ja/book/`
  (`mermaid.min.js` 2.9 MB × 2, `searchindex.js` 1.6 MB, etc.) are under git. `.gitignore` is not set up.
- `resources/bocchan.txt` (308 KB) is duplicated inside the repository.
- Backward-compat shims: deprecated support for the `LINDERA_CACHE` env var (`assets.rs:238`), and the
  user dictionary's "5-bit variant-count encoding" legacy-format compatibility (`viterbi.rs:409,953`).

### G. Test setup

- 278 inline tests total. Zero integration tests under a `tests/` directory.
- The 6 dictionary crates and the CLI have zero tests.
- As a refactoring safety net, a "tokenization-result snapshot (golden) test" is absent.

---

## Refactoring principles

1. **Do not mix behavior-changing and behavior-preserving refactoring.** Each PR is one or the other.
2. **Build the safety net first, in Phase 0.** Do not touch viterbi / segmenter without golden tests
   and benchmark baselines.
3. **Respect semver.** Phases 1–5 are non-breaking (patch/minor); breaking changes are consolidated
   into Phase 6 (v4.0.0).
4. Split each phase's PRs into **small, reviewable sizes** (rule of thumb: ±1,000 lines of diff or less).

---

## Phase 0: Build the safety net (no behavior change) — **done**

**Goal**: reach a state where every subsequent phase can mechanically verify "nothing is broken".

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 0-1 | Add golden (snapshot) tests | 8 snapshots (`insta`) in `lindera/tests/golden_tokenization.rs`: IPADIC / ko-dic / Jieba × Normal/Decompose + user dictionary + N-best. UniDic / NEologd / CC-CEDICT can be added via the same `golden_tests!` macro where the dictionaries are available | ✅ |
| 0-2 | CLI smoke tests | `lindera-cli/tests/cli.rs` (`assert_cmd`): help / version / list / invalid-dictionary error + mecab/wakati/json/decompose output checks with `embed-ipadic`. Changed the Makefile and CI CLI tests to `--features train,embed-ipadic` | ✅ |
| 0-3 | Record benchmark baselines | Documented in `BENCHMARKING.md`: same-machine comparison via criterion's `--save-baseline` / `--baseline` and the 3% judgment criterion | ✅ |
| 0-4 | Introduce coverage measurement (optional) | Record current values with `cargo llvm-cov` and watch for per-phase regressions | Not started (optional) |

- **Risk**: nearly none (additive only).
- **Done when**: golden tests for all dictionaries run green in CI.
- **Note**: each dictionary archive's tagged source archive on the GitHub mirror
  (`lindera/mecab-ipadic`, etc.) is MD5-identical to the lindera.dev distribution. Placing it in the
  `LINDERA_DICTIONARIES_PATH` cache directory makes builds possible even offline.

---

## Phase 1: Low-risk immediate cleanup (no behavior change) — **done**

**Goal**: sweep away the indisputable waste first to reduce diff noise in later phases.

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 1-1 | Exclude `docs/book/` and `docs/ja/book/` from git | `.gitignore` addition + `git rm -r --cached` (272 files, ~14 MB). Confirmed the committed artifacts are unused: `deploy-docs.yml` builds mdBook in CI and publishes to gh-pages | ✅ |
| 1-2 | Fix the `VERERSION` typo | Changed to `VERSION` across all 9 crates (6 dictionaries + `lindera-dictionary` + `lindera` + `lindera-cli`). Non-breaking since it is a private const | ✅ |
| 1-3 | Remove commented-out code | Removed the dead comments for all 6 dictionaries inside `resolve_embedded_loader` in `lindera/src/dictionary.rs` | ✅ |
| 1-4 | Remove unused dependencies | Removed `anyhow` / `byteorder` / `csv` / `serde_json` from the `[dependencies]` of the 6 dictionary crates (only `lindera-dictionary` retains them; build-dependencies unchanged) | ✅ |
| 1-5 | De-duplicate `bocchan.txt` | Removed the orphaned copies (zero references) in `lindera-nodejs/resources/` and `lindera-python/resources/`. Unified on `resources/bocchan.txt` | ✅ |
| 1-6 | Unify Cargo.toml formatting | Removed stray whitespace in `lindera-cc-cedict`, added a feature comment in `lindera-jieba` | ✅ |

- **Risk**: minimal. Only 1-1 needs a check of the docs deploy flow.
- **Done when**: `cargo build --workspace` / all tests / docs deploy are green.

---

## Phase 2: De-duplicate the 6 dictionary crates (no behavior change) — **done (2-1 to 2-3)**

**Goal**: consolidate the ~590 lines × 6 crates of structural duplication into a single declarative
definition.

### Design approach

Add the following to `lindera-dictionary` and reduce each dictionary crate to "parameter definitions
only":

1. **A `decl_dictionary!` macro (or shared function)** to generate `src/embedded.rs`. Drop the current
   name variations (`ipadic_data!`, `EmbeddedIPADICLoader`, etc.) and generate the data include with a
   generic `EmbeddedLoader` (holding the dictionary name as a field) plus a macro.
2. **Shared `build.rs`** reduced to 3–5 lines that just call
   `lindera_dictionary::assets::build_dictionary(FetchParams)`. Since `FetchParams` already has a shared
   implementation, each crate's build.rs becomes only constant definitions.

### Work items

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 2-1 | Implement the shared macro in `lindera-dictionary` | Added `#[macro_export] embedded_dictionary!($dir, $loader)` to `lindera-dictionary/src/macros.rs`. The `include_bytes!` path is injected via `$dir`, the loader struct name via `$loader`. Dropped the unreachable `#[cfg(not(feature))]` empty-array branch | ✅ |
| 2-2 | Replace the 6 crates' `embedded.rs` with macro calls | 88–100 lines per crate → 1 macro line (+ doc comment) | ✅ |
| 2-3 | Replace the 6 crates' `build.rs` with a shared function call | Added `build_embedded_dictionary(embed_enabled, FetchParams)` to `lindera-dictionary/src/assets.rs`. Each build.rs becomes a `FetchParams` definition + 1-line call (41 → 18 lines). jieba's `src_subdir: Some("dict-src")` is absorbed by `FetchParams`. Also removed the now-unneeded `serde_json` build-dependency | ✅ |
| 2-4 | Tidy the feature branching in `lindera/src/dictionary.rs` | Consolidate the 12 conditional imports and the 3× repeated `#[cfg(any(...))]` into a dictionary-registry-style macro | Not started (separate PR) |
| 2-5 | Preserve public name compatibility | Public names like `EmbeddedIPADICLoader` are generated with the same name by the macro, so it is **fully compatible** (no aliases needed). Call sites in `lindera/src/dictionary.rs` are unchanged | ✅ (compatibility preserved) |

- **Risk**: medium. Watch for feature-flag combinatorial explosion. CI must pass each `embed-*` feature
  individually plus the `embed-cjk`-family build matrix.
- **Done when**: builds and golden tests are green for all feature combinations; each dictionary crate's
  implementation is ≤ 20 lines.
- **Result**: embedded.rs + build.rs across 12 files went 786 → 160 lines. Net ~500-line reduction even
  after subtracting the shared implementation (macros.rs ~100 lines + assets helper ~29 lines). The 8
  golden tests + 111 unit tests unchanged, fmt/clippy clean. Local verification ran on
  ipadic/ko-dic/jieba (using the cache); the remaining 3 dictionaries (unidic/cc-cedict/neologd) use the
  same macro call and are fully verified in CI.
- **Scale**: medium (2-1 to 2-3 in one PR; 2-4 is a separate PR as facade tidying).

---

## Phase 3: Improve core-crate internal quality (no behavior change)

**Goal**: resolve duplication, oversized files, and `unwrap` in `lindera` / `lindera-dictionary` /
`lindera-crf`.
**Prerequisite**: the Phase 0 golden tests and bench baselines are mandatory.

### 3a. Unify the tag filters — **done**

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 3a-1 | New generic `tags` internal module | Added `parse_tags` (parses the config `tags` array) / `normalize_japanese_tags` (4-element normalization) / `TagPolicy { Keep, Remove }` / `apply_tag_filter` (the outer drain loop + decision) to `lindera/src/token_filter/tags.rs`. Because the tag-extraction logic differs subtly between filters (Japanese: join up to 4 elements, `keep` uses `min(4)` / `stop` uses `len>=4?4:1`; Korean: first element only), extraction is injected from each wrapper as a closure so the **behavior does not change by a single byte** | ✅ |
| 3a-2 | Reduce the 4 filters to thin wrappers | The public types, config formats, and behavior of `JapaneseKeepTagsTokenFilter`, etc. are fully preserved. Each `from_config`/`new`/`apply` delegates to the shared helper (core logic 4×~114 → 4×~35 lines) | ✅ |

The config-JSON compatibility and tokenization behavior are guaranteed by the 8 golden tests + the 12
existing tag-filter unit tests (all pass). The tests that make up most of each file are all kept as
evidence of behavior.

### 3b. Unify the loader layer — **done (scope narrowed)**

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 3b-1 | Consolidate verbatim loader duplication | **On re-investigation, the loader-layer duplication was smaller than the initial estimate (200→80 lines).** Each loader's `load` body differs in file name, type, and read method; the only **verbatim full duplication** is the 5-line "read_file → `AlignedVec<16>` → extend → `T::load`" block in `character_definition` and `unknown_dictionary`. Consolidated this into a `util::read_aligned_file` helper (the 2 files become 3 lines each). The `load`/`load_mmap` doubling in `connection_cost_matrix`/`prefix_dictionary`/`metadata` is the essential "with/without the `mmap` feature" branch, and since the return types of `read_file` (`Vec<u8>`) and `mmap_file` (`Mmap`) differ, it was preserved rather than forced together (avoiding over-macroization) | ✅ |
| 3b-2 | Disposition of the `DictionaryLoader` trait (`loader.rs`) | **On re-check it is not "unused".** Phase 2's `embedded_dictionary!` implements it on each `EmbeddedXxxLoader`, `FSDictionaryLoader` also implements it, and `lindera/src/dictionary.rs` uses it via `Box<dyn DictionaryLoader>`. The design where a default method returns an error is a reasonable shape for "loaders that implement only one of `load`/`load_from_path`". Judged: no change needed | ✅ (kept as is) |

- **Verification**: loaded a prebuilt FS dictionary via the CLI with `--dict <path>` (exercising
  `read_aligned_file`) and confirmed identical output to embedded. The 52 dictionary unit tests + 8
  golden tests unchanged, fmt/clippy clean.

### 3c. Normalize error handling — **done (3c-2 to 3c-5); 3c-1/3c-6 deferred**

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 3c-1 | Document the error strategy | State in CONTRIBUTING that "public APIs use `LinderaResult`; internal context-adding uses `anyhow`" | Deferred (documentation work, no code change — to be done separately) |
| 3c-2 | Convert the 5 `.unwrap()` in `builder.rs` to `?` | Replace `*BuilderOptions::builder()` (derive_builder-generated, all-default fields so it never actually fails) with `map_err(LinderaErrorKind::Build)?` | ✅ |
| 3c-3 | Turn the CSV-parse `unwrap()` in `prefix_dictionary.rs` into errors | Replace the 3 real-code `unwrap()` for `word_cost/left_id/right_id` in `build_word_entry_map` with a `let-else` binding merged with the preceding `is_none()` check. Behavior fully identical (the rest are `unwrap` inside `#[cfg(test)]`, allowed) | ✅ |
| 3c-4 | Convert `Regex::new().unwrap()` to `LazyLock` (std) | Moved the 3 regexes in `feature_extractor.rs` from in-function local creation to module-level `LazyLock<Regex>` (compiled once on first use, also improving train-time performance). The `unwrap()` on captures is left as-is since success is guaranteed by the regex group structure as an invariant | ✅ |
| 3c-5 | Fix the 2 env-var `unwrap()` in `assets.rs` | Changed `CARGO_PKG_VERSION` / `OUT_DIR` to `ok_or_else(...)?` (Cargo always sets them in a build script, but defensively) | ✅ |
| 3c-6 | Audit the unwraps in `lindera-crf`'s `trainer.rs` (28 sites) / `forward_backward.rs` (9 sites) | Deferred. On the train-only hot path, the `unwrap()` on numeric conversions have clear invariants. Replacing them with `expect()` is behavior-preserving but low value, and 37 changes are not worth the review cost | Deferred |

> 3c-2/3c-3/3c-5 (eliminating bare `unwrap` on the dictionary-build path) in one PR; 3c-4 (regex
> LazyLock) in a separate PR. Verification confirmed unchanged behavior via the 8 golden tests +
> dictionary unit/trainer tests.

### 3d. Split oversized files — **viterbi deferred (perf regression); others need re-evaluation**

| Target | Split proposal | Status |
| --- | --- | --- |
| `lindera-dictionary/src/viterbi.rs` (1,147 lines) | `viterbi/{lattice.rs, edge.rs, word_entry.rs}` | **Deferred** |
| `lindera/src/segmenter.rs` (1,882 lines) | `segmenter/mod.rs` + submodules | On hold (a hot path like viterbi, needs careful evaluation) |
| `lindera-dictionary/src/trainer/model.rs` (1,248 lines) | Separate the serialization part from the model body | Not started (train-only, not a hot path; relatively safe) |
| `lindera/src/token_filter/japanese_number.rs` (1,257 lines) | Separate the numeric-normalization state machine from the tests | Not started (token filter, needs a bench check) |

#### Why viterbi.rs was deferred (verification results)

On a production-equivalent build (`[profile.bench] lto = true, codegen-units = 1`), the baseline and the
split version were compared directly:

- **Full split** (Lattice in a separate file too): +5–30% regression on the non-LTO bench.
- **Data types only** (Lattice left at the module root, `#[inline]` added to data types): even on
  production-equivalent LTO, a **regression** of `tokenize` −5.0% / `tokenize-with-lattice` −5.3% /
  `details-long-text` −4.2% (the single file is faster).

viterbi is an extremely optimization-sensitive hot path: **module splitting itself** changes the
compiler's code generation (function layout, inlining, instruction-cache locality), and even adding
`#[inline]` cannot meet the 3% bar. The readability benefit does not justify the performance cost, so
**keep it a single file**.

> Lesson: even a "mechanical module split" of an oversized hot-path file can change performance. When
> pursuing 3d, always verify on a production-equivalent (LTO) bench, and do not split anything that
> regresses. `segmenter.rs` has the same concern, so make a bench a precondition before tackling the
> remaining 3d targets.

- **Done when**: golden tests, all unit tests, and the **production-equivalent (LTO) bench within 3%**.

---

## Phase 4: Introduce a shared binding layer — **done within the non-breaking scope (4-1/4-2); full version → Phase 6**

**Goal**: consolidate the 2,000+ lines of duplication across the 5 bindings into a shared crate and
resolve the API inconsistencies.

### Design approach (shared binding layer)

Create a new crate **`lindera-binding-core`** (FFI-independent, pure Rust):

- `TokenizerFacade` — the shared flow for builder construction / from_file / set_mode / filter
  addition / tokenize / tokenize_nbest. Each binding implements only the "FFI type ⇔ serde_json::Value"
  conversion.
- Shared DTOs — `TokenDto` / `SchemaDto` / `MetadataDto` (serde-enabled). Conversion to each FFI is a
  thin `From`/`TryFrom` implemented on the binding side.
- Errors unified into a single `BindingError` (`thiserror`); each binding holds only one mapping
  function to its own exception type.

Each binding's value-conversion helpers (`util.rs` / `convert.rs`, ~414 lines total) remain, with their
role limited to "FFI value → `serde_json::Value`" conversion (this is language-specific and cannot be
removed).

> **Environment note**: although we initially assumed "FFI build verification is heavy", we confirmed
> that all 5 bindings pass `cargo check` / `cargo test --lib` (the Rust part that needs no FFI
> toolchain). Therefore the consolidated pure logic can be verified with plain `cargo test`, allowing
> incremental, low-risk progress.

### Work items (binding extraction)

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 4-1 | Create `lindera-binding-core` + token extraction | New crate (registered in the workspace). Added the FFI-independent `TokenView::from_token(lindera::token::Token)` (extracts surface / byte range / position / word_id / is_unknown / details) and replaced `from_token` in all 5 bindings to go through `TokenView`. Token-extraction logic is consolidated in one place | ✅ |
| 4-2 | Extract the duplicated schema logic | Wrapping the whole class in a `CoreSchema` would turn public attributes like `#[pyo3(get)] fields` into getters and **break language API compatibility**, so — as with Token — only the "fully duplicated pure parts" are extracted. Added `schema::default_dictionary_fields()` (13 fields) and `schema::validate_record()` to core, and delegated `create_default`/`validate_record` in Python/PHP/Ruby/Node.js. Each class's structure and FFI attributes are unchanged. **Finding**: the bindings' `middle_pos/small_pos/fine_pos` is incompatible with `lindera::Schema::default()`'s `pos_detail_1/2/3` (only wasm uses the latter); carried over to 4-5 as an existing API inconsistency (kept the middle_pos family to avoid changing behavior). wasm already wraps `lindera::Schema`, so it is out of scope | ✅ |
| 4-3 | Extract Metadata | `CoreMetadata` (default values, schema wiring) into core | Not started |
| 4-4 | Extract Tokenizer/builder orchestration | `CoreTokenizerBuilder` / `CoreTokenizer` (build flow, tokenize, nbest) into core | Not started |
| 4-5 | Extract Error + resolve API inconsistencies | `CoreError` + a 1-line converter to each language exception. Unify the `Token.details` type, etc. (anything breaking as a language package aligns with the major bump) | Not started |
| 4-6 | Feature-parity table / value-conversion trait | Document the presence of segmenter, etc. Value conversion (`serde_json::Value` ⇔ FFI type) is inherently FFI-dependent, so limit it to a trait | Not started |

Each extraction is an independent PR. Token (4-1) → Schema (4-2) were done.

### Change of direction from 4-3 onward (important)

A structural fact revealed through 4-1/4-2: each binding's wrappers
(`Token`/`Schema`/`Metadata`/`TokenizerBuilder`/`Tokenizer`) are **tightly coupled to the FFI class**
(`#[pyclass]` / `#[napi]` / magnus / wasm-bindgen), and most methods are either delegation to
`inner.method()` or public attributes (`#[pyo3(get)]`, etc.). **The "pure logic" that can be safely
extracted non-breakingly (patch/minor) is limited to "processing not present in lindera" such as token
details extraction and schema default/validate — and that was fully captured in 4-1/4-2.**

- **4-3 Metadata**: mostly getters/setters (FFI-specific). The pure duplication is only the
  default-value constants, with little line-reduction benefit — not worth a PR (including resolving the
  recurring conflicts each time) → **skip** (organize in Phase 6, including the `middle_pos`
  inconsistency).
- **4-4 Tokenizer / 4-5 Error / value conversion**: the methods are FFI-public + `inner` delegation, and
  value conversion is FFI-dependent. Truly consolidating these — the "full facade" design (wrapping each
  binding with `CoreTokenizerBuilder`/`CoreTokenizer`/`CoreSchema` and purifying each class into an FFI
  translation layer) — entails **breaking language-API changes** such as turning public attributes into
  getters and unifying the `Token.details` type.

→ **The most beautiful full facade is a breaking change, and per semver is consolidated into Phase 6
(v4.0.0).** The non-breaking Phase 4 ends at 4-1/4-2.

- **Done when (non-breaking phase)**: the token/schema pure duplication is consolidated into
  `lindera-binding-core` (achieved).
- **Result**: created `lindera-binding-core`; `TokenView` (token extraction) and
  `schema::{default_dictionary_fields, validate_record}` are shared across 5/4 bindings.
- **To do in Phase 6**: the full facade (`CoreTokenizer`, etc.) + resolving API inconsistencies
  (`Token.details` type, `middle_pos`/`pos_detail`). Expected ~2,000-line reduction + core-API
  follow-up going 5 → 1.

---

## Phase 5: Tidy build infra / CI / CLI (no behavior change) — **done (5-1/5-2/5-3/5-5); 5-4 deferred**

**Goal**: structure the boilerplate repetition in the Makefile / GitHub Actions / CLI.

| # | Task | Detail | Status |
| --- | --- | --- | --- |
| 5-1 | Loop-ify the Makefile | Consolidated clean/format/lint/test/build for the 11 cargo crates (6 dictionaries + crf/dictionary/lindera/cli + the new `lindera-binding-core`) into pattern rules (`clean-%`, etc.) + per-crate feature variables. Exceptions where lint/test/build use different features (`lindera`'s build, `cli`'s test) are overridden via `TEST_FEATURES_*` / `BUILD_FEATURES_*`. The 5 FFI crates use bespoke tooling (maturin/napi/rake/composer/wasm-pack) and are preserved as explicit targets (overriding the pattern). The aggregate `clean/format/lint/test/build` uses `foreach`. **458 → 300 lines**, all target names/commands unchanged (all patterns verified with `make -n`), and `lindera-binding-core` added to the Makefile and publish | ✅ |
| 5-2 | CI: extract a composite action | Created `.github/actions/setup-rust` (checkout + toolchain, with optional toolchain/target/components/cache) and consolidated the checkout+toolchain in `regression.yml` (16 sites) + `release.yml` (23 sites). Since **a local composite action only resolves after checkout**, checkout stays in each job and the composite handles toolchain install (centralizing the dtolnay version and defaults) | ✅ |
| 5-3 | CI: unify the test jobs | Consolidated the 10 duplicated cargo-test jobs (crf/dictionary/6 dictionaries/lindera/cli) into a `workflow_call` reusable workflow `test-crate.yml` (inputs: crate/features/target/runs-on/cache; matrix on the caller side), called from both `regression.yml` and `release.yml` via `uses:`. Bindings go through `make` and are kept separately. The reusable workflow is CI-proven via `regression.yml` | ✅ |
| 5-4 | CI: unify the binding release-job matrix | De-duplicate the Ruby/PHP/Node.js platform matrix and output-ify version detection. **Deferred**: `release.yml`-only, not run on PRs and thus not CI-verifiable; failures break a release, so the risk is not worth the reduction value | Deferred |
| 5-5 | CLI refactor | **①②③ done**: ① consolidated the doubled output-format match in `tokenize()` into `write_output(format, tokens)`. ② consolidated the 18 `LinderaErrorKind::Io.with_error(anyhow::anyhow!(err))` sites into an `io_err` helper (the `with_error(err)` via `From` is kept to preserve behavior). ③ split main.rs into `commands/{list,tokenize,build,train,export}.rs`, with the shared `io_err` in `commands/mod.rs`. Moved each Args struct and subcommand function into its module. **main.rs 672 → 51 lines** (overall: main.rs 51 + commands/ ~640 lines). ④ all guaranteed by the Phase 0 CLI smoke tests (8). The CLI is not a hot path, so the split has no performance impact | ✅ |

- **Risk**: medium (CI breakage is hard to notice). Verify 5-3/5-4 on regression.yml first, and confirm
  the release flow with a dry-run or a test release before tagging. **5-1 has no CI impact** since CI
  does not depend on the Makefile and calls cargo directly (developer-only targets).
- **Done when**: the total CI definition line count is halved (~2,028 → ~1,000), the Makefile is ≤ 200
  lines, and both the regression / release workflows are green.
- **Scale**: medium (3–4 PRs).

---

## Phase 6: Public-API redesign and a single round of breaking changes (v4.0.0)

**Goal**: settle, in one major version, the items left at `#[deprecated]` in Phases 1–5 and the breaking
changes deferred for semver reasons.

| # | Task | Detail |
| --- | --- | --- |
| 6-1 | Encapsulate viterbi internals | Convert the `pub` fields of `WordId` / `WordEntry` / `Edge` / `PathEntry` / `Lattice` to accessor methods. Make `WordEntry`'s serialization details (`SERIALIZED_LEN`, etc.) private |
| 6-2 | Remove deprecated items | Remove the Phase 2 `EmbeddedXxxLoader` aliases and the `LINDERA_CACHE` env-var shim (`assets.rs:238`) |
| 6-3 | Decide on legacy-format compatibility code | For the user dictionary's "5-bit variant-count encoding" legacy compatibility (`viterbi.rs:409,953`), investigate the generation of supported dictionary binaries and either remove it or migrate to an explicit migration tool |
| 6-4 | `pub` audit | Inventory the public API with `cargo public-api`, etc., and clean up exposed implementation details (including the inconsistent `*Options` naming in builders) |
| 6-5 | Make the builder API return `Result` | Fix the public signatures of the `*BuilderOptions::builder().unwrap()` pattern handled internally in Phase 3c |
| 6-6 | Write a migration guide | Add a v3 → v4 migration guide (English and Japanese) to `docs/` |

- **Risk**: high (ecosystem impact). Cut a v4.0.0-alpha beforehand and set a validation period with key
  users (downstream projects such as lindera-tantivy).
- **Done when**: the `cargo public-api` diff matches the migration guide; all bindings are green on the
  v4 core.
- **Scale**: medium–large (4–6 PRs + an alpha/beta release cycle).

---

## Inter-phase dependencies and recommended order

```text
Phase 0 (safety net) ──┬─→ Phase 1 (immediate cleanup)
                       ├─→ Phase 2 (dictionary consolidation) ──┐
                       ├─→ Phase 3 (core quality) ──────────────┤
                       ├─→ Phase 4 (binding consolidation) ─────┼─→ Phase 6 (v4.0.0)
                       └─→ Phase 5 (build infra / CI) ──────────┘
```

- Phases 1, 2, 3, and 5 **can run in parallel** (conflict surface: Phases 2 and 3 share
  `lindera-dictionary`, so the 2 → 3b order is recommended).
- Phase 4 is best done after Phase 3's core API stabilizes (to reduce follow-up cost).
- Phase 6 only after all phases are complete.

## Effect estimate (approximate)

| Item | Before | After |
| --- | --- | --- |
| Dictionary-crate implementation lines | ~590 (6-crate duplication) | ~100 (definitions only) |
| Binding duplication lines | 2,000+ | ≤ 150 each, an FFI translation layer |
| Tag filters | ~800 lines | ~200 lines |
| Makefile | 458 lines | ~150 lines |
| CI definitions | ~2,028 lines | ~1,000 lines |
| Repository size | including 21 MB of docs artifacts | 21 MB smaller |
| Bare unwrap in non-test code | 30+ sites | 0 (only justified expect or `?`) |
| Cost to add a new dictionary | copy a whole crate set | 1 parameter-definition file |

## Common completion checklist for every PR

- [ ] `cargo fmt --check` / `cargo clippy --workspace --all-targets` (equivalent to the Makefile lint) is green
- [ ] Builds succeed for all feature combinations (at least the 6 individual `embed-*` + `embed-cjk` + `train` + `mmap`)
- [ ] The Phase 0 golden tests are green (i.e. tokenization results unchanged)
- [ ] Benchmark regression within 3% (when touching viterbi / segmenter / tokenizer)
- [ ] No unintended public-API changes (outside Phase 6)
