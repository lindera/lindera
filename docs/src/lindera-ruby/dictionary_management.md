# Dictionary Management

Lindera Ruby provides functions for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `Lindera.load_dictionary(uri)` to load a system dictionary. Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
```

**Embedded dictionaries (advanced)** -- if you built with an `embed-*` feature flag, you can load an embedded dictionary:

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
```

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary. Load
one for the system dictionary it is used with:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
user_dict = Lindera.load_user_dictionary_for('/path/to/user_dictionary.csv', dictionary)
```

A `.csv` is built with the system dictionary's metadata and checked against
its connection matrix as it is built: a row whose context ID lies outside the
matrix raises a `RuntimeError` that names its CSV line and surface (see
[Context IDs](../concepts/user_dictionary.md#context-ids)). A `.bin` is loaded
as it was built.

`Lindera.load_user_dictionary(uri, metadata)` loads a user dictionary with
metadata only. A `.csv` loaded this way is checked when it is combined with
the system dictionary, and the error names the entry by its word id:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary.csv', metadata)
```

Pass the user dictionary when building a tokenizer:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
user_dict = Lindera.load_user_dictionary_for('/path/to/user_dictionary.csv', dictionary)

tokenizer = Lindera::Tokenizer.new(dictionary, Lindera::Mode.normal, user_dict)
```

Or via the builder:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/path/to/ipadic')
builder.set_user_dictionary('/path/to/user_dictionary.csv')
tokenizer = builder.build
```

## Building Dictionaries

### System Dictionary

Build a system dictionary from source files:

```ruby
require 'lindera'

metadata = Lindera::Metadata.from_json_file('metadata.json')
Lindera.build_dictionary('/path/to/input_dir', '/path/to/output_dir', metadata)
```

The input directory should contain the dictionary source files (CSV lexicon, matrix.def, etc.).

Metadata loaded with `Lindera::Metadata.from_json_file` keeps the `space_penalty` rules of the `metadata.json`, and the built dictionary ships them. For ko-dic this means the left-space penalty stays on by default in the dictionary you build; turn it off per tokenizer with `TokenizerBuilder#set_space_penalty(false)` (see [Tokenizer API](./tokenizer_api.md#set_space_penaltyvalue)).

Metadata loaded with `Lindera::Metadata.from_json_file` also keeps `connection_id_mapping`, which the `metadata.json` of ko-dic and UniDic enables. The build then relabels the context IDs of the connection matrix by access frequency, as `lindera build` does, to improve the cache locality of connection-cost lookups; the results of tokenization do not change. Pass a context-ID frequency histogram as the keyword argument `context_id_freq:`, such as the `context_id_freq.txt` of the dictionary crate, to rank the IDs by their frequency in a corpus, as `lindera build --context-id-freq` does. Without one, the IDs are ranked by the count of dictionary entries, a much weaker proxy. A dictionary without `connection_id_mapping` does not read the histogram.

```ruby
metadata = Lindera::Metadata.from_json_file('/path/to/lindera-ko-dic/metadata.json')
Lindera.build_dictionary('/path/to/mecab-ko-dic', '/path/to/output_dir', metadata,
                         context_id_freq: '/path/to/lindera-ko-dic/context_id_freq.txt')
```

### User Dictionary

Build a user dictionary from a CSV file for the system dictionary it is used
with:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
Lindera.build_user_dictionary_for('user_words.csv', '/path/to/user_dictionary.bin', dictionary)
user_dict = Lindera.load_user_dictionary_for('/path/to/user_dictionary.bin', dictionary)
```

The CSV is built with the system dictionary's metadata and checked against
its connection matrix: a row whose context ID lies outside the matrix raises
a `RuntimeError` that names its CSV line and surface, and no file is written
(see [Context IDs](../concepts/user_dictionary.md#context-ids)). The second
argument is the path of the file to write. End it with `.bin`, since the
loaders choose how to load a user dictionary by its extension.

`Lindera.build_user_dictionary(kind, input_file, output_dir, metadata)` builds
a user dictionary with metadata only. Despite its name, `output_dir` is the
path of the file to write too, not a directory. The rows are checked when the
user dictionary is combined with the system dictionary, and the error names
the entry by its word id:

```ruby
require 'lindera'

metadata = Lindera::Metadata.from_json_file('metadata.json')
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/user_dictionary.bin', metadata)
```

The `metadata` parameter is optional. When omitted, default metadata values are used:

```ruby
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/user_dictionary.bin', nil)
```

> [!NOTE]
> The first argument (`kind`, `'ipadic'` above) is currently unused -- it is reserved for
> future use and has no effect on the build. Any string value can be passed for it today.
> To build for a system dictionary, use `Lindera.build_user_dictionary_for`.

## Metadata

The `Lindera::Metadata` class configures dictionary parameters.

### Creating Metadata

```ruby
require 'lindera'

# Create default metadata with standard settings
metadata = Lindera::Metadata.create_default
```

`Lindera::Metadata.new` takes all eight properties as required positional
arguments (each may be `nil` to fall back to its default) -- use it only when
you need to override specific values:

```ruby
metadata = Lindera::Metadata.new(
  'my_dict', # name
  'UTF-8',   # encoding
  -10_000,   # default_word_cost
  1288,      # default_left_context_id
  1288,      # default_right_context_id
  '*',       # default_field_value
  false,     # flexible_csv
  false      # skip_invalid_cost_or_id
)
```

### Loading from JSON

```ruby
metadata = Lindera::Metadata.from_json_file('metadata.json')
```

### Getting Metadata from a Dictionary

A loaded dictionary's metadata can be retrieved directly:

```ruby
dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
```

### Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `String` | `"default"` | Dictionary name |
| `encoding` | `String` | `"UTF-8"` | Character encoding |
| `default_word_cost` | `Integer` | `-10000` | Default cost for unknown words |
| `default_left_context_id` | `Integer` | `1288` | Default left context ID |
| `default_right_context_id` | `Integer` | `1288` | Default right context ID |
| `default_field_value` | `String` | `"*"` | Default value for missing fields |
| `flexible_csv` | `Boolean` | `false` | Allow flexible CSV parsing |
| `skip_invalid_cost_or_id` | `Boolean` | `false` | Skip entries with invalid cost or ID |

`to_h` (and its alias `to_hash`) returns these properties as a `Hash` of strings, together with `dictionary_schema_fields` and `user_dictionary_schema_fields` (comma-separated field names). When the metadata carries left-space penalty rules, as ko-dic's does, the hash also has a `space_penalty` entry holding them as a JSON string:

```ruby
metadata = Lindera::Metadata.from_json_file('/path/to/ko-dic/metadata.json')
metadata.to_h['space_penalty']
# '{"rules":[{"pos":["EC","EF",...],"cost":3000},{"pos":["JC","JKB",...],"cost":6000}]}'
```
