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
starts or ends with it, skips whitespace in the lattice as MeCab does
(SudachiDict excepted, as Sudachi keeps it),
resolves overlapping `char.def` lines as MeCab does, carries the context
across `、` and `。` within a line as MeCab does, returns N-best results
that each cover the whole input, lets unknown words start at every position
as MeCab does, creates them from each character's default category and
groups the characters that share a category, as MeCab does, keeps the
dashes and tildes of dictionary entries as written
instead of rewriting them, removing the `normalize_details` setting, outputs
the first CSV row among tied dictionary entries as MeCab does, chooses the
segmentation whose last word starts later when two cost the same, as MeCab
does, and has the lattice backtraces of `lindera-dictionary` return each
token's end offset. Dictionary directories built or downloaded with v6.2.0
or earlier no longer load and must be rebuilt or downloaded again
(dictionary format version 3).
This guide lists every breaking change and the one-line fixes for each.

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
| **Dictionary format version 3: dictionary directories built or downloaded with v6.2.0 or earlier no longer load** | Anyone who loads a dictionary from files (`load_dictionary` with a path, `lindera tokenize --dict <dir>`, the bindings' path and byte loaders, dictionaries saved in the WASM binding's OPFS storage); not the embedded dictionaries or user dictionaries | Rebuild with the 7.0.0 `lindera build`, or download again with the 7.0.0 `lindera download` or from the 7.0.0 release assets |
| **IPADIC and IPADIC-NEologd: `conjugation_type` and `conjugation_form` now name the right columns** | Anyone who reads these two fields by name with IPADIC or IPADIC-NEologd (`Token::get`, `Token::as_value`, `lindera tokenize -o json`, binding schemas) | Expect the two values to trade places; rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **Dictionary entries with whitespace are kept** | Text with U+3000 (IPADIC, IPADIC-NEologd, UniDic), text with spaces (SudachiDict), and a few entries that ended with whitespace | Expect U+3000 to be `記号,空白` / `空白` and SudachiDict to segment spaced text much closer to Sudachi; rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **Whitespace is skipped in the lattice: the words on either side of a space connect directly, as in MeCab** | Anyone who segments text containing whitespace with `keep_whitespace` false (the default), most visibly with ko-dic; not with SudachiDict, which keeps whitespace in the lattice as Sudachi does | Expect MeCab's segmentation for such text; `skip_whitespace(false)` (`"skip_whitespace": false`, `--disable-skip-whitespace`) restores the v6 handling of whitespace, but not the other output changes in this guide |
| **Overlapping `char.def` lines: the last line decides** | Text with `Ð` (U+00D0; every dictionary except ko-dic), `々` (U+3005) or `〇` (U+3007); `lindera train` users whose `char.def` has single-code-point or overlapping lines | Expect `Ð` to stay in the output as a letter; rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **The context is carried across `、` and `。` within a line, as in MeCab** | Anyone who segments Japanese text with `、` or `。` inside a line | Expect some words after `、` or `。` to be read differently, mostly as MeCab reads them, and the N-best costs of such lines to change; no setting restores the v6 behavior |
| **N-best: every result covers the whole input** | Anyone who asks for N-best results (`segment_nbest`, `tokenize_nbest`, `lindera tokenize -N`, the bindings' N-best methods) for input with more than one sentence | Expect the cheapest segmentations of the whole input from the second result on, and one result without tokens for an empty input; a cost threshold now applies to the whole input |
| **Unknown words start at every position, as in MeCab** | Anyone who segments text with runs of katakana, symbols or Latin letters in normal mode, most visibly katakana joined by `・`, Korean sentence-final punctuation such as `."`, and Latin words with CC-CEDICT | Expect such runs to be split where MeCab splits them, and katakana- and Latin-heavy text to take about 13% more instructions; no setting restores the v6 behavior |
| **Unknown words come from each character's default category and group the characters that share a category, as in MeCab** | Anyone who segments text where a character with several categories meets other characters; in the bundled dictionaries, a hanja numeral followed by hanja with ko-dic, and `〇` (U+3007) with ko-dic, CC-CEDICT and Jieba; Rust code that relies on the order of `CharacterDefinition::lookup_categories` | Expect such text to be segmented as MeCab segments it (ko-dic `三國史記` is one `SH` word, Jieba `二〇〇八年北京奧運會` is `二 / 〇〇八年北京奧運會`); no setting restores the v6 behavior |
| **IPADIC and IPADIC-NEologd: dashes and tildes in dictionary entries are kept as written, as in MeCab** | Anyone who segments text with `―` (U+2015), `—` (U+2014), `～` (U+FF5E) or `〜` (U+301C) with IPADIC or IPADIC-NEologd | Expect entries spelled with `―` or `～` to match text spelled the same way, and text with `—` or `〜` to miss the 12 IPADIC and 1,085 IPADIC-NEologd entries it found only through the old rewrite (a `mapping` character filter can fold the spellings); rebuild or re-download dictionaries made with v6.2.0 or earlier |
| **Entries with the same surface, context IDs and cost: the first CSV row is output, as in MeCab** | Anyone who reads token details such as the reading or the base form with IPADIC, IPADIC-NEologd, UniDic or ko-dic, or with a user dictionary that has such entries; N-best users | Expect such words to get MeCab's details (IPADIC `狡い` reads `ズルイ`, not `コスイ`), a user entry to still win a tie with a system entry, and the first N-best result to always be the 1-best; no setting restores the v6 behavior |
| **Segmentations of equal cost: the one whose last word starts later is chosen, as in MeCab** | Anyone who segments text, in rare places (9 of about 54,000 lines in the texts compared) | Expect such lines to be segmented as MeCab segments them (IPADIC `腸窒扶斯` is `腸 / 窒扶 / 斯`, not `腸 / 窒 / 扶斯`); no setting restores the v6 behavior |
| **`lindera tokenize` removes only the line terminator from each input line and writes a result for every line** | CLI users whose input lines start or end with whitespace, or who read the wakati or N-best output of lines without tokens | Expect the offsets to index the input line, U+3000 at the start or end of a line to be a token (IPADIC `記号,空白`), `--keep-whitespace` to output the spaces there, and an empty line or a line of spaces to give an empty wakati line and one N-best result; no setting restores the v6 behavior |
| **WASM: the `TokenizerBuilder` settings apply whichever way the dictionary is set** | WASM users who combine `setDictionaryInstance()` with `setKeepWhitespace()` or filters, or `setDictionary()` with `setUserDictionaryInstance()` | Expect those settings and the user dictionary to take effect; a dictionary instance whose `char.def` has no `SPACE` category now needs `setKeepWhitespace(true)`, as a dictionary set by URI does |
| **The `normalize_details` setting is removed: `Metadata::new` and `CoreMetadata::new` take 10 arguments, Ruby's `Metadata.new` takes 8** | Code that creates or reads dictionary metadata: Rust (`Metadata`, `lindera_binding::CoreMetadata`, `PrefixDictionaryBuilderOptions`) and the `Metadata` classes of the Python, Ruby, PHP and Node.js bindings | Remove the argument, option or property; a `metadata.json` that still has the key loads, and the key is ignored |
| **`Lattice::tokens_offset`, `tokens_offset_into`, `nbest_tokens_offset` and `NBestGenerator::next` return `(start, end, WordId)`** | Rust code that calls these `lindera_dictionary` functions directly (also as `lindera::dictionary::viterbi::…`, `lindera::dictionary::Lattice` and `lindera::dictionary::nbest::…`) | Destructure three fields and use the returned end instead of the next token's start |
| **`Lattice::tokens_offset_into` returns the BOS index of the best path (`Option<usize>`)** | Rust code that uses the `()` value of `tokens_offset_into` | Nothing for a call written as a statement; otherwise ignore the new value |

The language bindings (Python, Node.js, Ruby, PHP, WASM) and the CLI keep
their package names, and their APIs apart from one setting: the `Metadata`
classes of the Python, Node.js, Ruby and PHP bindings no longer take or
expose `normalize_details` (see
[The `normalize_details` setting is removed](#the-normalize_details-setting-is-removed)).
Their version number moves to 7.0.0. Like the Rust crates, they no longer
load a dictionary directory built or downloaded with v6.2.0 or earlier (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).
There are thirteen output differences, all described below; the last two are
in the CLI only and in the WASM binding only. With IPADIC or
IPADIC-NEologd, the values reported under the names `conjugation_type` and
`conjugation_form` trade places. The dictionaries now contain the entries
whose surface is or starts or ends with whitespace, which changes the
segmentation of text that contains such whitespace. Text that contains
whitespace is segmented as MeCab segments it, except with SudachiDict, which
keeps whitespace in the lattice as Sudachi does. `Ð`, `々` and `〇` get the
character categories that MeCab gives them, which changes the segmentation
of text that contains them. Within a line, the words after `、` and `。`
are read in the context of the words before them, which changes the
segmentation of some such text and the N-best costs of every such line. For
input with more than one sentence, the N-best results from the second one on
are the cheapest segmentations of the whole input, and an empty input gets
one N-best result without tokens. And in normal mode, an
unknown word can start inside a run of characters of one kind, as in MeCab,
which splits some runs of katakana, symbols and Latin letters. Unknown words
come from each character's default category, and a grouped unknown word
goes on while each character shares a category with the one before it, as
in MeCab, which makes a ko-dic hanja numeral and the hanja after it one
word and `〇` a symbol with ko-dic, CC-CEDICT and Jieba. With IPADIC
and IPADIC-NEologd, the entries spelled with `―` or `～` are found under
that spelling, as in MeCab, which changes the segmentation of text that
contains `―`, `—`, `～` or `〜`. Among dictionary entries that share the
surface, the context IDs and the cost, the first CSV row is output instead
of the last, as in MeCab, which changes the reading, the base form or other
details of such words, and the first N-best result is always the 1-best.
When two segmentations cost exactly the same, the one whose last word starts
later is chosen, as in MeCab, which changes the segmentation of a few lines.
And `lindera tokenize` removes only the line terminator from each input line
instead of all the whitespace at its ends, which keeps U+3000 at the start
or end of a line and makes the offsets index the line, and it writes a
result for every line, also in the wakati format and with `-N`. And the
WASM binding's `TokenizerBuilder` applies all its settings and the user
dictionary whichever way the dictionary was set, where v6 ignored some of
them.
Otherwise, for the same input and dictionary, v7.0.0 produces the same
tokens with the same positional details as v6.2.0.

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
argument, metadata, and schema helpers — is unchanged, except that
`CoreMetadata` no longer has `normalize_details` (see
[The `normalize_details` setting is removed](#the-normalize_details-setting-is-removed)).
The published
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

## Dictionary directories must be rebuilt

v7.0.0 writes dictionaries in format version 3 and rejects a dictionary
directory built or downloaded with v6.2.0 or earlier, which is at version 2,
when it loads it:

```text
Dictionary 'ipadic' has format version 2, but this build of Lindera reads format version 3. To fix this, rebuild it with `lindera build`, or download a matching prebuilt dictionary with `lindera download`.
```

Version 3 lists each code point's default category first in `char_def.bin`,
because unknown words now come from that category only (see
[Unknown words come from the default category, as in MeCab](#unknown-words-come-from-the-default-category-as-in-mecab)).
A version 2 file has the same layout, so without the check it would load
and silently give some characters the wrong default category. The version
also covers the other changes in what the dictionary builder writes, which
a v6.2.0 dictionary does not have:
[the IPADIC conjugation names](#ipadic-conjugation-field-names-corrected),
[the entries with whitespace](#dictionary-entries-with-whitespace-are-kept),
[the overlapping `char.def` lines](#overlapping-chardef-lines-are-resolved-as-in-mecab)
and [the dash and tilde spellings](#dash-and-tilde-spellings-are-kept-as-written).

| Dictionary | What you do |
| --- | --- |
| Embedded (`embed-*` features) | Nothing: it is built when the crate is compiled |
| Downloaded with `lindera download` | Download it again with the 7.0.0 CLI, which stores it under a directory of its own version |
| Built with `lindera build` | Build it again with the 7.0.0 CLI |
| Downloaded from the release assets | Download the 7.0.0 asset |
| Saved in the WASM binding's OPFS storage | Remove it with `removeDictionary()` and download the 7.0.0 asset with `downloadDictionary()`; a `hasDictionary()` check alone keeps the old one |
| User dictionaries (CSV and `.bin`) | Nothing: they have no `char_def.bin` and no format version |

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
from the library. The embedded dictionaries (`embed-ipadic`,
`embed-ipadic-neologd`) and the dictionaries that the 7.0.0 CLI fetches with
`lindera download` carry the corrected names. A dictionary directory built
or downloaded with v6.2.0 or earlier no longer loads (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt));
rebuild it with the v7.0.0 `lindera-ipadic/metadata.json` (or
`lindera-ipadic-neologd/metadata.json`), or download the 7.0.0 release asset.

If you build dictionaries with your own copy of the IPADIC `metadata.json`,
swap `"conjugation_form"` and `"conjugation_type"` in its
`dictionary_schema.fields` list.

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

The change is in the dictionary builder. The embedded dictionaries and the
dictionaries that the 7.0.0 CLI fetches with `lindera download` include
these entries; a dictionary directory built or downloaded with v6.2.0 or
earlier no longer loads (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).

## Whitespace is skipped in the lattice

With `keep_whitespace` false (the default), v7.0.0 skips whitespace in the
Viterbi lattice, as MeCab does: the word after a space connects directly to
the word before it. v6 dropped whitespace from the output but kept it in the
lattice as the `SPACE` unknown word, so the words around a space connected to
that node instead of to each other. How much that changed depends on the
dictionary's connection costs for the node: in ko-dic they are all zero, so
every space cut the context between the words around it; in IPADIC and
UniDic they are not zero, and the node's costs stood in for the connection
between the words.

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
no such setting, but it no longer loads anyway; the rebuilt one has it.

Token surfaces and offsets never include the skipped whitespace; whitespace
that belongs to a dictionary entry, inside it or at its end, stays in that
entry's token, as in MeCab (see the previous section). Skipping does not
change the segmentation of text without whitespace. `keep_whitespace(true)`
keeps whitespace in the lattice, so its output is the same as in v6; beyond
the whitespace tokens, it can differ from the default v7 output, which skips
whitespace.

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

The language bindings have no setter for it. The Python, Node.js, Ruby and
PHP bindings take it only through a configuration file loaded with
`from_file` / `fromFile`; the WASM binding cannot change it and uses the
dictionary's default.

## Overlapping char.def lines are resolved as in MeCab

A dictionary's `char.def` can list the same code point on several lines. Up
to v6.2.0, the dictionary builder gave such a code point the categories of
every line that covers it. v7.0.0 uses only the last line, as MeCab,
vibrato and Kuromoji do. In the bundled dictionaries, three characters
change (ko-dic calls the kanji categories `HANJA` and `HANJANUMERIC`, and
CC-CEDICT and Jieba call them `CHINESE` and `CHINESENUMERIC`):

| Character | v6 categories | v7 categories | Effect |
| --- | --- | --- | --- |
| `Ð` (U+00D0) | `SPACE`, `ALPHA` | `ALPHA` | Every dictionary except ko-dic maps U+00D0 to `SPACE` (a typo for U+000D inherited from mecab-ipadic) before the `ALPHA` range, so `Ð` was dropped as whitespace and split the word around it. `GUÐMUNDUR さん` now gives `GUÐMUNDUR` and `さん` instead of `GU`, `MUNDUR` and `さん` |
| `々` (U+3005) | `KANJI`, `SYMBOL` | `SYMBOL` | Dictionary words such as `人々` and `佐々木` are unchanged. After a kanji that the dictionary does not know, `々` is a token of its own, as in MeCab: `龘々` gives `龘` and `々` instead of one unknown word. With ko-dic, `々` is tagged `SY` instead of `SH` |
| `〇` (U+3007) | `KANJI`, `SYMBOL`, `KANJINUMERIC` | `SYMBOL`, `KANJINUMERIC` | Runs of numerals can be grouped differently. With ko-dic, `二〇二六年` is one unknown word (`SH`) instead of `二`, `〇`, `二六` and `年` (see also [Unknown words come from the default category, as in MeCab](#unknown-words-come-from-the-default-category-as-in-mecab)) |

The kanji numerals `一` to `九`, `十`, `百`, `千`, `万`, `億` and `兆` are
also listed on two lines, but keep their categories.

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

The change is in the dictionary builder. The embedded dictionaries and the
dictionaries that the 7.0.0 CLI fetches with `lindera download` have the new
categories; a dictionary directory built or downloaded with v6.2.0 or
earlier no longer loads (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).

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
first result is the 1-best segmentation. The costs of a line with `、` or
`。` change, the first result's included, because they now include the
connections across the cuts and the EOS connection only once.

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

An empty input now gets one result without tokens, with the connection
cost from BOS to EOS, as in MeCab and as an input of only whitespace
already did; v6 gave no result, because an empty input has no sentence.
This applies to `segment_nbest`, `tokenize_nbest`, their worker forms, the
bindings' N-best methods and `lindera tokenize -N`, which now writes a
result for an empty line.

## Unknown words start at every position, as in MeCab

A word that is not in the dictionary is read as an unknown word, built from
the character categories of the dictionary's `char.def` (katakana, Latin
letters, digits, symbols and so on). Most categories are set to group: a
run of characters that share a category, such as a run of katakana, becomes
one candidate, a *grouped unknown word*. Up to v6, normal mode created no unknown
words at the positions inside the last grouped unknown word, a shortcut
inherited from Kuromoji that MeCab does not have. After a dictionary word
that ends inside such a run, no unknown word could continue the path, so the
grouped unknown word that swallowed the dictionary word was often the only
way through. v7.0.0 creates unknown-word candidates at every position that a
path reaches, in every mode, under MeCab's condition: the category that
creates them is set to always create them (`INVOKE` in `char.def`), or no
dictionary word starts at that position. Decompose mode already did this, and its output does not
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

## Unknown words come from the default category, as in MeCab

A character can have more than one category in `char.def`: the kanji
numerals are `KANJINUMERIC KANJI` in IPADIC and UniDic,
`HANJANUMERIC HANJA` in ko-dic and `CHINESENUMERIC CHINESE` in CC-CEDICT
and Jieba, and `〇` (U+3007) is `SYMBOL` plus the numeral category. The first category of the line that
decides a character's categories is its *default category*. Up to v6.2.0,
Lindera created unknown-word candidates for every category of a character,
and went on with a grouped unknown word only while the next character had
the same category at the same position of its category list. v7.0.0
follows MeCab:

- Unknown words come from the default category only, with its `INVOKE`,
  `GROUP` and `LENGTH` settings and its `unk.def` entries.
- A grouped unknown word goes on while each character shares a category
  with the one before it.
- The shorter candidates of the length ladder go on while each character
  shares a category with the first one.

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `三國史記` (ko-dic) | `三` (`SH`), `國史` (`NNG`) and `記` (`NNG`) | `三國史記` (`SH`) |
| `十人十色` (ko-dic) | `十` (`SH`), `人` (`NNG`), `十` (`SH`) and `色` (`NNG`) | `十人十色` (`SH`) |
| `二十歲` (ko-dic) | `二十` (`SH`) and `歲` (`SH`) | `二十歲` (`SH`) |
| `〇〇〇` (ko-dic) | `〇〇〇` (`SH`) | `〇〇〇` (`SY`) |
| `二〇〇八年北京奧運會` (Jieba) | `二`, `〇〇`, `八`, `年`, `北京奧運` and `會` | `二` and `〇〇八年北京奧運會` (`w`) |
| `二〇二六年` (CC-CEDICT) | `二`, `〇`, `二`, `六` and `年` | `二` and `〇二六年` |

- **Korean**: many ko-dic numerals, such as `三` and `十`, are not
  dictionary words, and their default category `HANJANUMERIC` groups, so a
  numeral and the hanja after it become one `SH` word, as in MeCab.
- **`〇`**: its default category is `SYMBOL`, which creates unknown words
  even where a dictionary word starts, so it is a symbol (ko-dic `SY`,
  Jieba `w`). With CC-CEDICT and Jieba, its group runs on through the
  numerals and the Chinese characters after it, also in MeCab. A fix of the
  `char.def` of these two dictionaries is tracked in
  [#1162](https://github.com/lindera/lindera/issues/1162).
- **Japanese**: IPADIC, IPADIC-NEologd and UniDic have the kanji numerals
  and `〇` as dictionary words, and no line of the texts compared changes.
  SudachiDict was not compared.

With dictionaries built from the same sources, the number of lines that
Lindera segments and tags exactly as MeCab does rises, on top of the other
changes in this guide, from 3,913 to 3,917 of the 3,919 non-empty lines of
*Sangnoksu* with ko-dic (with the left-space penalty off, as MeCab has
none); every generated sentence that combines the characters with several
categories with other characters now matches MeCab with IPADIC,
IPADIC-NEologd, UniDic, ko-dic, CC-CEDICT and Jieba. No line that matched
before stops matching. The instruction
count falls by 0.3% to 4.8%.

`CharacterDefinition::lookup_categories` (in
`lindera_dictionary::dictionary::character_definition`) returns the default
category first, then the other categories in category id order. The 7.0.0
`char_def.bin` stores them in this order, which is why the dictionary format
version changes (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).

There is no setting to restore the v6 behavior.

## Dash and tilde spellings are kept as written

Japanese text spells the dash and the tilde in two ways each: the dash as
`―` (U+2015, HORIZONTAL BAR) or `—` (U+2014, EM DASH), the tilde as `～`
(U+FF5E, FULLWIDTH TILDE) or `〜` (U+301C, WAVE DASH). Up to v6.2.0, the
dictionary builder rewrote `―` to `—` and `～` to `〜` in every entry of
IPADIC and IPADIC-NEologd, whose `metadata.json` turned the rewrite on with
`normalize_details`. It rewrote the surface, the spelling under which the
segmenter finds the entry in the text, and the detail fields, such as the
base form and the reading. The text to segment was not rewritten, so an
entry that the dictionary source spells with `―` or `～` matched only text
that spelled it with `—` or `〜`. The rewrite was a workaround for an old
EUC-JP conversion. The dictionary sources are UTF-8 now, and `―` is the dash
that the common Shift_JIS and EUC-JP converters produce and that Aozora
Bunko texts use. v7.0.0 keeps every entry as written in the dictionary's CSV
files, as MeCab does:

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `ＣＤ―ＲＯＭ` (IPADIC) | `ＣＤ`, `―` and `ＲＯＭ` | `ＣＤ―ＲＯＭ` (`名詞,一般`) |
| `――` (IPADIC) | An unknown noun (`名詞,サ変接続`) | `記号,一般` |
| `あいいれなぁ～い` (IPADIC-NEologd) | `あいいれなぁ`, `～` and `い` | `あいいれなぁ～い` (`形容詞,自立`) |
| `CD―ROM` (IPADIC-NEologd) | `CD`, `―` and `ROM` | `CD―ROM` (`名詞,一般`) |
| `ＣＤ—ＲＯＭ` (IPADIC) | `ＣＤ—ＲＯＭ` (`名詞,一般`) | `ＣＤ`, `—` and `ＲＯＭ` |
| `——` (IPADIC) | `記号,一般` | An unknown noun (`名詞,サ変接続`) |

The detail fields keep the CSV spelling as well: the entry `ＣＤ―ＲＯＭ`
gives the base form `ＣＤ―ＲＯＭ`, where v6 gave `ＣＤ—ＲＯＭ`.

With dictionaries built from the same sources, the number of lines that
Lindera segments and tags exactly as MeCab does rises, on top of the other
changes in this guide, from 459 to 503 of 505 paragraphs of *Botchan* (from
2,690 to 2,744 of its 2,746 sentences) and from 3,377 to 3,581 of 3,587
paragraphs of *Kokoro* and *I Am a Cat*, two other novels from Aozora Bunko.
The figures are the same with IPADIC and IPADIC-NEologd, and no line that
matched before stops matching.

Text that writes these characters as `—` or `〜` no longer finds the entries
that it found only through the rewrite: 12 in IPADIC, all spelled with `―`,
and 1,085 in IPADIC-NEologd, most of them spelled with `～`. In the table
above, `ＣＤ—ＲＯＭ` is split and `——` is an unknown noun, as in MeCab.
IPADIC-NEologd has most of its words with a tilde in both spellings, so text
with either tilde still finds them.

To have both spellings find the entries, map the text to the dictionary's
spelling with the `mapping` character filter. IPADIC spells all its entries
with a dash with `―` and all those with a tilde with `〜`, so map `—` to `―`
and `～` to `〜`:

```sh
lindera tokenize --dict ipadic --char-filter 'mapping:{"mapping":{"—":"―","～":"〜"}}'
```

```yaml
# Configuration file
character_filters:
  - kind: mapping
    args:
      mapping:
        "—": "―"
        "～": "〜"
```

With this filter, `ＣＤ—ＲＯＭ` is one word and `——` is `記号,一般` again,
and text with `～` also finds IPADIC's entries with `〜`, which it did not
in v6 either. The token's surface is the mapped text, such as `ＣＤ―ＲＯＭ`;
its byte offsets point into the original text. Mapping the tilde the other
way, `〜` to `～`, would hide IPADIC's entries with `〜`. With
IPADIC-NEologd, a mapping trades words: some of its words exist in only one
spelling, 129 only with `―` and 85 only with `—`, 957 only with `～` and
1,813 only with `〜`, so mapping one spelling to the other makes text find
the words of one and miss those of the other. NFKC normalization
(the `unicode_normalize` character filter) does not fold these characters:
it turns `～` into `~` and leaves `―`, `—` and `〜` as they are.

User dictionaries were never rewritten and do not change. The change is in
the dictionary builder. The embedded dictionaries and the dictionaries that
the 7.0.0 CLI fetches with `lindera download` keep the spellings; a
dictionary directory built or downloaded with v6.2.0 or earlier no longer
loads (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).
UniDic, ko-dic, CC-CEDICT, Jieba and SudachiDict never turned the rewrite
on.

### The `normalize_details` setting is removed

`normalize_details` turned this rewrite on and did nothing else, so v7.0.0
removes it from the bundled `metadata.json` files and from the APIs that
create or expose dictionary metadata:

| API | Change |
| --- | --- |
| Rust | `Metadata::new` takes 10 arguments instead of 11 (`normalize_details` was the ninth), and the `Metadata::normalize_details` field and the `PrefixDictionaryBuilderOptions::normalize_details` method are gone. For binding authors, `lindera_binding::CoreMetadata` and `CoreMetadata::new` change the same way. See [Rust API changes in `lindera_dictionary`](#rust-api-changes-in-lindera_dictionary) |
| Python | The `normalize_details` keyword argument of `Metadata`, the property and the `to_dict()` key are gone; passing the keyword raises `TypeError` |
| Ruby | `Metadata.new` takes 8 positional arguments instead of 9, so passing 9 raises `ArgumentError`; the `normalize_details` reader and the `to_h` key are gone |
| PHP | The last argument of the `Metadata` constructor, `normalize_details`, the property and the `toArray()` key are gone; passing the argument raises `Error` (by name) or `ArgumentCountError` (as a ninth argument) |
| Node.js | The `normalizeDetails` option of `Metadata`, the property and the `toObject()` key are gone; TypeScript rejects the option, and at run time it is ignored |
| WASM | No change: its `Metadata` never exposed the setting |

A `metadata.json` that still contains `normalize_details`, such as one
written by v6, loads as before, whether it comes with a built dictionary,
is passed to `lindera build` or is read by a binding; the key is ignored
and is not written back.

## Tied entries resolve to the first CSV row, as in MeCab

A dictionary can have several entries with the same surface, the same left
and right context IDs and the same cost, which differ only in their details,
such as the reading or the base form. Such entries are tied: a path costs
the same with any of them, so the order in which the segmenter considers
them decides which one it outputs. Up to v6.2.0, Lindera added the
dictionary entries that start at a position in reverse order, and the search
keeps the earlier of two candidates with equal cost, so the last CSV row of
a tied group won. The reverse order came in with v2.1.0, apparently by
accident. v7.0.0 outputs the first row, as MeCab does:

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `あいつは狡い` (IPADIC) | `狡い` reads `コスイ` | `狡い` reads `ズルイ` |

IPADIC's `Adj.csv` has the two rows in this order:

```text
狡い,19,19,4682,形容詞,自立,*,*,形容詞・アウオ段,基本形,狡い,ズルイ,ズルイ
狡い,19,19,4682,形容詞,自立,*,*,形容詞・アウオ段,基本形,狡い,コスイ,コスイ
```

Tied entries exist for 5,637 surfaces in IPADIC, 90,991 in IPADIC-NEologd,
3,324 in UniDic and 175 in ko-dic. In the Japanese dictionaries, the rows of
a tied group never differ in the part of speech, only in fields such as the
reading and the base form; in ko-dic, 5 groups also differ in the part of
speech. With each tied surface on its own line, the number of surfaces whose
full details equal those of MeCab 0.996 rises from 1,607 to 5,637 of 5,637
with IPADIC, from 524 to 3,324 of 3,324 with UniDic, and from 8,205 to
86,319 of 86,464 with IPADIC-NEologd (surfaces with whitespace or control
characters left out). The other 145 IPADIC-NEologd surfaces contain `、`,
which no word spans (see
[The context is carried across `、` and `。`](#the-context-is-carried-across--and-)),
or belong to groups whose rows MeCab reads in another order (see below).
With ko-dic, all 175 agree when MeCab's dictionary is built from the CSV
files in name order.

On running text, the number of lines whose full details equal MeCab's rises,
on top of the other changes in this guide, from 7,989 to 8,096 of the 8,100
sentences of UD Japanese GSD and from 3,278 to 3,583 of the 3,587 paragraphs
of *Kokoro* and *I Am a Cat* with IPADIC, and from 3,394 to 3,571 of the
same 3,587 paragraphs with UniDic. No line is segmented differently. In all
the texts compared, 15 tokens change their part of speech, all to MeCab's.
Most of them are entries of the same surface with different context IDs and
costs whose paths happen to cost the same in that sentence, and the same
order decides between them: UniDic, for example, now reads `御前` as a
pronoun (`代名詞`) instead of a common noun (`名詞,普通名詞,一般`), as
MeCab does.

User dictionaries follow the same rule: among tied user entries, the first
row is now chosen instead of the last. A user entry that ties with a system
entry is still chosen over it, as in v6. This is the one intended difference
from MeCab, which chooses the system entry. Such a tie needs a user entry in
the detailed format that copies the context IDs and the cost of a system
entry, for example to give a word another reading. An entry in the simple
three-column format gets the cost and the context IDs of the dictionary's
`metadata.json` (`default_word_cost`, `default_left_context_id` and
`default_right_context_id`: `-10000`, `0` and `0` for IPADIC), so it
practically never ties with a system entry.

N-best search uses the same order. In addition, the first N-best result is
now always the 1-best segmentation, the output of `segment`; up to v6 it
could be another path when several paths had exactly the same cost. The
costs of the N-best results do not change: only results with the same cost
can change places, which can also change which of them is cut off at the
`n`-th result. N-best search takes about 3% to 4% more instructions
(`-N 3`); the speed of the 1-best segmentation does not change (within
0.2%).

With `unique` (`--nbest-unique`), every result is the cheapest path of its
word boundaries, and of tied entries it now holds the first CSV row in
every result, as the 1-best segmentation does. Up to v6, unique N-best went
through the paths in order of cost and kept the first path of each
segmentation, which could hold either row of a tied pair, and it went
through every path of a segmentation
before it reached the next one: a short line whose words have many entries
could take tens of seconds and gigabytes. The search now goes over word
boundaries, so it does not depend on the number of paths. Again, only
results with the same cost can change.

MeCab's `mecab-dict-index` reads the CSV files in the order the file system
lists them (readdir order), so for tied groups whose rows are in different
files (in IPADIC-NEologd and ko-dic), MeCab's choice depends on the file
system, while Lindera reads the files in name order.

The change is in the segmenter, not in the dictionary files. There is no
setting to restore the v6 behavior.

## Equal-cost segmentations keep the later-starting word, as in MeCab

Two segmentations can reach the same position with exactly the same cost
while their last words start at different positions. MeCab keeps the one
whose last word starts later: it puts every word at the head of the list of
words that end where it ends, so the words that start later come first in
that list, and of words of equal cost it keeps the first. Up to v6.2.0,
Lindera kept the one whose last word starts earlier. v7.0.0 chooses as
MeCab does:

| Input (dictionary) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| `腸窒扶斯` (IPADIC) | `腸 / 窒 / 扶斯` | `腸 / 窒扶 / 斯` |
| `にフリーホイールダイオードや` (IPADIC) | `フリーホイールダイオード` (one unknown word) | `フリー / ホイール / ダイオード` |
| `家系譜` (UniDic) | `家 / 系譜` | `家系 / 譜` |
| `차나 마셔` (ko-dic) | `차나 / 마셔` | `차 / 나 / 마셔` |

Both segmentations of `腸窒扶斯` cost 28,382. Such ties are rare. The texts
compared are *Botchan* (as one file, as paragraphs and as sentences), the
3,587 paragraphs of *Kokoro* and *I Am a Cat* and the 8,100 sentences of UD
Japanese GSD, each with IPADIC, IPADIC-NEologd and UniDic, and the 7,759
lines of *Sangnoksu* with ko-dic: about 54,000 lines in all. 9 of them
change, all to MeCab's segmentation, and none moves away from it. In Decompose mode, which MeCab does not have, 0 to 2 lines
per text change in the same way.

When whitespace is skipped (see
[Whitespace is skipped in the lattice](#whitespace-is-skipped-in-the-lattice)),
the word that ends later wins first, as in MeCab, which looks the next word
up from every position a word ends at: an entry that ends with the space
wins over a word that ends before it. One difference from MeCab remains in
theory: after skipped whitespace, MeCab keeps a node for every position the
previous word can end at, while Lindera keeps one, so two words that start
right after the same whitespace and tie through previous words that end at
different positions can be chosen differently. This needs an entry that
ends with whitespace and an exact tie, and it does not occur in the texts
compared.

Among words that start at the same position, the order of
[Tied entries resolve to the first CSV row, as in MeCab](#tied-entries-resolve-to-the-first-csv-row-as-in-mecab)
still applies, and the first N-best result is still the 1-best
segmentation. When the context is carried across `、` or `。`, a tie
between the words that end the previous sentence resolves the same way, as
in one lattice over the line: the word that starts later wins (see
[The context is carried across `、` and `。`](#the-context-is-carried-across--and-)).

The 1-best segmentation takes 0.6% to 0.9% more instructions with IPADIC,
IPADIC-NEologd and UniDic, with no measurable change in time, and about 2%
more instructions and 2% to 5% more time with ko-dic. N-best search
(`-N 3`) takes about 1% more instructions.

The change is in the segmenter, not in the dictionary files. There is no
setting to restore the v6 behavior.

## `lindera tokenize` removes only the line terminator and writes every line

`lindera tokenize` reads its input line by line. Up to v6.2.0, it removed
all the whitespace at both ends of each line before tokenizing it: spaces,
tabs, U+3000, U+00A0 and the other Unicode whitespace (`str::trim`). The
library, the language bindings and MeCab do not. v7.0.0 removes only the
line terminator (`\n`, or `\r\n`) and tokenizes the rest of the line as the
library does:

| Input line (IPADIC) | v6 | v7 (as MeCab) |
| --- | --- | --- |
| Two spaces, `東京`, a space | `東京` at bytes 0-6 | `東京` at bytes 2-8 |
| U+3000, `東京` | `東京` at 0-6 | U+3000 (`記号,空白`) at 0-3, `東京` at 3-9 |
| `東京`, U+3000 | `東京` at 0-6 | `東京` at 0-6, U+3000 (`記号,空白`) at 6-9 |
| U+00A0, `東京`, U+00A0 | `東京` at 0-6 | U+00A0 (unknown, `記号,一般`) at 0-2, `東京` at 2-8, U+00A0 at 8-10 |

- Spaces and tabs are `SPACE` characters, which the segmenter skips and
  drops from the output, so they still give no tokens; only the byte
  offsets of the tokens after them change, and now index the input line,
  as the library's offsets do.
- U+3000 at the start or end of a line is a token, as in the middle of a
  line, with IPADIC, IPADIC-NEologd (`記号,空白`) and UniDic (`空白`) (see
  [Dictionary entries with whitespace are kept](#dictionary-entries-with-whitespace-are-kept)).
  In the 538 lines of *Botchan* as one file, where 202 lines start with an
  indent of U+3000, 206 such tokens appear and no other token changes. Text
  without whitespace at the ends of its lines, such as the 8,100 sentences
  of UD Japanese GSD and the 3,587 paragraphs of *Kokoro* and *I Am a Cat*,
  gives the same output as before.
- With `--keep-whitespace`, the spaces at the start and end of a line are
  tokens too: two spaces, `東京` and a space give three tokens.
- With `-N`, a line of spaces or tabs and an empty line give one result
  without tokens (`NBEST 1` and `EOS`), as in MeCab, instead of no result
  (see [N-best results cover the whole input](#n-best-results-cover-the-whole-input)
  for the empty input). The costs of a line with U+3000 at an end include
  that token.
- The wakati format writes a line for every input line, or for every
  result with `-N`: a line without tokens, such as an empty line or a line
  of spaces, gives an empty line, as in MeCab. v6 wrote nothing for it, so
  the output had fewer lines than the input (*Botchan* as one file: 505
  output lines for its 538 input lines, 33 of them empty).
- The `\r` of a line that ends with `\r\n` is removed, and only that one
  `\r`. MeCab removes only the `\n` and outputs the `\r` as an unknown
  word; Lindera removes it so that text with Windows line endings does not
  end every line with a `\r` token.

The library and the language bindings never trimmed their input, so they do
not change, except for the N-best result of an empty input. The change is
in the CLI, not in the dictionary files. No option restores the v6 behavior; to get the v6 offsets and tokens,
remove the whitespace at the ends of each line before passing the text to
`lindera tokenize`.

## WASM: the builder settings apply whichever way the dictionary is set

Up to v6.2.0, `TokenizerBuilder.build()` in the WASM binding ignored part of
the builder's configuration, depending on how the dictionary was set:

| Dictionary set with | Ignored up to v6.2.0 |
| --- | --- |
| `setDictionaryInstance()`, such as a dictionary loaded from OPFS with `loadDictionaryFromBytes()` | `setKeepWhitespace()`, `appendCharacterFilter()` and `appendTokenFilter()` |
| `setDictionary()`, such as an `embedded://` dictionary | The user dictionary set with `setUserDictionaryInstance()` |

v7.0.0 applies every setting and the user dictionary either way, as the
[Tokenizer API](lindera-wasm/tokenizer_api.md) page describes. For example,
with IPADIC loaded from bytes, `setKeepWhitespace(true)` and the
`unicode_normalize` (NFKC) and `lowercase` filters turn `Ｌｉｎｄｅｒａ 東京`
into `lindera`, a space and `東京`; v6 gave `Ｌｉｎｄｅｒａ` and `東京`.

With a dictionary instance, `build()` now also checks what it checks for a
dictionary set by URI: unless whitespace is kept, the dictionary's
`char.def` must define the `SPACE` category. Every bundled dictionary does;
a custom dictionary without it now needs `setKeepWhitespace(true)`.

The other bindings, the CLI and the library do not change, and neither do
the dictionary files. No option restores the v6 behavior; to get it, leave
out the settings that v6 ignored.

## Rust API changes in `lindera_dictionary`

These affect only code that calls the lattice backtraces directly, that
creates dictionary metadata with `Metadata::new`, or that reads the
character categories of a dictionary:
`lindera_dictionary::viterbi::Lattice` and
`lindera_dictionary::nbest::NBestGenerator`, also reachable as
`lindera::dictionary::viterbi::…`, `lindera::dictionary::Lattice` and
`lindera::dictionary::nbest::…`, and
`lindera_dictionary::dictionary::metadata::Metadata`, also reachable as
`lindera::dictionary::Metadata`, and
`lindera_dictionary::dictionary::character_definition::CharacterDefinition`.
The `Segmenter`, `Tokenizer`,
`SegmentWorker` and `AnalysisWorker` APIs and the CLI are unaffected; the
language bindings change only in their `Metadata` class (see
[The `normalize_details` setting is removed](#the-normalize_details-setting-is-removed)).

| v6 | v7 |
| --- | --- |
| `Lattice::tokens_offset() -> Vec<(usize, WordId)>` | `Lattice::tokens_offset() -> Vec<TokenOffset>` |
| `Lattice::tokens_offset_into(&mut Vec<(usize, WordId)>)` | `Lattice::tokens_offset_into(&mut Vec<TokenOffset>) -> Option<usize>` |
| `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<(Vec<(usize, WordId)>, i64)>` | `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<NBestPath>` |
| `NBestGenerator::next() -> Option<(Vec<(usize, WordId)>, i64)>` | `NBestGenerator::next() -> Option<NBestPath>` |
| — | New type aliases in `viterbi`: `TokenOffset = (usize, usize, WordId)` and `NBestPath = (Vec<TokenOffset>, i64)` |
| — | New in `viterbi`: `BosContext`, `LatticeOptions::bos`, `LatticeExit`, `Lattice::exits_into` and `Lattice::exit_tokens_offset_into` |
| — | New in `nbest`: `NBestGenerator::from_exit` and `NBestGenerator::next_with_bos` |
| — | New in `nbest`: `UniqueNBestGenerator` |
| `Metadata::new(name, encoding, simple_word_cost, default_left_context_id, default_right_context_id, default_field_value, flexible_csv, skip_invalid_cost_or_id, normalize_details, schema, userdic_schema)` | `Metadata::new(name, encoding, simple_word_cost, default_left_context_id, default_right_context_id, default_field_value, flexible_csv, skip_invalid_cost_or_id, schema, userdic_schema)` |
| `Metadata::normalize_details` | Removed |
| `PrefixDictionaryBuilderOptions::normalize_details(value)` | Removed |
| — | New public field `Metadata::skip_whitespace: Option<bool>` (also `lindera_binding::CoreMetadata::skip_whitespace`) |
| `CharacterDefinition::lookup_categories(c)` returns the categories of every line that covers `c` | Returns the categories of the last line that covers `c`, its default category first and the others in category id order |
| `DICTIONARY_FORMAT_VERSION` is `2` | `3` |

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

`UniqueNBestGenerator` has the methods of `NBestGenerator` (`new`,
`from_exit`, `next` and `next_with_bos`) and yields the cheapest path of
each pair of word boundaries and BOS edge, in ascending order of cost; it is
what `nbest_tokens_offset` with `unique` and the segmenter use.

`nbest_tokens_offset` is meant for the default single BOS edge: with
several, it does not say which one a path starts from. Use
`NBestGenerator::next_with_bos` there.

`Metadata::new` loses its ninth argument, `normalize_details`: delete it
from the call. `Metadata` no longer has the field, and
`PrefixDictionaryBuilderOptions` (in `lindera_dictionary::builder::prefix_dictionary`)
no longer has the method, because the builder keeps every entry as written
(see
[Dash and tilde spellings are kept as written](#dash-and-tilde-spellings-are-kept-as-written)).
`Metadata` still reads a `metadata.json` that contains the key, and ignores
it. For binding authors, `lindera_binding::CoreMetadata` loses the field the
same way, and `CoreMetadata::new` takes 10 arguments instead of 11.

`Metadata` gains the public field `skip_whitespace: Option<bool>`, the
dictionary's default for whitespace skipping (see
[Whitespace is skipped in the lattice](#whitespace-is-skipped-in-the-lattice)),
and so does `lindera_binding::CoreMetadata`. Neither struct is
`#[non_exhaustive]`, so a struct literal that lists every field without
`..` no longer compiles: add `skip_whitespace: None`, or end the literal
with `..Default::default()`. `Metadata::new` and `CoreMetadata::new` set it
to `None`, which skips whitespace.

`CharacterDefinition::lookup_categories` keeps its signature. Its first
category is the one that creates unknown words (see
[Unknown words come from the default category, as in MeCab](#unknown-words-come-from-the-default-category-as-in-mecab));
code that only checks whether a character has a category is unaffected.
`DICTIONARY_FORMAT_VERSION`, the version that the builder writes and the
loaders accept, is now 3 (see
[Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt)).

## Who does not need to act

- **Users of the language bindings and the CLI**: the Python, Node.js, Ruby,
  PHP, and WASM packages and `lindera-cli` keep their package names, and
  their APIs apart from the `normalize_details` setting of `Metadata` (see
  [The `normalize_details` setting is removed](#the-normalize_details-setting-is-removed)).
  The restructuring is internal to the Rust crates. A dictionary directory
  built or downloaded with v6.2.0 or earlier has to be rebuilt or downloaded
  again (see
  [Dictionary directories must be rebuilt](#dictionary-directories-must-be-rebuilt));
  the embedded dictionaries need nothing. The output changes are
  the IPADIC conjugation fix, which matters only if you read those two fields
  by name, the whitespace entries, which matter only for text with U+3000
  (IPADIC, IPADIC-NEologd, UniDic) or with spaces (SudachiDict), the
  segmentation of text that contains whitespace, the `char.def` fix, which
  matters only for text with `Ð`, `々` or `〇`, the context carried across
  `、` and `。`, which matters only for text with `、` or `。` inside a line,
  the N-best fix, which matters only for N-best results for input with
  more than one sentence or for an empty input, the unknown words that
  start at every position, which matter for runs of katakana, symbols or
  Latin letters in normal mode, the unknown words made from the default
  category, which matter for ko-dic hanja numerals followed by hanja and for
  `〇` with ko-dic, CC-CEDICT and Jieba, the dash and tilde spellings, which
  matter only for text with `―`, `—`, `～` or `〜` with IPADIC or
  IPADIC-NEologd, the choice among
  tied entries, which matters only if you read token details such as the
  reading or the base form, or the order of N-best results with the same
  cost, the choice between segmentations of equal cost, which changes the
  segmentation of a few lines, in the CLI only, the whitespace at the
  ends of an input line and the output of lines without tokens, which
  matter only for lines that start or end with whitespace and for the
  wakati and N-best output of empty lines and lines of spaces, and, in the
  WASM binding only, the builder settings that v6 ignored for one way of
  setting the dictionary.
- **Projects staying on `lindera = "6"`**: the 6.x releases of `lindera` and
  `lindera-analysis` remain on crates.io and keep working together. Nothing
  changes until you bump the major version.
- **Users of the segmenter API only**: code that uses `Segmenter`,
  `load_dictionary`, `Mode`, and the other `lindera::…` items of v6 compiles
  unchanged with `lindera = "7"`, unless it calls the lattice backtraces of
  `lindera::dictionary::Lattice` or creates metadata with
  `lindera::dictionary::Metadata::new` (see
  [Rust API changes in `lindera_dictionary`](#rust-api-changes-in-lindera_dictionary)).
  Dictionary directories loaded by path still have to be rebuilt.
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
- If you call `Metadata::new`, delete its ninth argument,
  `normalize_details`, and stop using the `Metadata::normalize_details`
  field and `PrefixDictionaryBuilderOptions::normalize_details`.
- If you build `Metadata` with a struct literal that lists every field, add
  `skip_whitespace: None` or end it with `..Default::default()`.
- If you read `CharacterDefinition::lookup_categories`, expect the default
  category first and the others in category id order.

Binding authors:

- Depend on `lindera-binding` instead of `lindera-binding-core`, and replace
  `lindera_binding_core::` with `lindera_binding::`.
- Delete the `normalize_details` argument of `CoreMetadata::new`, which now
  takes 10 arguments, and stop using the `CoreMetadata::normalize_details`
  field.
- If you build `CoreMetadata` with a struct literal that lists every field,
  add `skip_whitespace: None` or end it with `..Default::default()`.

Build environments:

- Rename `LINDERA_DICTIONARIES_PATH` to `LINDERA_BUILD_DICTIONARY_CACHE_DIR`
  in shell profiles, CI configuration, and container images.

Everyone who loads a dictionary from files:

- Rebuild with `lindera build`, or download again, every dictionary
  directory made with v6.2.0 or earlier, including those saved in the WASM
  binding's OPFS storage: v7.0.0 rejects them (format version 3). The
  embedded dictionaries and user dictionaries need nothing.

Users of IPADIC or IPADIC-NEologd:

- If you read `conjugation_type` or `conjugation_form` by name (including
  the CLI's JSON output), expect the two values to trade places.
- If you build dictionaries with your own copy of the IPADIC `metadata.json`,
  swap the two names in it.

Users of IPADIC, IPADIC-NEologd, UniDic or SudachiDict with text that
contains U+3000 or spaces:

- Expect U+3000 to be `記号,空白` (IPADIC) or `空白` (UniDic) tokens, and
  SudachiDict to segment spaced text much closer to Sudachi. Drop U+3000 with
  `japanese_stop_tags` or NFKC normalization if you do not want it.

Everyone who segments text that contains whitespace:

- Expect MeCab's segmentation for such text (most visible with ko-dic; not
  with SudachiDict, which keeps whitespace in the lattice as Sudachi does). If
  you need the v6 handling of whitespace, set `skip_whitespace(false)`,
  `"skip_whitespace": false`, or `--disable-skip-whitespace`; the other
  output changes in this guide still apply.
- Expect the token of a dictionary entry that ends with a space (in
  IPADIC-NEologd, ko-dic and SudachiDict) to include that space, as in
  MeCab.

Users of any bundled dictionary with text that contains `Ð`, `々` or `〇`:

- Expect `Ð` to stay in the output, and `々` after an unknown kanji to be a
  token of its own.

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
- Expect an empty input to give one result without tokens, as an input of
  only whitespace does, instead of no result.
- Expect the first result to be the 1-best segmentation also when several
  paths have exactly the same cost; results with the same cost can change
  places.

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

Users of ko-dic, CC-CEDICT or Jieba with text that has hanja numerals or
`〇`:

- Expect a ko-dic hanja numeral and the hanja after it to be one unknown
  word (`三國史記` is one `SH` word), and `〇` to be a symbol (ko-dic `SY`,
  Jieba `w`) whose group, with CC-CEDICT and Jieba, runs on through the
  numerals and characters after it, as in MeCab. No setting restores the v6
  output.

Users of IPADIC or IPADIC-NEologd with text that contains `―`, `—`, `～` or
`〜`:

- Expect entries spelled with `―` or `～` to match text spelled the same
  way, as in MeCab, and text with `—` or `〜` to miss the 12 IPADIC and
  1,085 IPADIC-NEologd entries that it found only through the old rewrite.
  With IPADIC, map `—` to `―` and `～` to `〜` with the `mapping` character
  filter if both spellings should find the entries.

Users who read token details such as the reading or the base form:

- Expect words with tied entries to get the details of the first CSV row,
  as in MeCab: with IPADIC, `狡い` reads `ズルイ`, not `コスイ`. No setting
  restores the v6 output.
- In a user dictionary, expect the first of tied rows to be chosen. A user
  entry still wins a tie with a system entry, unlike in MeCab.

Everyone who compares the segmentation with v6 or with MeCab:

- Expect the few places where two segmentations cost exactly the same to be
  segmented as MeCab segments them, with the last word that starts later
  (IPADIC `腸窒扶斯` is `腸 / 窒扶 / 斯`). No setting restores the v6
  output.

Language bindings and CLI:

- Remove `normalize_details` where you create `Metadata`: the Python
  keyword argument, the ninth positional argument of Ruby's `Metadata.new`,
  the last argument of PHP's `Metadata` constructor, and the
  `normalizeDetails` option in Node.js. Stop reading the property and the
  key of the same name in `to_dict()`, `to_h`, `toArray()` and `toObject()`.
- With `lindera tokenize`, expect the offsets of a line that starts with
  whitespace to index the input line, U+3000 at the start or end of a line
  to be a token (IPADIC, IPADIC-NEologd, UniDic), `--keep-whitespace` to
  output the spaces there, `-N` to give one result for an empty line or a
  line of spaces, and the wakati format to write an empty line for a line
  without tokens. To get the v6 offsets and tokens, remove the whitespace
  at the ends of each line first.
- With the WASM `TokenizerBuilder`, expect `setKeepWhitespace()` and the
  appended filters to apply to a dictionary set with
  `setDictionaryInstance()`, and a user dictionary set with
  `setUserDictionaryInstance()` to apply to a dictionary set with
  `setDictionary()`.
- Nothing else to do beyond taking the 7.0.0 release, apart from the
  dictionary rebuild, the dictionary, whitespace, `、` and `。`, N-best,
  unknown-word, default-category, dash and tilde, tied-entry and equal-cost
  items above and the `lindera tokenize` and WASM items.
