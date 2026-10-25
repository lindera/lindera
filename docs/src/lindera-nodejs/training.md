# Training

Lindera Node.js supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-nodejs with the `train` feature enabled (enabled by default):

```bash
npm run build -- --features train
```

## Training a Model

> [!NOTE]
> The file paths below (`resources/training/*.csv`, `*.def`) are illustrative placeholders —
> this repository does not ship those exact files. For a complete, runnable example that
> generates a seed lexicon, corpus, and definition files on the fly and then trains/exports/builds
> a real dictionary, see
> [`lindera-nodejs/examples/train_and_export.js`](https://github.com/lindera/lindera/blob/main/lindera-nodejs/examples/train_and_export.js).

Use `train()` to train a CRF model from a seed lexicon and annotated corpus:

```javascript
const { train } = require("lindera");

train({
  seed: "resources/training/seed.csv",
  corpus: "resources/training/corpus.txt",
  charDef: "resources/training/char.def",
  unkDef: "resources/training/unk.def",
  featureDef: "resources/training/feature.def",
  rewriteDef: "resources/training/rewrite.def",
  output: "/tmp/model.dat",
  lambda: 1.0,
  maxIter: 100,
  maxThreads: 4,
});
```

### Training Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `seed` | `string` | required | Path to the seed lexicon file (CSV format) |
| `corpus` | `string` | required | Path to the annotated training corpus |
| `charDef` | `string` | required | Path to the character definition file (char.def) |
| `unkDef` | `string` | required | Path to the unknown word definition file (unk.def) |
| `featureDef` | `string` | required | Path to the feature definition file (feature.def) |
| `rewriteDef` | `string` | required | Path to the rewrite rule definition file (rewrite.def) |
| `output` | `string` | required | Output path for the trained model file |
| `lambda` | `number` | `1.0` | Regularization coefficient, finite and >= 0; with L2, MeCab's `-c C` is `1 / C`, so 1.0 is MeCab's `-c 1.0` |
| `maxIter` | `number` | `100` | Maximum number of training iterations, 1 or more; training stops earlier once the objective converges (see `eta`) |
| `maxThreads` | `number \| undefined` | `undefined` | Number of threads, 1 or more (undefined = auto-detect CPU cores) |
| `regularization` | `string` | `"l2"` | Regularization type: `"l1"`, `"l2"` or `"elasticnet"`, in any case (`"elastic_net"` and `"elastic-net"` also select Elastic Net), as `lindera train --regularization` takes it |
| `elasticNetL1Ratio` | `number` | `0.5` | L1 ratio of Elastic Net regularization, between 0 and 1; used only with `"elasticnet"` |
| `eta` | `number` | `5e-5` | Tolerance of the convergence test, finite and >= 0: training stops before `maxIter` once the relative change of the objective stays below it for 3 iterations in a row; `0` disables the test |

The settings are checked before any file is read: an unknown `regularization`, or a `lambda`, `elasticNetL1Ratio` or `eta` outside its range, throws an `InvalidArg` error. To train as v6 did by default, pass `regularization: "l1"` and `lambda: 0.01`, and `eta: 0` to stop only at `maxIter`.

## Exporting a Trained Model

After training, export the model to dictionary source files using `exportModel()`:

```javascript
const { exportModel } = require("lindera");

exportModel({
  model: "/tmp/model.dat",
  output: "/tmp/dictionary_source",
  metadata: "resources/training/metadata.json",
});
```

### Export Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `model` | `string` | required | Path to the trained model file (.dat) |
| `output` | `string` | required | Output directory for dictionary source files |
| `metadata` | `string \| undefined` | `undefined` | Path to a base metadata.json file |

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

The model file must come from this version's training. A model file written by an earlier version, which includes every model file written before 7.0.0, is rejected: `exportModel()` fails with an error that asks you to re-run training. Train such a model again with `train()`.

## Complete Workflow

The full workflow for training and using a custom dictionary:

```javascript
const {
  train,
  exportModel,
  buildDictionary,
  Metadata,
  Mode,
  TokenizerBuilder,
} = require("lindera");

// Step 1: Train the CRF model
train({
  seed: "resources/training/seed.csv",
  corpus: "resources/training/corpus.txt",
  charDef: "resources/training/char.def",
  unkDef: "resources/training/unk.def",
  featureDef: "resources/training/feature.def",
  rewriteDef: "resources/training/rewrite.def",
  output: "/tmp/model.dat",
  lambda: 1.0,
  maxIter: 100,
});

// Step 2: Export to dictionary source files
exportModel({
  model: "/tmp/model.dat",
  output: "/tmp/dictionary_source",
  metadata: "resources/training/metadata.json",
});

// Step 3: Build the dictionary from exported source files
const metadata = Metadata.fromJsonFile("/tmp/dictionary_source/metadata.json");
buildDictionary("/tmp/dictionary_source", "/tmp/dictionary", metadata);

// Step 4: Use the trained dictionary
const builder = new TokenizerBuilder();
builder.setDictionary("/tmp/dictionary");
builder.setMode(Mode.normal());
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("形態素解析のテスト");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.details.join(",")}`);
}
```
