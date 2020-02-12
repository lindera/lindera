# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/bayard-search/lindera](https://badges.gitter.im/bayard-search/lindera.svg)](https://gitter.im/bayard-search/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese morphological analysis library in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Build

The following products are required to build:

- Rust >= 1.39.0
- make >= 3.81

```text
% make build
```

## Usage

### Basic example

This example covers the basic usage of Lindera.

It will:
- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera::tokenizer::Tokenizer;

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
- <a href="https://docs.rs/lindera" target="_blank">lindera</a>

## Project links

lindera consists of several projects. The list is following:

- [Lindera](https://github.com/bayard-search/lindera)
- [Lindera Core](https://github.com/bayard-search/lindera-core)
- [Lindera Dictionary](https://github.com/bayard-search/lindera-dictionary)
- [Lindera IPADIC](https://github.com/bayard-search/lindera-ipadic)
- [lindera IPADIC Builder](https://github.com/bayard-search/lindera-ipadic-builder)
- [lindera UniDic Builder](https://github.com/bayard-search/lindera-unidic-builder)
- [Lindera CLI](https://github.com/bayard-search/lindera-cli)
