# Dictionary Management

Lindera Python provides functions for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `load_dictionary(uri)` to load a system dictionary. Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory:

```python
from lindera import load_dictionary

dictionary = load_dictionary("/path/to/ipadic")
```

**Embedded dictionaries (advanced)** -- if you built with an `embed-*` feature flag, you can load an embedded dictionary:

```python
dictionary = load_dictionary("embedded://ipadic")
```

A loaded `Dictionary` also exposes its own metadata:

```python
print(dictionary.metadata_name())      # e.g. "ipadic"
print(dictionary.metadata_encoding())  # e.g. "UTF-8"
metadata = dictionary.metadata()       # the full Metadata object
```

A user dictionary must share the metadata (schema, encoding, etc.) of the
system dictionary it augments. `load_user_dictionary_for` takes the system
dictionary and uses its metadata, as in
[`lindera-python/examples/tokenize_with_userdict.py`](https://github.com/lindera/lindera/blob/main/lindera-python/examples/tokenize_with_userdict.py)
(see [User Dictionaries](#user-dictionaries)).

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary. Load
one for the system dictionary it is used with:

```python
from lindera import load_dictionary, load_user_dictionary_for

dictionary = load_dictionary("/path/to/ipadic")
user_dict = load_user_dictionary_for("/path/to/user_dictionary.csv", dictionary)
```

A `.csv` is built with the system dictionary's metadata and checked against
its connection matrix as it is built: a row whose context ID lies outside the
matrix raises a `ValueError` that names its CSV line and surface (see
[Context IDs](../concepts/user_dictionary.md#context-ids)). A `.bin` is loaded
as it was built.

`load_user_dictionary(uri, metadata)` loads a user dictionary with metadata
only. A `.csv` loaded this way is checked when it is combined with the system
dictionary, and the error names the entry by its word id:

```python
from lindera import load_user_dictionary, Metadata

metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary.csv", metadata)
```

Pass the user dictionary when building a tokenizer:

```python
from lindera import Mode, Tokenizer, load_dictionary, load_user_dictionary_for

dictionary = load_dictionary("/path/to/ipadic")
user_dict = load_user_dictionary_for("/path/to/user_dictionary.csv", dictionary)

tokenizer = Tokenizer(dictionary, mode=Mode.normal(), user_dictionary=user_dict)
```

Or via the builder:

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("/path/to/ipadic")
    .set_user_dictionary("/path/to/user_dictionary.csv")
    .build()
)
```

## Building Dictionaries

### System Dictionary

Build a system dictionary from source files:

```python
from lindera import build_dictionary, Metadata

metadata = Metadata(name="custom", encoding="UTF-8")
build_dictionary("/path/to/input_dir", "/path/to/output_dir", metadata)
```

The input directory should contain the dictionary source files (CSV lexicon, matrix.def, etc.).

Metadata loaded with `Metadata.from_json_file` keeps the `space_penalty` rules of the `metadata.json`, and the built dictionary ships them. For ko-dic this means the left-space penalty stays on by default in the dictionary you build; turn it off per tokenizer with `TokenizerBuilder.set_space_penalty(False)` (see [Tokenizer API](./tokenizer_api.md#set_space_penaltyvalue)). A `Metadata` created with the constructor carries no rules.

Metadata loaded with `Metadata.from_json_file` also keeps `connection_id_mapping`, which the `metadata.json` of ko-dic and UniDic enables. The build then relabels the context IDs of the connection matrix by access frequency, as `lindera build` does, to improve the cache locality of connection-cost lookups; the results of tokenization do not change. Pass a context-ID frequency histogram as `context_id_freq`, such as the `context_id_freq.txt` of the dictionary crate, to rank the IDs by their frequency in a corpus, as `lindera build --context-id-freq` does. Without one, the IDs are ranked by the count of dictionary entries, a much weaker proxy. A dictionary without `connection_id_mapping` does not read the histogram.

```python
metadata = Metadata.from_json_file("/path/to/lindera-ko-dic/metadata.json")
build_dictionary(
    "/path/to/mecab-ko-dic",
    "/path/to/output_dir",
    metadata,
    context_id_freq="/path/to/lindera-ko-dic/context_id_freq.txt",
)
```

### User Dictionary

Build a user dictionary from a CSV file for the system dictionary it is used
with:

```python
from lindera import build_user_dictionary_for, load_dictionary, load_user_dictionary_for

dictionary = load_dictionary("/path/to/ipadic")
build_user_dictionary_for("user_words.csv", "/path/to/user_dictionary.bin", dictionary)
user_dict = load_user_dictionary_for("/path/to/user_dictionary.bin", dictionary)
```

The CSV is built with the system dictionary's metadata and checked against
its connection matrix: a row whose context ID lies outside the matrix raises
a `ValueError` that names its CSV line and surface, and no file is written
(see [Context IDs](../concepts/user_dictionary.md#context-ids)). The second
argument is the path of the file to write. End it with `.bin`, since the
loaders choose how to load a user dictionary by its extension.

`build_user_dictionary(kind, input_file, output_dir, metadata)` builds a user
dictionary with metadata only. Despite its name, `output_dir` is the path of
the file to write too, not a directory. The rows are checked when the user
dictionary is combined with the system dictionary, and the error names the
entry by its word id:

```python
from lindera import build_user_dictionary, Metadata

metadata = Metadata()
build_user_dictionary("ipadic", "user_words.csv", "/path/to/user_dictionary.bin", metadata)
```

The `metadata` parameter is optional. When omitted, default metadata values are used:

```python
build_user_dictionary("ipadic", "user_words.csv", "/path/to/user_dictionary.bin")
```

**Note:** the first argument (`"ipadic"` above) is currently unused and reserved
for future use -- it does not select or configure the build in any way. The
build behavior is controlled entirely by `metadata` (in particular
`metadata.user_dictionary_schema`). Any string may be passed today. To build
for a system dictionary, use `build_user_dictionary_for`.

## Metadata

The `Metadata` class configures dictionary parameters.

### Creating Metadata

```python
from lindera import Metadata

# Default metadata
metadata = Metadata()

# Custom metadata
metadata = Metadata(
    name="my_dictionary",
    encoding="UTF-8",
    default_word_cost=-10000,
)
```

### Loading from JSON

```python
metadata = Metadata.from_json_file("metadata.json")
```

### Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `str` | `"default"` | Dictionary name |
| `encoding` | `str` | `"UTF-8"` | Character encoding |
| `default_word_cost` | `int` | `-10000` | Default cost for unknown words |
| `default_left_context_id` | `int` | `1288` | Default left context ID |
| `default_right_context_id` | `int` | `1288` | Default right context ID |
| `default_field_value` | `str` | `"*"` | Default value for missing fields |
| `flexible_csv` | `bool` | `False` | Allow flexible CSV parsing |
| `skip_invalid_cost_or_id` | `bool` | `False` | Skip entries with invalid cost or ID |
| `dictionary_schema` | `Schema` | IPADIC schema | Schema for the main dictionary |
| `user_dictionary_schema` | `Schema` | Minimal schema | Schema for user dictionaries |

All properties support both getting and setting:

```python
metadata = Metadata()
metadata.name = "custom_dict"
metadata.encoding = "EUC-JP"
print(metadata.name)  # "custom_dict"
```

### `to_dict()`

Returns the metadata as a `dict` of strings: the properties above, with `dictionary_schema_fields` and `user_dictionary_schema_fields` (comma-separated field names) in place of the two schemas.

```python
metadata = Metadata(name="test")
print(metadata.to_dict())
```

When the metadata carries left-space penalty rules, as ko-dic's does, the `dict` also has a `space_penalty` entry holding them as a JSON string; the key is absent otherwise:

```python
metadata = Metadata.from_json_file("/path/to/ko-dic/metadata.json")
metadata.to_dict()["space_penalty"]
# '{"rules":[{"pos":["EC","EF",...],"cost":3000},{"pos":["JC","JKB",...],"cost":6000}]}'
```

## Schema

`Schema`, `FieldDefinition`, and `FieldType` describe the field layout of a
dictionary entry. A schema is used by `Metadata.dictionary_schema` and
`Metadata.user_dictionary_schema` (see the table above).

### FieldType

`FieldType` enumerates the category of a single field:

- `FieldType.Surface` -- surface form (word text)
- `FieldType.LeftContextId` -- left context ID
- `FieldType.RightContextId` -- right context ID
- `FieldType.Cost` -- word cost
- `FieldType.Custom` -- any other, dictionary-specific field

### FieldDefinition

`FieldDefinition` describes a single field within a schema.

#### `FieldDefinition(index, name, field_type, description=None)`

```python
from lindera import FieldDefinition, FieldType

field = FieldDefinition(0, "surface", FieldType.Surface, "Surface form")
```

**Properties** (read-only):

| Property | Type | Description |
| --- | --- | --- |
| `index` | `int` | Zero-based position of the field within the schema |
| `name` | `str` | Field name |
| `field_type` | `FieldType` | Field type |
| `description` | `str` or `None` | Optional human-readable description |

### Creating a Schema

`Schema` holds an ordered list of field names and provides lookups between
field name and index.

#### `Schema(fields)`

Creates a schema from a list of field names.

```python
from lindera import Schema

schema = Schema([
    "surface",
    "left_context_id",
    "right_context_id",
    "cost",
    "major_pos",
    "reading",
])
```

#### `Schema.create_default()`

A static method that returns the built-in default schema: 13 fields matching
the IPADIC-style layout (`surface`, `left_context_id`, `right_context_id`,
`cost`, `major_pos`, `pos_detail_1`, `pos_detail_2`, `pos_detail_3`,
`conjugation_type`, `conjugation_form`, `base_form`, `reading`,
`pronunciation`).

```python
schema = Schema.create_default()
```

### Schema Methods and Properties

| Member | Returns | Description |
| --- | --- | --- |
| `fields` (property) | `list[str]` | All field names, in order |
| `field_count()` | `int` | Total number of fields |
| `get_field_index(name)` | `int` or `None` | Index of the field named `name` |
| `get_field_name(index)` | `str` or `None` | Field name at `index` |
| `get_custom_fields()` | `list[str]` | Field names after the four fixed fields (`surface`, `left_context_id`, `right_context_id`, `cost`) |
| `get_field_by_name(name)` | `FieldDefinition` or `None` | Full field definition for `name` |
| `validate_record(record)` | `None` | Raises `ValueError` if `record` does not match the schema |
| `__len__()` | `int` | Same as `field_count()` |

```python
schema = Schema.create_default()

schema.field_count()               # 13
schema.get_field_index("cost")     # 3
schema.get_field_name(0)           # "surface"
schema.get_custom_fields()         # ["major_pos", "pos_detail_1", ..., "pronunciation"]
len(schema)                        # 13

field = schema.get_field_by_name("surface")
print(field.index, field.name, field.field_type)  # 0 surface FieldType.Surface

schema.validate_record([
    "東京", "1288", "1288", "100",
    "名詞", "固有名詞", "地域", "一般", "*", "*",
    "東京", "トウキョウ", "トーキョー",
])
```
