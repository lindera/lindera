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

Loads configuration from a YAML file and returns a new builder. See
[`lindera-python/resources/lindera.yml`](https://github.com/lindera/lindera/blob/main/lindera-python/resources/lindera.yml)
for a complete example covering `segmenter`, `character_filters`, and `token_filters`.

```python
builder = TokenizerBuilder().from_file("lindera.yml")
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

## Mode

`Mode` represents a tokenization mode. It is provided as a standalone helper for
inspecting or comparing modes; `TokenizerBuilder.set_mode()` and the `Tokenizer`
constructor currently accept only a plain mode string (`"normal"` or
`"decompose"`), not a `Mode` instance (see the limitation noted under `Penalty`
below).

### Creating a Mode

#### `Mode(mode_str=None)`

Creates a `Mode`. Accepts `"normal"` / `"Normal"` (the default when omitted) or
`"decompose"` / `"Decompose"`; any other value raises `ValueError`.

```python
from lindera import Mode

mode = Mode("normal")
mode = Mode("decompose")
mode = Mode()  # defaults to "normal"
```

### Methods

| Method | Returns | Description |
| --- | --- | --- |
| `__str__()` | `str` | `"normal"` or `"decompose"` |
| `__repr__()` | `str` | e.g. `"Mode.Normal"` |
| `is_normal()` | `bool` | `True` if the mode is `"normal"` |
| `is_decompose()` | `bool` | `True` if the mode is `"decompose"` |

```python
mode = Mode("decompose")
str(mode)            # "decompose"
repr(mode)           # "Mode.Decompose"
mode.is_normal()      # False
mode.is_decompose()   # True
```

## Penalty

`Penalty` configures the length-based penalty thresholds used by `"decompose"`
mode segmentation.

### Creating a Penalty

#### `Penalty(kanji_penalty_length_threshold=2, kanji_penalty_length_penalty=3000, other_penalty_length_threshold=7, other_penalty_length_penalty=1700)`

All arguments are optional and default to the values shown above.

```python
from lindera import Penalty

penalty = Penalty(
    kanji_penalty_length_threshold=2,
    kanji_penalty_length_penalty=3000,
    other_penalty_length_threshold=7,
    other_penalty_length_penalty=1700,
)
```

### Penalty Properties

All four fields support both getting and setting:

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `int` | `2` | Kanji-only surface length above which the penalty applies |
| `kanji_penalty_length_penalty` | `int` | `3000` | Cost penalty added for kanji-only surfaces longer than the threshold |
| `other_penalty_length_threshold` | `int` | `7` | Surface length above which the penalty applies for non-kanji-only surfaces |
| `other_penalty_length_penalty` | `int` | `1700` | Cost penalty added for non-kanji-only surfaces longer than the threshold |

```python
penalty.kanji_penalty_length_threshold = 3
print(penalty.kanji_penalty_length_threshold)  # 3
```

**Current limitation:** there is currently no way to pass a `Penalty` into a
`Tokenizer` or `TokenizerBuilder`. `set_mode()` and the `Tokenizer` constructor
only accept a plain mode string, and internally `"decompose"` mode always uses
`Penalty`'s default values -- constructing a custom `Penalty` instance has no
effect on tokenization yet.

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

## Error Handling

Lindera Python functions raise standard Python exceptions rather than a custom
exception type:

- `IOError` (an alias of `OSError`) -- for I/O-related failures, such as a
  missing or unreadable file
- `ValueError` -- for everything else, such as invalid configuration, parse
  errors, or tokenization failures

```python
from lindera import load_dictionary

try:
    dictionary = load_dictionary("/path/that/does/not/exist")
except ValueError as e:
    print(f"Failed to load dictionary: {e}")
```

A `LinderaError` class is also registered as `lindera.LinderaError`, but no
function in this crate currently raises it -- it can only be constructed and
raised manually. Catch `IOError`/`ValueError` (or the general `Exception`) when
handling errors from this library, not `LinderaError`.
