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
  # space_penalty: false # left-space penalty (Korean); on by default for dictionaries that ship rules (ko-dic). false turns it off, an object gives explicit rules

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
      # Merge the numeral tokens only. A counter merged into the number token is read as digits by japanese_number.
      tags:
        - "名詞,数"
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
| `max_grouping_len` | integer | *(unbounded)* | Cap on unknown-word grouping, in characters **beyond the first**, matching MeCab's `max-grouping-size` (MeCab defaults to 24). Applied at each position: a run longer than the cap is not grouped there, the single-character candidate (plus the length ladder and dictionary words) remains, and the remaining tail is grouped again once it fits, so no unknown token exceeds cap+1 characters. Omitting the key, or setting `0`, leaves grouping unbounded |
| `unknown_word_ladder` | bool | `true` | Also emit the shorter unknown-word candidates up to each category's `LENGTH` field in `char.def`, as MeCab and Vibrato do. Set to `false` to reproduce pre-v6 output exactly |
| `space_penalty` | bool or object | *(the dictionary's rules, if any)* | Left-space penalty (mecab-ko's `left-space-penalty-factor`): a candidate that starts right after whitespace and whose first part-of-speech tag is listed gets the cost added. Omitted or `null` keeps the rules the dictionary ships in its `metadata.json` (ko-dic ships mecab-ko-dic's, so the penalty is on by default for ko-dic; the other bundled dictionaries ship none); `false` turns it off; `true` requires the dictionary's rules (an error when it ships none); an object `{"rules": [{"pos": [...], "cost": n}, ...]}` gives explicit rules. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) |

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
