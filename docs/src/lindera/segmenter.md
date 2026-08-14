# Segmenter

The `Segmenter` is the core component that performs morphological analysis. It uses the Viterbi algorithm to find the optimal segmentation of input text based on a dictionary and cost model.

## Creating a Segmenter

A `Segmenter` requires three components:

- **Mode** - the tokenization strategy (`Normal` or `Decompose`)
- **Dictionary** - a system dictionary for morphological analysis
- **UserDictionary** (optional) - a supplementary dictionary for custom words

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("embedded://ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
```

## Tokenization Modes

### Mode::Normal

Standard tokenization based on the dictionary entries. Words are segmented faithfully according to what is registered in the dictionary.

```rust
use lindera::mode::Mode;

let mode = Mode::Normal;
```

### Mode::Decompose

Decomposes compound nouns into their constituent parts. This mode applies a configurable penalty to long compound words, encouraging the segmenter to split them into shorter components.

For example, with `Mode::Normal`, the compound word "関西国際空港" in the sentence "関西国際空港限定トートバッグ" remains part of a single token, while with `Mode::Decompose`, it is split into "関西", "国際", and "空港" (the surrounding context affects whether a compound is split; the same string in isolation may not split the same way).

```rust
use lindera::mode::Mode;

let mode = Mode::Decompose(Default::default());
```

## Dictionary Loading

Lindera provides the `load_dictionary` function to load dictionaries from various sources.

### Embedded Dictionaries

When built with the appropriate feature flag (e.g., `embed-ipadic`), dictionaries can be loaded directly from the binary:

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("embedded://ipadic")?;
```

Available embedded dictionary URIs:

- `embedded://ipadic` - IPADIC (Japanese)
- `embedded://ipadic-neologd` - IPADIC NEologd (Japanese)
- `embedded://unidic` - UniDic (Japanese)
- `embedded://ko-dic` - ko-dic (Korean)
- `embedded://cc-cedict` - CC-CEDICT (Chinese)
- `embedded://jieba` - Jieba (Chinese)

### External Dictionaries

Pre-built dictionary directories can be loaded from the filesystem:

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("/path/to/dictionary")?;
```

## Using with Tokenizer

The `Segmenter` is typically used through the `Tokenizer`, which adds support for character filters and token filters:

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "日本語の形態素解析を行うことができます。";
    let tokens = tokenizer.tokenize(text)?;

    for mut token in tokens {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

Note the `mut` binding on `token`: `Token::details` takes `&mut self`, so iterating with a plain `for token in tokens` fails to compile (`E0596: cannot borrow as mutable`).

## Building from Config

`Segmenter::from_config` builds a `Segmenter` from a `SegmenterConfig` (a `serde_json::Value`), the same configuration format used by `Tokenizer`/`TokenizerBuilder` (see [Configuration](../lindera-analysis/configuration.md)) but scoped to just the `segmenter:` section:

```rust
use serde_json::json;
use lindera::segmenter::{Segmenter, SegmenterConfig};

let config: SegmenterConfig = json!({
    "mode": "normal",
    "dictionary": "embedded://ipadic",
    "keep_whitespace": false,
    "use_mmap": false
});
let segmenter = Segmenter::from_config(&config)?;
```

## Memory-Mapped Loading

For a filesystem-based (not `embedded://`) dictionary, set `use_mmap` to
`true` to route the connection-cost matrix and prefix dictionary through
memory-mapped reads instead of a plain file read. Only the dictionary's
largest word-list files stay lazily paged this way; the connection-cost
matrix and the double-array trie are always fully materialized into owned
memory regardless. `use_mmap` is silently ignored for `embedded://`
dictionaries, since their data is already a static, zero-copy byte slice.
Requires the `mmap` cargo feature (enabled by default).

## Whitespace Handling

By default, whitespace-only tokens are dropped from the output for MeCab compatibility. Call `keep_whitespace(true)` on a `Segmenter` to keep them:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
```

## N-Best Segmentation

`segment_nbest` returns the top-`n` segmentations ordered by total path cost, each paired with its cost. Set `unique` to deduplicate results that share the same word boundaries but differ only in POS tags, and `cost_threshold` to discard paths whose cost exceeds `best_cost + threshold`:

```rust
let results = segmenter.segment_nbest(Cow::Borrowed("すもももももももものうち"), 3, false, None)?;
for (tokens, cost) in results {
    println!("cost={cost}");
    for token in tokens {
        println!("  {}", token.surface.as_ref());
    }
}
```

`segment_nbest_with_lattice` is the same operation but lets you pass in a reusable `Lattice` buffer to avoid reallocating one per call.

## Reusable Worker

`SegmentWorker` is a reusable segmentation session that owns the Viterbi lattice and the backtrace scratch buffer, so repeated calls avoid the per-call allocations `segment` pays. Create one with `new_worker` (clones the segmenter) or `into_worker` (consumes it, avoiding a user-dictionary copy), then call `segment`/`segment_nbest` on it:

```rust
let mut worker = segmenter.new_worker();
for line in lines {
    let tokens = worker.segment(line)?;
    for token in &tokens {
        println!("{}", token.surface.as_ref());
    }
}
```

The returned tokens borrow the worker, so they must be consumed before the next call (the usual per-line loop above compiles as-is). `set_mode` and `set_keep_whitespace` switch the configuration between calls.

A worker also bounds retained memory: one delimiter-free 32 KiB sentence grows the lattice to roughly 20 MB, and a plain `Lattice` keeps that forever. The worker automatically shrinks its lattice once a window of calls shows the capacity is oversized, and `shrink_to(text_len_hint)` forces a shrink immediately.

The worker is permanently bound to the dictionary of the segmenter that created it; there is no way to swap dictionaries under a live worker, which rules out a class of lattice-reuse bugs by construction. For multi-threaded use, create one worker per thread from a shared `Segmenter`.
