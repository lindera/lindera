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

Sets the tokenization mode. `mode` is a [`Lindera::Mode`](#mode):

- `Lindera::Mode.normal` -- Standard tokenization (default)
- `Lindera::Mode.decompose(penalty = nil)` -- Decomposes compound words into smaller units, with the length penalties of a [`Lindera::Penalty`](#penalty) (the default penalty for `nil`)

A value that is not a `Lindera::Mode`, such as a mode string (`'normal'`), a Symbol or `nil`, raises `TypeError`. To turn a mode name read at run time into a `Mode`, use [`Lindera::Mode.from_str`](#linderamodefrom_strname).

```ruby
builder.set_mode(Lindera::Mode.normal)
builder.set_mode(Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, nil)))
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
builder.set_user_dictionary('/path/to/user_dictionary.csv')
```

#### `set_keep_whitespace(keep)`

Controls whether whitespace tokens appear in the output. Kept whitespace also stays in the lattice, so the other tokens can differ from those of the default output, which skips whitespace in the lattice as MeCab does. To keep whitespace in the lattice without outputting it, use [`set_skip_whitespace(false)`](#set_skip_whitespaceskip) instead.

```ruby
builder.set_keep_whitespace(true)
```

#### `set_skip_whitespace(skip)`

Sets whether whitespace is skipped in the lattice. `skip` has the same meaning as `segmenter.skip_whitespace` in a [configuration file](../lindera-analysis/configuration.md):

- `true` -- Skips whitespace, so the words on either side of it connect directly, as MeCab does.
- `false` -- Keeps whitespace in the lattice as a node of its own (a whitespace dictionary entry or the `SPACE` unknown word), as before v7. Whitespace is still dropped from the output.
- `nil` -- The default of the dictionary's `metadata.json`: skipping, except with SudachiDict. Use it to undo an earlier call.

A value set here overrides the dictionary's default and a `skip_whitespace` loaded with `TokenizerBuilder.from_file`. It has no effect while whitespace tokens are kept (`set_keep_whitespace(true)`), because kept whitespace always stays in the lattice. Other values count by Ruby truthiness, as in `set_keep_whitespace`: anything but `false` and `nil` turns skipping on.

With IPADIC, for example, `Google が 新しい` reads `が` as a particle (`助詞`) by default and as a conjunction (`接続詞`) with `set_skip_whitespace(false)`. See [Segmenter](../lindera/segmenter.md#whitespace-handling) for details.

```ruby
# Keep whitespace in the lattice, as before v7
builder.set_skip_whitespace(false)
```

#### `set_space_penalty(value)`

Sets the left-space penalty for Korean: a candidate that starts right after whitespace and whose part-of-speech tag normally attaches to the preceding word (particles, endings, ...) gets a cost added. `value` has the same meaning as `segmenter.space_penalty` in a [configuration file](../lindera-analysis/configuration.md):

- `nil` -- The default: the rules the dictionary ships in its `metadata.json`, if any (ko-dic ships mecab-ko-dic's rules; the other bundled dictionaries ship none). Use it to undo an earlier call.
- `false` -- Turns the penalty off.
- `true` -- Requires the dictionary's rules; `build` raises `RuntimeError` for a dictionary that ships none.
- A hash with string keys, `{ 'rules' => [{ 'pos' => [...], 'cost' => n }, ...] }` -- Applies these rules instead. A candidate whose first part-of-speech tag is listed in `pos` gets `cost` added; the first matching rule wins.

A value that is not one of these forms, such as `1`, `'false'` or `{ 'rules' => 'JKS' }`, raises `RuntimeError`. A value that cannot be converted raises `TypeError`: a Symbol (also as a hash key or an element, so write `%w[JKS]` rather than `%i[JKS]`), a non-finite `Float` (NaN or Infinity), an `Integer` outside the 64-bit range, or an object other than `nil`, `true`, `false`, an `Integer`, a `Float`, a `String`, an `Array` or a `Hash`.

```ruby
# Turn the penalty off
builder.set_space_penalty(false)

# Explicit rules
builder.set_space_penalty({
  'rules' => [
    { 'pos' => %w[EC EF EP ETM ETN VCP XSA XSN XSV], 'cost' => 3000 },
    { 'pos' => %w[JC JKB JKC JKG JKO JKQ JKS JKV JX], 'cost' => 6000 }
  ]
})
```

> [!NOTE]
> Since v6.1.0, ko-dic applies the left-space penalty by default, as mecab-ko does. For example, `검색 이 잘 된다` reads `이` as the interjection `IC`; without the penalty, as the subject particle `JKS`. Call `builder.set_space_penalty(false)` to turn it off. A tokenizer created directly with `Lindera::Tokenizer.new(dictionary, ...)` always uses the default, so use `TokenizerBuilder` to turn it off. Only a ko-dic built by Lindera 6.1.0 or later ships the rules; with a ko-dic from the v6.0.0 release the penalty stays off and `true` fails until the dictionary is rebuilt. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

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

The arguments of `set_space_penalty`, `append_character_filter` and `append_token_filter` may nest arrays and hashes at most 128 levels deep. A deeper value, or one that contains itself, raises `ArgumentError`.

### Build

#### `build`

Builds and returns a `Tokenizer` with the configured settings.

```ruby
tokenizer = builder.build
```

## Tokenizer

`Lindera::Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `Lindera::Tokenizer.new(dictionary, mode = nil, user_dictionary = nil, space_penalty: nil)`

Creates a tokenizer directly from a loaded dictionary. `mode` is a [`Lindera::Mode`](#mode), as [`TokenizerBuilder#set_mode`](#set_modemode) takes it; `nil` (the default) means normal mode, and a value that is not a `Lindera::Mode`, such as a mode string (`'normal'`), raises `TypeError`. `user_dictionary` is optional.

The `space_penalty:` keyword takes the same values as [`TokenizerBuilder#set_space_penalty`](#set_space_penaltyvalue). `nil` (the default) uses the left-space penalty rules the dictionary ships, so with ko-dic the penalty is on; pass `false` to turn it off. An invalid setting raises the same errors as `set_space_penalty` and `build`.

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary)

# ko-dic without the left-space penalty
ko_tokenizer = Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ko-dic'), space_penalty: false)
```

With a user dictionary:

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
user_dict = Lindera.load_user_dictionary_for('/path/to/user_dictionary.csv', dictionary)
tokenizer = Lindera::Tokenizer.new(dictionary, Lindera::Mode.normal, user_dict)
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

#### `tokenize_surfaces(text)`

Tokenizes the input text and returns only the token surfaces, as an array of strings. This is the fast path for wakati-style use: no `Token` objects are created and no morphological details are loaded, so it is significantly faster than `tokenize` when only the surface strings are needed. The result equals `tokenizer.tokenize(text).map(&:surface)`.

```ruby
surfaces = tokenizer.tokenize_surfaces('形態素解析')
# ["形態素", "解析"]
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `text` | `String` | Text to tokenize |

**Returns:** `Array<String>`

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

`Lindera::Mode` is the tokenization mode that [`TokenizerBuilder#set_mode`](#set_modemode) and [`Tokenizer.new`](#linderatokenizernewdictionary-mode--nil-user_dictionary--nil-space_penalty-nil) take. It is the Ruby counterpart of the Rust `lindera::mode::Mode`: normal mode, or decompose mode with the [`Lindera::Penalty`](#penalty) it uses.

### Creating a Mode

`Lindera::Mode` has no `new` (`Lindera::Mode.new` raises `TypeError`). The following class methods create a mode.

#### `Lindera::Mode.normal`

Returns normal mode.

#### `Lindera::Mode.decompose(penalty = nil)`

Returns decompose mode with `penalty`, a `Lindera::Penalty`. `nil` or no argument uses the default penalty. A value that is not a `Lindera::Penalty` raises `TypeError`.

#### `Lindera::Mode.from_str(name)`

Returns the mode named `name`, as the Rust `Mode` parses it: `'normal'`, or `'decompose'` with the default penalty. The name is case sensitive; any other name, such as `'Normal'`, raises `RuntimeError` with an `Invalid mode` message. Use it for a mode name read at run time, such as from a setting.

```ruby
require 'lindera'

normal = Lindera::Mode.normal
decompose = Lindera::Mode.decompose
no_length_penalty = Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, 0))
from_setting = Lindera::Mode.from_str('decompose')
```

### Mode Methods

| Method | Returns | Description |
| --- | --- | --- |
| `name` | `String` | `"normal"` or `"decompose"` |
| `to_s` | `String` | Same as `name` |
| `inspect` | `String` | The name, followed by the penalty for decompose mode |
| `penalty` | `Penalty` or `nil` | A copy of the penalty of decompose mode; `nil` for normal mode |
| `normal?` | `Boolean` | `true` for normal mode |
| `decompose?` | `Boolean` | `true` for decompose mode |
| `==` | `Boolean` | `true` if the other value is a `Mode` with the same name and penalty |

A mode cannot be changed: it has no setter methods, and `penalty` returns a copy.

`==` compares the name and the penalty, and returns `false` for a value of another type, such as the string `'normal'`. `eql?` and `hash` stay by identity, as the Rust `Mode` is `Eq` but not `Hash`: two equal modes are different `Hash` keys, and `uniq` keeps both.

```ruby
mode = Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, 0))
mode.name                                   # "decompose"
mode.normal?                                # false
mode.decompose?                             # true
mode.penalty.kanji_penalty_length_penalty   # 0
mode.inspect
# "#<Lindera::Mode: decompose Penalty(kanji_threshold=2, kanji_penalty=0, other_threshold=7, other_penalty=0)>"

Lindera::Mode.normal.inspect   # "#<Lindera::Mode: normal>"
Lindera::Mode.normal.penalty   # nil

Lindera::Mode.decompose == Lindera::Mode.from_str('decompose')  # true
Lindera::Mode.decompose == mode                                 # false (the penalties differ)
Lindera::Mode.normal == 'normal'                                # false
```

## Penalty

`Lindera::Penalty` configures the length-based penalty thresholds used by decompose mode segmentation. Pass it to [`Lindera::Mode.decompose`](#linderamodedecomposepenalty--nil).

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

### Using a Penalty

Pass a `Penalty` to [`Lindera::Mode.decompose`](#linderamodedecomposepenalty--nil), and the mode to [`TokenizerBuilder#set_mode`](#set_modemode) or [`Tokenizer.new`](#linderatokenizernewdictionary-mode--nil-user_dictionary--nil-space_penalty-nil). Without length penalties, decompose mode keeps long words whole:

```ruby
require 'lindera'

text = '関西国際空港限定トートバッグ'
penalty = Lindera::Penalty.new(nil, 0, nil, 0)

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.set_mode(Lindera::Mode.decompose)
p builder.build.tokenize(text).map(&:surface)
# => ["関西", "国際", "空港", "限定", "トートバッグ"]

builder.set_mode(Lindera::Mode.decompose(penalty))
p builder.build.tokenize(text).map(&:surface)
# => ["関西国際空港", "限定", "トートバッグ"]

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary, Lindera::Mode.decompose(penalty))
p tokenizer.tokenize(text).map(&:surface)
# => ["関西国際空港", "限定", "トートバッグ"]
```

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

Returns the detail string at the specified index, or `nil` if the index is past the last detail. A negative index raises `RangeError` (see [Integer arguments](../lindera-ruby.md#integer-arguments)).

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

#### `to_h` / `to_hash`

Returns the token as a plain `Hash` keyed by Symbols, keeping each field's
natural Ruby type, so it serializes without a custom encoder. `to_h` and
`to_hash` are the same method.

```ruby
require 'json'

data = tokenizer.tokenize('東京')[0].to_h
# {surface: "東京", byte_start: 0, byte_end: 6, position: 0,
#  word_id: 12345, is_unknown: false, details: [...]}

JSON.generate(tokens.map(&:to_h))
```

**Returns:** `Hash` with the keys `:surface`, `:byte_start`, `:byte_end`,
`:position`, `:word_id`, `:is_unknown`, and `:details`.

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
