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

This is useful for loading a user dictionary that must share the same
metadata (schema, encoding, etc.) as the system dictionary it augments, as in
[`lindera-python/examples/tokenize_with_userdict.py`](https://github.com/lindera/lindera/blob/main/lindera-python/examples/tokenize_with_userdict.py):

```python
from lindera import Tokenizer, load_dictionary, load_user_dictionary

dictionary = load_dictionary("embedded://ipadic")
metadata = dictionary.metadata()
user_dictionary = load_user_dictionary("/path/to/user_dictionary.csv", metadata)

tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dictionary)
```

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary.

```python
from lindera import load_user_dictionary, Metadata

metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary", metadata)
```

Pass the user dictionary when building a tokenizer:

```python
from lindera import Tokenizer, load_dictionary, load_user_dictionary, Metadata

dictionary = load_dictionary("/path/to/ipadic")
metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary", metadata)

tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dict)
```

Or via the builder:

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("/path/to/ipadic")
    .set_user_dictionary("/path/to/user_dictionary")
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

### User Dictionary

Build a user dictionary from a CSV file:

```python
from lindera import build_user_dictionary, Metadata

metadata = Metadata()
build_user_dictionary("ipadic", "user_words.csv", "/path/to/output_dir", metadata)
```

The `metadata` parameter is optional. When omitted, default metadata values are used:

```python
build_user_dictionary("ipadic", "user_words.csv", "/path/to/output_dir")
```

**Note:** the first argument (`"ipadic"` above) is currently unused and reserved
for future use -- it does not select or configure the build in any way. The
build behavior is controlled entirely by `metadata` (in particular
`metadata.user_dictionary_schema`). Any string may be passed today.

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
| `normalize_details` | `bool` | `False` | Normalize morphological details |
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

Returns a dictionary representation of the metadata:

```python
metadata = Metadata(name="test")
print(metadata.to_dict())
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
