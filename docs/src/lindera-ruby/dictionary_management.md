# Dictionary Management

Lindera Ruby provides functions for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `Lindera.load_dictionary(uri)` to load a system dictionary.

**Embedded dictionaries** (requires the corresponding `embed-*` feature):

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
```

**External dictionaries** (loaded from a directory on disk):

```ruby
dictionary = Lindera.load_dictionary('/path/to/dictionary')
```

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary.

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)
```

Pass the user dictionary when building a tokenizer:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)

tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dict)
```

Or via the builder:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.set_user_dictionary('/path/to/user_dictionary')
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

### User Dictionary

Build a user dictionary from a CSV file:

```ruby
require 'lindera'

metadata = Lindera::Metadata.from_json_file('metadata.json')
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/output_dir', metadata)
```

The `metadata` parameter is optional. When omitted, default metadata values are used:

```ruby
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/output_dir', nil)
```

## Metadata

The `Lindera::Metadata` class configures dictionary parameters.

### Creating Metadata

```ruby
require 'lindera'

# Default metadata
metadata = Lindera::Metadata.new

# Create default metadata with standard settings
metadata = Lindera::Metadata.create_default
```

### Loading from JSON

```ruby
metadata = Lindera::Metadata.from_json_file('metadata.json')
```

### Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `String` | `"default"` | Dictionary name |
| `encoding` | `String` | `"UTF-8"` | Character encoding |
| `compress_algorithm` | `String` | `"deflate"` | Compression algorithm |
| `default_word_cost` | `Integer` | `-10000` | Default cost for unknown words |
| `default_left_context_id` | `Integer` | `1288` | Default left context ID |
| `default_right_context_id` | `Integer` | `1288` | Default right context ID |
| `default_field_value` | `String` | `"*"` | Default value for missing fields |
| `flexible_csv` | `Boolean` | `false` | Allow flexible CSV parsing |
| `skip_invalid_cost_or_id` | `Boolean` | `false` | Skip entries with invalid cost or ID |
| `normalize_details` | `Boolean` | `false` | Normalize morphological details |
