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

Sets the tokenization mode. `mode` is a [`Mode`](#mode):

- `Mode.normal()` -- Standard tokenization (default)
- `Mode.decompose(penalty=None)` -- Decomposes compound words into smaller units, with the length penalties of a [`Penalty`](#penalty); `None` uses the default penalty

A value that is not a `Mode`, such as the string `"normal"`, raises `TypeError`. To set a mode from its name, such as a configuration value, create it with [`Mode.from_str()`](#modefrom_strname).

```python
from lindera import Mode, Penalty

builder.set_mode(Mode.normal())
builder.set_mode(Mode.decompose(Penalty(kanji_penalty_length_penalty=0)))
builder.set_mode(Mode.from_str("decompose"))
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
builder.set_user_dictionary("/path/to/user_dictionary.csv")
```

#### `set_keep_whitespace(keep)`

Controls whether whitespace tokens appear in the output. Kept whitespace also stays in the lattice, so the other tokens can differ from those of the default output, which skips whitespace in the lattice as MeCab does. To keep whitespace in the lattice without outputting it, use [`set_skip_whitespace(False)`](#set_skip_whitespaceskip) instead.

```python
builder.set_keep_whitespace(True)
```

#### `set_skip_whitespace(skip)`

Sets whether whitespace is skipped in the lattice. `skip` has the same meaning as `segmenter.skip_whitespace` in a [configuration file](../lindera-analysis/configuration.md):

- `True` -- Skips whitespace, so the words on either side of it connect directly, as MeCab does.
- `False` -- Keeps whitespace in the lattice as a node of its own (a whitespace dictionary entry or the `SPACE` unknown word), as before v7. Whitespace is still dropped from the output.
- `None` -- The default of the dictionary's `metadata.json`: skipping, except with SudachiDict. Use it to undo an earlier call.

A value set here overrides the dictionary's default and a `skip_whitespace` loaded with `from_file`. It has no effect while whitespace tokens are kept (`set_keep_whitespace(True)`), because kept whitespace always stays in the lattice. Any other value raises `TypeError`.

With IPADIC, for example, `Google が 新しい` reads `が` as a particle (`助詞`) by default and as a conjunction (`接続詞`) with `set_skip_whitespace(False)`. See [Segmenter](../lindera/segmenter.md#whitespace-handling) for details.

```python
# Keep whitespace in the lattice, as before v7
builder.set_skip_whitespace(False)
```

#### `set_space_penalty(value)`

Sets the left-space penalty for Korean: a candidate that starts right after whitespace and whose part-of-speech tag normally attaches to the preceding word (particles, endings, ...) gets a cost added. `value` has the same meaning as `segmenter.space_penalty` in a [configuration file](../lindera-analysis/configuration.md):

- `None` -- The default: the rules the dictionary ships in its `metadata.json`, if any (ko-dic ships mecab-ko-dic's rules; the other bundled dictionaries ship none). Use it to undo an earlier call.
- `False` -- Turns the penalty off.
- `True` -- Requires the dictionary's rules; `build()` raises `ValueError` for a dictionary that ships none.
- A `dict` of the form `{"rules": [{"pos": [...], "cost": n}, ...]}` -- Applies these rules instead. A candidate whose first part-of-speech tag is listed in `pos` gets `cost` added; the first matching rule wins.

Any other value raises `ValueError` (a value with no JSON equivalent, such as an arbitrary object, raises `TypeError`, as in `append_token_filter`).

```python
# Turn the penalty off
builder.set_space_penalty(False)

# Explicit rules
builder.set_space_penalty({
    "rules": [
        {"pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000},
        {"pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000},
    ]
})
```

> [!NOTE]
> Since v6.1.0, ko-dic applies the left-space penalty by default, as mecab-ko does. For example, `검색 이 잘 된다` reads `이` as the interjection `IC`; without the penalty, as the subject particle `JKS`. Call `builder.set_space_penalty(False)` to turn it off. Only a ko-dic built by Lindera 6.1.0 or later ships the rules; with a ko-dic from the v6.0.0 release the penalty stays off and `True` fails until the dictionary is rebuilt. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

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

The arguments of `set_space_penalty`, `append_character_filter` and `append_token_filter` may nest lists and dicts at most 128 levels deep. A deeper value, or one that contains itself, raises `ValueError`.

### Build

#### `build()`

Builds and returns a `Tokenizer` with the configured settings.

```python
tokenizer = builder.build()
```

## Tokenizer

`Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `Tokenizer(dictionary, mode=None, user_dictionary=None, space_penalty=None)`

Creates a tokenizer directly from a loaded dictionary.

`mode` takes a [`Mode`](#mode), as [`TokenizerBuilder.set_mode`](#set_modemode) does; `None` (the default) is normal mode. Any other value, such as the string `"normal"`, raises `TypeError`.

`space_penalty` takes the same values as [`TokenizerBuilder.set_space_penalty`](#set_space_penaltyvalue). `None` (the default) uses the left-space penalty rules the dictionary ships, so with ko-dic the penalty is on; pass `False` to turn it off. An invalid setting raises the same errors as `set_space_penalty` and `build()`.

```python
from lindera import Mode, Tokenizer, load_dictionary

dictionary = load_dictionary("embedded://ipadic")
tokenizer = Tokenizer(dictionary, mode=Mode.normal())

# ko-dic without the left-space penalty
ko_tokenizer = Tokenizer(load_dictionary("embedded://ko-dic"), space_penalty=False)
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

#### `tokenize_surfaces(text)`

Tokenizes the input text and returns only the token surfaces, as a list of strings. This is the fast path for wakati-style use: no `Token` objects are created and no morphological details are loaded, so it is significantly faster than `tokenize` when only the surface strings are needed. The result equals `[t.surface for t in tokenizer.tokenize(text)]`.

```python
surfaces = tokenizer.tokenize_surfaces("形態素解析")
# ["形態素", "解析"]
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `str` | Text to tokenize |

**Returns:** `list[str]`

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

`Mode` is the tokenization mode that [`TokenizerBuilder.set_mode()`](#set_modemode) and the [`Tokenizer` constructor](#tokenizerdictionary-modenone-user_dictionarynone-space_penaltynone) take. It is the counterpart of the Rust `lindera::mode::Mode`: normal mode, or decompose mode with its [`Penalty`](#penalty).

### Creating a Mode

`Mode` has no constructor, so `Mode()` raises `TypeError`, and it has no `Mode.Normal` or `Mode.Decompose` attributes. Create a mode with one of the following static methods.

#### `Mode.normal()`

Returns normal mode: standard tokenization.

#### `Mode.decompose(penalty=None)`

Returns decompose mode, which adds the length penalties of `penalty` to the cost of long words, so that compound words are decomposed into smaller units. `None` (the default) uses the default penalty, `Penalty()`. A value that is not a `Penalty` raises `TypeError`.

#### `Mode.from_str(name)`

Returns the mode named `name`, as the Rust `Mode` parses a name: `"normal"` for normal mode, and `"decompose"` for decompose mode with the default penalty. Use it for a mode name read at run time, such as a configuration value. The name is case sensitive: any other name, such as `"Normal"`, raises `ValueError` with the message `Invalid mode`. A value that is not a `str` raises `TypeError`.

```python
from lindera import Mode, Penalty

normal = Mode.normal()
decompose = Mode.decompose()
whole_words = Mode.decompose(Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0))
from_name = Mode.from_str("decompose")  # equal to Mode.decompose()
```

### Mode Properties

Both properties are read-only:

| Property | Type | Description |
| --- | --- | --- |
| `name` | `str` | `"normal"` or `"decompose"` |
| `penalty` | `Penalty` or `None` | A copy of the penalty of decompose mode; `None` for normal mode |

### Methods

| Method | Returns | Description |
| --- | --- | --- |
| `__str__()` | `str` | The name: `"normal"` or `"decompose"` |
| `__repr__()` | `str` | `"Mode.normal()"`, or `Mode.decompose(...)` with the penalty |
| `__eq__(other)` | `bool` | `True` if `other` is a `Mode` with the same name and penalty |
| `is_normal()` | `bool` | `True` for normal mode |
| `is_decompose()` | `bool` | `True` for decompose mode |

```python
mode = Mode.decompose()
mode.name              # 'decompose'
mode.penalty           # Penalty(kanji_threshold=2, kanji_penalty=3000, other_threshold=7, other_penalty=1700)
str(mode)              # 'decompose'
repr(mode)             # 'Mode.decompose(Penalty(kanji_threshold=2, kanji_penalty=3000, other_threshold=7, other_penalty=1700))'
mode.is_normal()       # False
mode.is_decompose()    # True
Mode.normal().penalty  # None
repr(Mode.normal())    # 'Mode.normal()'
```

Two modes are equal when their names and penalties are, as the Rust `Mode` compares them. A `Mode` is never equal to a value of another type, such as the string `"normal"`:

```python
Mode.from_str("decompose") == Mode.decompose()                               # True
Mode.decompose() == Mode.decompose(Penalty(kanji_penalty_length_penalty=0))  # False
Mode.normal() == "normal"                                                    # False
```

A `Mode` cannot be changed. Setting a property raises `AttributeError`, and `penalty` returns a copy, so changing the returned `Penalty` leaves the mode as it was; `Mode.decompose()` keeps a copy of the `Penalty` it is given as well. A `Mode` is not hashable, as the Rust `Mode` implements `Eq` but not `Hash`: `hash(mode)` raises `TypeError`, so a `Mode` cannot be a `dict` key or a `set` member.

```python
mode = Mode.decompose()
penalty = mode.penalty
penalty.kanji_penalty_length_penalty = 0
mode.penalty.kanji_penalty_length_penalty  # 3000
```

## Penalty

`Penalty` configures the length-based penalty thresholds used by decompose mode
segmentation.

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

### Using a Penalty

Pass a `Penalty` to [`Mode.decompose()`](#modedecomposepenaltynone), and the mode to [`TokenizerBuilder.set_mode()`](#set_modemode) or to the [`Tokenizer` constructor](#tokenizerdictionary-modenone-user_dictionarynone-space_penaltynone). Without length penalties, decompose mode keeps long words whole:

```python
from lindera import Mode, Penalty, Tokenizer, TokenizerBuilder, load_dictionary

text = "関西国際空港限定トートバッグ"
penalty = Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0)

builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
tokenizer = builder.set_mode(Mode.decompose()).build()
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西', '国際', '空港', '限定', 'トートバッグ']

tokenizer = builder.set_mode(Mode.decompose(penalty)).build()
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西国際空港', '限定', 'トートバッグ']

tokenizer = Tokenizer(load_dictionary("embedded://ipadic"), mode=Mode.decompose(penalty))
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西国際空港', '限定', 'トートバッグ']
```

Normal mode has no penalty: `Mode.normal()` takes no arguments.

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

#### `to_dict()`

Returns the token as a plain `dict`, keeping each field's natural Python type,
so it serializes without a custom encoder.

```python
import json

data = tokenizer.tokenize("東京")[0].to_dict()
# {'surface': '東京', 'byte_start': 0, 'byte_end': 6, 'position': 0,
#  'word_id': 12345, 'is_unknown': False, 'details': [...]}

json.dumps([token.to_dict() for token in tokens])
```

**Returns:** `dict` with the keys `surface`, `byte_start`, `byte_end`,
`position`, `word_id`, `is_unknown`, and `details`.

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
