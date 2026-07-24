# Dictionary Management

## Loading Dictionaries from OPFS

The recommended way to use dictionaries in WASM is to download them from [GitHub Releases](https://github.com/lindera/lindera/releases) and load them via OPFS. This avoids embedding large dictionaries in the WASM binary.

### Loading from Bytes

Use `loadDictionaryFromBytes()` to construct a `Dictionary` from raw byte arrays stored in OPFS or other browser storage.

#### `loadDictionaryFromBytes(metadata, dictDa, dictVals, dictWordsIdx, dictWords, matrixMtx, charDef, unk)`

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

See [OPFS Dictionary Storage](./opfs.md) for the full OPFS workflow including downloading and caching.

## Embedded Dictionaries (Advanced)

If you built with an `embed-*` feature flag, you can load embedded dictionaries via the `embedded://` URI scheme. This increases the WASM binary size significantly.

> [!NOTE]
> `lindera-wasm-web-ipadic` in the examples below is an illustrative package name for a local build with the `embed-ipadic` feature, not something published to npm. Only `lindera-wasm-web` and `lindera-wasm-bundler` are actually published; see [NPM Package Naming Convention](./installation.md#npm-package-naming-convention).

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

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();
```

### Using with Tokenizer Constructor

```javascript
import { loadDictionary, Tokenizer } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");
```

## Dictionary Class

The `Dictionary` class represents a loaded morphological analysis dictionary.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `name` | `string` | Dictionary name (e.g., `"ipadic"`) |
| `encoding` | `string` | Character encoding of the dictionary |
| `metadata` | `Metadata` | Full metadata object |

```javascript
console.log(dictionary.name);     // "ipadic"
console.log(dictionary.encoding); // "utf-8"
```

## User Dictionaries

User dictionaries allow you to add custom words that are not in the system dictionary.

### Loading a User Dictionary

```javascript
import { loadUserDictionary } from 'lindera-wasm-web';

const metadata = dictionary.metadata;
const userDict = loadUserDictionary("/path/to/user_dict.csv", metadata);
```

### Using a User Dictionary with Tokenizer

```javascript
import { loadDictionaryFromBytes, loadUserDictionary, Tokenizer } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);
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

`metadata` must be an actual `Metadata` instance, not a plain object literal -- the generated binding asserts the argument is a `Metadata` and throws otherwise. Create one with `Metadata.createDefault()` and set the fields you need:

```javascript
import { buildDictionary, Metadata } from 'lindera-wasm-web';

const metadata = Metadata.createDefault();
metadata.name = "custom-dict";
metadata.encoding = "utf-8";

buildDictionary("/path/to/source/dir", "/path/to/output/dir", metadata);
```

### Building a User Dictionary

```javascript
import { buildUserDictionary } from 'lindera-wasm-web';

buildUserDictionary("/path/to/user_dict.csv", "/path/to/output/dir");
```

The `metadata` parameter is optional for `buildUserDictionary`. If omitted, default metadata is used.

## Metadata

The `Metadata` class configures dictionary parameters.

### Constructor

```javascript
const metadata = new Metadata(name?, encoding?);
```

- **Parameters**:
  - `name` (string, optional) -- Dictionary name (default: `"default"`)
  - `encoding` (string, optional) -- Character encoding (default: `"UTF-8"`)

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
| `dictionary_schema` | `Schema` | IPADIC schema | Schema for the main dictionary |
| `user_dictionary_schema` | `Schema` | Minimal schema | Schema for user dictionaries |

All properties support both getting and setting:

```javascript
const metadata = Metadata.createDefault();
metadata.name = "custom_dict";
metadata.encoding = "EUC-JP";
console.log(metadata.name); // "custom_dict"
```

> [!NOTE]
> Unlike the Python, Node.js, Ruby, and PHP bindings, the WASM `Metadata` class does not expose `default_word_cost`, `default_left_context_id`, `default_right_context_id`, `default_field_value`, `flexible_csv`, `skip_invalid_cost_or_id`, or `normalize_details` as gettable/settable properties (see `lindera-wasm/src/metadata.rs`). These always fall back to the shared binding defaults (word cost `-10000`, context IDs `1288`, field value `"*"`, flags `false`) and cannot be customized from JavaScript.

You can also access the metadata from a loaded dictionary via `dictionary.metadata`.

### Schema

The `Schema` class defines the field structure of dictionary entries.

#### Schema Constructor

```javascript
const schema = new Schema(["surface", "left_id", "right_id", "cost", "pos", "reading"]);
```

#### Schema Static Methods

- `Schema.create_default()` -- Creates a built-in 13-field schema loosely modeled on IPADIC's layout: the four system fields (`surface`, `left_context_id`, `right_context_id`, `cost`) followed by nine generic feature fields (`major_pos`, `pos_detail_1`-`pos_detail_3`, `conjugation_type`, `conjugation_form`, `base_form`, `reading`, `pronunciation`). These names -- and the `conjugation_type`/`conjugation_form` order -- differ from the real `lindera-ipadic` dictionary schema (`part_of_speech`, `part_of_speech_subcategory_1`-`_3`, `conjugation_form`, `conjugation_type`, ...). To match an actual IPADIC dictionary's schema, use `dictionary.metadata.dictionary_schema` from a loaded dictionary instead

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
