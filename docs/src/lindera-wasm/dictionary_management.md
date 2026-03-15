# Dictionary Management

## Embedded Dictionaries

The simplest way to use dictionaries in WASM is to embed them at build time using feature flags. Embedded dictionaries are loaded via the `embedded://` URI scheme.

### Loading an Embedded Dictionary

```javascript
import { loadDictionary } from 'lindera-wasm-ipadic-web';

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
import { loadDictionary, Tokenizer } from 'lindera-wasm-ipadic-web';

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
const dict = loadDictionary("embedded://ipadic");
console.log(dict.name);     // "ipadic"
console.log(dict.encoding); // "utf-8"
```

## User Dictionaries

User dictionaries allow you to add custom words that are not in the system dictionary.

### Loading a User Dictionary

```javascript
import { loadUserDictionary } from 'lindera-wasm-ipadic-web';

const metadata = dictionary.metadata;
const userDict = loadUserDictionary("/path/to/user_dict.csv", metadata);
```

### Using a User Dictionary with Tokenizer

```javascript
import { loadDictionary, loadUserDictionary, Tokenizer } from 'lindera-wasm-ipadic-web';

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
import { buildDictionary } from 'lindera-wasm-ipadic-web';

const metadata = {
    name: "custom-dict",
    encoding: "utf-8",
    // ... other metadata fields
};

buildDictionary("/path/to/source/dir", "/path/to/output/dir", metadata);
```

### Building a User Dictionary

```javascript
import { buildUserDictionary } from 'lindera-wasm-ipadic-web';

buildUserDictionary("/path/to/user_dict.csv", "/path/to/output/dir");
```

The `metadata` parameter is optional for `buildUserDictionary`. If omitted, default metadata is used.

## Metadata

The `Metadata` object contains dictionary configuration such as:

- Dictionary name
- Character encoding
- Compression algorithm
- Schema definitions for dictionary fields

You can access the metadata from a loaded dictionary via `dictionary.metadata`.
