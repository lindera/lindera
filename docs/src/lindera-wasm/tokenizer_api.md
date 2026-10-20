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

Every setter returns a builder handle that shares the same configuration, so
the calls can be chained (`builder.setMode(Mode.normal()).setDictionary(...)`) or
made one statement at a time. The returned handle is a new object, but it
configures the same underlying builder.

#### `setMode(mode)`

Sets the tokenization mode. A builder that is not given one uses normal mode.

- **Parameters**: `mode` (Mode) -- The [`Mode`](#mode): `Mode.normal()`, or `Mode.decompose()` with the [`Penalty`](#penalty) of decompose mode. The `Mode` stays usable, so the same one can be passed again.
- **Returns**: `TokenizerBuilder`

A value that is not a `Mode`, such as a mode name, throws `Error("expected instance of Mode")`. To select a mode by its name, create it with [`Mode.fromStr()`](#mode).

```javascript
builder.setMode(Mode.normal());
builder.setMode(Mode.decompose(new Penalty(undefined, 0)));
builder.setMode(Mode.fromStr("decompose"));
```

#### `setDictionary(uri)`

Sets the dictionary to use for tokenization. It replaces a dictionary set with `setDictionaryInstance()`.

- **Parameters**: `uri` (string) -- Dictionary URI (e.g., `"embedded://ipadic"`)
- **Returns**: `TokenizerBuilder`

```javascript
builder.setDictionary("embedded://ipadic");
```

#### `setDictionaryInstance(dictionary)`

Sets a pre-loaded dictionary instance for tokenization.
Use this when the dictionary has been loaded from bytes (e.g., via `loadDictionaryFromBytes()`) instead of from a URI.
It replaces a dictionary set with `setDictionary()`: whichever of the two is called last sets the dictionary. Every other setting of the builder (the mode, `setUserDictionaryInstance()`, `setKeepWhitespace()`, `setSkipWhitespace()`, `setSpacePenalty()` and the appended filters) applies to it as to a dictionary set by URI.

- **Parameters**: `dictionary` (Dictionary) -- A loaded dictionary object
- **Returns**: `TokenizerBuilder`

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

Sets a pre-loaded user dictionary instance. Load one from bytes with `loadUserDictionaryFromBytesFor()` or `loadUserDictionaryFromBytes()` (CSV) or `loadUserDictionaryBinFromBytes()` (prebuilt `.bin`); URI-based user dictionaries are not available on WebAssembly. The user dictionary applies to a dictionary set with either `setDictionary()` or `setDictionaryInstance()`.

- **Parameters**: `userDictionary` (UserDictionary) -- A loaded user dictionary object
- **Returns**: `TokenizerBuilder`

#### `setKeepWhitespace(keep)`

Sets whether whitespace tokens are preserved in the output. Kept whitespace also stays in the lattice, so the other tokens can differ from those of the default output, which skips whitespace in the lattice as MeCab does. To keep whitespace in the lattice without outputting it, use [`setSkipWhitespace(false)`](#setskipwhitespaceskip) instead.

- **Parameters**: `keep` (boolean) -- `true` to keep whitespace tokens
- **Returns**: `TokenizerBuilder`

```javascript
builder.setKeepWhitespace(true);
```

#### `setSkipWhitespace(skip)`

Sets whether whitespace is skipped in the lattice, with the same meaning as `segmenter.skip_whitespace` in a [configuration file](../lindera-analysis/configuration.md). `true` skips whitespace, so the words on either side of it connect directly, as MeCab does. `false` keeps whitespace in the lattice as a node of its own (a whitespace dictionary entry or the `SPACE` unknown word), as before v7; whitespace is still dropped from the output. `null` or `undefined` goes back to the default of the dictionary's `metadata.json`: skipping, except with SudachiDict.

A value set here overrides the dictionary's default, for a dictionary set with `setDictionary()` or `setDictionaryInstance()`. It has no effect while whitespace tokens are kept (`setKeepWhitespace(true)`), because kept whitespace always stays in the lattice. With IPADIC, for example, `Google が 新しい` reads `が` as a particle (`助詞`) by default and as a conjunction (`接続詞`) with `setSkipWhitespace(false)`. See [Segmenter](../lindera/segmenter.md#whitespace-handling) for details.

- **Parameters**: `skip` (boolean | null | undefined) -- The setting; `null` or `undefined` for the dictionary's default. The argument is required in the TypeScript declaration. Other values count by JavaScript truthiness.
- **Returns**: `TokenizerBuilder`

```javascript
// Keep whitespace in the lattice, as before v7
builder.setSkipWhitespace(false);
```

#### `setSpacePenalty(value)`

Sets the left-space penalty for Korean: a candidate that starts right after whitespace and whose part-of-speech tag normally attaches to the preceding word (particles, endings, ...) gets a cost added.

- **Parameters**: `value` (boolean | object | null | undefined) -- The setting, with the same meaning as `segmenter.space_penalty` in a [configuration file](../lindera-analysis/configuration.md)
- **Returns**: `TokenizerBuilder`

`value` takes one of these forms; any other value throws an error string:

- `null` or `undefined` -- The default: the rules the dictionary ships in its `metadata.json`, if any (ko-dic ships mecab-ko-dic's rules; the other bundled dictionaries ship none). Use it to undo an earlier call.
- `false` -- Turns the penalty off.
- `true` -- Requires the dictionary's rules; `build()` throws for a dictionary that ships none.
- An object of the form `{ rules: [{ pos: [...], cost: n }, ...] }` -- Applies these rules instead. A candidate whose first part-of-speech tag is listed in `pos` gets `cost` added; the first matching rule wins.

The setting applies both to a dictionary set with `setDictionary()` and to one set with `setDictionaryInstance()`, such as a ko-dic loaded from OPFS with `loadDictionaryFromBytes()`. The dictionary object itself is not changed. The `Tokenizer` constructor takes the same setting as its `spacePenalty` argument.

```javascript
// Turn the penalty off
builder.setSpacePenalty(false);

// Explicit rules
builder.setSpacePenalty({
    rules: [
        { pos: ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], cost: 3000 },
        { pos: ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], cost: 6000 },
    ],
});
```

> [!NOTE]
> Since v6.1.0, ko-dic applies the left-space penalty by default, as mecab-ko does. For example, `검색 이 잘 된다` reads `이` as the interjection `IC`; without the penalty, as the subject particle `JKS`. Call `builder.setSpacePenalty(false)` to turn it off. Only a ko-dic built by Lindera 6.1.0 or later ships the rules; with a ko-dic from the v6.0.0 release the penalty stays off and `true` fails until the dictionary is rebuilt. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

#### `appendCharacterFilter(name, args)`

Appends a character filter to the preprocessing pipeline.

- **Parameters**:
  - `name` (string) -- Filter name (e.g., `"unicode_normalize"`, `"japanese_iteration_mark"`)
  - `args` (object, optional) -- Filter configuration
- **Returns**: `TokenizerBuilder`

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(name, args)`

Appends a token filter to the postprocessing pipeline.

- **Parameters**:
  - `name` (string) -- Filter name (e.g., `"japanese_stop_tags"`, `"lowercase"`)
  - `args` (object, optional) -- Filter configuration
- **Returns**: `TokenizerBuilder`

```javascript
builder.appendTokenFilter("japanese_stop_tags", {
    tags: ["助詞,格助詞,一般", "助詞,係助詞", "助詞,連体化", "助動詞", "記号,句点", "記号,読点"]
});
```

The arguments of `setSpacePenalty`, `appendCharacterFilter` and `appendTokenFilter` may nest arrays, objects and `Map`s at most 128 levels deep. A deeper value, or one that contains itself, throws an error string.

#### `build()`

Builds and returns a configured `Tokenizer` instance. Every setting applies
whether the dictionary was set with `setDictionary()` or with
`setDictionaryInstance()`. The builder remains usable afterwards, so
multiple tokenizers can be built from the same configuration.

- **Returns**: `Tokenizer`

```javascript
const tokenizer = builder.build();
```

## Tokenizer

The main tokenizer class. Can be created via `TokenizerBuilder.build()` or directly via the constructor.

### Tokenizer Constructor

```javascript
const tokenizer = new Tokenizer(dictionary, mode, userDictionary, spacePenalty);
```

- **Parameters**:
  - `dictionary` (Dictionary) -- A loaded dictionary object
  - `mode` (Mode, optional) -- The [`Mode`](#mode), as [`setMode()`](#setmodemode) takes it. `null` or `undefined` (the default) is normal mode. The `Mode` stays usable, so the same one can be passed to several tokenizers and builders. A value that is not a `Mode` throws `Error("expected instance of Mode")`.
  - `userDictionary` (UserDictionary, optional) -- A loaded user dictionary
  - `spacePenalty` (boolean | object, optional) -- The left-space penalty, in the forms [`setSpacePenalty()`](#setspacepenaltyvalue) accepts. `null` or `undefined` (the default) uses the rules the dictionary ships. An invalid setting throws an error string, as `setSpacePenalty()` and `build()` do.

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
  - `n` (number) -- Number of results to return, an integer of 0 or more
  - `unique` (boolean, optional) -- Deduplicate results with identical segmentation (default: `false`)
  - `costThreshold` (bigint, optional) -- Only return paths within `bestCost + threshold`; it must fit in 64 bits
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

### `loadUserDictionaryFromBytesFor(csv, dictionary)`

Builds a user dictionary from CSV bytes (UTF-8) for a loaded system dictionary: the CSV is built with the dictionary's metadata and checked against its connection matrix, so a row whose context ID lies outside the matrix fails with an error that names its CSV line and surface. The dictionary is borrowed and stays usable.

- **Parameters**:
  - `csv` (Uint8Array) -- The user dictionary CSV content
  - `dictionary` (Dictionary) -- The system dictionary the user dictionary will be used with
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

console.log(version()); // e.g., "7.0.0"
```

## Utility Classes

### Mode

The tokenization mode: normal, or decompose with a [`Penalty`](#penalty). It is the counterpart of the Rust `Mode` (see [Tokenization Modes](../lindera/segmenter.md#tokenization-modes)), and [`setMode()`](#setmodemode) and the [`Tokenizer` constructor](#tokenizer-constructor) take it.

`Mode` has no public constructor: `new Mode()` is a TypeScript error, and in JavaScript it creates an object whose methods throw. Create a mode with a static method instead:

| Static method | Description |
| --- | --- |
| `Mode.normal()` | Normal mode: standard tokenization based on dictionary cost |
| `Mode.decompose(penalty?)` | Decompose mode: compound words are decomposed using penalty-based segmentation. `penalty` is a [`Penalty`](#penalty); `null` or `undefined` (the default) uses the default penalty. A value that is not a `Penalty` throws an error string (`penalty must be a Penalty`) |
| `Mode.fromStr(name)` | The mode named `"normal"` or `"decompose"` (with the default penalty). The name is case sensitive; any other name throws an error string that contains `Invalid mode` |

| Property / Method | Type | Description |
| --- | --- | --- |
| `name` | `string` | `"normal"` or `"decompose"` (read-only) |
| `penalty` | `Penalty` or `undefined` | A copy of the penalty of decompose mode, or `undefined` for normal mode (read-only) |
| `isNormal()` | `boolean` | Whether this is normal mode |
| `isDecompose()` | `boolean` | Whether this is decompose mode |
| `equals(other)` | `boolean` | Whether `other` has the same name and penalty. A value that is not a `Mode` throws `Error("expected instance of Mode")` |
| `toString()` | `string` | The name, as `name` returns it |

A `Mode` cannot be changed. `Mode.decompose()` keeps its own copy of the `Penalty`, and `penalty` returns another copy, so changing either `Penalty` afterwards does not change the mode. The `Penalty` passed to `Mode.decompose()` stays usable, and so does a `Mode` passed to `setMode()` or to the `Tokenizer` constructor.

```javascript
import { Mode, Penalty } from 'lindera-wasm-ipadic';

const mode = Mode.decompose(new Penalty(undefined, 0));
console.log(mode.name, mode.isNormal(), mode.isDecompose());
// decompose false true
console.log(`${mode}`, mode.penalty.kanji_penalty_length_penalty);
// decompose 0
console.log(mode.equals(Mode.decompose()), Mode.fromStr("decompose").equals(Mode.decompose()));
// false true
console.log(Mode.normal().penalty);
// undefined
```

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

Pass it to [`Mode.decompose()`](#mode), and the mode to [`setMode()`](#setmodemode) or to the [`Tokenizer` constructor](#tokenizer-constructor). The `Penalty` is read, not consumed, so the same one can be passed again. Without length penalties, decompose mode keeps long words whole, as normal mode does:

```javascript
import { Mode, Penalty, Tokenizer, TokenizerBuilder, loadDictionary } from 'lindera-wasm-ipadic';

const text = "関西国際空港限定トートバッグ";
const surfaces = (tokenizer) => tokenizer.tokenize(text).map((token) => token.surface);

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
console.log(surfaces(builder.setMode(Mode.normal()).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose()).build()));
// [ '関西', '国際', '空港', '限定', 'トートバッグ' ]

const mode = Mode.decompose(new Penalty(undefined, 0, undefined, 0));
console.log(surfaces(builder.setMode(mode).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"), mode);
console.log(surfaces(tokenizer));
// [ '関西国際空港', '限定', 'トートバッグ' ]
```

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
> `LinderaError` is exported as a utility class, but the current error paths in `TokenizerBuilder`, `Tokenizer`, `Mode`, and the dictionary-loading functions (`lindera-wasm/src/tokenizer.rs`, `lindera-wasm/src/mode.rs`, `lindera-wasm/src/dictionary.rs`) all reject with `JsValue::from_str(...)`, not a `JsLinderaError`/`LinderaError` instance. In practice, failures thrown by these APIs surface in JavaScript as plain strings. The exception is an argument that is not an instance of the class a method takes, such as a mode name passed to `setMode()`: the JS glue checks it before calling into Rust and throws an `Error` whose message is `expected instance of <class>` (here, `expected instance of Mode`). Catch them with `catch (e) { ... }` and treat `e` as a `string` or an `Error`, not as a `LinderaError` instance.

## Snake-Case Aliases

For consistency with the Python API, these methods are also available in snake\_case form:

| camelCase | snake\_case |
| --- | --- |
| `setMode()` | `set_mode()` |
| `setDictionary()` | `set_dictionary()` |
| `setDictionaryInstance()` | `set_dictionary_instance()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `setSkipWhitespace()` | `set_skip_whitespace()` |
| `setSpacePenalty()` | `set_space_penalty()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeSurfaces()` | `tokenize_surfaces()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytes()` | `load_user_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytesFor()` | `load_user_dictionary_from_bytes_for()` |
| `loadUserDictionaryBinFromBytes()` | `load_user_dictionary_bin_from_bytes()` |

The methods of `Mode` (`fromStr()`, `isNormal()`, `isDecompose()`) have no snake\_case aliases.
