# Tokenizer API

This page documents the JavaScript/TypeScript API exposed by lindera-wasm.

## TokenizerBuilder

Builder class for creating a configured `Tokenizer` instance.

### Constructor

```javascript
const builder = new TokenizerBuilder();
```

Creates a new builder with default settings.

### Methods

#### `setMode(mode)`

Sets the tokenization mode.

- **Parameters**: `mode` (string) -- `"normal"` or `"decompose"`
- **Returns**: void

```javascript
builder.setMode("normal");
```

#### `setDictionary(uri)`

Sets the dictionary to use for tokenization.

- **Parameters**: `uri` (string) -- Dictionary URI (e.g., `"embedded://ipadic"`)
- **Returns**: void

```javascript
builder.setDictionary("embedded://ipadic");
```

#### `setDictionaryInstance(dictionary)`

Sets a pre-loaded dictionary instance for tokenization.
Use this when the dictionary has been loaded from bytes (e.g., via `loadDictionaryFromBytes()`) instead of from a URI.

- **Parameters**: `dictionary` (Dictionary) -- A loaded dictionary object
- **Returns**: void

```javascript
import { loadDictionaryFromBytes } from 'lindera-wasm';
import { loadDictionaryFiles } from 'lindera-wasm/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictTrie, files.dictValsIdx, files.dictVals,
    files.dictWordsIdx, files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

builder.setDictionaryInstance(dictionary);
```

#### `setUserDictionaryInstance(userDictionary)`

Sets a pre-loaded user dictionary instance. Load one from bytes with `loadUserDictionaryFromBytes()` (CSV) or `loadUserDictionaryBinFromBytes()` (prebuilt `.bin`); URI-based user dictionaries are not available on WebAssembly.

- **Parameters**: `userDictionary` (UserDictionary) -- A loaded user dictionary object
- **Returns**: void

#### `setKeepWhitespace(keep)`

Sets whether whitespace tokens are preserved in the output.

- **Parameters**: `keep` (boolean) -- `true` to keep whitespace tokens
- **Returns**: void

```javascript
builder.setKeepWhitespace(true);
```

#### `appendCharacterFilter(name, args)`

Appends a character filter to the preprocessing pipeline.

- **Parameters**:
  - `name` (string) -- Filter name (e.g., `"unicode_normalize"`, `"japanese_iteration_mark"`)
  - `args` (object, optional) -- Filter configuration
- **Returns**: void

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(name, args)`

Appends a token filter to the postprocessing pipeline.

- **Parameters**:
  - `name` (string) -- Filter name (e.g., `"japanese_stop_tags"`, `"lowercase"`)
  - `args` (object, optional) -- Filter configuration
- **Returns**: void

```javascript
builder.appendTokenFilter("japanese_stop_tags", {
    tags: ["助詞,格助詞,一般", "助詞,係助詞", "助詞,連体化", "助動詞", "記号,句点", "記号,読点"]
});
```

#### `build()`

Builds and returns a configured `Tokenizer` instance. Consumes the builder.

- **Returns**: `Tokenizer`

```javascript
const tokenizer = builder.build();
```

## Tokenizer

The main tokenizer class. Can be created via `TokenizerBuilder.build()` or directly via the constructor.

### Tokenizer Constructor

```javascript
const tokenizer = new Tokenizer(dictionary, mode, userDictionary);
```

- **Parameters**:
  - `dictionary` (Dictionary) -- A loaded dictionary object
  - `mode` (string, optional) -- Tokenization mode (`"normal"` or `"decompose"`, defaults to `"normal"`)
  - `userDictionary` (UserDictionary, optional) -- A loaded user dictionary

### Tokenizer Methods

#### `tokenize(text)`

Tokenizes the input text.

- **Parameters**: `text` (string) -- Text to tokenize
- **Returns**: `Token[]` -- Array of token objects

```javascript
const tokens = tokenizer.tokenize("関西国際空港");
```

#### `tokenizeSurfaces(text)`

Tokenizes the input text and returns only the token surfaces. This is the fast path for wakati-style use: no token objects are created and no morphological details are loaded, so it is significantly faster than `tokenize` when only the surface strings are needed. The result equals `tokenizer.tokenize(text).map((t) => t.surface)`. (Unrelated to Web Workers.)

- **Parameters**: `text` (string) -- Text to tokenize
- **Returns**: `string[]` -- Array of surface strings

```javascript
const surfaces = tokenizer.tokenizeSurfaces("関西国際空港");
// ["関西国際空港"]
```

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

Returns N-best tokenization results ordered by total path cost.

- **Parameters**:
  - `text` (string) -- Text to tokenize
  - `n` (number) -- Number of results to return
  - `unique` (boolean, optional) -- Deduplicate results with identical segmentation (default: `false`)
  - `costThreshold` (bigint, optional) -- Only return paths within `bestCost + threshold`
- **Returns**: Array of `{ tokens: Token[], cost: number }`

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);

// With a cost threshold -- note that it must be passed as a bigint literal
const resultsWithThreshold = tokenizer.tokenizeNbest("すもももももももものうち", 3, false, 100n);
```

## Token

A single token produced by the tokenizer, as a plain JavaScript object.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `surface` | `string` | Surface form of the token |
| `byteStart` | `number` | Start byte offset in the original text |
| `byteEnd` | `number` | End byte offset in the original text |
| `position` | `number` | Position index of the token |
| `wordId` | `number` | Word ID in the dictionary |
| `isUnknown` | `boolean` | Whether the token is an unknown word |
| `details` | `string[]` | Morphological detail fields |

> [!NOTE]
> Tokens are plain objects, not `wasm_bindgen` class instances. A class instance keeps its data on the Rust heap until the JavaScript side drops it, so a synchronous loop that tokenizes without yielding accumulates memory; nothing is allocated on the Rust side for a plain object, and the result passes through `JSON.stringify`, `structuredClone`, and worker transfer without conversion. Field names are camelCase, matching the `lindera-nodejs` binding.

### Reading Details

Index `details` directly. Out-of-range indexes yield `undefined`.

```javascript
const pos = token.details[0];     // e.g., "名詞"
const reading = token.details[7]; // e.g., "トウキョウ"
```

### Serializing a Token

A token is already a plain object, so it serializes as-is.

```javascript
console.log(JSON.stringify(token, null, 2));
```

## Helper Functions

> [!NOTE]
> The examples below import from `lindera-wasm-ipadic`, an illustrative package name for a local build with the `embed-ipadic` feature -- it is not published to npm. Only `lindera-wasm` is actually published; see [NPM Package Naming Convention](./installation.md#npm-package-naming-convention).

### `loadDictionary(uri)`

Loads a dictionary from the specified URI.

- **Parameters**: `uri` (string) -- Dictionary URI (e.g., `"embedded://ipadic"`)
- **Returns**: `Dictionary`

```javascript
import { loadDictionary } from 'lindera-wasm-ipadic';

const dict = loadDictionary("embedded://ipadic");
```

### `loadUserDictionaryFromBytes(csv, metadata)`

Builds a user dictionary from CSV bytes (UTF-8), obtained via `fetch`, a file input, or OPFS.

- **Parameters**:
  - `csv` (Uint8Array) -- The user dictionary CSV content
  - `metadata` (Metadata) -- Metadata of the system dictionary the user dictionary will be used with (e.g. `dictionary.metadata`)
- **Returns**: `UserDictionary`

### `loadUserDictionaryBinFromBytes(bytes)`

Loads a prebuilt user dictionary (the output of `lindera build --user`) from bytes.

- **Parameters**: `bytes` (Uint8Array) -- The `.bin` content
- **Returns**: `UserDictionary`

### `version()` / `getVersion()`

Returns the version string of the lindera-wasm package.

- **Returns**: `string`

```javascript
import { version } from 'lindera-wasm-ipadic';

console.log(version()); // e.g., "5.3.0"
```

## Enums and Utility Classes

### Mode

Tokenization mode enum.

| Value | Description |
| --- | --- |
| `Mode.Normal` | Standard tokenization based on dictionary cost |
| `Mode.Decompose` | Decompose compound words using penalty-based segmentation |

### Penalty

Configuration for decompose mode. Controls how aggressively compound words are decomposed.

```javascript
const penalty = new Penalty(
    kanjiThreshold?,     // Kanji length threshold (default: 2)
    kanjiPenalty?,       // Kanji length penalty (default: 3000)
    otherThreshold?,     // Other character length threshold (default: 7)
    otherPenalty?,       // Other character length penalty (default: 1700)
);
```

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `number` | `2` | Length threshold for kanji compound splitting |
| `kanji_penalty_length_penalty` | `number` | `3000` | Penalty cost for kanji compounds exceeding threshold |
| `other_penalty_length_threshold` | `number` | `7` | Length threshold for non-kanji compound splitting |
| `other_penalty_length_penalty` | `number` | `1700` | Penalty cost for non-kanji compounds exceeding threshold |

### LinderaError

Error type for Lindera operations.

```javascript
const error = new LinderaError("message");
console.log(error.message);    // "message"
console.log(error.toString()); // "message"
```

| Property / Method | Type | Description |
| --- | --- | --- |
| `message` | `string` | Error message |
| `toString()` | `string` | Returns the error message |

> [!NOTE]
> `LinderaError` is exported as a utility class, but the current error paths in `TokenizerBuilder`, `Tokenizer`, and the dictionary-loading functions (`lindera-wasm/src/tokenizer.rs`, `lindera-wasm/src/dictionary.rs`) all reject with `JsValue::from_str(...)`, not a `JsLinderaError`/`LinderaError` instance. In practice, failures thrown by these APIs surface in JavaScript as plain strings, so catch them with `catch (e) { ... }` and treat `e` as a `string`, not as a `LinderaError` instance.

## Snake-Case Aliases

For consistency with the Python API, all methods are also available in snake\_case form:

| camelCase | snake\_case |
| --- | --- |
| `setMode()` | `set_mode()` |
| `setDictionary()` | `set_dictionary()` |
| `setDictionaryInstance()` | `set_dictionary_instance()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeSurfaces()` | `tokenize_surfaces()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytes()` | `load_user_dictionary_from_bytes()` |
| `loadUserDictionaryBinFromBytes()` | `load_user_dictionary_bin_from_bytes()` |
