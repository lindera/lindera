# lindera-segmenter

The morphological segmenter of [Lindera](https://github.com/lindera/lindera):
the `Segmenter` API and the dictionary loading functions it is built on.

Up to v6 this crate was published as `lindera`. The
[`lindera`](https://crates.io/crates/lindera) crate is now a facade that
re-exports this crate, so most users should depend on `lindera` rather than on
`lindera-segmenter` directly.

## What it provides

- `segmenter::Segmenter`: Viterbi-based morphological segmentation with the
  segmentation `mode::Mode`, whitespace handling, and space penalties
- `dictionary`: `load_dictionary` / `load_user_dictionary` for embedded,
  downloaded, and user dictionaries, plus the low-level `lindera-dictionary`
  modules re-exported as `dictionary::core`, `dictionary::builder`,
  `dictionary::viterbi`, and so on
- `token::Token`: a segmented token with access to its dictionary details

## Example

```rust
use std::borrow::Cow;

use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

// Requires the `embed-ipadic` feature.
let dictionary = load_dictionary("embedded://ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let mut tokens = segmenter.segment(Cow::Borrowed("関西国際空港限定トートバッグ"))?;
for token in tokens.iter_mut() {
    println!("{}\t{}", token.surface, token.details().join(","));
}
```

## Features

The `embed-*` features (`embed-ipadic`, `embed-unidic`, `embed-ko-dic`, ...
and the `embed-cjk*` bundles) embed a dictionary in the binary, `mmap`
(default) loads dictionary files through memory mapping, and `train` enables
the CRF-based dictionary training API under `dictionary::trainer`.

## License

MIT

The crate version follows the Lindera workspace version.
