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

