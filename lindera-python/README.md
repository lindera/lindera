# lindera-python

Python binding for [Lindera](https://github.com/lindera/lindera), a Japanese morphological analysis engine.

## Overview

lindera-python provides a comprehensive Python interface to the Lindera 1.1.1 morphological analysis engine, supporting Japanese, Korean, and Chinese text analysis. This implementation includes all major features:

- **Multi-language Support**: Japanese (IPADIC, UniDic), Korean (ko-dic), Chinese (CC-CEDICT)
- **Character Filters**: Text preprocessing with mapping, regex, Unicode normalization, and Japanese iteration mark handling
- **Token Filters**: Post-processing filters including lowercase, length filtering, stop words, and Japanese-specific filters
- **Flexible Configuration**: Configurable tokenization modes and penalty settings
- **Metadata Support**: Complete dictionary schema and metadata management

## Features

### Core Components

- **TokenizerBuilder**: Fluent API for building customized tokenizers
- **Tokenizer**: High-performance text tokenization with integrated filtering
- **CharacterFilter**: Pre-processing filters for text normalization
- **TokenFilter**: Post-processing filters for token refinement
- **Metadata & Schema**: Dictionary structure and configuration management
- **Training & Export** (optional): Train custom morphological analysis models from corpus data

### Supported Dictionaries

- **Japanese**: IPADIC (embedded), UniDic (embedded)
- **Korean**: ko-dic (embedded)
- **Chinese**: CC-CEDICT (embedded)
- **Custom**: User dictionary support

### Filter Types

**Character Filters:**

- Mapping filter (character replacement)
- Regex filter (pattern-based replacement)
- Unicode normalization (NFKC, etc.)
- Japanese iteration mark normalization

**Token Filters:**

- Text case transformation (lowercase, uppercase)
- Length filtering (min/max character length)
- Stop words filtering
- Japanese-specific filters (base form, reading form, etc.)
- Korean-specific filters

## Install project dependencies

- pyenv : <https://github.com/pyenv/pyenv?tab=readme-ov-file#installation>
- Poetry : <https://python-poetry.org/docs/#installation>
- Rust : <https://www.rust-lang.org/tools/install>

## Install Python

```shell
# Install Python
% pyenv install 3.13.5
```

## Setup repository and activate virtual environment

```shell
# Clone lindera-python project repository
% git clone git@github.com:lindera/lindera-python.git
% cd lindera-python

# Set Python version for this project
% pyenv local 3.13.5

# Make Python virtual environment
% python -m venv .venv

# Activate Python virtual environment
% source .venv/bin/activate

# Initialize lindera-python project
(.venv) % make init
```

## Install lindera-python as a library in the virtual environment

This command takes a long time because it builds a library that includes all the dictionaries.

```shell
(.venv) % make develop
```

## Quick Start

### Basic Tokenization

```python
from lindera import TokenizerBuilder

# Create a tokenizer with default settings
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")
tokenizer = builder.build()

# Tokenize Japanese text
text = "すもももももももものうち"
tokens = tokenizer.tokenize(text)

for token in tokens:
    print(f"Text: {token.text}, Position: {token.position}")
```

### Using Character Filters

```python
from lindera import TokenizerBuilder

# Create tokenizer builder
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")

# Add character filters
builder.append_character_filter("mapping", {"mapping": {"ー": "-"}})
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})

# Build tokenizer with filters
tokenizer = builder.build()
text = "テストー１２３"
tokens = tokenizer.tokenize(text)  # Will apply filters automatically
```

### Using Token Filters

```python
from lindera import TokenizerBuilder

# Create tokenizer builder
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")

# Add token filters
builder.append_token_filter("lowercase")
builder.append_token_filter("length", {"min": 2, "max": 10})
builder.append_token_filter("japanese_stop_tags", {"tags": ["助詞", "助動詞"]})

# Build tokenizer with filters
tokenizer = builder.build()
tokens = tokenizer.tokenize("テキストの解析")
```

### Integrated Pipeline

```python
from lindera import TokenizerBuilder

# Build tokenizer with integrated filters
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")

# Add character filters
builder.append_character_filter("mapping", {"mapping": {"ー": "-"}})
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})

# Add token filters  
builder.append_token_filter("lowercase")
builder.append_token_filter("japanese_base_form")

# Build and use
tokenizer = builder.build()
tokens = tokenizer.tokenize("コーヒーショップ")
```

### Working with Metadata

```python
from lindera import Metadata

# Get metadata for a specific dictionary
metadata = Metadata.load("embedded://ipadic")
print(f"Dictionary: {metadata.dictionary_name}")
print(f"Version: {metadata.dictionary_version}")

# Access schema information
schema = metadata.dictionary_schema
print(f"Schema has {len(schema.fields)} fields")
print(f"Fields: {schema.fields[:5]}")  # First 5 fields
```

## Advanced Usage

### Filter Configuration Examples

Character filters and token filters accept configuration as dictionary arguments:

```python
from lindera import TokenizerBuilder

builder = TokenizerBuilder()
builder.set_dictionary("embedded://ipadic")

# Character filters with dict configuration
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})
builder.append_character_filter("japanese_iteration_mark", {
    "normalize_kanji": "true",
    "normalize_kana": "true"
})
builder.append_character_filter("mapping", {
    "mapping": {"リンデラ": "lindera", "トウキョウ": "東京"}
})

# Token filters with dict configuration  
builder.append_token_filter("japanese_katakana_stem", {"min": 3})
builder.append_token_filter("length", {"min": 2, "max": 10})
builder.append_token_filter("japanese_stop_tags", {
    "tags": ["助詞", "助動詞", "記号"]
})

# Filters without configuration can omit the dict
builder.append_token_filter("lowercase")
builder.append_token_filter("japanese_base_form")

tokenizer = builder.build()
```

See `examples/` directory for comprehensive examples including:

- `tokenize.py`: Basic tokenization
- `tokenize_with_filters.py`: Using character and token filters
- `tokenize_with_userdict.py`: Custom user dictionary
- `train_and_export.py`: Train and export custom dictionaries (requires `train` feature)
- Multi-language tokenization
- Advanced configuration options

## Dictionary Support

### Japanese

- **IPADIC**: Default Japanese dictionary, good for general text
- **UniDic**: Academic dictionary with detailed morphological information

### Korean  

- **ko-dic**: Standard Korean dictionary for morphological analysis

### Chinese

- **CC-CEDICT**: Community-maintained Chinese-English dictionary

### Custom Dictionaries

- User dictionary support for domain-specific terms
- CSV format for easy customization

## Dictionary Training (Experimental)

lindera-python supports training custom morphological analysis models from annotated corpus data when built with the `train` feature.

### Building with Training Support

```shell
# Install with training support
(.venv) % maturin develop --features train
```

### Training a Model

```python
import lindera

# Train a model from corpus
lindera.train(
    seed="path/to/seed.csv",           # Seed lexicon
    corpus="path/to/corpus.txt",       # Training corpus
    char_def="path/to/char.def",       # Character definitions
    unk_def="path/to/unk.def",         # Unknown word definitions
    feature_def="path/to/feature.def", # Feature templates
    rewrite_def="path/to/rewrite.def", # Rewrite rules
    output="model.dat",                # Output model file
    lambda_=0.01,                      # L1 regularization
    max_iter=100,                      # Max iterations
    max_threads=None                   # Auto-detect CPU cores
)
```

### Exporting Dictionary Files

```python
# Export trained model to dictionary files
lindera.export(
    model="model.dat",              # Trained model
    output="exported_dict/",        # Output directory
    metadata="metadata.json"        # Optional metadata file
)
```

This will create:

- `lex.csv`: Lexicon file
- `matrix.def`: Connection cost matrix
- `unk.def`: Unknown word definitions
- `char.def`: Character definitions
- `metadata.json`: Dictionary metadata (if provided)

See `examples/train_and_export.py` for a complete example.

## API Reference

### Core Classes

- `TokenizerBuilder`: Fluent builder for tokenizer configuration
- `Tokenizer`: Main tokenization engine
- `Token`: Individual token with text, position, and linguistic features
- `CharacterFilter`: Text preprocessing filters
- `TokenFilter`: Token post-processing filters
- `Metadata`: Dictionary metadata and configuration
- `Schema`: Dictionary schema definition

### Training Functions (requires `train` feature)

- `train()`: Train a morphological analysis model from corpus
- `export()`: Export trained model to dictionary files

See the `test_basic.py` file for comprehensive API usage examples.
