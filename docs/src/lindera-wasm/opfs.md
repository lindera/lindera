# OPFS Dictionary Storage

Lindera WASM provides OPFS (Origin Private File System) helper utilities for persistent dictionary caching in web browsers. This allows you to download dictionaries once and reuse them across sessions without embedding them in the WASM binary.

## Overview

The OPFS helpers are distributed as a separate JavaScript module (`opfs.js`) alongside the WASM package. They provide functions to download, store, load, and manage dictionaries using the browser's Origin Private File System.

Dictionaries are stored under the OPFS path `lindera/dictionaries/<name>/`.

## Import

```javascript
import { downloadDictionary, loadDictionaryFiles, removeDictionary,
         listDictionaries, hasDictionary } from 'lindera-wasm-web/opfs';
```

## Functions

### `downloadDictionary(url, name, options?)`

Downloads a dictionary zip archive, extracts it, and stores the files in OPFS.

The archive should be a zip file containing the 8 required dictionary files, optionally nested in a subdirectory.

- **Parameters**:
  - `url` (string) -- URL of the dictionary zip archive
  - `name` (string) -- Name to store the dictionary under (e.g., `"ipadic"`)
  - `options` (object, optional):
    - `onProgress` (function) -- Progress callback
- **Returns**: `Promise<void>`

```javascript
await downloadDictionary(
    "https://example.com/ipadic.zip",
    "ipadic",
    {
        onProgress: (progress) => {
            switch (progress.phase) {
                case "downloading":
                    console.log(`Downloading: ${progress.loaded}/${progress.total} bytes`);
                    break;
                case "extracting":
                    console.log("Extracting archive...");
                    break;
                case "storing":
                    console.log("Storing in OPFS...");
                    break;
                case "complete":
                    console.log("Done!");
                    break;
            }
        },
    },
);
```

#### Progress Callback

The `onProgress` callback receives an object with the following shape:

| Property | Type | Description |
| --- | --- | --- |
| `phase` | `string` | `"downloading"`, `"extracting"`, `"storing"`, or `"complete"` |
| `loaded` | `number \| undefined` | Bytes downloaded (only during `"downloading"` phase) |
| `total` | `number \| undefined` | Total bytes if known (only during `"downloading"` phase) |

### `loadDictionaryFiles(name)`

Loads dictionary files from OPFS as an object of `Uint8Array` values.

The returned object can be passed directly to `loadDictionaryFromBytes()`.

- **Parameters**: `name` (string) -- The dictionary name (e.g., `"ipadic"`)
- **Returns**: `Promise<DictionaryFiles>`

```javascript
const files = await loadDictionaryFiles("ipadic");
```

#### DictionaryFiles

| Property | Type | Source File |
| --- | --- | --- |
| `metadata` | `Uint8Array` | `metadata.json` |
| `dictDa` | `Uint8Array` | `dict.da` (Double-Array Trie) |
| `dictVals` | `Uint8Array` | `dict.vals` (word value data) |
| `dictWordsIdx` | `Uint8Array` | `dict.wordsidx` (word details index) |
| `dictWords` | `Uint8Array` | `dict.words` (word details) |
| `matrixMtx` | `Uint8Array` | `matrix.mtx` (connection cost matrix) |
| `charDef` | `Uint8Array` | `char_def.bin` (character definitions) |
| `unk` | `Uint8Array` | `unk.bin` (unknown word dictionary) |

### `removeDictionary(name)`

Removes a dictionary from OPFS.

- **Parameters**: `name` (string) -- The dictionary name to remove
- **Returns**: `Promise<void>`

```javascript
await removeDictionary("ipadic");
```

### `listDictionaries()`

Lists all dictionaries stored in OPFS.

- **Returns**: `Promise<string[]>` -- Array of dictionary names

```javascript
const names = await listDictionaries();
console.log(names); // e.g., ["ipadic", "unidic"]
```

### `hasDictionary(name)`

Checks if a dictionary exists in OPFS.

- **Parameters**: `name` (string) -- The dictionary name to check
- **Returns**: `Promise<boolean>`

```javascript
if (await hasDictionary("ipadic")) {
    console.log("Dictionary is cached");
}
```

## Complete Workflow

A typical workflow for using OPFS-based dictionaries:

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles, hasDictionary } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    const DICT_NAME = "ipadic";
    const DICT_URL = "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip";

    // Download dictionary if not already cached
    if (!await hasDictionary(DICT_NAME)) {
        await downloadDictionary(DICT_URL, DICT_NAME, {
            onProgress: ({ phase, loaded, total }) => {
                if (phase === "downloading" && total) {
                    console.log(`${(loaded / total * 100).toFixed(1)}%`);
                }
            },
        });
    }

    // Load dictionary from OPFS
    const files = await loadDictionaryFiles(DICT_NAME);
    const dictionary = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
        files.dictWords, files.matrixMtx, files.charDef, files.unk,
    );

    // Build tokenizer
    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dictionary);
    builder.setMode("normal");
    const tokenizer = builder.build();

    // Tokenize
    const tokens = tokenizer.tokenize("形態素解析を行います");
    tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
}

main();
```

## Required Dictionary Files

A valid dictionary archive must contain these 8 files:

| File | Description |
| --- | --- |
| `metadata.json` | Dictionary metadata (name, encoding, schema, etc.) |
| `dict.da` | Double-Array Trie structure |
| `dict.vals` | Word value data |
| `dict.wordsidx` | Word details index |
| `dict.words` | Word details (morphological features) |
| `matrix.mtx` | Connection cost matrix |
| `char_def.bin` | Character category definitions |
| `unk.bin` | Unknown word dictionary |

## Browser Compatibility

OPFS requires a [secure context](https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts) (HTTPS or localhost) and is supported in:

- Chrome 86+
- Edge 86+
- Firefox 111+
- Safari 15.2+

The zip extraction uses the `DecompressionStream` API, which requires:

- Chrome 80+
- Edge 80+
- Firefox 113+
- Safari 16.4+
