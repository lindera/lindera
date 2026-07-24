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

#### `Lindera::TokenizerBuilder.from_file(file_path)`

Loads configuration from a JSON file and returns a new builder. This is a class
method, not chained off an existing instance.

```ruby
builder = Lindera::TokenizerBuilder.from_file('config.json')
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

## Mode

`Lindera::Mode` represents a tokenization mode. It is provided as a standalone helper for inspecting or comparing modes; `TokenizerBuilder#set_mode` and `Tokenizer.new` currently accept only a plain mode string (`"normal"` or `"decompose"`), not a `Mode` instance (see the limitation noted under `Penalty` below).

### Creating a Mode

#### `Lindera::Mode.new(mode_str)`

Creates a `Mode`. The argument is required, but may be `nil`. Accepts `"normal"` / `"Normal"` (used when `mode_str` is `nil`) or `"decompose"` / `"Decompose"`; any other value raises `ArgumentError`.

```ruby
require 'lindera'

mode = Lindera::Mode.new('normal')
mode = Lindera::Mode.new('decompose')
mode = Lindera::Mode.new(nil)  # defaults to "normal"
```

### Mode Methods

| Method | Returns | Description |
| --- | --- | --- |
| `to_s` | `String` | `"normal"` or `"decompose"` |
| `name` | `String` | Same as `to_s` |
| `inspect` | `String` | e.g. `"#<Lindera::Mode: decompose>"` |
| `normal?` | `Boolean` | `true` if the mode is `"normal"` |
| `decompose?` | `Boolean` | `true` if the mode is `"decompose"` |

```ruby
mode = Lindera::Mode.new('decompose')
mode.to_s        # "decompose"
mode.normal?      # false
mode.decompose?   # true
```

## Penalty

`Lindera::Penalty` configures the length-based penalty thresholds used by `"decompose"` mode segmentation.

### Creating a Penalty

#### `Lindera::Penalty.new(kanji_penalty_length_threshold, kanji_penalty_length_penalty, other_penalty_length_threshold, other_penalty_length_penalty)`

All four positional arguments are required, but each may be `nil` to fall back to its default (shown below).

```ruby
require 'lindera'

penalty = Lindera::Penalty.new(2, 3000, 7, 1700)
penalty = Lindera::Penalty.new(nil, nil, nil, nil)  # uses all defaults
```

### Penalty Properties

All properties are read-only (there are no setter methods):

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `Integer` | `2` | Kanji-only surface length above which the penalty applies |
| `kanji_penalty_length_penalty` | `Integer` | `3000` | Cost penalty added for kanji-only surfaces longer than the threshold |
| `other_penalty_length_threshold` | `Integer` | `7` | Surface length above which the penalty applies for non-kanji-only surfaces |
| `other_penalty_length_penalty` | `Integer` | `1700` | Cost penalty added for non-kanji-only surfaces longer than the threshold |

```ruby
penalty = Lindera::Penalty.new(nil, nil, nil, nil)
penalty.kanji_penalty_length_threshold  # 2
```

**Current limitation:** there is currently no way to pass a `Penalty` into a `Tokenizer` or `TokenizerBuilder`. `set_mode` and `Tokenizer.new` only accept a plain mode string, and internally `"decompose"` mode always uses `Penalty`'s default values -- constructing a custom `Penalty` instance has no effect on tokenization yet.

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
| `details` | `Array<String>` | Morphological details (part of speech, reading, etc.) |

Additionally, the predicate method `unknown?` returns `true` if the word is not in the dictionary:

```ruby
token.unknown?  # => false
```

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

## Schema

`Lindera::Schema` holds an ordered list of field names and provides lookups between field name and index. It is used by `Metadata#dictionary_schema` and `Metadata#user_dictionary_schema` (see [Dictionary Management](./dictionary_management.md)).

### Creating a Schema

#### `Lindera::Schema.new(fields)`

Creates a schema from an array of field names.

```ruby
require 'lindera'

schema = Lindera::Schema.new(%w[
  surface
  left_context_id
  right_context_id
  cost
  major_pos
  reading
])
```

#### `Lindera::Schema.create_default`

Returns the built-in default schema: 13 fields matching the IPADIC-style layout (`surface`, `left_context_id`, `right_context_id`, `cost`, `major_pos`, `pos_detail_1`, `pos_detail_2`, `pos_detail_3`, `conjugation_type`, `conjugation_form`, `base_form`, `reading`, `pronunciation`).

```ruby
schema = Lindera::Schema.create_default
```

### Schema Methods

| Method | Returns | Description |
| --- | --- | --- |
| `fields` | `Array<String>` | All field names, in order |
| `get_all_fields` | `Array<String>` | Same as `fields` |
| `field_count` | `Integer` | Total number of fields |
| `get_field_index(name)` | `Integer` or `nil` | Index of the field named `name` |
| `get_field_name(index)` | `String` or `nil` | Field name at `index` |
| `get_custom_fields` | `Array<String>` | Field names after the four fixed fields (`surface`, `left_context_id`, `right_context_id`, `cost`) |
| `get_field_by_name(name)` | `FieldDefinition` or `nil` | Full field definition for `name` |
| `validate_record(record)` | `nil` | Raises `ArgumentError` if `record` does not match the schema |
| `to_s` | `String` | e.g. `"Schema(fields=13)"` |
| `inspect` | `String` | Full field list |

```ruby
schema = Lindera::Schema.create_default

schema.field_count                  # 13
schema.get_field_index('cost')      # 3
schema.get_field_name(0)            # "surface"
schema.get_custom_fields            # ["major_pos", "pos_detail_1", ..., "pronunciation"]

field = schema.get_field_by_name('surface')
puts "#{field.index} #{field.name} #{field.field_type}"  # 0 surface surface

schema.validate_record([
  '東京', '1288', '1288', '100',
  '名詞', '固有名詞', '地域', '一般', '*', '*',
  '東京', 'トウキョウ', 'トーキョー'
])
```

## FieldDefinition

`Lindera::FieldDefinition` describes a single field within a `Schema`. Instances are only obtained from `Schema#get_field_by_name` -- there is no public constructor (`Lindera::FieldDefinition.new` raises `TypeError`).

### FieldDefinition Properties

| Property | Type | Description |
| --- | --- | --- |
| `index` | `Integer` | Zero-based position of the field within the schema |
| `name` | `String` | Field name |
| `field_type` | `FieldType` | Field type |
| `description` | `String` or `nil` | Optional human-readable description |

```ruby
schema = Lindera::Schema.create_default
field = schema.get_field_by_name('surface')

field.index         # 0
field.name          # "surface"
field.field_type    # #<Lindera::FieldType: surface>
field.description    # nil (the default schema does not set descriptions)
```

## FieldType

`Lindera::FieldType` enumerates the category of a single field. Like `FieldDefinition`, instances are only obtained from a `Schema` (via `FieldDefinition#field_type`) -- there is no public constructor.

`to_s` (and `inspect`) return one of:

- `"surface"` -- surface form (word text)
- `"left_context_id"` -- left context ID
- `"right_context_id"` -- right context ID
- `"cost"` -- word cost
- `"custom"` -- any other, dictionary-specific field

```ruby
field = Lindera::Schema.create_default.get_field_by_name('surface')
field.field_type.to_s  # "surface"
```
