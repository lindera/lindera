# Configuration

Lindera is able to read YAML format configuration files.
Specify the path to the following file in the environment variable `LINDERA_CONFIG_PATH`. You can use it easily without having to code the behavior of the tokenizer in Rust code.

```yaml
segmenter:
  mode: "normal"
  dictionary: "embedded://ipadic"
  # user_dictionary: "./resources/user_dict/ipadic_simple_userdic.csv"
  # keep_whitespace: false
  # use_mmap: false # only meaningful for filesystem (non-embedded://) dictionaries
  # max_grouping_len: 24 # cap on unknown-word grouping; omit or 0 for unbounded
  # unknown_word_ladder: true # emit shorter unknown-word candidates (default: true)

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

## Segmenter Options

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `mode` | string | `"normal"` | Segmentation mode: `"normal"` or `"decompose"` |
| `dictionary` | string | *(required)* | Dictionary URI, e.g. `"embedded://ipadic"` |
| `user_dictionary` | string | *(none)* | Path to a user dictionary |
| `keep_whitespace` | bool | `false` | Emit whitespace tokens instead of ignoring them (MeCab ignores them) |
| `use_mmap` | bool | on when the `mmap` feature is compiled in (the default) | Memory-map the dictionary; only meaningful for filesystem (non-`embedded://`) dictionaries |
| `max_grouping_len` | integer | *(unbounded)* | Maximum characters **beyond the first** that an unknown-word grouping may span, matching MeCab's `max-grouping-size` (MeCab defaults to 24). A grouped candidate longer than this is not emitted; the single-character unknown word is emitted instead. Omitting the key, or setting `0`, leaves grouping unbounded |
| `unknown_word_ladder` | bool | `true` | Also emit the shorter unknown-word candidates up to each category's `LENGTH` field in `char.def`, as MeCab and Vibrato do. Set to `false` to reproduce pre-v6 output exactly |

```shell
% export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera_analysis::tokenizer::TokenizerBuilder;
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
