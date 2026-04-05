# Training

Lindera Ruby supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-ruby with the `train` feature enabled:

```bash
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

## Training a Model

Use `Lindera::Trainer.train` to train a CRF model from a seed lexicon and annotated corpus:

```ruby
require 'lindera'

Lindera::Trainer.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  0.01,  # lambda (L1 regularization)
  100,   # max_iter
  nil    # max_threads (nil = auto-detect CPU cores)
)
```

### Training Parameters

Parameters are passed as positional arguments in the following order:

| Position | Name | Type | Description |
| --- | --- | --- | --- |
| 1 | seed | `String` | Path to the seed lexicon file (CSV format) |
| 2 | corpus | `String` | Path to the annotated training corpus |
| 3 | char_def | `String` | Path to the character definition file (char.def) |
| 4 | unk_def | `String` | Path to the unknown word definition file (unk.def) |
| 5 | feature_def | `String` | Path to the feature definition file (feature.def) |
| 6 | rewrite_def | `String` | Path to the rewrite rule definition file (rewrite.def) |
| 7 | output | `String` | Output path for the trained model file |
| 8 | lambda | `Float` | L1 regularization cost (0.0--1.0) |
| 9 | max_iter | `Integer` | Maximum number of training iterations |
| 10 | max_threads | `Integer` or `nil` | Number of threads (`nil` = auto-detect CPU cores) |

## Exporting a Trained Model

After training, export the model to dictionary source files using `Lindera::Trainer.export`:

```ruby
require 'lindera'

Lindera::Trainer.export(
  '/tmp/model.dat',
  '/tmp/dictionary_source',
  'resources/training/metadata.json'
)
```

### Export Parameters

| Position | Name | Type | Description |
| --- | --- | --- | --- |
| 1 | model | `String` | Path to the trained model file (.dat) |
| 2 | output | `String` | Output directory for dictionary source files |
| 3 | metadata | `String` or `nil` | Path to a base metadata.json file |

The export creates the following files in the output directory:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions
- `metadata.json` -- Updated metadata (when `metadata` parameter is provided)

## Complete Workflow

The full workflow for training and using a custom dictionary:

```ruby
require 'lindera'

# Step 1: Train the CRF model
Lindera::Trainer.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  0.01,  # lambda
  100,   # max_iter
  nil    # max_threads
)

# Step 2: Export to dictionary source files
Lindera::Trainer.export(
  '/tmp/model.dat',
  '/tmp/dictionary_source',
  'resources/training/metadata.json'
)

# Step 3: Build the dictionary from exported source files
metadata = Lindera::Metadata.from_json_file('/tmp/dictionary_source/metadata.json')
Lindera.build_dictionary('/tmp/dictionary_source', '/tmp/dictionary', metadata)

# Step 4: Use the trained dictionary
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/tmp/dictionary')
builder.set_mode('normal')
tokenizer = builder.build

tokens = tokenizer.tokenize('形態素解析のテスト')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```
