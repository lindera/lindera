# lindera-nodejs

Node.js binding for [Lindera](https://github.com/lindera/lindera), a Japanese morphological analysis engine.

## Overview

lindera-nodejs provides a comprehensive Node.js interface to the Lindera morphological analysis engine, supporting Japanese, Korean, and Chinese text analysis. This implementation includes all major features:

- **Multi-language Support**: Japanese (IPADIC, UniDic), Korean (ko-dic), Chinese (CC-CEDICT)
- **Character Filters**: Text preprocessing with mapping, regex, Unicode normalization, and Japanese iteration mark handling
- **Token Filters**: Post-processing filters including lowercase, length filtering, stop words, and Japanese-specific filters
- **Flexible Configuration**: Configurable tokenization modes and penalty settings
- **Metadata Support**: Complete dictionary schema and metadata management
- **TypeScript Support**: Full type definitions included out of the box

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

- Node.js 18+ : <https://nodejs.org/>
- Rust : <https://www.rust-lang.org/tools/install>
- @napi-rs/cli : `npm install -g @napi-rs/cli`

## Setup repository

```shell
# Clone lindera project repository
git clone git@github.com:lindera/lindera.git
cd lindera
```

## Install lindera-nodejs

This command builds the library with development settings (debug build).

```shell
cd lindera-nodejs
npm install
npm run build
```

## Quick Start

### Basic Tokenization

```javascript
const { loadDictionary, Tokenizer } = require("lindera");

// Load dictionary
const dictionary = loadDictionary("embedded://ipadic");

// Create a tokenizer
const tokenizer = new Tokenizer(dictionary, "normal");

// Tokenize Japanese text
const text = "すもももももももものうち";
const tokens = tokenizer.tokenize(text);

for (const token of tokens) {
  console.log(`Text: ${token.surface}, Position: ${token.byteStart}-${token.byteEnd}`);
}
```

### Using Character Filters

```javascript
const { TokenizerBuilder } = require("lindera");

// Create tokenizer builder
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("embedded://ipadic");

// Add character filters
builder.appendCharacterFilter("mapping", { mapping: { "ー": "-" } });
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });

// Build tokenizer with filters
const tokenizer = builder.build();
const text = "テストー１２３";
const tokens = tokenizer.tokenize(text); // Will apply filters automatically
```

### Using Token Filters

```javascript
const { TokenizerBuilder } = require("lindera");

// Create tokenizer builder
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("embedded://ipadic");

// Add token filters
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("length", { min: 2, max: 10 });
builder.appendTokenFilter("japanese_stop_tags", { tags: ["助詞", "助動詞"] });

// Build tokenizer with filters
const tokenizer = builder.build();
const tokens = tokenizer.tokenize("テキストの解析");
```

### Integrated Pipeline

```javascript
const { TokenizerBuilder } = require("lindera");

// Build tokenizer with integrated filters
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("embedded://ipadic");

// Add character filters
builder.appendCharacterFilter("mapping", { mapping: { "ー": "-" } });
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });

// Add token filters
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("japanese_base_form");

// Build and use
const tokenizer = builder.build();
const tokens = tokenizer.tokenize("コーヒーショップ");
```

### Working with Metadata

```javascript
const { Metadata } = require("lindera");

// Create metadata with default values
const metadata = new Metadata();
console.log(`Name: ${metadata.name}`);
console.log(`Encoding: ${metadata.encoding}`);

// Create metadata from a JSON file
const loaded = Metadata.fromJsonFile("metadata.json");
console.log(loaded.toObject());
```

## Advanced Usage

### Filter Configuration Examples

Character filters and token filters accept configuration as object arguments:

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");

// Character filters with object configuration
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
builder.appendCharacterFilter("japanese_iteration_mark", {
  normalize_kanji: true,
  normalize_kana: true,
});
builder.appendCharacterFilter("mapping", {
  mapping: { "リンデラ": "lindera", "トウキョウ": "東京" },
});

// Token filters with object configuration
builder.appendTokenFilter("japanese_katakana_stem", { min: 3 });
builder.appendTokenFilter("length", { min: 2, max: 10 });
builder.appendTokenFilter("japanese_stop_tags", {
  tags: ["助詞", "助動詞", "記号"],
});

// Filters without configuration can omit the object
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("japanese_base_form");

const tokenizer = builder.build();
```

See `examples/` directory for comprehensive examples including:

- `tokenize.js`: Basic tokenization
- `tokenize_with_filters.js`: Using character and token filters
- `tokenize_with_userdict.js`: Custom user dictionary
- `train_and_export.js`: Train and export custom dictionaries (requires `train` feature)
- `tokenize_with_decompose.js`: Decompose mode tokenization

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

lindera-nodejs supports training custom morphological analysis models from annotated corpus data when built with the `train` feature.

### Building with Training Support

```shell
npm run build -- --features train
```

### Training a Model

```javascript
const { train } = require("lindera");

// Train a model from corpus
train({
  seed: "path/to/seed.csv",
  corpus: "path/to/corpus.txt",
  charDef: "path/to/char.def",
  unkDef: "path/to/unk.def",
  featureDef: "path/to/feature.def",
  rewriteDef: "path/to/rewrite.def",
  output: "model.dat",
  lambda: 0.01,
  maxIter: 100,
});
```

### Exporting Dictionary Files

```javascript
const { exportModel } = require("lindera");

// Export trained model to dictionary files
exportModel({
  model: "model.dat",
  output: "exported_dict/",
  metadata: "metadata.json",
});
```

This will create:

- `lex.csv`: Lexicon file
- `matrix.def`: Connection cost matrix
- `unk.def`: Unknown word definitions
- `char.def`: Character definitions
- `metadata.json`: Dictionary metadata (if provided)

See `examples/train_and_export.js` for a complete example.

## API Reference

### Core Classes

- `TokenizerBuilder`: Fluent builder for tokenizer configuration
- `Tokenizer`: Main tokenization engine
- `Token`: Individual token with text, position, and linguistic features
- `Metadata`: Dictionary metadata and configuration
- `Schema`: Dictionary schema definition

### Training Functions (requires `train` feature)

- `train()`: Train a morphological analysis model from corpus
- `exportModel()`: Export trained model to dictionary files
