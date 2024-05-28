# Lindera Tokenizer

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) [![Crates.io](https://img.shields.io/crates/v/lindera-tokenizer.svg)](https://crates.io/crates/lindera-tokenizer)

A morphological analysis library in Rust. This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

The following products are required to build:

- Rust >= 1.46.0

## Usage

Put the following in Cargo.toml:

```toml
[dependencies]
lindera-tokenizer = { version = "0.24.0", features = ["ipadic"] }
```

### Basic example

This example covers the basic usage of Lindera.

It will:

- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera_tokenizer::tokenizer::Tokenizer;
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };

    let config = TokenizerConfig {
        dictionary,
        user_dictionary: None,
        mode: Mode::Normal,
    };

    // create tokenizer
    let tokenizer = Tokenizer::from_config(config)?;

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

```shell
% cargo run --features=ipadic --example=ipadic_basic
```

You can see the result as follows:

```text
関西国際空港
限定
トートバッグ
```

### User dictionary example

You can give user dictionary entries along with the default system dictionary. User dictionary should be a CSV with following format.

```csv
<surface>,<part_of_speech>,<reading>
```

For example:

```shell
% cat ./resources/simple_userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

With an user dictionary, `Tokenizer` will be created as follows:

```rust
use std::path::PathBuf;

use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig};
use lindera_core::viterbi::Mode;
use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };

    let user_dictionary = Some(UserDictionaryConfig {
        kind: DictionaryKind::IPADIC,
        path: PathBuf::from("./resources/ipadic_simple_userdic.csv"),
    });

    let config = TokenizerConfig {
        dictionary,
        user_dictionary,
        mode: Mode::Normal,
    };

    let tokenizer = Tokenizer::from_config(config)?;

    // tokenize the text
    let tokens = tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")?;

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}
```

The above example can be by `cargo run --example`:

```shell
% cargo run --features=ipadic --example=ipadic_userdic
東京スカイツリー
の
最寄り駅
は
とうきょうスカイツリー駅
です
```

## API reference

The API reference is available. Please see following URL:

- [lindera-tokenizer](https://docs.rs/lindera-tokenizer)
