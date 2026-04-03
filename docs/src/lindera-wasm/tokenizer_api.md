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
import { loadDictionaryFromBytes } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

builder.setDictionaryInstance(dictionary);
```

#### `setUserDictionary(uri)`

Sets a user-defined dictionary by URI.

- **Parameters**: `uri` (string) -- Path or URI to the user dictionary
- **Returns**: void

```javascript
builder.setUserDictionary("file:///path/to/user_dict.csv");
```

#### `setUserDictionaryInstance(userDictionary)`

Sets a pre-loaded user dictionary instance. Use this when the user dictionary has been loaded from bytes instead of from a URI.

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
    tags: ["助詞", "助動詞", "記号"]
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

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

Returns N-best tokenization results ordered by total path cost.

- **Parameters**:
  - `text` (string) -- Text to tokenize
  - `n` (number) -- Number of results to return
  - `unique` (boolean, optional) -- Deduplicate results with identical segmentation (default: `false`)
  - `costThreshold` (number, optional) -- Only return paths within `bestCost + threshold`
- **Returns**: Array of `{ tokens: object[], cost: number }`

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
```

## Token

Represents a single token produced by the tokenizer.

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

### Token Methods

#### `getDetail(index)`

Returns the detail string at the specified index.

- **Parameters**: `index` (number) -- Zero-based index into the details array
- **Returns**: `string | undefined`

```javascript
const pos = token.getDetail(0);   // e.g., "名詞"
const reading = token.getDetail(7); // e.g., "トウキョウ"
```

#### `toJSON()`

Returns a plain JavaScript object representation of the token.

- **Returns**: `object` with keys: `surface`, `byteStart`, `byteEnd`, `position`, `wordId`, `isUnknown`, `details`

```javascript
console.log(JSON.stringify(token.toJSON(), null, 2));
```

## Helper Functions

### `loadDictionary(uri)`

Loads a dictionary from the specified URI.

- **Parameters**: `uri` (string) -- Dictionary URI (e.g., `"embedded://ipadic"`)
- **Returns**: `Dictionary`

```javascript
import { loadDictionary } from 'lindera-wasm-web-ipadic';

const dict = loadDictionary("embedded://ipadic");
```

### `loadUserDictionary(uri, metadata)`

Loads a user dictionary from the specified URI.

- **Parameters**:
  - `uri` (string) -- Path or URI to the user dictionary file
  - `metadata` (Metadata) -- Dictionary metadata object
- **Returns**: `UserDictionary`

### `buildDictionary(inputDir, outputDir, metadata)`

Builds a compiled dictionary from source files.

- **Parameters**:
  - `inputDir` (string) -- Path to the directory containing source dictionary files
  - `outputDir` (string) -- Path to the output directory
  - `metadata` (Metadata) -- Dictionary metadata object
- **Returns**: void

### `buildUserDictionary(inputFile, outputDir, metadata?)`

Builds a compiled user dictionary from a CSV file.

- **Parameters**:
  - `inputFile` (string) -- Path to the user dictionary CSV file
  - `outputDir` (string) -- Path to the output directory
  - `metadata` (Metadata, optional) -- Dictionary metadata object
- **Returns**: void

### `version()` / `getVersion()`

Returns the version string of the lindera-wasm package.

- **Returns**: `string`

```javascript
import { version } from 'lindera-wasm-web-ipadic';

console.log(version()); // e.g., "2.1.1"
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

## Snake-Case Aliases

For consistency with the Python API, all methods are also available in snake\_case form:

| camelCase | snake\_case |
| --- | --- |
| `setMode()` | `set_mode()` |
| `setDictionary()` | `set_dictionary()` |
| `setDictionaryInstance()` | `set_dictionary_instance()` |
| `setUserDictionary()` | `set_user_dictionary()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionary()` | `load_user_dictionary()` |
| `buildDictionary()` | `build_dictionary()` |
| `buildUserDictionary()` | `build_user_dictionary()` |
