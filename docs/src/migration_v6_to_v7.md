# Migrating from v6 to v7

Lindera v7.0.0 turns the `lindera` crate into a facade: it re-exports the
morphological segmenter, now published as `lindera-segmenter`, and — through
the new `analysis` feature, on by default — the analysis chain of
`lindera-analysis` as `lindera::analysis`. One dependency line gives you the
`Segmenter`, the `Tokenizer`, and every filter. The release also renames
`lindera-binding-core` to `lindera-binding` and removes the deprecated
`LINDERA_DICTIONARIES_PATH` fallback, as announced in v5.0.0, corrects
the IPADIC schema, which had the `conjugation_type` and `conjugation_form`
names swapped, and keeps dictionary entries whose surface is whitespace or
starts or ends with it. This guide lists every breaking change and the
one-line fixes for each.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| `lindera` is a facade: `lindera::analysis` re-exports `lindera-analysis` (feature `analysis`, on by default) | Rust users who depend on both `lindera` and `lindera-analysis` | Bump both to `"7"` in the same step, or drop `lindera-analysis` and import from `lindera::analysis::…` |
| `default-features = false` now means "segmenter only": it turns off `analysis` (and, as before, `mmap`) | Rust users of `lindera` with `default-features = false` who want `lindera::analysis` | Add `"analysis"` to `features` (and keep `"mmap"` listed for memory-mapped loading) |
| The former `lindera` crate is published as `lindera-segmenter` | Nobody directly | Nothing — `lindera = "7"` re-exports it |
| The raw `lindera-dictionary` modules are reachable under `lindera::dictionary` | Direct `lindera-dictionary` users | Nothing — optionally drop the direct dependency and use `lindera::dictionary::core::…`, `::builder`, `::viterbi`, … |
| **The implicit features `lindera-ipadic`, `lindera-ipadic-neologd`, `lindera-unidic`, `lindera-sudachidict`, `lindera-ko-dic`, `lindera-cc-cedict`, and `lindera-jieba` are gone** | Anyone enabling a dictionary crate by name in `features = [...]` | Use the `embed-*` features instead |
| **`lindera-binding-core` is renamed to `lindera-binding`** | Rust users of the binding helper crate | Depend on `lindera-binding` and replace `lindera_binding_core::` with `lindera_binding::` |
| **`LINDERA_DICTIONARIES_PATH` is removed** | Anyone still setting the deprecated build-cache variable | Set `LINDERA_BUILD_DICTIONARY_CACHE_DIR`; the old name is now ignored |
| **IPADIC and IPADIC-NEologd: `conjugation_type` and `conjugation_form` now name the right columns** | Anyone who reads these two fields by name with IPADIC or IPADIC-NEologd (`Token::get`, `Token::as_value`, `lindera tokenize -o json`, binding schemas) | Expect the two values to trade places; rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **Dictionary entries with whitespace are kept** | Text with U+3000 (IPADIC, IPADIC-NEologd, UniDic), text with spaces (SudachiDict), and a few entries that ended with whitespace | Expect U+3000 to be `記号,空白` / `空白` and SudachiDict to segment spaced text much closer to Sudachi; rebuild or re-download dictionaries made with v6.2.0 or earlier |

The language bindings (Python, Node.js, Ruby, PHP, WASM) and the CLI keep
their APIs and package names; only their version number moves to 7.0.0.
There are two output differences, both described below. With IPADIC or
IPADIC-NEologd, the values reported under the names `conjugation_type` and
`conjugation_form` trade places. And the dictionaries now contain the entries
whose surface is or starts or ends with whitespace, which changes the
segmentation of text that contains such whitespace. Otherwise, for the same
input and dictionary, v7.0.0 produces the same tokens with the same
positional details as v6.2.0.

## The lindera crate is now a facade

Up to v6 the Rust API was split between two crates that you had to depend on
separately: `lindera` (the segmenter and the dictionary loading API) and
`lindera-analysis` (character filters, token filters, and the `Tokenizer`).
In v7 the `lindera` crate is a thin facade over both:

```text
lindera ──┬─▶ lindera-segmenter ──┬─▶ lindera-dictionary
          │                       ├─▶ dictionary crates   [features: embed-*]
          │                       └─▶ lindera-trainer     [feature: train]
          └─▶ lindera-analysis    [feature: analysis, default on] ──▶ lindera-segmenter
```

Its public tree keeps every `lindera::…` path of v6 and adds
`lindera::analysis`:

```text
lindera
├── LinderaResult, get_version()
├── dictionary, error, mode, segmenter, space_penalty, token, worker   (from lindera-segmenter, as in v6)
└── analysis   [feature: analysis, default on]   (lindera-analysis: character_filter, token_filter, tokenizer, worker)
```

### One dependency instead of two

```toml
# v6
[dependencies]
lindera = "6"
lindera-analysis = "6"

# v7
[dependencies]
lindera = "7"
```

```rust
// v6
use lindera_analysis::tokenizer::Tokenizer;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;

// v7
use lindera::analysis::tokenizer::Tokenizer;
use lindera::analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
```

Keeping a direct `lindera-analysis = "7"` dependency also works:
`lindera::analysis` is that same crate, so
`lindera_analysis::tokenizer::Tokenizer` and
`lindera::analysis::tokenizer::Tokenizer` are one type and existing `use`
paths need not change.

### Update lindera and lindera-analysis together

If you keep both dependencies, bump them in the same step. `lindera-analysis`
6.x depends on the `lindera` 6.x segmenter, so a manifest with the `lindera`
line moved to `"7"` while `lindera-analysis` stays at `"6"` pulls two
versions of the segmenter into the build:

```toml
# Two Segmenter types: lindera 7 (via lindera-segmenter) and lindera 6 (via lindera-analysis 6)
[dependencies]
lindera = "7"
lindera-analysis = "6"
```

`lindera::segmenter::Segmenter` is then the v7 type while
`lindera_analysis::tokenizer::Tokenizer::new` expects the v6 one, and the
call fails to compile with a type mismatch. Move both lines to `"7"` at once,
or drop `lindera-analysis` as shown above.

### `default-features = false` now means segmenter only

The facade's default features are `mmap` and `analysis`. With
`default-features = false` you get the pure segmenter, and both are off:
`lindera::analysis` does not exist, and memory-mapped loading is no longer
the default for filesystem dictionaries (as in v6, where
`default-features = false` also turned `mmap` off). Re-enable what you need:

```toml
# Segmenter with memory-mapped loading (the v6 default build)
lindera = { version = "7", default-features = false, features = ["mmap"] }

# Segmenter and analysis chain, without memory-mapped loading as the default
lindera = { version = "7", default-features = false, features = ["analysis"] }
```

`memmap2` itself stays linked either way through `lindera-dictionary`'s
default features; the facade's `mmap` feature only selects the default for
`use_mmap` in `load_dictionary` and friends, and
`load_dictionary_with_options(uri, true)` works without it.

### The segmenter is published as lindera-segmenter

The `lindera` crate of v6 and earlier continues as `lindera-segmenter` 7.0.0,
with the same modules (`dictionary`, `error`, `mode`, `segmenter`,
`space_penalty`, `token`, `worker`) and the same API. The facade re-exports
every one of them at its root, so every `lindera::…` path that compiled in v6
compiles in v7. The crate exists so that `lindera-analysis` has a segmenter
to build on without depending on the facade that re-exports it; nobody needs
to depend on `lindera-segmenter` directly, and application code should keep
depending on `lindera`.

## `lindera::dictionary` exposes the raw dictionary crate

Code that reached into the building blocks of `lindera-dictionary` — the
dictionary data structures, the dictionary builder, the Viterbi lattice —
used to need a direct `lindera-dictionary` dependency. In v7 these modules
are re-exported inside `lindera::dictionary`, next to the high-level loading
API:

```text
lindera::dictionary
├── Dictionary, Metadata, Schema, load_dictionary, load_user_dictionary, DictionaryKind, …   (unchanged)
├── core          lindera_dictionary::dictionary (prefix_dictionary, character_definition, connection_cost_matrix, unknown_dictionary, …)
├── builder, error, loader, mode, nbest, space_penalty, util, viterbi
├── embedded_dictionary!, include_bytes_aligned!
└── trainer       lindera_trainer, with the `train` feature (unchanged)
```

All existing `lindera::dictionary::*` paths stay valid, and the re-exports
are the same items — `lindera::dictionary::Dictionary` and
`lindera::dictionary::core::Dictionary` are one type. A direct
`lindera-dictionary` dependency keeps working; it is simply no longer
required:

```rust
// v6: needs lindera-dictionary in Cargo.toml
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::viterbi::Lattice;

// v7: reachable through the facade
use lindera::dictionary::core::prefix_dictionary::PrefixDictionary;
use lindera::dictionary::viterbi::Lattice;
```

> [!NOTE]
> `lindera::dictionary::core` shares its name with Rust's `core` crate. A
> glob import such as `use lindera::dictionary::*;` brings that module into
> scope and shadows the `core` crate for the rest of the module, so paths
> like `core::fmt::Debug` stop resolving to the standard library there.
> Import the items you need by name instead.

## Implicit dictionary features removed

In v6 the optional dictionary dependencies of `lindera` doubled as Cargo
features: `features = ["lindera-ipadic"]` was accepted, and pulled in the
crate without embedding a dictionary. The facade has no dictionary
dependencies of its own, so these seven implicit features no longer exist:
`lindera-ipadic`, `lindera-ipadic-neologd`, `lindera-unidic`,
`lindera-sudachidict`, `lindera-ko-dic`, `lindera-cc-cedict`, and
`lindera-jieba`. Cargo rejects a manifest that names one of them. Use the
`embed-*` features, which have always been the documented way to embed a
dictionary:

```toml
# v6
[dependencies]
lindera = { version = "6", features = ["lindera-ipadic"] }

# v7
[dependencies]
lindera = { version = "7", features = ["embed-ipadic"] }
```

## lindera-binding-core renamed to lindera-binding

This only matters if you build your own language binding on the shared
helper crate. `lindera-binding` 7.0.0 depends on the `lindera` facade (with
the `analysis` feature) instead of on `lindera` plus `lindera-analysis`; its
own API — `CoreTokenizerBuilder`, `CoreTokenizer`, `TokenView`, and the
argument, metadata, and schema helpers — is unchanged. The published
`lindera-binding-core` releases (4.0.0 through 6.2.0) stay on crates.io as
they are and are not yanked, but receive no further versions.

```toml
# v6
[dependencies]
lindera-binding-core = "6"

# v7
[dependencies]
lindera-binding = "7"
```

```rust
// v6
use lindera_binding_core::tokenizer::CoreTokenizer;

// v7
use lindera_binding::tokenizer::CoreTokenizer;
```

## LINDERA_DICTIONARIES_PATH removed

v5.0.0 renamed the build-time dictionary cache variable to
`LINDERA_BUILD_DICTIONARY_CACHE_DIR` and kept the old name,
`LINDERA_DICTIONARIES_PATH`, as a deprecated fallback with a build warning
(see [Migration v4 to v5](./migration_v4_to_v5.md)). v7.0.0 removes the
fallback. A build that sets only the old name no longer uses the cache — and
no longer warns about it: the dictionary crates download and build their
dictionary as if no cache were configured. Rename the variable wherever it is
set (shell profiles, CI configuration, container images):

```shell
# v6
export LINDERA_DICTIONARIES_PATH=/path/to/cache

# v7
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
```

## IPADIC conjugation field names corrected

IPADIC stores 活用型, the conjugation type (for example `五段・カ行イ音便`),
in column 8 and 活用形, the conjugation form (for example `連用タ接続`), in
column 9. From v1.0.0 through v6.2.0, the `lindera-ipadic` and
`lindera-ipadic-neologd` schemas named the two columns the other way round,
so every lookup by name returned the other value. v7.0.0 corrects the names.
UniDic and SudachiDict were already correct.

For the token `書い` in `書いた`:

| Field name | v6 | v7 |
| --- | --- | --- |
| `conjugation_type` | `連用タ接続` | `五段・カ行イ音便` |
| `conjugation_form` | `五段・カ行イ音便` | `連用タ接続` |

Only access by name changes: `Token::get("conjugation_type")` and
`Token::get("conjugation_form")`, the JSON from `Token::as_value()` and
`lindera tokenize -o json`, and the field names in the dictionary schema
that the bindings expose. Positional access — `details`, and the MeCab and
wakati output formats — is unchanged, because the values were always in the
right columns. If your code reads these fields by name, or swapped them back
as a workaround, update it.

The names come from the `metadata.json` stored in each built dictionary, not
from the library, and the dictionary format version is unchanged. The
embedded dictionaries (`embed-ipadic`, `embed-ipadic-neologd`) and the
dictionaries that the 7.0.0 CLI fetches with `lindera download` carry the
corrected names. A dictionary directory built or downloaded with v6.2.0 or
earlier still loads in v7.0.0 but keeps the old names. For such a directory,
either:

- rebuild it with the v7.0.0 `lindera-ipadic/metadata.json` (or
  `lindera-ipadic-neologd/metadata.json`), or download the 7.0.0 release
  asset, or
- swap `"conjugation_form"` and `"conjugation_type"` in the
  `dictionary_schema.fields` list of its `metadata.json`. The dictionary data
  itself is unchanged, so no rebuild is needed.

If you build dictionaries with your own copy of the IPADIC `metadata.json`,
apply the same swap to that copy.

## Dictionary entries with whitespace are kept

Up to v6.2.0, the dictionary builder trimmed the surface of every lexicon
entry. An entry whose surface is whitespace was dropped, and an entry that
starts or ends with whitespace was stored under the trimmed surface, where
a cheaper whitespace variant could take over the entry without whitespace.
v7.0.0 reads the surface exactly as written, as MeCab and Sudachi do:

| Dictionary | Change |
| --- | --- |
| IPADIC, IPADIC-NEologd | U+3000 (ideographic space) is the dictionary entry `記号,空白` instead of an unknown `名詞,サ変接続`, so the word after it is also read as in MeCab: in `東京　都`, `都` is `名詞,一般` instead of `名詞,接尾`. `ルーマニア` has the base form `ルーマニア` instead of `ルーマニア` followed by U+3000 |
| UniDic | U+3000 is the entry `空白` instead of an unknown `名詞,普通名詞,サ変可能` |
| SudachiDict | U+0020 uses the dictionary's half-width space entry (`空白`) instead of the `SPACE` unknown word, so text with spaces is segmented much closer to Sudachi (on 600 sentences with a space between every word, agreement with Sudachi went from 201 to 597). With `keep_whitespace(true)`, each space is its own token. U+3000 is unchanged, because SudachiDict has no entry for it |
| ko-dic | `에듀` and `캘리` on their own are `NNP` (person names) instead of the `NNG` of their trailing-space variants |
| CC-CEDICT, Jieba | No change |

Entries that end with whitespace (55 in IPADIC-NEologd, 4 in ko-dic and 4 in
SudachiDict) can now match, and their token then ends with that whitespace,
as in MeCab: `GeForce GTX Titan X です` gives the token `GeForce GTX Titan X`
followed by a space with IPADIC-NEologd.

U+3000 is not in the `SPACE` character category, so it is a token even with
`keep_whitespace(false)`, as in MeCab. To remove it, drop `記号,空白`
(IPADIC) or `空白` (UniDic) with the `japanese_stop_tags` token filter, or
turn it into U+0020 with the `unicode_normalize` character filter (NFKC).

The change is in the dictionary builder; the dictionary format version is
unchanged. The embedded dictionaries and the dictionaries that the 7.0.0 CLI
fetches with `lindera download` include these entries. A dictionary
directory built or downloaded with v6.2.0 or earlier still loads in v7.0.0
but keeps the old entries until you rebuild it with `lindera build` or
download the 7.0.0 release asset.

## Who does not need to act

- **Users of the language bindings and the CLI**: the Python, Node.js, Ruby,
  PHP, and WASM packages and `lindera-cli` keep their APIs and package names.
  The restructuring is internal to the Rust crates. The output changes are
  the IPADIC conjugation fix, which matters only if you read those two fields
  by name, and the whitespace entries, which matter only for text with U+3000
  (IPADIC, IPADIC-NEologd, UniDic) or with spaces (SudachiDict).
- **Projects staying on `lindera = "6"`**: the 6.x releases of `lindera` and
  `lindera-analysis` remain on crates.io and keep working together. Nothing
  changes until you bump the major version.
- **Users of the segmenter API only**: code that uses `Segmenter`,
  `load_dictionary`, `Mode`, and the other `lindera::…` items of v6 compiles
  unchanged with `lindera = "7"`. The default build now also compiles
  `lindera-analysis` and its dependencies (kanaria, regex, serde_yaml_ng,
  unicode-blocks, unicode-normalization, unicode-segmentation); to keep the
  v6 dependency tree, set `default-features = false, features = ["mmap"]`.

## Upgrade checklist

Rust crate users:

- Bump `lindera` to `"7"` and, in the same commit, either bump
  `lindera-analysis` to `"7"` or remove it and import from
  `lindera::analysis::…`.
- Optionally replace `lindera_analysis::` with `lindera::analysis::` in `use`
  paths — both name the same types.
- If you set `default-features = false`, add `"analysis"` (for the
  `Tokenizer` and filters) and `"mmap"` (for memory-mapped loading) to
  `features` as needed.
- Replace `features = ["lindera-<dictionary>"]` with
  `features = ["embed-<dictionary>"]`.
- Optionally drop a direct `lindera-dictionary` dependency in favor of
  `lindera::dictionary::core`, `::builder`, `::viterbi`, and the other
  re-exported modules.

Binding authors:

- Depend on `lindera-binding` instead of `lindera-binding-core`, and replace
  `lindera_binding_core::` with `lindera_binding::`.

Build environments:

- Rename `LINDERA_DICTIONARIES_PATH` to `LINDERA_BUILD_DICTIONARY_CACHE_DIR`
  in shell profiles, CI configuration, and container images.

Users of IPADIC or IPADIC-NEologd:

- If you read `conjugation_type` or `conjugation_form` by name (including
  the CLI's JSON output), expect the two values to trade places.
- Rebuild or re-download dictionary directories made with v6.2.0 or earlier,
  or swap the two names in their `metadata.json`.

Users of IPADIC, IPADIC-NEologd, UniDic or SudachiDict with text that
contains U+3000 or spaces:

- Expect U+3000 to be `記号,空白` (IPADIC) or `空白` (UniDic) tokens, and
  SudachiDict to segment spaced text much closer to Sudachi. Drop U+3000 with
  `japanese_stop_tags` or NFKC normalization if you do not want it.
- Rebuild or re-download dictionary directories made with v6.2.0 or earlier
  to get the whitespace entries.

Language bindings and CLI:

- Nothing to do beyond taking the 7.0.0 release, apart from the dictionary
  items above.
