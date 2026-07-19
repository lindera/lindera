# lindera-analysis

Text analysis chain for [Lindera](https://github.com/lindera/lindera):
character filters, token filters, and the `Tokenizer` that composes them
around a `lindera::segmenter::Segmenter`.

The [`lindera`](https://crates.io/crates/lindera) crate provides pure
morphological segmentation; this crate layers Lucene-style analysis on top:

```text
input text → character filters → Segmenter → token filters → tokens
```

## Usage

```toml
[dependencies]
lindera = "5.0"
lindera-analysis = "5.0"
```

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::Tokenizer;

let dictionary = load_dictionary("/path/to/ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let tokenizer = Tokenizer::new(segmenter);
let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")?;
```

Provided filters include Unicode normalization, mapping, and regex character
filters, and Japanese/Korean dictionary-driven token filters (base form,
reading form, part-of-speech keep/stop tags, compound words, numbers,
katakana stemming, and more) mirroring Lucene's kuromoji / nori analyzers.

## License

MIT
