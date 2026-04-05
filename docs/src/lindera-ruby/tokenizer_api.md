# Tokenizer API

## TokenizerBuilder

`Lindera::TokenizerBuilder` configures and constructs a `Tokenizer` instance using the builder pattern.

### Constructors

#### `Lindera::TokenizerBuilder.new`

Creates a new builder with default configuration.

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
```

#### `Lindera::TokenizerBuilder.new.from_file(file_path)`

Loads configuration from a JSON file and returns a new builder.

```ruby
builder = Lindera::TokenizerBuilder.new.from_file('config.json')
```

### Configuration Methods

#### `set_mode(mode)`

Sets the tokenization mode.

- `"normal"` -- Standard tokenization (default)
- `"decompose"` -- Decomposes compound words into smaller units

```ruby
builder.set_mode('normal')
```

#### `set_dictionary(path)`

Sets the system dictionary path or URI.

```ruby
# Use an embedded dictionary
builder.set_dictionary('embedded://ipadic')

# Use an external dictionary
builder.set_dictionary('/path/to/dictionary')
```

#### `set_user_dictionary(uri)`

Sets the user dictionary URI.

```ruby
builder.set_user_dictionary('/path/to/user_dictionary')
```

#### `set_keep_whitespace(keep)`

Controls whether whitespace tokens appear in the output.

```ruby
builder.set_keep_whitespace(true)
```

#### `append_character_filter(kind, args)`

Appends a character filter to the preprocessing pipeline. The `args` parameter is a hash with string keys.

```ruby
builder.append_character_filter('unicode_normalize', { 'kind' => 'nfkc' })
```

#### `append_token_filter(kind, args)`

Appends a token filter to the postprocessing pipeline. The `args` parameter is a hash with string keys, or `nil` if the filter requires no arguments.

```ruby
builder.append_token_filter('lowercase', nil)
```

### Build

#### `build`

Builds and returns a `Tokenizer` with the configured settings.

```ruby
tokenizer = builder.build
```

## Tokenizer

`Lindera::Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `Lindera::Tokenizer.new(dictionary, mode, user_dictionary)`

Creates a tokenizer directly from a loaded dictionary.

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', nil)
```

With a user dictionary:

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dict)
```

### Tokenizer Methods

#### `tokenize(text)`

Tokenizes the input text and returns an array of `Token` objects.

```ruby
tokens = tokenizer.tokenize('形態素解析')
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `String` | Text to tokenize |

**Returns:** `Array<Token>`

#### `tokenize_nbest(text, n, unique, cost_threshold)`

Returns the N-best tokenization results, each paired with its total path cost.

```ruby
results = tokenizer.tokenize_nbest('すもももももももものうち', 3, false, nil)
results.each do |tokens, cost|
  puts "#{cost}: #{tokens.map(&:surface).inspect}"
end
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `String` | Text to tokenize |
| `n` | `Integer` | Number of results to return |
| `unique` | `Boolean` or `nil` | Deduplicate results (default: `false`) |
| `cost_threshold` | `Integer` or `nil` | Maximum cost difference from the best path (default: `nil`) |

**Returns:** `Array<Array(Array<Token>, Integer)>`

## Token

`Token` represents a single morphological token.

### Properties

| Property | Type | Description |
| --- | --- | --- |
| `surface` | `String` | Surface form of the token |
| `byte_start` | `Integer` | Start byte position in the original text |
| `byte_end` | `Integer` | End byte position in the original text |
| `position` | `Integer` | Token position index |
| `word_id` | `Integer` | Dictionary word ID |
| `is_unknown` | `Boolean` | `true` if the word is not in the dictionary |
| `details` | `Array<String>` or `nil` | Morphological details (part of speech, reading, etc.) |

### Token Methods

#### `get_detail(index)`

Returns the detail string at the specified index, or `nil` if the index is out of range.

```ruby
token = tokenizer.tokenize('東京')[0]
pos = token.get_detail(0)        # e.g., "名詞"
subpos = token.get_detail(1)     # e.g., "固有名詞"
reading = token.get_detail(7)    # e.g., "トウキョウ"
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `index` | `Integer` | Zero-based index into the details array |

**Returns:** `String` or `nil`

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats
