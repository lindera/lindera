# Dictionary Management

Lindera Python provides functions for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `load_dictionary(uri)` to load a system dictionary.

**Embedded dictionaries** (requires the corresponding `embed-*` feature):

```python
from lindera import load_dictionary

dictionary = load_dictionary("embedded://ipadic")
```

**External dictionaries** (loaded from a directory on disk):

```python
dictionary = load_dictionary("/path/to/dictionary")
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

dictionary = load_dictionary("embedded://ipadic")
metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary", metadata)

tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dict)
```

Or via the builder:

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
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
from lindera import Metadata, CompressionAlgorithm

# Default metadata
metadata = Metadata()

# Custom metadata
metadata = Metadata(
    name="my_dictionary",
    encoding="UTF-8",
    compress_algorithm=CompressionAlgorithm.Deflate,
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
| `compress_algorithm` | `CompressionAlgorithm` | `Deflate` | Compression algorithm |
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

### CompressionAlgorithm

Available compression algorithms:

| Value | Description |
| --- | --- |
| `CompressionAlgorithm.Deflate` | DEFLATE compression (default) |
| `CompressionAlgorithm.Zlib` | Zlib compression |
| `CompressionAlgorithm.Gzip` | Gzip compression |
| `CompressionAlgorithm.Raw` | No compression |
