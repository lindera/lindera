# Quick Start

This example covers the basic usage of Lindera.

It will:

- Create a segmenter in normal mode
- Segment the input text
- Output the tokens

This example uses the `embed-ipadic` feature, which downloads the IPADIC dictionary and embeds it into the binary automatically at build time — no manual dictionary download is required.

```rust
use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = segmenter.segment(Cow::Borrowed(text))?;
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

The above example can be run as follows:

```shell
% cargo run --features embed-ipadic --example=segment
```

> [!TIP]
> If you prefer not to embed the dictionary into the binary, download a pre-built IPADIC dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases), extract it to a local directory (e.g., `/path/to/ipadic`), and call `load_dictionary("/path/to/ipadic")` instead — no `embed-ipadic` feature needed in that case. See [Feature Flags](../development/feature_flags.md) for details.

You can see the result as follows:

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

> [!NOTE]
> Character filters, token filters, and the `Tokenizer` API are provided by
> the companion `lindera-analysis` crate (as of v5.0). Add
> `lindera-analysis = "5.0"` to your dependencies if you need the analysis
> chain.
