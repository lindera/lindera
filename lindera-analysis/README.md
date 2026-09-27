# lindera-analysis

Text analysis chain for [Lindera](https://github.com/lindera/lindera):
character filters, token filters, and the `Tokenizer` that composes them
around a `lindera_segmenter::segmenter::Segmenter`.

The [`lindera-segmenter`](https://crates.io/crates/lindera-segmenter) crate
provides pure morphological segmentation; this crate layers Lucene-style
analysis on top:

```text
input text → character filters → Segmenter → token filters → tokens
```

## Usage

The usual way to use this crate is through the
[`lindera`](https://crates.io/crates/lindera) facade, which re-exports it as
`lindera::analysis` (enabled by the default `analysis` feature):

```toml
[dependencies]
lindera = "7"
```

```rust
use lindera::analysis::tokenizer::Tokenizer;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("/path/to/ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let tokenizer = Tokenizer::new(segmenter);
let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")?;
```

### As a direct dependency

To depend on this crate without the facade, pair it with `lindera-segmenter`
and use the `lindera_segmenter::` paths for the segmenter types:

```toml
[dependencies]
lindera-segmenter = "7"
lindera-analysis = "7"
```

```rust
use lindera_analysis::tokenizer::Tokenizer;
use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

let dictionary = load_dictionary("/path/to/ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let tokenizer = Tokenizer::new(segmenter);
```

The `embed-*` features (`embed-ipadic`, `embed-unidic`, `embed-ko-dic`, ...)
and the `mmap` feature (default) are forwarded to `lindera-segmenter`.

Provided filters include Unicode normalization, mapping, and regex character
filters, and Japanese/Korean dictionary-driven token filters (base form,
reading form, part-of-speech keep/stop tags, compound words, numbers,
katakana stemming, and more) mirroring Lucene's kuromoji / nori analyzers.

## License

MIT
