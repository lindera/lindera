# Tokenizer API

## TokenizerBuilder

`TokenizerBuilder` configures and constructs a `Tokenizer` instance using the builder pattern.

### Constructors

#### `TokenizerBuilder()`

Creates a new builder with default configuration.

```python
from lindera import TokenizerBuilder

builder = TokenizerBuilder()
```

#### `TokenizerBuilder().from_file(file_path)`

Loads configuration from a JSON file and returns a new builder.

```python
builder = TokenizerBuilder().from_file("config.json")
```

### Configuration Methods

All setter methods return `self` for method chaining.

#### `set_mode(mode)`

Sets the tokenization mode.

- `"normal"` -- Standard tokenization (default)
- `"decompose"` -- Decomposes compound words into smaller units

```python
builder.set_mode("normal")
```

#### `set_dictionary(path)`

Sets the system dictionary path or URI.

```python
# Use an embedded dictionary
builder.set_dictionary("embedded://ipadic")

# Use an external dictionary
builder.set_dictionary("/path/to/dictionary")
```

#### `set_user_dictionary(uri)`

Sets the user dictionary URI.

```python
builder.set_user_dictionary("/path/to/user_dictionary")
```

#### `set_keep_whitespace(keep)`

Controls whether whitespace tokens appear in the output.

```python
builder.set_keep_whitespace(True)
```

#### `append_character_filter(kind, args=None)`

Appends a character filter to the preprocessing pipeline.

```python
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})
```

#### `append_token_filter(kind, args=None)`

Appends a token filter to the postprocessing pipeline.

```python
builder.append_token_filter("lowercase", {})
```

### Build

#### `build()`

Builds and returns a `Tokenizer` with the configured settings.

```python
tokenizer = builder.build()
```

## Tokenizer

`Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `Tokenizer(dictionary, mode="normal", user_dictionary=None)`

Creates a tokenizer directly from a loaded dictionary.

```python
from lindera import Tokenizer, load_dictionary

dictionary = load_dictionary("embedded://ipadic")
tokenizer = Tokenizer(dictionary, mode="normal")
```

### Tokenizer Methods

#### `tokenize(text)`

Tokenizes the input text and returns a list of `Token` objects.

```python
tokens = tokenizer.tokenize("形態素解析")
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `str` | Text to tokenize |

**Returns:** `list[Token]`

#### `tokenize_nbest(text, n, unique=False, cost_threshold=None)`

Returns the N-best tokenization results, each paired with its total path cost.

```python
results = tokenizer.tokenize_nbest("すもももももももものうち", n=3)
for tokens, cost in results:
    print(cost, [t.surface for t in tokens])
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `str` | Text to tokenize |
| `n` | `int` | Number of results to return |
| `unique` | `bool` | Deduplicate results (default: `False`) |
| `cost_threshold` | `int` or `None` | Maximum cost difference from the best path (default: `None`) |

**Returns:** `list[tuple[list[Token], int]]`

## Token

`Token` represents a single morphological token.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `surface` | `str` | Surface form of the token |
| `byte_start` | `int` | Start byte position in the original text |
| `byte_end` | `int` | End byte position in the original text |
| `position` | `int` | Token position index |
| `word_id` | `int` | Dictionary word ID |
| `is_unknown` | `bool` | `True` if the word is not in the dictionary |
| `details` | `list[str]` or `None` | Morphological details (part of speech, reading, etc.) |

### Token Methods

#### `get_detail(index)`

Returns the detail string at the specified index, or `None` if the index is out of range.

```python
token = tokenizer.tokenize("東京")[0]
pos = token.get_detail(0)        # e.g., "名詞"
subpos = token.get_detail(1)     # e.g., "固有名詞"
reading = token.get_detail(7)    # e.g., "トウキョウ"
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `index` | `int` | Zero-based index into the details list |

**Returns:** `str` or `None`

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats
