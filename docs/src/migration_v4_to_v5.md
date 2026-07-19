# Migrating from v4 to v5

Lindera v5.0.0 restructures the workspace around a lean core: the default
build of the `lindera` crate is now a pure morphological segmenter, and the
dictionary-training pipeline lives in its own crate. This guide lists every
breaking change and the one-line fixes for each.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| `analysis` removed from default features | Rust users of `Tokenizer`, character filters, or token filters | Add `analysis` to your lindera feature list |
| `lindera-dictionary` no longer has a `train` feature | Direct `lindera-dictionary --features train` users | Depend on `lindera-trainer` (or the `lindera` facade's `train` feature) |

The language bindings (Python, Node.js, Ruby, PHP, WASM) and the CLI are
unaffected: they enable the required features themselves, and their APIs and
output are unchanged. Tokenization output is also unchanged — v5 produces
byte-for-byte identical tokens to v4 for the same input and dictionary.

## Pure segmenter by default

The `analysis` cargo feature (introduced in v4.1.0) gates the analysis chain:
the `character_filter`, `token_filter`, and `tokenizer` modules. In v5.0 it is
no longer part of the default feature set, so the default build provides the
`Segmenter` API only.

If you use `Tokenizer` or any filter, add the feature:

```toml
# v4
[dependencies]
lindera = "4.0"

# v5
[dependencies]
lindera = { version = "5.0", features = ["analysis"] }
```

If you only segment text, nothing changes in your code — and your dependency
tree shrinks (kanaria, unicode-normalization, unicode-segmentation,
unicode-blocks, and serde_yaml_ng are no longer built):

```rust
use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("/path/to/ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let tokens = segmenter.segment(Cow::Borrowed("関西国際空港限定トートバッグ"))?;
```

## Training moved to the lindera-trainer crate

The CRF training pipeline (`TrainerConfig`, `Trainer`, `Corpus`, `Model`,
`SerializableModel`) moved from `lindera-dictionary`'s `train`-gated `trainer`
module into the new `lindera-trainer` crate. As a result,
`lindera-dictionary` no longer depends on `lindera-crf` or `regex`.

**Through the `lindera` facade nothing changes** — the `train` feature now
pulls `lindera-trainer` and re-exports it under the same path:

```rust
// Works in both v4 and v5 (with the `train` feature):
use lindera::dictionary::trainer::{Corpus, Trainer, TrainerConfig};
```

Only direct users of `lindera-dictionary --features train` need to switch:

```toml
# v4
[dependencies]
lindera-dictionary = { version = "4.0", features = ["train"] }

# v5
[dependencies]
lindera-dictionary = "5.0"
lindera-trainer = "5.0"
```

```rust
// v4
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// v5
use lindera_trainer::{Corpus, Trainer, TrainerConfig};
```

The `lindera train` → `lindera export` → `lindera build` CLI workflow is
unchanged.
