# Migrating from v5 to v6

Lindera v6.0.0 rebuilds prebuilt user dictionaries, changes tokenization
output in two targeted ways (a Decompose-mode accuracy fix and a new
default-on unknown-word feature), reshapes the JavaScript bindings' token
type, and renames several `lindera-dictionary` Rust APIs. Most users only
need to rebuild their dictionaries; direct users of
`lindera_dictionary::viterbi`/`mode` types, JavaScript users, and anyone
relying on exact legacy output for out-of-vocabulary text have more to check.

If you are upgrading from **v5.2 or earlier**, the system-dictionary format
change described below applies to you as well. It shipped in v5.3.0, so
upgrading from v5.3.0 does not require rebuilding system dictionaries — but
it does require rebuilding user dictionaries, for an unrelated reason.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| **Prebuilt user-dictionary `.bin` files no longer load** | Anyone loading a user dictionary from `.bin` | Rebuild from the CSV source with `lindera build --user` |
| **Trained `model.dat` files no longer load** | Anyone re-using a `model.dat` produced by an earlier build | Re-run `lindera train` |
| Decompose-mode length penalty is now exact for non-3-byte characters | `Mode::Decompose` output on text with 1-, 2-, or 4-byte UTF-8 characters | Nothing — this is a correctness fix; re-check Decompose output if you pin it exactly |
| Unknown-word length ladder is on by default | Normal- and Decompose-mode output on out-of-vocabulary text | Set `unknown_word_ladder(false)` / `--disable-unknown-word-ladder` for v5-identical output |
| New `max_grouping_len` option (`--max-grouping-len`, config key, builder/worker setters) | Anyone who wants MeCab's `max-grouping-size` cap | Nothing — the default is unchanged (unbounded) |
| JS bindings return plain objects; the `Token` class is gone | Node.js and WASM users | Replace `token.getDetail(i)` with `token.details[i]`; WASM users also switch to camelCase field names |
| Several `lindera_dictionary::viterbi`/`mode` items renamed or removed, and `Lattice::set_text`/`set_text_nbest` take two more arguments | Direct users of `Lattice`/`Edge`/`Penalty`/`Mode` (not the `lindera` crate's `Segmenter`/`Tokenizer`/`Worker` API) | Update call sites per the table below |
| Built dictionary format is version 2 (`dict.da` → `dict.trie` + `dict.valsidx`) — **shipped in v5.3.0** | Self-built system dictionaries created by **v5.2 or earlier** | Rebuild with `lindera build`, or re-download with `lindera download` |
| `loadDictionaryFromBytes()` takes 9 arguments — **shipped in v5.3.0** | WASM users loading dictionaries from bytes, upgrading from **v5.2 or earlier** | Pass `dictTrie` and `dictValsIdx` instead of `dictDa` |
| OPFS `DictionaryFiles` replaces `dictDa` with `dictTrie` + `dictValsIdx` — **shipped in v5.3.0** | WASM users of the `opfs` helpers, upgrading from **v5.2 or earlier** | Re-download OPFS-cached dictionaries |

## Prebuilt user dictionaries must be rebuilt

Lindera v6 upgrades `daachorse` from 4.x to 5.0, which changed the serialized
form of the Aho-Corasick automaton that a user dictionary embeds. A `.bin`
file built by any v5 release therefore fails to load:

```text
LinderaError(kind=Deserialize, source=InvalidAutomatonError: invalid serialized automaton)
```

Unlike system dictionaries, user-dictionary `.bin` files carry no
`format_version`, so there is no version check to produce a friendlier
message — the failure surfaces as the deserialize error above.

Rebuild from the CSV source with v6:

```sh
lindera build --user \
  --src ./user_dict.csv \
  --dest ./build \
  --metadata ./lindera-ipadic/metadata.json
```

If you load a user dictionary from a `.csv` file rather than a prebuilt
`.bin`, it is compiled at load time and no action is needed.

## Trained model files must be regenerated

`lindera train` now writes a byte-reproducible `model.dat`: training the same
inputs twice with the same flags produces identical bytes, so the artifact can
be checksummed, cached and diffed.

Getting there required storing the connection matrix and the unknown-word
categories as ordered maps rather than hash maps — rkyv lays an archived
`HashMap` out in source iteration order, and that order is seeded per instance.
This changes the on-disk layout, so a `model.dat` written by an earlier build
fails to load:

```text
failed to deserialize model: ... re-run `lindera train` to regenerate it.
```

`model.dat` is an intermediate between `lindera train` and `lindera export`,
not a distributed artifact — regenerate it by re-running `lindera train`.
Dictionaries already exported from an old model are unaffected, and the trained
weights are identical either way: only the serialization order changed.

Two related notes:

- `SerializableModel::connection_matrix` and `SerializableModel::unk_categories`
  are now `BTreeMap` rather than `HashMap`. This matters only if you read those
  public fields directly; the writer methods are unchanged.
- Reproducibility holds for any `--max-threads` value: the gradient and the
  loss are summed over a fixed partition of the training data in a fixed
  order, so the thread count affects only speed, never the trained model.
- `lindera export` output is byte-reproducible too: `metadata.json` no longer
  carries an `updated_at` timestamp, and every export writer emits entries in
  a deterministic order. Accordingly, the `ModelInfo.updated_at` field was
  removed from the Rust API (`lindera-dictionary`); old `metadata.json` files
  that still contain the key keep parsing.
- `DictionaryRewriter::rewrite_cached` and `clear_cache` were removed: the
  cache was keyed on the full feature string, which is unique per row in a
  real lexicon, so it never hit and only retained memory. Call `rewrite`
  instead — it no longer requires `&mut`.
- The `features` parameter of the six `FeatureExtractor::extract_*` methods
  changed from `&[String]` to `&[&str]`, so callers no longer have to
  allocate one `String` per feature field. A caller holding a `&[String]`
  can pass `&v.iter().map(|s| s.as_str()).collect::<Vec<_>>()`.
- The fixed partition changed the summation order, so weights trained
  **before** this change are not byte-identical to weights trained after it —
  at any thread count, including `--max-threads 1`. Re-run `lindera train` if
  you need artifacts that compare equal from now on.

## Dictionary format version 2 (shipped in v5.3.0)

> This change shipped in **v5.3.0**, not in v6.0.0. If you are upgrading
> from v5.3.0 your system dictionaries already use format version 2 and load
> unchanged. Follow this section only when upgrading from **v5.2 or
> earlier**.

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
loader checks it. Loading a dictionary built by v5.2 or earlier fails with an
actionable error instead of misreading the headerless binary files:

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

This is additive to (and independent of) `max_grouping_len`, the new option
described below for capping whole-run grouping.

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

### New option: `max_grouping_len` (default unchanged)

v6 adds MeCab's `max-grouping-size` semantics: when a groupable run of
out-of-vocabulary characters extends more than `max_grouping_len` characters
beyond the first, the grouped candidate is not emitted and single-character
unknown words are used instead. The default is unbounded, which is exactly
what v5 did, so **this changes nothing unless you opt in**:

```rust,ignore
let segmenter = Segmenter::new(mode, dictionary, None)
    .max_grouping_len(Some(24)); // MeCab's own default
```

```sh
lindera tokenize -d ipadic --max-grouping-len 24 input.txt
```

Also available as `set_max_grouping_len(Option<usize>)` on
`SegmentWorker`/`AnalysisWorker`, `set_segmenter_max_grouping_len(usize)` on
`TokenizerBuilder`, and the `max_grouping_len` key in a segmenter config.

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
| `Lattice::set_text(..., search_mode)` | `Lattice::set_text(..., search_mode, max_grouping_len, unknown_word_ladder)` | Two trailing arguments for the new options; pass `None` and `true` to match v6 defaults, or `None` and `false` for v5 behavior. `set_text_nbest` changed identically |
| — | `Lattice::take_max_char_len()` (new) | Additive: reports and resets the largest sentence seen, for worker-style shrink accounting |

`set_text` and `set_text_nbest` also gained a documented panic: they assert
that the sentence is shorter than `u16::MAX` characters, because an edge now
stores its start position as a `u16`. Every path through `Segmenter` is safe
because it splits sentences at `MAX_SENTENCE_BYTES`; only code that drives a
`Lattice` directly needs to split its own input.

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
accumulation behavior documented as a known limitation in v5. `NbestResult`
stopped being a class along with `Token`: it is now a plain object with the
same `tokens` and `cost` properties.

Because neither is a class any more, they are no longer exported from the
package. `Token`, `JsToken`, `NbestResult`, and `JsNbestResult` are all gone
from `index.js`, so `require("lindera-nodejs").Token` is `undefined` and
`instanceof` checks against them no longer compile or run:

```javascript
// v5
const { Token } = require("lindera-nodejs");
if (tokens[0] instanceof Token) { /* ... */ }

// v6 — the values are plain objects; check a property instead
if (typeof tokens[0].surface === "string") { /* ... */ }
```

TypeScript users: the interface describing a token is now called `Token`
(it was `JsTokenData` while `Token` was taken by the class).

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

`Token` is also no longer exported from the module, so
`import { Token } from 'lindera-wasm-...'` fails. There is nothing to import
in its place — `tokenize` hands back plain objects directly.

`tokenizeNbest` in WASM already returned plain objects in v5 and is
unchanged.

## Who does not need to act

- **Python, Ruby, and PHP bindings**: unaffected by the JavaScript changes
  above. They keep their `Token` classes, because their finalization is
  deterministic and never had the memory problem that motivated the JS
  rework. They each gained a plain-data conversion method — `to_dict()`,
  `to_h` (also `to_hash`), and `toArray()` respectively — but nothing was
  removed or renamed.
- **User dictionaries loaded from `.csv`**: compiled at load time, so they
  pick up the new format automatically. (Prebuilt `.bin` files *do* need
  rebuilding — see the first section.)
- **`lindera` crate's public API**: `Segmenter`, `Tokenizer`,
  `SegmentWorker`, and `AnalysisWorker` signatures are unchanged (aside from
  the additive `unknown_word_ladder`/`max_grouping_len` options); code that
  loads a dictionary from a path or URI compiles and runs unchanged once the
  dictionary itself is rebuilt or re-downloaded, and `embed-*` dictionaries
  are compiled by each dictionary crate's build script, so they always match
  the running version.

## Upgrade checklist

Everyone:

- **Rebuild prebuilt user-dictionary `.bin` files** from their CSV source
  with `lindera build --user`. This is required regardless of which v5 you
  are coming from.
- If you pin exact tokenization output for out-of-vocabulary or non-Japanese
  text, re-check it — the Decompose penalty fix and the unknown-word ladder
  (default on) can both change it. Set `unknown_word_ladder(false)` if you
  need v5-identical unknown-word output.

JavaScript (Node.js and WASM):

- Replace `token.getDetail(i)` with `token.details[i]`.
- Node.js: replace `tokenizeObjects(text)` with `tokenize(text)`; stop
  importing `Token`/`NbestResult` (they are no longer exported); TypeScript
  users rename the `JsTokenData` type to `Token`.
- WASM: switch token field reads to camelCase (`byteStart`, `byteEnd`,
  `wordId`, `isUnknown`), drop `toJSON()` calls, and stop importing `Token`.

Direct `lindera_dictionary` users:

- Update call sites per the Rust API table above, including the two new
  `set_text`/`set_text_nbest` arguments.

Upgrading from v5.2 or earlier (these shipped in v5.3.0):

- Rebuild self-built system dictionaries with `lindera build`, or re-download
  prebuilt ones with `lindera download`.
- WASM: update `loadDictionaryFromBytes()` calls to the 9-argument signature
  (`dictTrie` and `dictValsIdx` in place of `dictDa`).
- WASM: remove v5-era dictionaries from OPFS and download v6 archives.
