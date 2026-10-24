# Training

Lindera Python supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-python with the `train` feature enabled (enabled by default):

```bash
maturin develop --features train
```

## Training a Model

Use `lindera.train()` to train a CRF model from a seed lexicon and annotated corpus:

```python
import lindera

lindera.train(
    seed="resources/training/seed.csv",
    corpus="resources/training/corpus.txt",
    char_def="resources/training/char.def",
    unk_def="resources/training/unk.def",
    feature_def="resources/training/feature.def",
    rewrite_def="resources/training/rewrite.def",
    output="/tmp/model.dat",
    lambda_=1.0,
    max_iter=100,
    max_threads=4,
)
```

### Training Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `seed` | `str` | required | Path to the seed lexicon file (CSV format) |
| `corpus` | `str` | required | Path to the annotated training corpus |
| `char_def` | `str` | required | Path to the character definition file (char.def) |
| `unk_def` | `str` | required | Path to the unknown word definition file (unk.def) |
| `feature_def` | `str` | required | Path to the feature definition file (feature.def) |
| `rewrite_def` | `str` | required | Path to the rewrite rule definition file (rewrite.def) |
| `output` | `str` | required | Output path for the trained model file |
| `lambda_` | `float` | `1.0` | L2 regularization coefficient, >= 0 (1.0 is MeCab's `-c 1.0`); the bindings train with L2 regularization, whose type cannot be chosen yet ([#1262](https://github.com/lindera/lindera/issues/1262)) |
| `max_iter` | `int` | `100` | Maximum number of training iterations; training stops earlier once the objective converges (its relative change stays below 5e-5 for 3 iterations in a row) |
| `max_threads` | `int` or `None` | `None` | Number of threads (None = auto-detect CPU cores) |

## Exporting a Trained Model

After training, export the model to dictionary source files using `lindera.export()`:

```python
import lindera

lindera.export(
    model="/tmp/model.dat",
    output="/tmp/dictionary_source",
    metadata="resources/training/metadata.json",
)
```

### Export Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `model` | `str` | required | Path to the trained model file (.dat) |
| `output` | `str` | required | Output directory for dictionary source files |
| `metadata` | `str` or `None` | `None` | Path to a base metadata.json file |

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

The model file must come from this version's training. A model file written by an earlier version, which includes every model file written before 7.0.0, is rejected: `lindera.export()` fails with an error that asks you to re-run training. Train such a model again with `lindera.train()`.

## Complete Workflow

The full workflow for training and using a custom dictionary:

```python
import lindera

# Step 1: Train the CRF model
lindera.train(
    seed="resources/training/seed.csv",
    corpus="resources/training/corpus.txt",
    char_def="resources/training/char.def",
    unk_def="resources/training/unk.def",
    feature_def="resources/training/feature.def",
    rewrite_def="resources/training/rewrite.def",
    output="/tmp/model.dat",
    lambda_=1.0,
    max_iter=100,
)

# Step 2: Export to dictionary source files
lindera.export(
    model="/tmp/model.dat",
    output="/tmp/dictionary_source",
    metadata="resources/training/metadata.json",
)

# Step 3: Build the dictionary from exported source files
metadata = lindera.Metadata.from_json_file("/tmp/dictionary_source/metadata.json")
lindera.build_dictionary("/tmp/dictionary_source", "/tmp/dictionary", metadata)

# Step 4: Use the trained dictionary
tokenizer = (
    lindera.TokenizerBuilder()
    .set_dictionary("/tmp/dictionary")
    .set_mode(lindera.Mode.normal())
    .build()
)

tokens = tokenizer.tokenize("形態素解析のテスト")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```
