# Migrating from v5 to v6

Lindera v6.0.0 changes the on-disk format of built dictionaries: the system
prefix dictionary's byte-wise `daachorse` Aho-Corasick automaton (`dict.da`) is
replaced by a char-wise double-array trie (`dict.trie`) plus a word values
index (`dict.valsidx`), and `metadata.json` now records the format version so
that a mismatched dictionary fails loudly at load instead of decoding into
garbage. Tokenization output is unchanged — v6 produces byte-for-byte
identical tokens to v5 for the same input and dictionary.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| Built dictionary format is now version 2 (`dict.da` → `dict.trie` + `dict.valsidx`) | Self-built dictionaries | Rebuild with `lindera build` |
| `metadata.json` records `format_version`; mismatches are refused at load | Anyone loading a v5-era built dictionary | Rebuild, or re-download with `lindera download` |
| `loadDictionaryFromBytes()` now takes 9 arguments | WASM users loading dictionaries from bytes | Pass `dictTrie` and `dictValsIdx` instead of `dictDa` |
| OPFS `DictionaryFiles` replaces `dictDa` with `dictTrie` + `dictValsIdx` | WASM users of the `opfs` helpers | Re-download OPFS-cached dictionaries |

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

## Who needs to act

### Self-built dictionaries

Rebuild any dictionary you compiled yourself from its source files with the
v6 CLI, using the same `lindera build` invocation as before — the command-line
interface is unchanged:

```sh
lindera build \
  --src ./dictionary-source \
  --dest ./dictionary \
  --metadata ./metadata.json
```

Alternatively, re-download a matching prebuilt dictionary:

```sh
lindera download ipadic
```

Downloaded dictionaries are version-isolated under
`<data dir>/lindera/dictionaries/<version>/` (for example
`~/.local/share/lindera/dictionaries/6.0.0/` on Linux), so the v5 copies are
neither reused nor overwritten and no cleanup is strictly needed.

### WASM: `loadDictionaryFromBytes()`

The former `dictDa` argument is replaced by `dictTrie` and `dictValsIdx`,
growing the signature from 8 to 9 arguments:

```javascript
// v5 — 8 arguments
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

// v6 — 9 arguments
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictTrie, files.dictValsIdx, files.dictVals,
    files.dictWordsIdx, files.dictWords, files.matrixMtx, files.charDef, files.unk,
);
```

### WASM: OPFS helpers

The `DictionaryFiles` object returned by `loadDictionaryFiles()` now carries
`dictTrie` and `dictValsIdx` properties instead of `dictDa`, and the set of
files stored in OPFS changed accordingly. A dictionary downloaded into OPFS
with v5 lacks `dict.trie` and `dict.valsidx`, so remove it and download a v6
archive:

```javascript
import { downloadDictionary, removeDictionary } from 'lindera-wasm-web/opfs';

await removeDictionary("ipadic");
await downloadDictionary(DICT_URL_V6, "ipadic");
```

## Who does not need to act

- **User dictionaries**: prebuilt user-dictionary `.bin` files are not
  affected. They keep using a `daachorse` automaton internally and continue to
  load without rebuilding. User dictionaries loaded from `.csv` are compiled
  at load time as before.
- **Tokenization output**: unchanged. For the same input and dictionary, v6
  produces byte-for-byte identical tokens to v5.
- **Rust API**: no signatures changed. Code that loads a dictionary from a
  path or URI compiles and runs unchanged once the dictionary itself is
  rebuilt or re-downloaded, and `embed-*` dictionaries are compiled by each
  dictionary crate's build script, so they always match the running version.

## Upgrade checklist

- Rebuild self-built system dictionaries with the v6 `lindera build`, or
  re-download prebuilt ones with `lindera download`.
- WASM: update `loadDictionaryFromBytes()` calls to the 9-argument signature
  (`dictTrie` and `dictValsIdx` in place of `dictDa`).
- WASM: remove v5-era dictionaries from OPFS and download v6 archives.
- Nothing to do for user-dictionary `.bin` files, and no output changes to
  re-validate.
