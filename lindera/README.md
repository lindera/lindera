# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Tokenization examples

### Basic tokenization

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.1.0", features = ["embed-ipadic"] }
```

This example covers the basic usage of Lindera.

It will:

- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = tokenizer.tokenize(text)?;
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.text.as_ref(), details);
    }

    Ok(())
}
```

The above example can be run as follows:

```shell
% cargo run --features=embed-ipadic --example=tokenize
```

You can see the result as follows:

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    UNK
```

### Tokenization with user dictionary

You can give user dictionary entries along with the default system dictionary. User dictionary should be a CSV with following format.

```csv
<surface>,<part_of_speech>,<reading>
```

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.1.0", features = ["embed-ipadic"] }
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

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;

fn main() -> LinderaResult<()> {
    let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("ipadic_simple_userdic.csv");

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ipadic")
        .join("metadata.json");
    let metadata: Metadata = serde_json::from_reader(
        File::open(metadata_file)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
            .unwrap(),
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
    .unwrap();

    let dictionary = load_dictionary("embedded://ipadic")?;
    let user_dictionary = load_user_dictionary(user_dict_path.to_str().unwrap(), &metadata)?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        Some(user_dictionary), // Assuming no user dictionary is provided
    );

    // Create a tokenizer.
    let tokenizer = Tokenizer::new(segmenter);

    // Tokenize a text.
    let text = "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です";
    let mut tokens = tokenizer.tokenize(text)?;

    // Print the text and tokens.
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.text.as_ref(), details);
    }

    Ok(())
}
```

The above example can be by `cargo run --example`:

```shell
% cargo run --features=embed-ipadic --example=tokenize_with_user_dict
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

### Tokenize with filters

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.1.0", features = ["embed-ipadic"] }
```

This example covers the basic usage of Lindera Analysis Framework.

It will:

- Apply character filter for Unicode normalization (NFKC)
- Tokenize the input text with IPADIC
- Apply token filters for removing stop tags (Part-of-speech) and Japanese Katakana stem filter

```rust
    use lindera::character_filter::BoxCharacterFilter;
    use lindera::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
    use lindera::character_filter::unicode_normalize::{
        UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
    };
    use lindera::dictionary::load_dictionary;
    use lindera::mode::Mode;
    use lindera::segmenter::Segmenter;
    use lindera::token_filter::BoxTokenFilter;
    use lindera::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
    use lindera::token_filter::japanese_number::JapaneseNumberTokenFilter;
    use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
    use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        None, // Assuming no user dictionary is provided
    );

    let unicode_normalize_char_filter =
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

    let japanese_iterration_mark_char_filter =
        JapaneseIterationMarkCharacterFilter::new(true, true);

    let japanese_compound_word_token_filter = JapaneseCompoundWordTokenFilter::new(
        DictionaryKind::IPADIC,
        vec!["名詞,数".to_string(), "名詞,接尾,助数詞".to_string()]
            .into_iter()
            .collect(),
        Some("複合語".to_string()),
    );

    let japanese_number_token_filter =
        JapaneseNumberTokenFilter::new(Some(vec!["名詞,数".to_string()].into_iter().collect()));

    let japanese_stop_tags_token_filter = JapaneseStopTagsTokenFilter::new(
        vec![
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
        ]
        .into_iter()
        .collect(),
    );

    // Create a tokenizer.
    let mut tokenizer = Tokenizer::new(segmenter);

    tokenizer
        .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
        .append_character_filter(BoxCharacterFilter::from(
            japanese_iterration_mark_char_filter,
        ))
        .append_token_filter(BoxTokenFilter::from(japanese_compound_word_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_number_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_stop_tags_token_filter));

    // Tokenize a text.
    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。";
    let tokens = tokenizer.tokenize(text)?;

    // Print the text and tokens.
    println!("text: {}", text);
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
% cargo run --features=embed-ipadic --example=tokenize_with_filters
```

You can see the result as follows:

```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: "Lindera", start: 0, end: 21, details: Some(["UNK"])
token: "形態素", start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "ユーザー", start: 63, end: 75, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: "辞書", start: 75, end: 81, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: "利用", start: 84, end: 90, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: "可能", start: 90, end: 96, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```

## Configuration file

Lindera is able to read YAML format configuration files.
Specify the path to the following file in the environment variable LINDERA_CONFIG. You can use it easily without having to code the behavior of the tokenizer in Rust code.

```yaml
segmenter:
  mode: "normal"
  dictionary:
    kind: "ipadic"
  user_dictionary:
    path: "./resources/ipadic_simple.csv"
    kind: "ipadic"

character_filters:
  - kind: "unicode_normalize"
    args:
      kind: "nfkc"
  - kind: "japanese_iteration_mark"
    args:
      normalize_kanji: true
      normalize_kana: true
  - kind: mapping
    args:
       mapping:
         リンデラ: Lindera

token_filters:
  - kind: "japanese_compound_word"
    args:
      kind: "ipadic"
      tags:
        - "名詞,数"
        - "名詞,接尾,助数詞"
      new_tag: "名詞,数"
  - kind: "japanese_number"
    args:
      tags:
        - "名詞,数"
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "接続詞"
        - "助詞"
        - "助詞,格助詞"
        - "助詞,格助詞,一般"
        - "助詞,格助詞,引用"
        - "助詞,格助詞,連語"
        - "助詞,係助詞"
        - "助詞,副助詞"
        - "助詞,間投助詞"
        - "助詞,並立助詞"
        - "助詞,終助詞"
        - "助詞,副助詞／並立助詞／終助詞"
        - "助詞,連体化"
        - "助詞,副詞化"
        - "助詞,特殊"
        - "助動詞"
        - "記号"
        - "記号,一般"
        - "記号,読点"
        - "記号,句点"
        - "記号,空白"
        - "記号,括弧閉"
        - "その他,間投"
        - "フィラー"
        - "非言語音"
  - kind: "japanese_katakana_stem"
    args:
      min: 3
  - kind: "remove_diacritical_mark"
    args:
      japanese: false
```

```shell
% export LINDERA_CONFIG=./resources/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera::tokenizer::TokenizerBuilder;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // Creates a new `TokenizerConfigBuilder` instance.
    // If the `LINDERA_CONFIG` environment variable is set, it will attempt to load the initial settings from the specified path.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("lindera.yml");

    let builder = TokenizerBuilder::from_file(&path)?;

    let tokenizer = builder.build()?;

    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
    println!("text: {text}");

    let tokens = tokenizer.tokenize(&text)?;

    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.text, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

## Environment Variables

### LINDERA_DICTS

The `LINDERA_DICTS` environment variable specifies a directory for caching dictionary source files. This enables:

- **Offline builds**: Once downloaded, dictionary source files are preserved for future builds
- **Faster builds**: Subsequent builds skip downloading if valid cached files exist
- **Reproducible builds**: Ensures consistent dictionary versions across builds

Usage:

```shell
export LINDERA_DICTS=/path/to/cache
cargo build --features=ipadic
```

When set, dictionary source files are stored in `$LINDERA_DICTS/<version>/` where `<version>` is the lindera-dictionary crate version. The cache validates files using MD5 checksums - invalid files are automatically re-downloaded.

### LINDERA_CONFIG

The `LINDERA_CONFIG` environment variable specifies the path to a YAML configuration file for the tokenizer. This allows you to configure tokenizer behavior without modifying Rust code.

```shell
export LINDERA_CONFIG=./resources/lindera.yml
```

See the [Configuration file](#configuration-file) section for details on the configuration format.

### DOCS_RS

The `DOCS_RS` environment variable is automatically set by docs.rs when building documentation. When this variable is detected, Lindera creates dummy dictionary files instead of downloading actual dictionary data, allowing documentation to be built without network access or large file downloads.

This is primarily used internally by docs.rs and typically doesn't need to be set by users.

### LINDERA_WORKDIR

The `LINDERA_WORKDIR` environment variable is automatically set during the build process by the lindera-dictionary crate. It points to the directory containing the built dictionary data files and is used internally by dictionary crates to locate their data files.

This variable is set automatically and should not be modified by users.

## API reference

The API reference is available. Please see following URL:

- [lindera](https://docs.rs/lindera)
