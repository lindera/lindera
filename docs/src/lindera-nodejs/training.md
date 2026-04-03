# Training

Lindera Node.js supports training custom CRF-based morphological analysis models from annotated corpora. This functionality requires the `train` feature.

## Prerequisites

Build lindera-nodejs with the `train` feature enabled (enabled by default):

```bash
npm run build -- --features train
```

## Training a Model

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
  lambda: 0.01,
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
| `lambda` | `number` | `0.01` | L1 regularization cost (0.0--1.0) |
| `maxIter` | `number` | `100` | Maximum number of training iterations |
| `maxThreads` | `number \| undefined` | `undefined` | Number of threads (undefined = auto-detect CPU cores) |

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

The export creates the following files in the output directory:

- `lex.csv` -- Lexicon entries with trained costs
- `matrix.def` -- Connection cost matrix
- `unk.def` -- Unknown word definitions
- `char.def` -- Character category definitions
- `metadata.json` -- Updated metadata (when `metadata` parameter is provided)

## Complete Workflow

The full workflow for training and using a custom dictionary:

```javascript
const {
  train,
  exportModel,
  buildDictionary,
  Metadata,
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
  lambda: 0.01,
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
const tokenizer = new TokenizerBuilder()
  .setDictionary("/tmp/dictionary")
  .setMode("normal")
  .build();

const tokens = tokenizer.tokenize("形態素解析のテスト");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.details.join(",")}`);
}
```
