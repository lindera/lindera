# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese morphological analysis library in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Build

The following products are required to build:

- Rust >= 1.46.0

```text
% cargo build --release
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
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    // create tokenizer
    let mut tokenizer = Tokenizer::new()?;

    // tokenize the text
    let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")?;

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

### User dictionary example

#### Simple user dictionary

You can give user dictionary entries along with the default system dictionary. User dictionary should be a CSV with following format.

```
<surface_form>,<part_of_speech>,<reading>
```

For example:
```shell
% cat userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

#### Detailed user dictionary

You can also give user dictionary the same format as the IPA dictionary.

For example:
```shell
% cat userdic_with_cost.csv
東京スカイツリー,1288,1288,-1000,名詞,固有名詞,一般,カスタム名詞,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
東武スカイツリーライン,1288,1288,-1000,名詞,固有名詞,一般,カスタム名詞,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン
とうきょうスカイツリー駅,1288,1288,-1000,名詞,固有名詞,一般,カスタム名詞,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ
```

#### With an user dictionary, `Tokenizer` will be created as follows

```rust
use std::path::Path;

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use lindera_core::viterbi::Mode;
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    // create tokenizer
    let config = TokenizerConfig {
        user_dict_path: Some(&Path::new("resources/userdic.csv")),
        mode: Mode::Normal,
        ..TokenizerConfig::default()
    };
    let mut tokenizer = Tokenizer::with_config(config)?;

    // tokenize the text
    let tokens = tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")?;

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}
```

#### The above example can be by `cargo run --example`

```shell
% cd lindera/lindera
% cargo run --example userdic_example
東京スカイツリー
の
最寄り駅
は
とうきょうスカイツリー駅
です
```

## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera" target="_blank">lindera</a>
