# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

The following products are required to build:

- Rust >= 1.46.0

## Tokenization examples

### Basic tokenization

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "0.33.0", features = ["ipadic"] }
```

This example covers the basic usage of Lindera.

It will:

- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera::{
    DictionaryConfig, DictionaryKind, LinderaResult, Mode, Tokenizer, TokenizerConfig,
};

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
% cargo run --features=ipadic --example=ipadic_basic_example
```

You can see the result as follows:

```text
関西国際空港
限定
トートバッグ
```

### Tokenization with user dictionary

You can give user dictionary entries along with the default system dictionary. User dictionary should be a CSV with following format.

```csv
<surface>,<part_of_speech>,<reading>
```

Put the following in Cargo.toml:

```toml
[dependencies]
lindera-tokenizer = { version = "0.31.0", features = ["ipadic"] }
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

use lindera::{
    DictionaryConfig, DictionaryKind, LinderaResult, Mode, Tokenizer, TokenizerConfig,
    UserDictionaryConfig,
};

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
% cargo run --features=ipadic --example=ipadic_userdic_example
東京スカイツリー
の
最寄り駅
は
とうきょうスカイツリー駅
です
```

## Analysis examples

### Basic analysis

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "0.31.0", features = ["ipadic", "filter"] }
```

This example covers the basic usage of Lindera Analysis Framework.

It will:

- Apply character filter for Unicode normalization (NFKC)
- Tokenize the input text with IPADIC
- Apply token filters for removing stop tags (Part-of-speech) and Japanese Katakana stem filter

```rust
use std::collections::HashSet;

use lindera::{
    Analyzer, BoxCharacterFilter, BoxTokenFilter, DictionaryConfig, DictionaryKind,
    JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
    JapaneseIterationMarkCharacterFilter, JapaneseIterationMarkCharacterFilterConfig,
    JapaneseNumberTokenFilter, JapaneseNumberTokenFilterConfig,
    JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig, LinderaResult, Mode,
    Tokenizer, TokenizerConfig, UnicodeNormalizeCharacterFilter,
    UnicodeNormalizeCharacterFilterConfig, UnicodeNormalizeKind,
};

fn main() -> LinderaResult<()> {
    let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();

    let unicode_normalize_character_filter_config =
            UnicodeNormalizeCharacterFilterConfig::new(UnicodeNormalizeKind::NFKC);
    let unicode_normalize_character_filter =
        UnicodeNormalizeCharacterFilter::new(unicode_normalize_character_filter_config);
    character_filters.push(BoxCharacterFilter::from(unicode_normalize_character_filter));

    let japanese_iteration_mark_character_filter_config =
        JapaneseIterationMarkCharacterFilterConfig::new(true, true);
    let japanese_iteration_mark_character_filter = JapaneseIterationMarkCharacterFilter::new(
        japanese_iteration_mark_character_filter_config,
    );
    character_filters.push(BoxCharacterFilter::from(
        japanese_iteration_mark_character_filter,
    ));

    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };

    let config = TokenizerConfig {
        dictionary,
        user_dictionary: None,
        mode: Mode::Normal,
    };

    let tokenizer = Tokenizer::from_config(config).unwrap();

    let mut token_filters: Vec<BoxTokenFilter> = Vec::new();

    let japanese_compound_word_token_filter_config =
        JapaneseCompoundWordTokenFilterConfig::new(
            DictionaryKind::IPADIC,
            HashSet::from_iter(vec!["名詞,数".to_string()]),
            Some("名詞,数".to_string()),
        )?;
    let japanese_compound_word_token_filter =
        JapaneseCompoundWordTokenFilter::new(japanese_compound_word_token_filter_config);
    token_filters.push(BoxTokenFilter::from(japanese_compound_word_token_filter));

    let japanese_number_token_filter_config =
        JapaneseNumberTokenFilterConfig::new(Some(HashSet::from_iter(vec![
            "名詞,数".to_string()
        ])));
    let japanese_number_token_filter =
        JapaneseNumberTokenFilter::new(japanese_number_token_filter_config);
    token_filters.push(BoxTokenFilter::from(japanese_number_token_filter));

    let japanese_stop_tags_token_filter_config =
        JapaneseStopTagsTokenFilterConfig::new(HashSet::from_iter(vec![
            "接続詞".to_string(),
            "助詞".to_string(),
            "助詞,格助詞".to_string(),
            "助詞,格助詞,一般".to_string(),
            "助詞,格助詞,引用".to_string(),
            "助詞,格助詞,連語".to_string(),
            "助詞,係助詞".to_string(),
            "助詞,副助詞".to_string(),
            "助詞,間投助詞".to_string(),
            "助詞,並立助詞".to_string(),
            "助詞,終助詞".to_string(),
            "助詞,副助詞／並立助詞／終助詞".to_string(),
            "助詞,連体化".to_string(),
            "助詞,副詞化".to_string(),
            "助詞,特殊".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
            "記号,一般".to_string(),
            "記号,読点".to_string(),
            "記号,句点".to_string(),
            "記号,空白".to_string(),
            "記号,括弧閉".to_string(),
            "その他,間投".to_string(),
            "フィラー".to_string(),
            "非言語音".to_string(),
        ]));
    let japanese_stop_tags_token_filter =
        JapaneseStopTagsTokenFilter::new(japanese_stop_tags_token_filter_config);
    token_filters.push(BoxTokenFilter::from(japanese_stop_tags_token_filter));

    let analyzer = Analyzer::new(character_filters, tokenizer, token_filters);

    let mut text =
        "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
    println!("text: {}", text);

    // tokenize the text
    let tokens = analyzer.analyze(&mut text)?;

    // output the tokens
    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.text, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

The above example can be run as follows:

```shell
% cargo run --features=ipadic,filter --example=analysis_example
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

- [lindera](https://docs.rs/lindera)
