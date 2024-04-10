# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A morphological analysis library in Rust. This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

The following products are required to build:

- Rust >= 1.46.0


## Usage

Put the following in Cargo.toml:

```
[dependencies]
lindera_analyzer = { version = "0.24.0", features = ["ipadic", "filter"] }
```

### Basic example

This example covers the basic usage of Lindera Analysis Framework.

It will:
- Apply character filter for Unicode normalization (NFKC)
- Tokenize the input text with IPADIC
- Apply token filters for removing stop tags (Part-of-speech) and Japanese Katakana stem filter

```rust
use std::{fs, path::PathBuf};

use lindera_analyzer::analyzer::Analyzer;
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("lindera_ipadic_conf.json");

    let config_bytes = fs::read(path).unwrap();

    let analyzer = Analyzer::from_slice(&config_bytes).unwrap();

    let mut text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
    println!("text: {}", text);

    // tokenize the text
    let tokens = analyzer.analyze(&mut text)?;

    // output the tokens
    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.text,
            token.byte_start,
            token.byte_end,
            token.details
        );
    }

    Ok(())
}
```

The above example can be run as follows:

```shell script
% cargo run --features=ipadic,filter --example=analyze
```

You can see the result as follows:
```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: Lindera, start: 0, end: 21, details: Some(["UNK"])
token: 形態素, start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: 解析, start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: エンジン, start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: ユーザ, start: 0, end: 26, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: 辞書, start: 26, end: 32, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: 利用, start: 35, end: 41, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: 可能, start: 41, end: 47, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-analyzer" target="_blank">lindera</a>
