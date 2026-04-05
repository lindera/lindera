# Tokenizer API

## TokenizerBuilder

`TokenizerBuilder` configures and constructs a `Tokenizer` instance using the builder pattern.

### Constructors

#### `new TokenizerBuilder()`

Creates a new builder with default configuration.

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
```

#### `new TokenizerBuilder().fromFile(filePath)`

Loads configuration from a JSON file and returns a new builder.

```javascript
const builder = new TokenizerBuilder().fromFile("config.json");
```

### Configuration Methods

All setter methods return `this` for method chaining.

#### `setMode(mode)`

Sets the tokenization mode.

- `"normal"` -- Standard tokenization (default)
- `"decompose"` -- Decomposes compound words into smaller units

```javascript
builder.setMode("normal");
```

#### `setDictionary(path)`

Sets the system dictionary path or URI.

```javascript
// Use an embedded dictionary
builder.setDictionary("embedded://ipadic");

// Use an external dictionary
builder.setDictionary("/path/to/dictionary");
```

#### `setUserDictionary(uri)`

Sets the user dictionary URI.

```javascript
builder.setUserDictionary("/path/to/user_dictionary");
```

#### `setKeepWhitespace(keep)`

Controls whether whitespace tokens appear in the output.

```javascript
builder.setKeepWhitespace(true);
```

#### `appendCharacterFilter(kind, args?)`

Appends a character filter to the preprocessing pipeline.

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(kind, args?)`

Appends a token filter to the postprocessing pipeline.

```javascript
builder.appendTokenFilter("lowercase", {});
```

### Build

#### `build()`

Builds and returns a `Tokenizer` with the configured settings.

```javascript
const tokenizer = builder.build();
```

## Tokenizer

`Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `new Tokenizer(dictionary, mode?, userDictionary?)`

Creates a tokenizer directly from a loaded dictionary.

```javascript
const { Tokenizer, loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");
```

### Tokenizer Methods

#### `tokenize(text)`

Tokenizes the input text and returns an array of `Token` objects.

```javascript
const tokens = tokenizer.tokenize("形態素解析");
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `string` | Text to tokenize |

**Returns:** `Token[]`

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

Returns the N-best tokenization results, each containing tokens and total path cost.

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const { tokens, cost } of results) {
  console.log(cost, tokens.map((t) => t.surface));
}
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `string` | Text to tokenize |
| `n` | `number` | Number of results to return |
| `unique` | `boolean` | Deduplicate results (default: `false`) |
| `costThreshold` | `number \| undefined` | Maximum cost difference from the best path (default: `undefined`) |

**Returns:** `Array<{ tokens: Token[], cost: number }>`

## Token

`Token` represents a single morphological token.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `surface` | `string` | Surface form of the token |
| `byteStart` | `number` | Start byte position in the original text |
| `byteEnd` | `number` | End byte position in the original text |
| `position` | `number` | Token position index |
| `wordId` | `number` | Dictionary word ID |
| `isUnknown` | `boolean` | `true` if the word is not in the dictionary |
| `details` | `string[] \| null` | Morphological details (part of speech, reading, etc.) |

### Token Methods

#### `getDetail(index)`

Returns the detail string at the specified index, or `null` if the index is out of range.

```javascript
const token = tokenizer.tokenize("東京")[0];
const pos = token.getDetail(0);      // e.g., "名詞"
const subpos = token.getDetail(1);   // e.g., "固有名詞"
const reading = token.getDetail(7);  // e.g., "トウキョウ"
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `index` | `number` | Zero-based index into the details array |

**Returns:** `string | null`

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats
