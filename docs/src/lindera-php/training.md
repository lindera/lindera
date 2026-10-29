# Training

Lindera PHP supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-php with the `train` feature enabled (`train` is already enabled by default in `lindera-php`):

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
    lambda: 1.0,
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
| `$lambda` | `float` | `1.0` | Regularization coefficient, finite and >= 0; `0` disables regularization for every type; with L2, MeCab's `-c C` is `1 / C`, so 1.0 is MeCab's `-c 1.0` |
| `$max_iter` | `int` | `100` | Maximum number of training iterations, 1 or more; training stops earlier once the objective converges (see `$eta`) |
| `$max_threads` | `int\|null` | `null` | Number of threads, 1 or more (`null` = auto-detect CPU cores) |
| `$regularization` | `string\|null` | `'l2'` | Regularization type: `'l1'`, `'l2'` or `'elasticnet'`, in any case (`'elastic_net'` and `'elastic-net'` also select Elastic Net), as `lindera train --regularization` takes it |
| `$elastic_net_l1_ratio` | `float\|null` | `0.5` | L1 ratio of Elastic Net regularization, between 0 and 1; `0` uses only L2, `1` uses only L1; used only with `'elasticnet'` |
| `$eta` | `float\|null` | `5e-5` | Tolerance of the convergence test, finite and >= 0: training stops before `$max_iter` once the relative change of the objective stays below it for 3 iterations in a row; `0` disables the test |

`null` selects the default of each optional parameter. The settings are checked before any file is read: an unknown `$regularization`, or a `$lambda`, `$elastic_net_l1_ratio` or `$eta` outside its range, throws a `ValueError`. To train with v6's default settings (the model still differs from v6's, since the first line search changed in [#1290](https://github.com/lindera/lindera/issues/1290)), pass `regularization: 'l1'` and `lambda: 0.01`, and `eta: 0` to stop only at `$max_iter`.

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

The export creates the following files in the output directory, the same files that [`lindera export`](../lindera-cli/commands.md#export) creates:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions (copied from the trained model)
- `feature.def` -- Feature template definitions (copied from the trained model)
- `rewrite.def` -- Feature rewrite rules (copied from the trained model)
- `left-id.def` -- Left context ID to feature string mapping
- `right-id.def` -- Right context ID to feature string mapping
- `metadata.json` -- Updated metadata (when `$metadata` parameter is provided)

The model file must come from this version's training. A model file written by an earlier version, which includes every model file written before 7.0.0, is rejected: `Lindera\Trainer::export()` fails with an error that asks you to re-run training. Train such a model again with `Lindera\Trainer::train()`.

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
    lambda: 1.0,
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
$builder->setDictionary('/tmp/dictionary');
$builder->setMode(Lindera\Mode::normal());
$tokenizer = $builder->build();

$tokens = $tokenizer->tokenize('形態素解析のテスト');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```
