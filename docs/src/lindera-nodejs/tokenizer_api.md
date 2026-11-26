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

#### `setMode(mode)`

Sets the tokenization mode, a [`Mode`](#mode):

- `Mode.normal()` -- Standard tokenization (default)
- `Mode.decompose(penalty?)` -- Decomposes compound words into smaller units, with the [`Penalty`](#penalty) the mode holds

A value that is not a `Mode`, such as the mode name `"normal"`, throws an `InvalidArg` error. A mode name read at run time, such as a setting, becomes a `Mode` with [`Mode.fromStr()`](#modefromstrname).

```javascript
const { Mode } = require("lindera");

builder.setMode(Mode.normal());
builder.setMode(Mode.decompose({ kanjiPenaltyLengthPenalty: 0 }));
builder.setMode(Mode.fromStr("decompose"));
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

#### `setBreakCharacters(chars)`

Sets the break characters. `chars` has the same meaning as `segmenter.break_characters` in a [configuration file](../lindera-analysis/configuration.md):

- A string -- Each of its characters is a break character: no unknown word crosses a boundary between a break character and another character, and in decompose mode no system-dictionary word does either. User-dictionary words and words made of break characters only stay.
- `""` -- No break characters, which gives MeCab's segmentation.
- `null` or `undefined` -- The default, `・`. Use it to undo an earlier call.

The argument is required. A value set here overrides a `break_characters` loaded with `fromFile`. Any other value throws.

With IPADIC, for example, `レノン・ジョン`, which is not in the dictionary, is `レノン`, `・` and `ジョン` by default and one unknown word with `setBreakCharacters("")`. See [Segmenter](../lindera/segmenter.md#break-characters) for details.

```javascript
// Split as MeCab does
builder.setBreakCharacters("");
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
> Since v6.1.0, ko-dic applies the left-space penalty by default, because its `metadata.json` ships mecab-ko-dic's rules. This changes the Korean output (for example, `이` in `검색 이 잘 된다` is the interjection `IC` with the penalty and the subject particle `JKS` without it). Call `setSpacePenalty(false)` to turn it off. For a `Tokenizer` created directly, pass `false` as the fourth argument: `new Tokenizer(dictionary, Mode.normal(), null, false)`. A ko-dic downloaded from the v6.0.0 release ships no rules, so with it the penalty stays off and `setSpacePenalty(true)` makes `build()` throw. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

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

#### `new Tokenizer(dictionary, mode?, userDictionary?, spacePenalty?)`

Creates a tokenizer directly from a loaded dictionary.

`mode` takes a [`Mode`](#mode), as [`setMode`](#setmodemode) does, with the same errors. `null` or `undefined` (the default) is normal mode. For decompose mode with a custom penalty, pass `Mode.decompose(penalty)`.

`spacePenalty` takes the same values as [`setSpacePenalty`](#setspacepenaltyvalue). `null` or `undefined` (the default) uses the left-space penalty rules the dictionary ships. An invalid setting throws, as `setSpacePenalty` and `build()` do.

```javascript
const { Mode, Tokenizer, loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, Mode.normal());

// ko-dic without the left-space penalty
const koTokenizer = new Tokenizer(loadDictionary("embedded://ko-dic"), Mode.normal(), null, false);
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

#### `tokenizeBatch(texts, maxThreads?)`

Tokenizes every text of an array, each as `tokenize` does, on up to `maxThreads` threads, and returns their tokens in the order of the texts. The results are the same for every number of threads. The call is synchronous: the JavaScript thread waits until every text is tokenized, while the tokenization runs on several threads.

```javascript
const batch = tokenizer.tokenizeBatch(["形態素解析", "すもももももももものうち"]);
for (const tokens of batch) {
  console.log(tokens.map((t) => t.surface));
}
```

Every text is tokenized as a call of its own: an array of the lines of a document gives the tokens of each line, not those of the whole document as one text. A short array runs on one thread (a thread needs at least 16 KiB of input), and every thread holds its own copy of the user dictionary, if any. A batch whose results are large, such as the tokens of several megabytes of text, can run faster on fewer threads than the default: when many threads make the memory of the tokens at once, much of the time goes to the kernel.

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `texts` | `string[]` | Texts to tokenize |
| `maxThreads` | `number \| undefined` | The most threads, an integer of 1 or more (default: the number of CPUs); any other number throws an `InvalidArg` error |

**Returns:** `Token[][]`

#### `tokenizeSurfacesBatch(texts, maxThreads?)`

Tokenizes every text of an array as `tokenizeSurfaces` does, on up to `maxThreads` threads, as `tokenizeBatch` does.

**Parameters:** as `tokenizeBatch`.

**Returns:** `string[][]`

#### `tokenizeNbestBatch(texts, n, unique?, costThreshold?, maxThreads?)`

Tokenizes every text of an array as `tokenizeNbest` does, on up to `maxThreads` threads, as `tokenizeBatch` does. `n`, `unique` and `costThreshold` apply to each text.

**Parameters:** `texts` and `maxThreads` as `tokenizeBatch`; `n`, `unique` and `costThreshold` as `tokenizeNbest`.

**Returns:** `NbestResult[][]`

#### `tokenizeBatchAsync(texts, maxThreads?)`

Tokenizes every text of an array as `tokenizeBatch` does, and returns a `Promise` of the same results. The texts are tokenized on a thread of libuv's pool, on up to `maxThreads` threads, while the event loop runs; the token objects are then made on the JavaScript thread.

```javascript
const batch = await tokenizer.tokenizeBatchAsync(["形態素解析", "すもももももももものうち"]);
for (const tokens of batch) {
  console.log(tokens.map((t) => t.surface));
}
```

- A `maxThreads` that is not an integer of 1 or more rejects the `Promise` with the `InvalidArg` error that `tokenizeBatch` throws. An argument of the wrong type, such as a string instead of an array, throws when the method is called, as for every method.
- The token objects are made on the JavaScript thread. A batch with few results is made at once, as `tokenizeBatch` makes it; a larger one is made in parts, with a turn of the event loop between two parts (through `setImmediate`), so that timers and I/O run while the objects are made, and the parts take no thread of libuv's pool. With IPADIC on a Linux x86_64 machine, on `bocchan.txt` 20 times (10,760 lines, 1.39 million tokens) on 8 threads, the event loop's largest delay was about 15 ms during `tokenizeBatchAsync` and about 8 ms during `tokenizeSurfacesBatchAsync`, most of it a garbage collection of the young generation; `tokenizeBatchAsync` took about 1.6 s, and the synchronous `tokenizeBatch`, which holds the event loop for its whole time, about 1.5 s.
- Fake timers that replace `setImmediate`, as those of Jest or Sinon can, hold the `Promise` of a batch made in parts until they run the immediates. When `setImmediate` is not a function, the parts left are made at once.
- A batch takes one thread of libuv's pool while it runs (the pool has 4 by default, `UV_THREADPOOL_SIZE`), which file system and DNS calls share. Batches on the same tokenizer run one after the other; single-text calls on it do not wait for them.

**Parameters:** as `tokenizeBatch`.

**Returns:** `Promise<Token[][]>`

#### `tokenizeSurfacesBatchAsync(texts, maxThreads?)`

Tokenizes every text of an array as `tokenizeSurfacesBatch` does, and returns a `Promise` of the same results, as `tokenizeBatchAsync` does.

**Parameters:** as `tokenizeBatch`.

**Returns:** `Promise<string[][]>`

#### `tokenizeNbestBatchAsync(texts, n, unique?, costThreshold?, maxThreads?)`

Tokenizes every text of an array as `tokenizeNbestBatch` does, and returns a `Promise` of the same results, as `tokenizeBatchAsync` does. An `n`, `costThreshold` or `maxThreads` outside its range rejects the `Promise`.

**Parameters:** as `tokenizeNbestBatch`.

**Returns:** `Promise<NbestResult[][]>`

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
| `dependency` | `TokenDependency` (optional) | The bunsetsu of the token and its head; the property is there only when the token filter `japanese_dependency` ran |

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

## TokenDependency

The bunsetsu of a token and the bunsetsu it depends on. The token filter `japanese_dependency` puts it in the `dependency` of every token; see [Token Filter](../lindera-dependency/token_filter.md) for the filter, its model, and how a text is cut into sentences.

The distributed packages have the filter, from 7.1.0 on: the `dependency` feature is in the default build. They have no model. The filter takes the path of a model file: the one Lindera releases for IPADIC is `dependency.model` in `lindera-dependency-ipadic-<version>.zip`, on the page of each [release](https://github.com/lindera/lindera/releases), which `lindera download dependency-ipadic` also installs ([Installation](../lindera-dependency/installation.md#the-model)). There is one for each of `unidic`, `unidic-cwj` and `unidic-csj` too, with the name of the dictionary in place of `ipadic`.

| Property | Type | Description |
| --- | --- | --- |
| `sentence` | `number` | The number of the sentence in the text, from 0 |
| `bunsetsu` | `number` | The number of the bunsetsu of the token in its sentence, from 0 |
| `head` | `number \| null` | The number of the bunsetsu that one depends on; `null` for the last bunsetsu of a sentence |
| `role` | `string` | What the token is in its bunsetsu: `head` (the head word), `function` (the function word), `head_function` (both) or `other` |
| `score` | `number` | The score of the decision that attached the bunsetsu to its head; 0 where nothing was decided |

```javascript
const { TokenizerBuilder } = require("lindera");

const tokenizer = new TokenizerBuilder()
  .setDictionary("embedded://ipadic")
  .appendTokenFilter("japanese_dependency", { model: "ipadic.model" })
  .build();
for (const token of tokenizer.tokenize("彼は赤い花を見た。")) {
  const { bunsetsu, head, role } = token.dependency;
  console.log(token.surface, bunsetsu, head, role);
}
```

```text
彼 0 3 head
は 0 3 function
赤い 1 2 head_function
花 2 3 head
を 2 3 function
見 3 null head
た 3 null function
。 3 null other
```

`TokenDependency` is a plain object, a TypeScript `interface` as `Token` is, and `JSON.stringify` of a token keeps it. A token without the filter has no `dependency` property, and is the object it was before.

## Mode and Penalty

`TokenizerBuilder.setMode()` and the `Tokenizer` constructor take the mode as a [`Mode`](#mode), which holds the penalty of decompose mode as a [`Penalty`](#penalty).

### `Mode`

The tokenization mode: normal, or decompose with a [`Penalty`](#penalty). It is the counterpart of the Rust `lindera::mode::Mode` (see [Segmenter](../lindera/segmenter.md#tokenization-modes)), and [`setMode()`](#setmodemode) and the [`Tokenizer` constructor](#new-tokenizerdictionary-mode-userdictionary-spacepenalty) take it:

- Normal mode -- standard dictionary-cost-based tokenization
- Decompose mode -- penalty-based decomposition of compound words

`Mode` has no constructor, so `new Mode()` throws; the static factories below create a mode. A mode cannot be changed.

#### `Mode.normal()`

Creates normal mode.

```javascript
const { Mode } = require("lindera");

const mode = Mode.normal();
```

#### `Mode.decompose(penalty?)`

Creates decompose mode with `penalty`, a [`Penalty`](#penalty) object. `null` or `undefined` (the default) uses the default penalty, and a field left out keeps its default. A value that is not an object, or a field that is not an integer in its range, throws an `InvalidArg` error.

```javascript
const decompose = Mode.decompose(); // the default penalty
const custom = Mode.decompose({ kanjiPenaltyLengthPenalty: 0 });
```

#### `Mode.fromStr(name)`

Creates a mode from its name, as the Rust `Mode` parses one: `"normal"` is normal mode, and `"decompose"` is decompose mode with the default penalty. The name is case sensitive; any other name, including `"Normal"` and `"Decompose"`, throws an `Error` whose message contains `Invalid mode`. Use it for a mode name read at run time, such as a setting:

```javascript
const mode = Mode.fromStr("decompose");
console.log(mode.equals(Mode.decompose()));
// true
```

#### Properties and Methods

| Member | Type | Description |
| --- | --- | --- |
| `name` | `string` | The name of the mode, `"normal"` or `"decompose"` (a read-only property) |
| `penalty` | `Penalty \| null` | The penalty of decompose mode, as a copy with every field set, or `null` for normal mode (a read-only property). Changing the copy does not change the mode |
| `isNormal()` | `boolean` | Whether this is normal mode |
| `isDecompose()` | `boolean` | Whether this is decompose mode |
| `equals(other)` | `boolean` | Whether `other` has the same name and penalty. A value that is not a `Mode` throws an `InvalidArg` error |
| `toString()` | `string` | The name of the mode, as `name` |

`===` compares objects, not modes: each call of `Mode.normal()` returns a new object, so compare modes with `equals`.

```javascript
const { Mode } = require("lindera");

const mode = Mode.decompose({ kanjiPenaltyLengthPenalty: 0 });
console.log(mode.name, mode.isNormal(), mode.isDecompose());
// decompose false true

// A copy with every field set: the field left out has its default
const penalty = mode.penalty;
console.log(penalty.kanjiPenaltyLengthPenalty, penalty.otherPenaltyLengthPenalty);
// 0 1700

// Changing the copy does not change the mode
penalty.otherPenaltyLengthPenalty = 0;
console.log(mode.penalty.otherPenaltyLengthPenalty);
// 1700

console.log(Mode.normal().penalty);
// null

console.log(Mode.decompose().equals(Mode.fromStr("decompose")));
// true
console.log(mode.equals(Mode.decompose()));
// false
console.log(`${Mode.normal()}`);
// normal
```

### `Penalty`

A plain object (the TypeScript type `Penalty`) with the penalty parameters of decompose mode. Every field is optional, and a field left out keeps its default:

| Property | Type | Description |
| --- | --- | --- |
| `kanjiPenaltyLengthThreshold` | `number` | Length threshold for kanji sequences before a penalty is applied, an integer 0 or more (default: `2`) |
| `kanjiPenaltyLengthPenalty` | `number` | Penalty value for long kanji sequences, a 32-bit signed integer (default: `3000`) |
| `otherPenaltyLengthThreshold` | `number` | Length threshold for other character sequences before a penalty is applied, an integer 0 or more (default: `7`) |
| `otherPenaltyLengthPenalty` | `number` | Penalty value for long other-character sequences, a 32-bit signed integer (default: `1700`) |

Pass it to [`Mode.decompose()`](#modedecomposepenalty), and the mode to [`setMode()`](#setmodemode) or to the [`Tokenizer` constructor](#new-tokenizerdictionary-mode-userdictionary-spacepenalty). Without length penalties, decompose mode keeps long words whole, as normal mode does:

```javascript
const { Mode, Tokenizer, TokenizerBuilder, loadDictionary } = require("lindera");

const text = "関西国際空港限定トートバッグ";
const penalty = { kanjiPenaltyLengthPenalty: 0, otherPenaltyLengthPenalty: 0 };
const surfaces = (tokenizer) => tokenizer.tokenize(text).map((token) => token.surface);

const builder = new TokenizerBuilder().setDictionary("embedded://ipadic");
console.log(surfaces(builder.setMode(Mode.normal()).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose()).build()));
// [ '関西', '国際', '空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose(penalty)).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"), Mode.decompose(penalty));
console.log(surfaces(tokenizer));
// [ '関西国際空港', '限定', 'トートバッグ' ]
```
