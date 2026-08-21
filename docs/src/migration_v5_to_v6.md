# Migrating from v5 to v6

Lindera v6.0.0 changes the on-disk format of built dictionaries, changes a
few `lindera-dictionary` Rust APIs, and changes tokenization output in two
targeted ways (a Decompose-mode accuracy fix, and a new default-on
unknown-word feature). Most users only need to rebuild or re-download their
dictionaries; direct users of `lindera_dictionary::viterbi`/`mode` types and
anyone relying on exact legacy output for out-of-vocabulary text have a
couple of extra things to check.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| Built dictionary format is now version 2 (`dict.da` → `dict.trie` + `dict.valsidx`) | Self-built dictionaries | Rebuild with `lindera build` |
| `metadata.json` records `format_version`; mismatches are refused at load | Anyone loading a v5-era built dictionary | Rebuild, or re-download with `lindera download` |
| `loadDictionaryFromBytes()` now takes 9 arguments | WASM users loading dictionaries from bytes | Pass `dictTrie` and `dictValsIdx` instead of `dictDa` |
| OPFS `DictionaryFiles` replaces `dictDa` with `dictTrie` + `dictValsIdx` | WASM users of the `opfs` helpers | Re-download OPFS-cached dictionaries |
| Decompose-mode length penalty is now exact for non-3-byte characters | `Mode::Decompose` output on text with 1-, 2-, or 4-byte UTF-8 characters | Nothing — this is a correctness fix; re-check Decompose output if you pin it exactly |
| Unknown-word length ladder is on by default | Normal- and Decompose-mode output on out-of-vocabulary text | Set `unknown_word_ladder(false)` / `--disable-unknown-word-ladder` for v5-identical output |
| Several `lindera_dictionary::viterbi`/`mode` items renamed or removed | Direct users of `Lattice`/`Edge`/`Penalty`/`Mode` (not the `lindera` crate's `Segmenter`/`Tokenizer`/`Worker` API) | Update call sites per the table below |
| JS bindings return plain objects; the `Token` class is gone | Node.js and WASM users | Replace `token.getDetail(i)` with `token.details[i]`; WASM users also switch to camelCase field names |

## Dictionary format version 2

The system prefix dictionary is no longer stored as a serialized `daachorse`
Aho-Corasick automaton (`dict.da`). The builder now constructs a char-wise
double-array trie with `crawdad` at build time and writes it as `dict.trie`,
together with a `u32` prefix-sum index into the word values (`dict.valsidx`).
At runtime the trie is walked in place over its serialized bytes: loading it
is an O(1) header check instead of a deserialization pass, and under `mmap`
it stays lazily paged like every other large component.

A built dictionary directory now contains these 9 files:

| File | Description |
| --- | --- |
| `metadata.json` | Dictionary metadata, now including `format_version` |
| `dict.trie` | Char-wise double-array trie structure (new) |
| `dict.valsidx` | Word values index (new) |
| `dict.vals` | Word value data |
| `dict.wordsidx` | Word details index |
| `dict.words` | Word details (morphological features) |
| `matrix.mtx` | Connection cost matrix |
| `char_def.bin` | Character category definitions |
| `unk.bin` | Unknown word dictionary |

All files other than `dict.trie` and `dict.valsidx` are unchanged from v5;
`dict.da` no longer exists.

### Loading an old dictionary now fails with a clear error

`metadata.json` in a built dictionary records `format_version: 2`, and the
loader checks it. Loading a dictionary built by v5 fails with an actionable
error instead of misreading the headerless binary files:

```text
Dictionary 'ipadic' has format version 1, but this build of Lindera reads format version 2. To fix this, rebuild it with `lindera build`, or download a matching prebuilt dictionary with `lindera download`.
```

## Tokenization output changes

Unlike the format change above, these two changes can alter the *tokens*
Lindera produces for the same input and dictionary.

### Decompose-mode penalty accuracy fix

`Mode::Decompose`'s length penalty used to approximate a span's character
count as `(stop_byte - start_byte) / 3`, which is exact only for text made
entirely of 3-byte UTF-8 characters (common Japanese kanji/kana). Lindera's
Viterbi lattice is now character-indexed internally, so the penalty uses the
real character count. This only changes Decompose output for spans
containing 1-byte (ASCII), 2-byte, or 4-byte (e.g. emoji, some CJK extension
characters) UTF-8 characters; pure-Japanese Decompose output is unaffected.
There is no option to restore the old (inexact) behavior, since it was a bug
fix rather than a behavior choice.

### Unknown-word length ladder (new, on by default)

For a run of out-of-vocabulary characters, MeCab and Vibrato generate a
"ladder" of candidate unknown-word lengths (`1..=LENGTH`, where `LENGTH` is
a per-category field in the dictionary's `char.def`) and let the Viterbi
search pick whichever length yields the lowest-cost path. Lindera parsed
`LENGTH` from `char.def` but never used it at runtime, so it only ever
generated a single 1-character candidate or (for grouping categories) one
candidate spanning the whole run — never anything in between.

Starting in v6.0.0, Lindera generates this candidate ladder by default. For
IPADIC this affects the `KANJI`, `HIRAGANA`, and `KATAKANA` categories
(`LENGTH=2`); most other categories declare `LENGTH=0` and are unaffected.
The practical effect is that consecutive out-of-vocabulary kanji or kana
characters can now be grouped into multi-character unknown words when that
scores lower than splitting them one character at a time — for example, an
unrecognized two-kanji compound is now tokenized as one word instead of two.

This is additive to (and independent of) the `max_grouping_len` option
introduced in v5.4 for capping whole-run grouping.

To reproduce v5-identical output on text containing out-of-vocabulary
kanji/kana runs, disable the ladder:

```rust,ignore
let segmenter = Segmenter::new(mode, dictionary, None)
    .unknown_word_ladder(false);
```

```sh
lindera tokenize -d ipadic --disable-unknown-word-ladder input.txt
```

`AnalysisWorker`/`SegmentWorker` expose the same toggle via
`set_unknown_word_ladder(bool)`, and `TokenizerBuilder` via
`set_segmenter_unknown_word_ladder(bool)`.

## Rust API changes in `lindera_dictionary`

These affect only code that uses `lindera_dictionary::viterbi::{Lattice,
Edge}` or `lindera_dictionary::mode::{Mode, Penalty}` directly (for example,
custom lattice inspection or alternative tokenizer front-ends). The `lindera`
crate's `Segmenter`, `Tokenizer`, `SegmentWorker`, and `AnalysisWorker` APIs
are unaffected — they gained the `unknown_word_ladder`/`max_grouping_len`
options above but no signatures changed.

| v5 | v6 | Why |
| --- | --- | --- |
| `Lattice::text_len()` | `Lattice::char_len()` | The lattice is now character-indexed instead of byte-indexed; the rename makes call sites that assumed bytes fail to compile instead of silently misbehaving |
| `Lattice::edges_at(pos)` | `Lattice::edges_at_char(pos)` | Same reason: `pos` is now a character position |
| `Lattice::paths_at(pos)` | `Lattice::paths_at_char(pos)` | Same reason |
| `Lattice::capacity()` / `shrink_to(n)` | Same names, now denominated in characters (slots) instead of bytes | A byte length remains a valid, conservative `shrink_to` argument (a sentence never has more characters than bytes) |
| `Edge::num_chars()` | Removed | The packed `Edge` no longer stores its end position (it always equals the lattice slot it lives in), so this can no longer be computed from the edge alone |
| `Penalty::penalty(&Edge)` | `Penalty::penalty(&Edge, num_chars: usize)` | The caller now passes the exact character span (see the Decompose accuracy fix above) |
| `Mode::penalty_cost(&Edge)` | Removed | Had no callers in the codebase; re-implement via `Penalty::penalty` if needed |

## JavaScript bindings: tokens are plain objects

Both JS bindings previously returned `Token` class instances. Those instances
own memory outside the JavaScript heap, released only when the host runs a
finalizer — which napi defers to the event loop, and which wasm-bindgen leaves
to the caller. A synchronous loop that tokenizes a large input without
yielding therefore accumulated memory even under forced GC. In v6 both
bindings return plain objects instead, which the JavaScript GC owns outright.

Practical benefits: memory stays flat in batch loops, and results work
directly with `JSON.stringify`, `structuredClone`, and worker transfer.

### `getDetail(i)` is removed (Node.js and WASM)

A plain object has no methods, so read the array instead:

```javascript
// v5
const pos = token.getDetail(0);

// v6
const pos = token.details[0];
```

Out-of-range reads now yield `undefined` rather than `null`.

### Node.js: `tokenizeObjects` is removed

`tokenizeObjects` existed only as a v5-era workaround for the memory problem
above. `tokenize` now returns the same plain objects, so drop the call:

```javascript
// v5
const tokens = tokenizer.tokenizeObjects(text);

// v6
const tokens = tokenizer.tokenize(text);
```

`tokenizeNbest` returns plain objects too, so it no longer has the
accumulation behavior documented as a known limitation in v5.

### WASM: field names are now camelCase

The removed `Token` class exposed snake_case fields while its own `toJSON()`
already emitted camelCase. v6 settles on camelCase, matching the Node.js
binding:

```javascript
// v5
token.byte_start, token.byte_end, token.word_id, token.is_unknown

// v6
token.byteStart, token.byteEnd, token.wordId, token.isUnknown
```

`surface`, `position`, and `details` are unchanged. Code that already called
`toJSON()` and read the result was using these names, and keeps working —
except that `toJSON()` itself is gone, since the token is already a plain
object:

```javascript
// v5
const data = token.toJSON();

// v6
const data = token; // already plain
```

`tokenizeNbest` in WASM already returned plain objects in v5 and is
unchanged.

## Who does not need to act

- **User dictionaries**: prebuilt user-dictionary `.bin` files are not
  affected. They keep using a `daachorse` automaton internally and continue to
  load without rebuilding. User dictionaries loaded from `.csv` are compiled
  at load time as before.
- **`lindera` crate's public API**: `Segmenter`, `Tokenizer`,
  `SegmentWorker`, and `AnalysisWorker` signatures are unchanged (aside from
  the additive `unknown_word_ladder`/`max_grouping_len` options); code that
  loads a dictionary from a path or URI compiles and runs unchanged once the
  dictionary itself is rebuilt or re-downloaded, and `embed-*` dictionaries
  are compiled by each dictionary crate's build script, so they always match
  the running version.

## Upgrade checklist

- Rebuild self-built system dictionaries with the v6 `lindera build`, or
  re-download prebuilt ones with `lindera download`.
- WASM: update `loadDictionaryFromBytes()` calls to the 9-argument signature
  (`dictTrie` and `dictValsIdx` in place of `dictDa`).
- WASM: remove v5-era dictionaries from OPFS and download v6 archives.
- If you pin exact tokenization output for out-of-vocabulary or non-Japanese
  text, re-check it — the Decompose penalty fix and the unknown-word ladder
  (default on) can both change it. Set `unknown_word_ladder(false)` if you
  need v5-identical unknown-word output.
- If you use `lindera_dictionary::viterbi`/`mode` types directly, update
  call sites per the Rust API table above.
- Nothing to do for user-dictionary `.bin` files.
