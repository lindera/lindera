# Training

Lindera Ruby supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-ruby with the `train` feature enabled:

```bash
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

## Training a Model

Use `Lindera.train` to train a CRF model from a seed lexicon and annotated corpus:

```ruby
require 'lindera'

Lindera.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  1.0,   # lambda (regularization coefficient)
  100,   # max_iter
  nil    # max_threads (nil = auto-detect CPU cores)
)
```

### Training Parameters

The first seven parameters are required positional arguments. `lambda`, `max_iter` and `max_threads` are optional positional arguments after them, and the regularization type, the Elastic Net ratio and `eta` are keyword arguments; `nil`, or leaving an argument out, selects its default:

| Position | Name | Type | Description |
| --- | --- | --- | --- |
| 1 | seed | `String` | Path to the seed lexicon file (CSV format) |
| 2 | corpus | `String` | Path to the annotated training corpus |
| 3 | char_def | `String` | Path to the character definition file (char.def) |
| 4 | unk_def | `String` | Path to the unknown word definition file (unk.def) |
| 5 | feature_def | `String` | Path to the feature definition file (feature.def) |
| 6 | rewrite_def | `String` | Path to the rewrite rule definition file (rewrite.def) |
| 7 | output | `String` | Output path for the trained model file |
| 8 | lambda | `Float` or `nil` | Regularization coefficient, finite and >= 0, default 1.0; `0` disables regularization for every type; with L2, MeCab's `-c C` is `1 / C`, so 1.0 is MeCab's `-c 1.0` |
| 9 | max_iter | `Integer` or `nil` | Maximum number of training iterations, 1 or more, default 100; training stops earlier once the objective converges (see `eta:`) |
| 10 | max_threads | `Integer` or `nil` | Number of threads, 1 or more (`nil` = auto-detect CPU cores) |
| keyword | regularization: | `String` or `nil` | Regularization type, default `'l2'`: `'l1'`, `'l2'` or `'elasticnet'`, in any case (`'elastic_net'` and `'elastic-net'` also select Elastic Net), as `lindera train --regularization` takes it |
| keyword | elastic_net_l1_ratio: | `Float` or `nil` | L1 ratio of Elastic Net regularization, between 0 and 1, default 0.5; `0` uses only L2, `1` uses only L1; used only with `'elasticnet'` |
| keyword | eta: | `Float` or `nil` | Tolerance of the convergence test, finite and >= 0, default 5e-5: training stops before `max_iter` once the relative change of the objective stays below it for 3 iterations in a row; `0` disables the test |

The settings are checked before any file is read: an unknown `regularization:` raises an `ArgumentError`, and a `lambda`, `elastic_net_l1_ratio:` or `eta:` outside its range a `RangeError`. To train with v6's default settings (the model still differs from v6's, since the first line search changed in [#1290](https://github.com/lindera/lindera/issues/1290)), pass a `lambda` of 0.01 and `regularization: 'l1'`, and `eta: 0` to stop only at `max_iter`:

```ruby
Lindera.train(*inputs, '/tmp/model.dat', 0.01, regularization: 'l1', eta: 0)
```

## Exporting a Trained Model

After training, export the model to dictionary source files using `Lindera.export`:

```ruby
require 'lindera'

Lindera.export(
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

The export creates the following files in the output directory, the same files that [`lindera export`](../lindera-cli/commands.md#export) creates:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions (copied from the trained model)
- `feature.def` -- Feature template definitions (copied from the trained model)
- `rewrite.def` -- Feature rewrite rules (copied from the trained model)
- `left-id.def` -- Left context ID to feature string mapping
- `right-id.def` -- Right context ID to feature string mapping
- `metadata.json` -- Updated metadata (when `metadata` parameter is provided)

The model file must come from this version's training. A model file written by an earlier version, which includes every model file written before 7.0.0, is rejected: `Lindera.export` fails with an error that asks you to re-run training. Train such a model again with `Lindera.train`.

## Complete Workflow

The full workflow for training and using a custom dictionary:

```ruby
require 'lindera'

# Step 1: Train the CRF model
Lindera.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  1.0,   # lambda
  100,   # max_iter
  nil    # max_threads
)

# Step 2: Export to dictionary source files
Lindera.export(
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
builder.set_mode(Lindera::Mode.normal)
tokenizer = builder.build

tokens = tokenizer.tokenize('形態素解析のテスト')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```
