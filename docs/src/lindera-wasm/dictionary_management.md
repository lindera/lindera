# Dictionary Management

## Embedded Dictionaries

The simplest way to use dictionaries in WASM is to embed them at build time using feature flags. Embedded dictionaries are loaded via the `embedded://` URI scheme.

### Loading an Embedded Dictionary

```javascript
import { loadDictionary } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
```

Available embedded dictionary URIs (depending on which features were enabled at build time):

| URI | Feature Flag |
| --- | --- |
| `embedded://ipadic` | `embed-ipadic` |
| `embedded://unidic` | `embed-unidic` |
| `embedded://ko-dic` | `embed-ko-dic` |
| `embedded://cc-cedict` | `embed-cc-cedict` |
| `embedded://jieba` | `embed-jieba` |

### Using with TokenizerBuilder

When using `TokenizerBuilder`, you set the dictionary URI directly:

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();
```

### Using with Tokenizer Constructor

You can also pass a loaded dictionary to the `Tokenizer` constructor:

```javascript
import { loadDictionary, Tokenizer } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");
```

## Loading from Bytes (OPFS)

When dictionaries are stored in OPFS or other browser storage, you can load them directly from raw byte arrays using `loadDictionaryFromBytes()`. This avoids filesystem access and works in browser environments.

### `loadDictionaryFromBytes(metadata, dictDa, dictVals, dictWordsIdx, dictWords, matrixMtx, charDef, unk)`

Constructs a `Dictionary` from the binary data of each dictionary component file. Compressed data (built with the `compress` feature) is automatically decompressed.

- **Parameters**:
  - `metadata` (`Uint8Array`) -- Contents of `metadata.json`
  - `dictDa` (`Uint8Array`) -- Contents of `dict.da` (Double-Array Trie)
  - `dictVals` (`Uint8Array`) -- Contents of `dict.vals` (word value data)
  - `dictWordsIdx` (`Uint8Array`) -- Contents of `dict.wordsidx` (word details index)
  - `dictWords` (`Uint8Array`) -- Contents of `dict.words` (word details)
  - `matrixMtx` (`Uint8Array`) -- Contents of `matrix.mtx` (connection cost matrix)
  - `charDef` (`Uint8Array`) -- Contents of `char_def.bin` (character definitions)
  - `unk` (`Uint8Array`) -- Contents of `unk.bin` (unknown word dictionary)
- **Returns**: `Dictionary`

```javascript
import { loadDictionaryFromBytes, TokenizerBuilder } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

// Load dictionary files from OPFS
const files = await loadDictionaryFiles("ipadic");

// Create a Dictionary from bytes
const dictionary = loadDictionaryFromBytes(
    files.metadata,
    files.dictDa,
    files.dictVals,
    files.dictWordsIdx,
    files.dictWords,
    files.matrixMtx,
    files.charDef,
    files.unk,
);

// Use with TokenizerBuilder
const builder = new TokenizerBuilder();
builder.setDictionaryInstance(dictionary);
builder.setMode("normal");
const tokenizer = builder.build();
```

See [OPFS Dictionary Storage](./opfs.md) for the full OPFS workflow.

## Dictionary Class

The `Dictionary` class represents a loaded morphological analysis dictionary.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `name` | `string` | Dictionary name (e.g., `"ipadic"`) |
| `encoding` | `string` | Character encoding of the dictionary |
| `metadata` | `Metadata` | Full metadata object |

```javascript
const dict = loadDictionary("embedded://ipadic");
console.log(dict.name);     // "ipadic"
console.log(dict.encoding); // "utf-8"
```

## User Dictionaries

User dictionaries allow you to add custom words that are not in the system dictionary.

### Loading a User Dictionary

```javascript
import { loadUserDictionary } from 'lindera-wasm-web-ipadic';

const metadata = dictionary.metadata;
const userDict = loadUserDictionary("/path/to/user_dict.csv", metadata);
```

### Using a User Dictionary with Tokenizer

```javascript
import { loadDictionary, loadUserDictionary, Tokenizer } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
const userDict = loadUserDictionary("/path/to/user_dict.csv", dictionary.metadata);
const tokenizer = new Tokenizer(dictionary, "normal", userDict);
```

### User Dictionary CSV Format

The user dictionary CSV follows the same format as the Lindera user dictionary:

```csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
```

Each line contains: `surface,part_of_speech,reading`

## Building Dictionaries

You can build compiled dictionaries from source files using the JavaScript API.

### Building a System Dictionary

```javascript
import { buildDictionary } from 'lindera-wasm-web-ipadic';

const metadata = {
    name: "custom-dict",
    encoding: "utf-8",
    // ... other metadata fields
};

buildDictionary("/path/to/source/dir", "/path/to/output/dir", metadata);
```

### Building a User Dictionary

```javascript
import { buildUserDictionary } from 'lindera-wasm-web-ipadic';

buildUserDictionary("/path/to/user_dict.csv", "/path/to/output/dir");
```

The `metadata` parameter is optional for `buildUserDictionary`. If omitted, default metadata is used.

## Metadata

The `Metadata` class configures dictionary parameters.

### Constructor

```javascript
const metadata = new Metadata(name?, encoding?, compressAlgorithm?);
```

- **Parameters**:
  - `name` (string, optional) -- Dictionary name (default: `"default"`)
  - `encoding` (string, optional) -- Character encoding (default: `"UTF-8"`)
  - `compressAlgorithm` (CompressionAlgorithm, optional) -- Compression method (default: `Deflate`)

### Static Methods

#### `Metadata.createDefault()`

Creates a `Metadata` instance with default values.

```javascript
const metadata = Metadata.createDefault();
```

### Metadata Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | Dictionary name |
| `encoding` | `string` | `"UTF-8"` | Character encoding |
| `compress_algorithm` | `CompressionAlgorithm` | `Deflate` | Compression algorithm |
| `dictionary_schema` | `Schema` | IPADIC schema | Schema for the main dictionary |
| `user_dictionary_schema` | `Schema` | Minimal schema | Schema for user dictionaries |

All properties support both getting and setting:

```javascript
const metadata = Metadata.createDefault();
metadata.name = "custom_dict";
metadata.encoding = "EUC-JP";
console.log(metadata.name); // "custom_dict"
```

You can also access the metadata from a loaded dictionary via `dictionary.metadata`.

### CompressionAlgorithm

| Value | Description |
| --- | --- |
| `CompressionAlgorithm.Deflate` | DEFLATE compression (default) |
| `CompressionAlgorithm.Zlib` | Zlib compression |
| `CompressionAlgorithm.Gzip` | Gzip compression |
| `CompressionAlgorithm.Raw` | No compression |

### Schema

The `Schema` class defines the field structure of dictionary entries.

#### Schema Constructor

```javascript
const schema = new Schema(["surface", "left_id", "right_id", "cost", "pos", "reading"]);
```

#### Schema Static Methods

- `Schema.create_default()` -- Creates a default IPADIC-like schema

#### Schema Methods

| Method | Returns | Description |
| --- | --- | --- |
| `get_field_index(name)` | `number \| undefined` | Get field index by name |
| `field_count()` | `number` | Total number of fields |
| `get_field_name(index)` | `string \| undefined` | Get field name by index |
| `get_custom_fields()` | `string[]` | Fields beyond index 3 (morphological features) |
| `get_all_fields()` | `string[]` | All field names |
| `get_field_by_name(name)` | `FieldDefinition \| undefined` | Get full field definition |

#### FieldDefinition

| Property | Type | Description |
| --- | --- | --- |
| `index` | `number` | Field position index |
| `name` | `string` | Field name |
| `field_type` | `FieldType` | Field type enum |
| `description` | `string \| undefined` | Optional description |

#### FieldType

| Value | Description |
| --- | --- |
| `FieldType.Surface` | Word surface text |
| `FieldType.LeftContextId` | Left context ID |
| `FieldType.RightContextId` | Right context ID |
| `FieldType.Cost` | Word cost |
| `FieldType.Custom` | Morphological feature field |
