# Dictionary Management

Lindera Node.js provides functions for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `loadDictionary(uri)` to load a system dictionary. Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory:

```javascript
const { loadDictionary } = require("lindera-nodejs");

const dictionary = loadDictionary("/path/to/ipadic");
```

**Embedded dictionaries (advanced)** -- if you built with an `embed-*` feature flag, you can load an embedded dictionary:

```javascript
const dictionary = loadDictionary("embedded://ipadic");
```

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary.

```javascript
const { loadUserDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);
```

Pass the user dictionary when building a tokenizer:

```javascript
const { Tokenizer, loadDictionary, loadUserDictionary, Metadata } = require("lindera-nodejs");

const dictionary = loadDictionary("/path/to/ipadic");
const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);

const tokenizer = new Tokenizer(dictionary, "normal", userDict);
```

Or via the builder:

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

const tokenizer = new TokenizerBuilder()
  .setDictionary("/path/to/ipadic")
  .setUserDictionary("/path/to/user_dictionary")
  .build();
```

## Building Dictionaries

### System Dictionary

Build a system dictionary from source files:

```javascript
const { buildDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata({ name: "custom", encoding: "UTF-8" });
buildDictionary("/path/to/input_dir", "/path/to/output_dir", metadata);
```

The input directory should contain the dictionary source files (CSV lexicon, matrix.def, etc.).

### User Dictionary

Build a user dictionary from a CSV file:

```javascript
const { buildUserDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata();
buildUserDictionary("ipadic", "user_words.csv", "/path/to/output_dir", metadata);
```

The `metadata` parameter is optional. When omitted, default metadata values are used:

```javascript
buildUserDictionary("ipadic", "user_words.csv", "/path/to/output_dir");
```

## Metadata

The `Metadata` class configures dictionary parameters.

### Creating Metadata

```javascript
const { Metadata } = require("lindera-nodejs");

// Default metadata
const metadata = new Metadata();

// Custom metadata
const metadata = new Metadata({
  name: "my_dictionary",
  encoding: "UTF-8",
  defaultWordCost: -10000,
});
```

### Loading from JSON

```javascript
const metadata = Metadata.fromJsonFile("metadata.json");
```

### Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | Dictionary name |
| `encoding` | `string` | `"UTF-8"` | Character encoding |
| `defaultWordCost` | `number` | `-10000` | Default cost for unknown words |
| `defaultLeftContextId` | `number` | `1288` | Default left context ID |
| `defaultRightContextId` | `number` | `1288` | Default right context ID |
| `defaultFieldValue` | `string` | `"*"` | Default value for missing fields |
| `flexibleCsv` | `boolean` | `false` | Allow flexible CSV parsing |
| `skipInvalidCostOrId` | `boolean` | `false` | Skip entries with invalid cost or ID |
| `normalizeDetails` | `boolean` | `false` | Normalize morphological details |
| `dictionarySchema` | `Schema` | IPADIC schema | Schema for the main dictionary |
| `userDictionarySchema` | `Schema` | Minimal schema | Schema for user dictionaries |

All properties support both getting and setting:

```javascript
const metadata = new Metadata();
metadata.name = "custom_dict";
metadata.encoding = "EUC-JP";
console.log(metadata.name); // "custom_dict"
```

### `toObject()`

Returns a plain object representation of the metadata:

```javascript
const metadata = new Metadata({ name: "test" });
console.log(metadata.toObject());
```

## Schema

The `Schema` class defines the field structure of dictionary entries.

### Creating a Schema

```javascript
const { Schema } = require("lindera-nodejs");

// Default IPADIC-compatible schema
const schema = Schema.createDefault();

// Custom schema
const custom = new Schema(["surface", "left_id", "right_id", "cost", "pos", "reading"]);
```

### Schema Methods

| Method | Returns | Description |
| --- | --- | --- |
| `getFieldIndex(name)` | `number \| null` | Get field index by name |
| `fieldCount()` | `number` | Total number of fields |
| `getFieldName(index)` | `string \| null` | Get field name by index |
| `getCustomFields()` | `string[]` | Fields beyond index 4 (morphological features) |
| `getAllFields()` | `string[]` | All field names |
| `getFieldByName(name)` | `FieldDefinition \| null` | Get full field definition |
| `validateRecord(record)` | `void` | Validate a CSV record against the schema |

```javascript
const schema = Schema.createDefault();

console.log(schema.fieldCount());           // 13 (IPADIC format)
console.log(schema.getFieldIndex("pos1"));  // e.g., 4
console.log(schema.getAllFields());          // ["surface", "left_id", ...]
console.log(schema.getCustomFields());      // Fields after index 4
```

### FieldDefinition

| Property | Type | Description |
| --- | --- | --- |
| `index` | `number` | Field position index |
| `name` | `string` | Field name |
| `fieldType` | `FieldType` | Field type enum |
| `description` | `string \| undefined` | Optional description |

### FieldType

| Value | Description |
| --- | --- |
| `FieldType.Surface` | Word surface text |
| `FieldType.LeftContextId` | Left context ID |
| `FieldType.RightContextId` | Right context ID |
| `FieldType.Cost` | Word cost |
| `FieldType.Custom` | Morphological feature field |
