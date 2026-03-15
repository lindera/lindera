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
    lambda_=0.01,
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
| `lambda_` | `float` | `0.01` | L1 regularization cost (0.0--1.0) |
| `max_iter` | `int` | `100` | Maximum number of training iterations |
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

The export creates the following files in the output directory:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions
- `metadata.json` -- Updated metadata (when `metadata` parameter is provided)

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
    lambda_=0.01,
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
    .set_mode("normal")
    .build()
)

tokens = tokenizer.tokenize("形態素解析のテスト")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```
