# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/bayard-search/lindera](https://badges.gitter.im/bayard-search/lindera.svg)](https://gitter.im/bayard-search/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

Lindera is a Japanese Morphological Analysis Library in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).  
Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Basic example

This example covers the basic usage of Lindera.

It will:
- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera::tokenizer::tokenizer::Tokenizer;

fn main() -> std::io::Result<()> {
    // create tokenizer
    let mut tokenizer = Tokenizer::default_normal();

    // tokenize the text
    let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ");

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}
```

The above example can be run as follows:
```shell script
% cargo run --example basic_example
```

You can see the result as follows:
```text
関西国際空港
限定
トートバッグ
```

## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera" target="_blank">Lindera</a>

## Project links

lindera consists of several projects. The list is following:
- [Lindera](https://github.com/bayard-search/lindera): library
- [Lindera Core](https://github.com/bayard-search/lindera-core): Core library
- [Lindera Dictionary](https://github.com/bayard-search/lindera-dictionary): Compiled Japanese dictionary loader
- [Lindera IPADIC](https://github.com/bayard-search/lindera-ipadic): Compiled Japanese dictionary based on IPADIC
- [lindera IPADIC Builder](https://github.com/bayard-search/lindera-ipadic-builder): Dictionary builder for IPADIC
- [lindera UniDic Builder](https://github.com/bayard-search/lindera-unidic-builder): Dictionary builder for UniDic
- [Lindera CLI](https://github.com/bayard-search/lindera-cli): Command-line interface
