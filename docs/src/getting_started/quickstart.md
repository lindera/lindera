# Quick Start

This example covers the basic usage of Lindera.

It will:

- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

First, download a pre-built IPADIC dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and extract it to a local directory (e.g., `/path/to/ipadic`).

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("/path/to/ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = tokenizer.tokenize(text)?;
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
% cargo run --example=tokenize
```

> [!TIP]
> If you embed the dictionary into the binary using the `embed-ipadic` feature (advanced usage), you can use `load_dictionary("embedded://ipadic")` instead of specifying a file path. See [Feature Flags](../development/feature_flags.md) for details.

You can see the result as follows:

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```
