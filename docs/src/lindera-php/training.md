# Training

Lindera PHP supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-php with the `train` feature enabled:

```bash
cargo build -p lindera-php --features train,embed-ipadic
```

## Training a Model

Use `Lindera\Trainer::train()` to train a CRF model from a seed lexicon and annotated corpus:

```php
<?php

Lindera\Trainer::train(
    seed: 'resources/training/seed.csv',
    corpus: 'resources/training/corpus.txt',
    char_def: 'resources/training/char.def',
    unk_def: 'resources/training/unk.def',
    feature_def: 'resources/training/feature.def',
    rewrite_def: 'resources/training/rewrite.def',
    output: '/tmp/model.dat',
    lambda: 0.01,
    max_iter: 100,
    max_threads: null,
);
```

### Training Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `$seed` | `string` | required | Path to the seed lexicon file (CSV format) |
| `$corpus` | `string` | required | Path to the annotated training corpus |
| `$char_def` | `string` | required | Path to the character definition file (char.def) |
| `$unk_def` | `string` | required | Path to the unknown word definition file (unk.def) |
| `$feature_def` | `string` | required | Path to the feature definition file (feature.def) |
| `$rewrite_def` | `string` | required | Path to the rewrite rule definition file (rewrite.def) |
| `$output` | `string` | required | Output path for the trained model file |
| `$lambda` | `float` | `0.01` | L1 regularization cost (0.0--1.0) |
| `$max_iter` | `int` | `100` | Maximum number of training iterations |
| `$max_threads` | `int\|null` | `null` | Number of threads (`null` = auto-detect CPU cores) |

## Exporting a Trained Model

After training, export the model to dictionary source files using `Lindera\Trainer::export()`:

```php
<?php

Lindera\Trainer::export(
    model: '/tmp/model.dat',
    output: '/tmp/dictionary_source',
    metadata: 'resources/training/metadata.json',
);
```

### Export Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `$model` | `string` | required | Path to the trained model file (.dat) |
| `$output` | `string` | required | Output directory for dictionary source files |
| `$metadata` | `string\|null` | `null` | Path to a base metadata.json file |

The export creates the following files in the output directory:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions
- `metadata.json` -- Updated metadata (when `$metadata` parameter is provided)

## Complete Workflow

The full workflow for training and using a custom dictionary:

```php
<?php

// Step 1: Train the CRF model
Lindera\Trainer::train(
    seed: 'resources/training/seed.csv',
    corpus: 'resources/training/corpus.txt',
    char_def: 'resources/training/char.def',
    unk_def: 'resources/training/unk.def',
    feature_def: 'resources/training/feature.def',
    rewrite_def: 'resources/training/rewrite.def',
    output: '/tmp/model.dat',
    lambda: 0.01,
    max_iter: 100,
);

// Step 2: Export to dictionary source files
Lindera\Trainer::export(
    model: '/tmp/model.dat',
    output: '/tmp/dictionary_source',
    metadata: 'resources/training/metadata.json',
);

// Step 3: Build the dictionary from exported source files
$metadata = Lindera\Metadata::fromJsonFile('/tmp/dictionary_source/metadata.json');
Lindera\Dictionary::build('/tmp/dictionary_source', '/tmp/dictionary', $metadata);

// Step 4: Use the trained dictionary
$builder = new Lindera\TokenizerBuilder();
$tokenizer = $builder
    ->setDictionary('/tmp/dictionary')
    ->setMode('normal')
    ->build();

$tokens = $tokenizer->tokenize('形態素解析のテスト');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```
