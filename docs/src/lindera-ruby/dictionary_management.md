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

User dictionaries add custom vocabulary on top of a system dictionary.

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)
```

Pass the user dictionary when building a tokenizer:

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)

tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dict)
```

Or via the builder:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/path/to/ipadic')
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
| `default_word_cost` | `Integer` | `-10000` | Default cost for unknown words |
| `default_left_context_id` | `Integer` | `1288` | Default left context ID |
| `default_right_context_id` | `Integer` | `1288` | Default right context ID |
| `default_field_value` | `String` | `"*"` | Default value for missing fields |
| `flexible_csv` | `Boolean` | `false` | Allow flexible CSV parsing |
| `skip_invalid_cost_or_id` | `Boolean` | `false` | Skip entries with invalid cost or ID |
| `normalize_details` | `Boolean` | `false` | Normalize morphological details |
