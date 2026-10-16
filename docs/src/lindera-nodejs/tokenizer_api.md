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

Loads configuration from a YAML file and returns a new builder.

```javascript
const builder = new TokenizerBuilder().fromFile("lindera.yml");
```

See [`lindera-nodejs/resources/lindera.yml`](https://github.com/lindera/lindera/blob/main/lindera-nodejs/resources/lindera.yml) for a complete example configuration file, including dictionary, character filter, and token filter settings.

### Configuration Methods

All setter methods return the builder itself (`this`), so they can be chained or called one statement at a time.

#### `setMode(mode, penalty?)`

Sets the tokenization mode.

- `"normal"` -- Standard tokenization (default)
- `"decompose"` -- Decomposes compound words into smaller units

`penalty` is the [`Penalty`](#penalty) object that `"decompose"` mode uses; `null` or `undefined` (the default) uses the default penalty. A penalty with `"normal"` throws, as an invalid mode does, and a value that is not an object, or a field that is not an integer in its range, throws an `InvalidArg` error.

```javascript
builder.setMode("normal");
builder.setMode("decompose", { kanjiPenaltyLengthPenalty: 0 });
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
builder.setUserDictionary("/path/to/user_dictionary.csv");
```

#### `setKeepWhitespace(keep)`

Controls whether whitespace tokens appear in the output. Kept whitespace also stays in the lattice, so the other tokens can differ from those of the default output, which skips whitespace in the lattice as MeCab does. To keep whitespace in the lattice without outputting it, use [`setSkipWhitespace(false)`](#setskipwhitespaceskip) instead.

```javascript
builder.setKeepWhitespace(true);
```

#### `setSkipWhitespace(skip)`

Sets whether whitespace is skipped in the lattice. The value means the same as `segmenter.skip_whitespace` in a [configuration file](../lindera-analysis/configuration.md).

| Value | Effect |
| --- | --- |
| `true` | Skips whitespace, so the words on either side of it connect directly, as MeCab does |
| `false` | Keeps whitespace in the lattice as a node of its own (a whitespace dictionary entry or the `SPACE` unknown word), as before v7; whitespace is still dropped from the output |
| `null` or `undefined` | Default: the setting of the dictionary's `metadata.json` (skipping, except with SudachiDict) |

A value set here overrides the dictionary's default and a `skip_whitespace` loaded with `fromFile`. It has no effect while whitespace tokens are kept (`setKeepWhitespace(true)`), because kept whitespace always stays in the lattice. The argument is required; any other value throws an `Error`.

With IPADIC, for example, `Google が 新しい` reads `が` as a particle (`助詞`) by default and as a conjunction (`接続詞`) with `setSkipWhitespace(false)`. See [Segmenter](../lindera/segmenter.md#whitespace-handling) for details.

```javascript
// Keep whitespace in the lattice, as before v7
builder.setSkipWhitespace(false);

// Back to the dictionary's default
builder.setSkipWhitespace(null);
```

#### `setSpacePenalty(value)`

Sets the left-space penalty for Korean (mecab-ko's `left-space-penalty-factor`): a candidate that starts right after whitespace and whose first part-of-speech tag is listed in a rule gets that rule's cost added. The value means the same as `segmenter.space_penalty` in a [configuration file](../lindera-analysis/configuration.md).

| Value | Effect |
| --- | --- |
| `null` or `undefined` | Default: the rules the dictionary ships in its `metadata.json`, if any |
| `false` | Turns the penalty off |
| `true` | Requires the dictionary's rules; `build()` throws if the dictionary ships none |
| `{ rules: [{ pos: [...], cost: n }, ...] }` | Applies these rules instead of the dictionary's |

Any other value throws an `Error`.

```javascript
// Turn the penalty off
builder.setSpacePenalty(false);

// Explicit rules (these are the ones ko-dic ships)
builder.setSpacePenalty({
  rules: [
    { pos: ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], cost: 3000 },
    { pos: ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], cost: 6000 },
  ],
});

// Back to the dictionary's default
builder.setSpacePenalty(null);
```

> [!NOTE]
> Since v6.1.0, ko-dic applies the left-space penalty by default, because its `metadata.json` ships mecab-ko-dic's rules. This changes the Korean output (for example, `이` in `검색 이 잘 된다` is the interjection `IC` with the penalty and the subject particle `JKS` without it). Call `setSpacePenalty(false)` to turn it off. For a `Tokenizer` created directly, pass `false` as the fourth argument: `new Tokenizer(dictionary, "normal", null, false)`. A ko-dic downloaded from the v6.0.0 release ships no rules, so with it the penalty stays off and `setSpacePenalty(true)` makes `build()` throw. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

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

The arguments of `setSpacePenalty`, `appendCharacterFilter` and `appendTokenFilter` may nest arrays and objects at most 128 levels deep. A deeper value, or one that contains itself, throws an `Error`.

### Build

#### `build()`

Builds and returns a `Tokenizer` with the configured settings.

```javascript
const tokenizer = builder.build();
```

## Tokenizer

`Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `new Tokenizer(dictionary, mode?, userDictionary?, spacePenalty?, penalty?)`

Creates a tokenizer directly from a loaded dictionary.

`spacePenalty` takes the same values as [`setSpacePenalty`](#setspacepenaltyvalue). `null` or `undefined` (the default) uses the left-space penalty rules the dictionary ships. An invalid setting throws, as `setSpacePenalty` and `build()` do.

`penalty` takes a [`Penalty`](#penalty) for `"decompose"` mode, as [`setMode`](#setmodemode-penalty) does, with the same errors.

```javascript
const { Tokenizer, loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");

// ko-dic without the left-space penalty
const koTokenizer = new Tokenizer(loadDictionary("embedded://ko-dic"), "normal", null, false);
```

### Tokenizer Methods

#### `tokenize(text)`

Tokenizes the input text and returns an array of plain token objects.

```javascript
const tokens = tokenizer.tokenize("形態素解析");
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `string` | Text to tokenize |

**Returns:** `Token[]`

Tokens are plain JavaScript objects, not class instances, so they are reclaimed by ordinary garbage collection and pass through `JSON.stringify`, `structuredClone`, and worker transfer without conversion. See [Token](#token).

#### `tokenizeSurfaces(text)`

Tokenizes the input text and returns only the token surfaces, as an array of strings. This is the fast path for wakati-style use: no token objects are built and no morphological details are loaded, so it is significantly faster than `tokenize` when only the surface strings are needed. The result equals `tokenizer.tokenize(text).map((t) => t.surface)`.

```javascript
const surfaces = tokenizer.tokenizeSurfaces("形態素解析");
// ["形態素", "解析"]
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `string` | Text to tokenize |

**Returns:** `string[]`

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
| `n` | `number` | Number of results to return, an integer of 0 or more |
| `unique` | `boolean` | Deduplicate results (default: `false`) |
| `costThreshold` | `number \| undefined` | Maximum cost difference from the best path (default: `undefined`) |

**Returns:** `NbestResult[]`, where each `NbestResult` is `{ tokens: Token[], cost: number }`

## Token

`Token` is a plain object describing a single morphological token. It is a
TypeScript `interface`, not a class: it has no methods and no prototype, and
every field is read directly.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `surface` | `string` | Surface form of the token |
| `byteStart` | `number` | Start byte position in the original text |
| `byteEnd` | `number` | End byte position in the original text |
| `position` | `number` | Token position index |
| `wordId` | `number` | Dictionary word ID |
| `isUnknown` | `boolean` | `true` if the word is not in the dictionary |
| `details` | `string[]` | Morphological details (part of speech, reading, etc.) |

### Reading Details

Index `details` directly. Out-of-range indexes yield `undefined`.

```javascript
const token = tokenizer.tokenize("東京")[0];
const pos = token.details[0];      // e.g., "名詞"
const subpos = token.details[1];   // e.g., "固有名詞"
const reading = token.details[7];  // e.g., "トウキョウ"
```

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats

## Mode and Penalty

`TokenizerBuilder.setMode()` and the `Tokenizer` constructor take the mode as a plain string
(`"normal"` or `"decompose"`) and the penalty of decompose mode as a [`Penalty`](#penalty) object.

### `Mode`

A string enum with two values:

- `Mode.Normal` -- standard dictionary-cost-based tokenization
- `Mode.Decompose` -- penalty-based decomposition of compound words

### `Penalty`

A plain object (the TypeScript type `Penalty`) with the penalty parameters of decompose mode. Every field is optional, and a field left out keeps its default:

| Property | Type | Description |
| --- | --- | --- |
| `kanjiPenaltyLengthThreshold` | `number` | Length threshold for kanji sequences before a penalty is applied, an integer 0 or more (default: `2`) |
| `kanjiPenaltyLengthPenalty` | `number` | Penalty value for long kanji sequences, a 32-bit signed integer (default: `3000`) |
| `otherPenaltyLengthThreshold` | `number` | Length threshold for other character sequences before a penalty is applied, an integer 0 or more (default: `7`) |
| `otherPenaltyLengthPenalty` | `number` | Penalty value for long other-character sequences, a 32-bit signed integer (default: `1700`) |

Pass it with `"decompose"` to [`setMode()`](#setmodemode-penalty) or to the [`Tokenizer` constructor](#new-tokenizerdictionary-mode-userdictionary-spacepenalty-penalty). Without length penalties, decompose mode keeps long words whole:

```javascript
const { Tokenizer, TokenizerBuilder, loadDictionary } = require("lindera");

const text = "関西国際空港限定トートバッグ";
const penalty = { kanjiPenaltyLengthPenalty: 0, otherPenaltyLengthPenalty: 0 };
const surfaces = (tokenizer) => tokenizer.tokenize(text).map((token) => token.surface);

const builder = new TokenizerBuilder().setDictionary("embedded://ipadic");
console.log(surfaces(builder.setMode("decompose").build()));
// [ '関西', '国際', '空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode("decompose", penalty).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"), "decompose", null, null, penalty);
console.log(surfaces(tokenizer));
// [ '関西国際空港', '限定', 'トートバッグ' ]
```

A penalty with `"normal"` mode throws.
