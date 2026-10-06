# Migrating from v6 to v7

Lindera v7.0.0 turns the `lindera` crate into a facade: it re-exports the
morphological segmenter, now published as `lindera-segmenter`, and — through
the new `analysis` feature, on by default — the analysis chain of
`lindera-analysis` as `lindera::analysis`. One dependency line gives you the
`Segmenter`, the `Tokenizer`, and every filter. The release also renames
`lindera-binding-core` to `lindera-binding` and removes the deprecated
`LINDERA_DICTIONARIES_PATH` fallback, as announced in v5.0.0, corrects
the IPADIC schema, which had the `conjugation_type` and `conjugation_form`
names swapped, keeps dictionary entries whose surface is whitespace or
starts or ends with it, skips whitespace in the lattice as MeCab does,
resolves overlapping `char.def` lines as MeCab does, carries the context
across `、` and `。` within a line as MeCab does, returns N-best results
that each cover the whole input, lets unknown words start at every position
as MeCab does, and has the lattice backtraces of `lindera-dictionary` return
each token's end offset. This guide lists every breaking change and the
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
| **Whitespace is skipped in the lattice: the words on either side of a space connect directly, as in MeCab** | Anyone who segments text containing whitespace with `keep_whitespace` false (the default), most visibly with ko-dic | Expect MeCab's segmentation for such text; `skip_whitespace(false)` (`"skip_whitespace": false`, `--disable-skip-whitespace`) restores the v6 handling of whitespace, but not the other output changes in this guide |
| **Overlapping `char.def` lines: the last line decides** | Text with `Ð` (U+00D0; every dictionary except ko-dic), `々` (U+3005) or `〇` (U+3007); `lindera train` users whose `char.def` has single-code-point or overlapping lines | Expect `Ð` to stay in the output as a letter; rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **The context is carried across `、` and `。` within a line, as in MeCab** | Anyone who segments Japanese text with `、` or `。` inside a line | Expect some words after `、` or `。` to be read differently, mostly as MeCab reads them, and the N-best costs of such lines to change; no setting restores the v6 behavior |
| **N-best: every result covers the whole input** | Anyone who asks for N-best results (`segment_nbest`, `tokenize_nbest`, `lindera tokenize -N`, the bindings' N-best methods) for input with more than one sentence | Expect the cheapest segmentations of the whole input from the second result on; a cost threshold now applies to the whole input |
| **Unknown words start at every position, as in MeCab** | Anyone who segments text with runs of katakana, symbols or Latin letters in normal mode, most visibly katakana joined by `・`, Korean sentence-final punctuation such as `."`, and Latin words with CC-CEDICT | Expect such runs to be split where MeCab splits them, and katakana- and Latin-heavy text to take about 13% more instructions; no setting restores the v6 behavior |
| **`Lattice::tokens_offset`, `tokens_offset_into`, `nbest_tokens_offset` and `NBestGenerator::next` return `(start, end, WordId)`** | Rust code that calls these `lindera_dictionary` functions directly (also as `lindera::dictionary::viterbi::…`, `lindera::dictionary::Lattice` and `lindera::dictionary::nbest::…`) | Destructure three fields and use the returned end instead of the next token's start |
| **`Lattice::tokens_offset_into` returns the BOS index of the best path (`Option<usize>`)** | Rust code that uses the `()` value of `tokens_offset_into` | Nothing for a call written as a statement; otherwise ignore the new value |

The language bindings (Python, Node.js, Ruby, PHP, WASM) and the CLI keep
their APIs and package names; only their version number moves to 7.0.0.
There are seven output differences, all described below. With IPADIC or
IPADIC-NEologd, the values reported under the names `conjugation_type` and
`conjugation_form` trade places. The dictionaries now contain the entries
whose surface is or starts or ends with whitespace, which changes the
segmentation of text that contains such whitespace. Text that contains
whitespace is segmented as MeCab segments it. `Ð`, `々` and `〇` get the
character categories that MeCab gives them, which changes the segmentation
of text that contains them. Within a line, the words after `、` and `。`
are read in the context of the words before them, which changes the
segmentation of some such text and the N-best costs of every such line. For
input with more than one sentence, the N-best results from the second one on
are the cheapest segmentations of the whole input. And in normal mode, an
unknown word can start inside a run of characters of one kind, as in MeCab,
which splits some runs of katakana, symbols and Latin letters. Otherwise,
for the same input and dictionary, v7.0.0 produces the same tokens with the
same positional details as v6.2.0.

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
SudachiDict) now match only where the text has that whitespace, as in MeCab:
with IPADIC-NEologd, the entry `GeForce GTX Titan X` followed by a space
matches `GeForce GTX Titan X です` but no longer `GeForce GTX Titan Xです`.
The token ends with that whitespace, as in MeCab, whether whitespace stays
in the lattice (`keep_whitespace(true)`, or `skip_whitespace(false)`,
SudachiDict's default) or is skipped (the default for the other
dictionaries, see the next section). Skipping leaves out only the whitespace
after the entry: in `GeForce GTX Titan X  です`, with two spaces, the token
ends with the first space. 50 of the 55 IPADIC-NEologd entries end with a
space, as do all those of ko-dic and SudachiDict; the other 5 end with
U+3000 or U+00A0, which are not `SPACE` characters and are never skipped.
The Decompose length penalty counts the entry's own whitespace, as it does
when whitespace stays in the lattice. With `unique`, N-best results that
differ only in that whitespace are no longer folded into one, such as
ko-dic's `에듀` followed by the space (`NNG`) and `에듀` (`NNP`) in
`에듀 센터`.

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

## Whitespace is skipped in the lattice

With `keep_whitespace` false (the default), v7.0.0 skips whitespace in the
Viterbi lattice, as MeCab does: the word after a space connects directly to
the word before it. v6 dropped whitespace from the output but kept it in the
lattice as the `SPACE` unknown word. Dictionaries trained with MeCab never
see a connection to or from that entry (in ko-dic its connection costs are
all zero), so every space cut the context between the words around it.

The output changes only for sentences that contain whitespace. That includes
a sentence that ends with the `\n` or `\t` it was split at, such as a line
that does not end with `。`: its last word now connects to the end of the
sentence directly.

| Dictionary | Effect | Example |
| --- | --- | --- |
| ko-dic | Spaced Korean text now follows mecab-ko | `2년 전 대회`: `전` is `NNG`, not `저/NP + ㄴ/JX`; `하고 있다`: `있` is `VX`, not `VV` |
| IPADIC, IPADIC-NEologd, UniDic | A word after a half-width space connects to the word before it, as in MeCab | IPADIC `Google が 新しい`: `が` is a case particle, not a conjunction; `東京 都`: `都` is a suffix, as in MeCab |
| SudachiDict | No change: its `metadata.json` sets `skip_whitespace` to `false`, because Sudachi keeps whitespace in the lattice and the costs assume it | — |
| CC-CEDICT, Jieba | No change in the best path (these dictionaries have no connection costs); N-best costs no longer include the whitespace nodes | — |

The default comes from the dictionary's `metadata.json`; a dictionary that
does not set `skip_whitespace` skips. A SudachiDict built by Lindera 6.x has
no such setting, so it skips whitespace until it is rebuilt or the setting
is added to its `metadata.json`.

Token surfaces and offsets never include the skipped whitespace; whitespace
that belongs to a dictionary entry, inside it or at its end, stays in that
entry's token, as in MeCab (see the previous section). Skipping does not
change the segmentation of text without whitespace. `keep_whitespace(true)`
keeps whitespace in the lattice and its output is unchanged.

To get the v6 handling of whitespace back, turn skipping off; whitespace is
still dropped from the output. This does not undo the other output changes in
this guide, such as
[Unknown words start at every position, as in MeCab](#unknown-words-start-at-every-position-as-in-mecab).
Leaving `skip_whitespace` out of the configuration, or setting it to `null`,
keeps the dictionary's default:

```rust
// Segmenter
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
// SegmentWorker
worker.set_skip_whitespace(false);
```

```yaml
# Configuration file / TokenizerBuilder::set_segmenter_skip_whitespace(false)
segmenter:
  skip_whitespace: false
```

```sh
lindera tokenize --disable-skip-whitespace
```

The language bindings take the setting through the configuration file.

## Overlapping char.def lines are resolved as in MeCab

A dictionary's `char.def` can list the same code point on several lines. Up
to v6.2.0, the dictionary builder gave such a code point the categories of
every line that covers it. v7.0.0 uses only the last line, as MeCab,
vibrato and Kuromoji do, and lists that line's categories in the order in
which `char.def` defines them. In the bundled dictionaries, three characters
change (ko-dic calls the kanji categories `HANJA` and `HANJANUMERIC`, and
CC-CEDICT and Jieba call them `CHINESE` and `CHINESENUMERIC`):

| Character | v6 categories | v7 categories | Effect |
| --- | --- | --- | --- |
| `Ð` (U+00D0) | `SPACE`, `ALPHA` | `ALPHA` | Every dictionary except ko-dic maps U+00D0 to `SPACE` (a typo for U+000D inherited from mecab-ipadic) before the `ALPHA` range, so `Ð` was dropped as whitespace and split the word around it. `GUÐMUNDUR さん` now gives `GUÐMUNDUR` and `さん` instead of `GU`, `MUNDUR` and `さん` |
| `々` (U+3005) | `KANJI`, `SYMBOL` | `SYMBOL` | Dictionary words such as `人々` and `佐々木` are unchanged. After a kanji that the dictionary does not know, `々` is a token of its own, as in MeCab: `龘々` gives `龘` and `々` instead of one unknown word. With ko-dic, `々` is tagged `SY` instead of `SH` |
| `〇` (U+3007) | `KANJI`, `SYMBOL`, `KANJINUMERIC` | `SYMBOL`, `KANJINUMERIC` | Runs of numerals can be grouped differently. With ko-dic, `二〇二六年` gives `二〇二六` and `年` instead of `二`, `〇`, `二六` and `年` |

The kanji numerals `一` to `九`, `十`, `百`, `千`, `万`, `億` and `兆` are
also listed on two lines, but keep their categories and their order.

`lindera train` now reads `char.def` with the dictionary builder. It used
to keep only the first category of each line, skip lines that name a single
code point (such as `0x0020 SPACE`), and resolve overlaps by start position.
The `%t` feature template now gives the first category of the last line
that covers the character, which is MeCab's default type. The bundled
training files have neither kind of line, so the models trained from them
do not change.

A range line without a category is now an error in `lindera build` and
`lindera train`, as in MeCab; it would otherwise reset the earlier lines to
`DEFAULT`.

The change is in the dictionary builder; the dictionary format version is
unchanged. The embedded dictionaries and the dictionaries that the 7.0.0 CLI
fetches with `lindera download` have the new categories. A dictionary
directory built or downloaded with v6.2.0 or earlier still loads in v7.0.0
but keeps the old categories until you rebuild it with `lindera build` or
download the 7.0.0 release asset.

## The context is carried across `、` and `。`

The segmenter cuts the input into sentences at `\n`, `\t`, `。` and `、`,
and after 32 KiB without any of them, to keep each lattice small. Up to v6,
every sentence started from the beginning-of-sentence context (BOS) and
ended with the connection to the end of the sentence (EOS), so the word
after `、` or `。` was read as if it began a text. MeCab does not cut
inside a line.
v7.0.0 carries the context across the cuts at `、` and `。` and across the
forced cuts: the next sentence starts from the words that end the previous
one, and EOS is paid only at a `\n`, a `\t` or the end of the input. The
sentences up to that point form a segment, and segments stay independent.
The best path of a segment, and its cost, are those of one lattice over the
whole segment, which is what MeCab gives for a line:

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `彼は、ああ言った` (IPADIC) | `ああ` is an interjection (`感動詞`) | `ああ` is an adverb (`副詞`) |
| `だから、こんなに答える` (IPADIC) | `こんな` (`連体詞`) and `に` | `こんなに` (`副詞`) |
| `選手が泳ぎ、さらにカヌーで進んだ` (UniDic) | `さらに` is a conjunction (`接続詞`) | `さらに` is an adverb (`副詞`) |
| `言語としては、Javaのオブジェクト` (UniDic) | `J`, `a`, `v` and `a`, each `記号,文字` | `Java` (`名詞,普通名詞`) |

On the sentences of UD Japanese GSD that contain `、`, the change alters
50 lines with UniDic: 27 come closer to the gold segmentation and
part-of-speech tags, 7 move away (sign test p = 0.0008), and 16 are ties.
Of 505 paragraphs of *Botchan*, the number that Lindera segments and tags
exactly as MeCab does went from 352 to 438 with IPADIC and from 428 to 503
with UniDic. Text without `、`, `。` and forced cuts, such as text cut only
at `\n` and `\t`, is not affected. Carrying the context costs the 1-best
segmentation about 1% more instructions.

The result can still differ from one lattice over the segment:

- No word spans a cut. A dictionary entry that contains `、` or `。` never
  matches: MeCab reads UniDic's `一、二塁` as one noun, Lindera as `一`,
  `、` and `二塁`. An unknown word is not grouped across a cut, and a word
  that would span a forced cut is split there.
- The left-space penalty (ko-dic) does not apply to a word that starts
  right at a forced cut, even when whitespace precedes the cut.

The N-best results follow the same model. Within a segment, they are
exactly the N best paths of one lattice over the segment: `unique` compares
the word boundaries of the whole segment, and the cost threshold is
measured from the first result. Across segments, the results are the
cheapest combinations of one path per segment (see the next section). The
first result is the 1-best segmentation, unless several paths have exactly
the same cost. The costs of a line with `、` or `。` change, the first
result's included, because they now include the connections across the cuts
and the EOS connection only once.

There is no setting to restore the v6 behavior.

## N-best results cover the whole input

The N-best methods (`Segmenter::segment_nbest`, `Tokenizer::tokenize_nbest`,
their `_with_lattice` and worker forms, `lindera tokenize -N` and the
bindings' N-best methods) cut the input into sentences as the 1-best
segmentation does. Up to v6, they searched each sentence on its own, and
the k-th result joined the k-th path of every sentence. From the second
result on, that changed every sentence at once, so the results were not the
cheapest segmentations of the input and their costs could go down, and a
sentence with fewer than k paths was missing from the k-th result. v7.0.0
searches the input segment by segment (see the previous section) and
returns the N cheapest combinations of one path per segment, in ascending
order of total cost, and every result covers the whole input:

| Input (IPADIC, `-N 3`) | v6 | v7 |
| --- | --- | --- |
| `東京、です` | 3079 `東京 、 です`; 23850 `東 京 、 で す`; 24249 `東 京 、 で す` | 3357 `東京 、 です`; 12059 `東京 、 です` (`、` as `名詞,数`); 13633 `東京 、 で す` |
| `。東京` | 13 `。 東京`; 30433 `。 東 京`; 13520 `東 京` (no `。`) | 852 `。 東京`; 13569 `。 東 京`; 13636 `。 東 京` |

Each of these lines is one segment, so its v7 results are the three best
paths of one lattice over the line, the ones MeCab returns. The first result
is the 1-best segmentation, and the output for input with a single sentence
does not change. The cost threshold (`cost_threshold`,
`--nbest-cost-threshold`) is now measured over the whole input: a result is
kept when its total cost is within the threshold of the first result's. v6
applied the threshold to each sentence separately. With `unique`, the
results still have distinct word boundaries.

## Unknown words start at every position, as in MeCab

A word that is not in the dictionary is read as an unknown word, built from
the character categories of the dictionary's `char.def` (katakana, Latin
letters, digits, symbols and so on). Most categories are set to group: a
run of characters of one category, such as a run of katakana, becomes one
candidate, a *grouped unknown word*. Up to v6, normal mode created no unknown
words at the positions inside the last grouped unknown word, a shortcut
inherited from Kuromoji that MeCab does not have. After a dictionary word
that ends inside such a run, no unknown word could continue the path, so the
grouped unknown word that swallowed the dictionary word was often the only
way through. v7.0.0 creates unknown-word candidates at every position that a
path reaches, in every mode, under MeCab's condition: the category is set to
always create them (`INVOKE` in `char.def`), or no dictionary word starts at
that position. Decompose mode already did this, and its output does not
change.

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `それが「⁂⁂第一だ` (IPADIC) | `「⁂⁂`, one unknown noun that swallows the dictionary symbol `「` | `「` (`記号,括弧開`) and `⁂⁂` |
| `ジョン・レノンが歌う` (IPADIC) | `ジョン・レノン`, one unknown noun | `ジョン`, `・` and `レノン` |
| `ホテル・コルテシアに泊まる` (UniDic) | `ホテル・コルテシア`, one unknown noun | `ホテル`, `・` and `コルテシア` |
| `"좋아."` (ko-dic) | `아` (`EC`) and `."` (`SY`) | `아` (`EF`), `.` (`SF`) and `"` (`SY`) |
| `The` (CC-CEDICT) | `The` | `T` and `he` |

- **Japanese**: IPADIC, IPADIC-NEologd and UniDic put `・` in the katakana
  range of their `char.def`. After a dictionary word and `・`, an unknown
  katakana word could not start, so `ジョン・レノン` became one unknown word.
  Now the unknown word starts after them, as in MeCab. A run that starts
  with an unknown word, such as `レノン・ジョン`, stays one unknown word, in
  MeCab too. In the same way, a dictionary symbol is no longer merged into
  the unknown symbols after it.
- **Korean**: a sentence-final `."`, `?"`, `!"` or `.'` was one symbol
  (`SY`). Now `.`, `?` and `!` are sentence-final punctuation (`SF`), as in
  MeCab, and the ending before them can change with it, as in `"좋아."`.
- **Chinese**: with CC-CEDICT, a Latin word that starts with `A`, `B`, `P`,
  `Q` or `T` is split after that letter (`The` gives `T` and `he`, `TBS`
  gives `T`, `B` and `S`), and a run of Japanese kana is split into single
  characters, as MeCab splits them with the same dictionary. The splits
  come from the dictionary: every unknown word costs −3,200 whatever its
  length, the connection matrix has a single cell with no cost, and these
  letters are entries that cost −400, so a path with more words costs less:
  `T` and `he` (−3,600) beat `The` (−3,200). The shortcut hid this before.
  Jieba is not affected.
- **SudachiDict** output can change in the same kinds of runs.
- **N-best**: the first result changes as the 1-best segmentation does, and
  the other results can now include splits such as `G` and `oogle` for
  `Google` (UniDic).

With dictionaries built from the same sources, the number of lines that
Lindera segments and tags exactly as MeCab does rises, on top of the other
changes in this guide, from 589 to 596 of 600 Japanese sentences and from
438 to 459 of 505 paragraphs of *Botchan* with IPADIC, from 590 to 596 and
from 441 to 459 with IPADIC-NEologd, and from 877 to 1,498 of 1,500 Korean
sentences with ko-dic (with the left-space penalty off, as MeCab has none).
No line that matched before stops matching, and UniDic is unchanged on these
texts. On UD Japanese GSD with UniDic, 64 sentences change: 53 come closer
to the gold segmentation and part-of-speech tags, 10 move away (sign test
p = 3.4 × 10⁻⁸), and 1 is a tie. The 10 all match MeCab's output; GSD keeps
some names joined by `・` and some numbers as one word.

Text with many runs of katakana or Latin letters costs more, because every
position inside a run now gets candidates, as in MeCab. Against the same
release without this change, counting the speedups made with it, the
instruction count rises by 13% on UD Japanese GSD (Wikipedia text; 14% to
17% for N-best), and falls by 0.8% on *Botchan* and on Korean and by 14% to
16% in Decompose mode. A line of 10,240 katakana without a delimiter costs
3 to 5 times as much in 1-best and 3 to 15 times in N-best, and the peak
memory of `lindera tokenize -N 3` on it grows from 12 to 56 MiB with IPADIC
and from 18 to 127 MiB with UniDic.

There is no setting to restore the v6 behavior. `unknown_word_ladder(false)`
(`--disable-unknown-word-ladder`) still turns off the shorter unknown-word
candidates, but no longer reproduces the output of Lindera before v6.

## Rust API changes in `lindera_dictionary`

These affect only code that calls the lattice backtraces directly:
`lindera_dictionary::viterbi::Lattice` and
`lindera_dictionary::nbest::NBestGenerator`, also reachable as
`lindera::dictionary::viterbi::…`, `lindera::dictionary::Lattice` and
`lindera::dictionary::nbest::…`. The `Segmenter`, `Tokenizer`,
`SegmentWorker` and `AnalysisWorker` APIs, the language bindings and the
CLI are unaffected.

| v6 | v7 |
| --- | --- |
| `Lattice::tokens_offset() -> Vec<(usize, WordId)>` | `Lattice::tokens_offset() -> Vec<TokenOffset>` |
| `Lattice::tokens_offset_into(&mut Vec<(usize, WordId)>)` | `Lattice::tokens_offset_into(&mut Vec<TokenOffset>) -> Option<usize>` |
| `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<(Vec<(usize, WordId)>, i64)>` | `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<NBestPath>` |
| `NBestGenerator::next() -> Option<(Vec<(usize, WordId)>, i64)>` | `NBestGenerator::next() -> Option<NBestPath>` |
| — | New type aliases in `viterbi`: `TokenOffset = (usize, usize, WordId)` and `NBestPath = (Vec<TokenOffset>, i64)` |
| — | New in `viterbi`: `BosContext`, `LatticeOptions::bos`, `LatticeExit`, `Lattice::exits_into` and `Lattice::exit_tokens_offset_into` |
| — | New in `nbest`: `NBestGenerator::from_exit` and `NBestGenerator::next_with_bos` |

Each token is now `(start, end, word_id)` instead of `(start, word_id)`,
with byte offsets within the sentence. With whitespace skipped
(`LatticeOptions::skip_whitespace`, new in v7), a token does not always end
where the next one starts: skipped whitespace can lie in between, and only
the lattice knows how much of it belongs to the entry. The returned end leaves
the skipped whitespace out and keeps the whitespace an entry ends with (see
[Dictionary entries with whitespace are kept](#dictionary-entries-with-whitespace-are-kept)).
With whitespace kept in the lattice, as `set_text` and `set_text_nbest` do,
the end is the next token's start, or the end of the sentence for the last
token, as before. `nbest_tokens_offset` with `unique` now compares the
`(start, end)` pairs of the paths instead of their starts.

```rust
use lindera::dictionary::viterbi::TokenOffset;

// v6: a token ends where the next one starts
let offsets = lattice.tokens_offset();
for (i, &(start, word_id)) in offsets.iter().enumerate() {
    let end = offsets.get(i + 1).map_or(sentence.len(), |&(next, _)| next);
    handle(&sentence[start..end], word_id);
}

// v7: the end is returned
let offsets: Vec<TokenOffset> = lattice.tokens_offset();
for &(start, end, word_id) in &offsets {
    handle(&sentence[start..end], word_id);
}
```

The paths of `nbest_tokens_offset` and `NBestGenerator::next` hold the same
triples.

`tokens_offset_into` also returns the BOS index of the best path: the index
in `LatticeOptions::bos` of the BOS edge it starts from, `Some(0)` with the
default single BOS edge, and `None` when the lattice holds no complete path.
A call written as a statement compiles unchanged; where a `()` value is
expected, such as the body of a closure passed to `for_each`, add a `;`.

The new items are what the segmenter uses to carry the context across
`、` and `。` (see
[The context is carried across `、` and `。`](#the-context-is-carried-across--and-)).
They change nothing for existing code:

- `BosContext` and `LatticeOptions::bos`: a sentence can start from several
  BOS edges, each with a right context ID and a cost. The default, an empty
  slice, is the single BOS edge of v6.
- `LatticeExit`, `Lattice::exits_into` and `Lattice::exit_tokens_offset_into`:
  for each right context ID of the words that end the sentence, the best path
  to such a word without the EOS connection, and its tokens.
- `NBestGenerator::from_exit` and `NBestGenerator::next_with_bos`: the paths
  that end with the right context ID of an exit, without EOS, in ascending
  order of cost, and each path together with the BOS index it starts from.
  `next` returns the paths of `next_with_bos` without the BOS index.

`nbest_tokens_offset` is meant for the default single BOS edge: with
several, it does not say which one a path starts from. Use
`NBestGenerator::next_with_bos` there.

## Who does not need to act

- **Users of the language bindings and the CLI**: the Python, Node.js, Ruby,
  PHP, and WASM packages and `lindera-cli` keep their APIs and package names.
  The restructuring is internal to the Rust crates. The output changes are
  the IPADIC conjugation fix, which matters only if you read those two fields
  by name, the whitespace entries, which matter only for text with U+3000
  (IPADIC, IPADIC-NEologd, UniDic) or with spaces (SudachiDict), the
  segmentation of text that contains whitespace, the `char.def` fix, which
  matters only for text with `Ð`, `々` or `〇`, the context carried across
  `、` and `。`, which matters only for text with `、` or `。` inside a line,
  the N-best fix, which matters only for N-best results for input with
  more than one sentence, and the unknown words that start at every
  position, which matter for runs of katakana, symbols or Latin letters in
  normal mode.
- **Projects staying on `lindera = "6"`**: the 6.x releases of `lindera` and
  `lindera-analysis` remain on crates.io and keep working together. Nothing
  changes until you bump the major version.
- **Users of the segmenter API only**: code that uses `Segmenter`,
  `load_dictionary`, `Mode`, and the other `lindera::…` items of v6 compiles
  unchanged with `lindera = "7"`, unless it calls the lattice backtraces of
  `lindera::dictionary::Lattice` (see
  [Rust API changes in `lindera_dictionary`](#rust-api-changes-in-lindera_dictionary)).
  The default build now also compiles
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
- If you call `Lattice::tokens_offset`, `tokens_offset_into`,
  `nbest_tokens_offset` or `NBestGenerator::next`, destructure
  `(start, end, word_id)` and use the returned end instead of the next
  token's start.
- If you use the `()` value of `Lattice::tokens_offset_into`, for example
  as the body of a closure, add a `;`: it now returns the BOS index of the
  best path.

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

Everyone who segments text that contains whitespace:

- Expect MeCab's segmentation for such text (most visible with ko-dic). If
  you need the v6 handling of whitespace, set `skip_whitespace(false)`,
  `"skip_whitespace": false`, or `--disable-skip-whitespace`; the other
  output changes in this guide still apply.
- Expect the token of a dictionary entry that ends with a space (in
  IPADIC-NEologd, ko-dic and SudachiDict) to include that space, as in
  MeCab.

Users of any bundled dictionary with text that contains `Ð`, `々` or `〇`:

- Expect `Ð` to stay in the output, and `々` after an unknown kanji to be a
  token of its own.
- Rebuild or re-download dictionary directories made with v6.2.0 or earlier
  to get the new character categories.

Users of `lindera train`:

- Give every range line of your `char.def` at least one category.
- If your `char.def` has single-code-point or overlapping lines, retrain:
  the `%t` feature now follows those lines as MeCab does.

Users of Japanese text with `、` or `。` inside a line:

- Expect some words after `、` or `。` to be read in the context of the
  words before them, mostly as MeCab reads them. No setting restores the v6
  output.

Users of N-best results:

- Expect every result to cover the whole input, and the results from the
  second one on to change for input with more than one sentence.
- For a line with `、` or `。`, expect every cost to change, the first
  result's included: the costs are those of one lattice over the line.
- If you set a cost threshold, it now applies to the total cost of the
  whole input, not to each sentence.

Users of normal mode with text that has runs of katakana, symbols or Latin
letters:

- Expect such runs to be split where MeCab splits them: katakana joined by
  `・` (IPADIC, IPADIC-NEologd, UniDic), unknown symbols right after a
  dictionary symbol, Korean sentence-final punctuation such as `."`, and,
  with CC-CEDICT, Latin words that start with `A`, `B`, `P`, `Q` or `T`
  and runs of Japanese kana. No setting restores the v6 output, and
  `unknown_word_ladder(false)` no longer gives the output of Lindera before
  v6.
- Expect katakana- and Latin-heavy text to take about 13% more
  instructions, and very long katakana runs in N-best to take much more
  time and memory.

Language bindings and CLI:

- Nothing to do beyond taking the 7.0.0 release, apart from the dictionary,
  whitespace, `、` and `。`, N-best and unknown-word items above.
