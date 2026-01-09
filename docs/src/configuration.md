# Configuration

Lindera is able to read YAML format configuration files.
Specify the path to the following file in the environment variable `LINDERA_CONFIG_PATH`. You can use it easily without having to code the behavior of the tokenizer in Rust code.

```yaml
segmenter:
  mode: "normal"
  dictionary:
    kind: "ipadic"
  user_dictionary:
    path: "./resources/user_dict/ipadic_simple.csv"
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
% export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera::tokenizer::TokenizerBuilder;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // Load tokenizer configuration from file
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("config")
        .join("lindera.yml");

    let builder = TokenizerBuilder::from_file(&path)?;

    let tokenizer = builder.build()?;

    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
    println!("text: {text}");

    let tokens = tokenizer.tokenize(&text)?;

    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.surface, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```
