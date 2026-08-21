# 設定

LinderaはYAML形式の設定ファイルを読み込むことができます。
環境変数 `LINDERA_CONFIG_PATH` にファイルのパスを指定してください。Rustコードでトークナイザーの動作をコーディングすることなく、簡単に利用できます。

```yaml
segmenter:
  mode: "normal"
  dictionary: "embedded://ipadic"
  # user_dictionary: "./resources/user_dict/ipadic_simple_userdic.csv"
  # keep_whitespace: false
  # use_mmap: false # ファイルシステム辞書（embedded:// ではない辞書）にのみ意味がある
  # max_grouping_len: 24 # 未知語のグルーピング長の上限。省略または 0 で無制限
  # unknown_word_ladder: true # 短い未知語候補も生成する（デフォルト: true）

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

## Segmenter のオプション

| キー | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `mode` | string | `"normal"` | 分割モード。`"normal"` または `"decompose"` |
| `dictionary` | string | *(必須)* | 辞書 URI。例: `"embedded://ipadic"` |
| `user_dictionary` | string | *(なし)* | ユーザー辞書のパス |
| `keep_whitespace` | bool | `false` | 空白トークンを無視せずに出力する（MeCab は無視する） |
| `use_mmap` | bool | `mmap` feature が有効なとき on（デフォルト） | 辞書をメモリマップする。ファイルシステム辞書（`embedded://` ではない辞書）にのみ意味がある |
| `max_grouping_len` | integer | *(無制限)* | 未知語のグルーピングが**先頭の 1 文字を超えて**span できる最大文字数。MeCab の `max-grouping-size` に相当する（MeCab のデフォルトは 24）。これを超えるグルーピング候補は生成されず、代わりに 1 文字の未知語が出力される。キーを省略するか `0` を指定すると無制限になる |
| `unknown_word_ladder` | bool | `true` | MeCab や Vibrato と同様に、`char.def` の各カテゴリの `LENGTH` フィールドまでの短い未知語候補も生成する。v6 より前の出力を正確に再現するには `false` を指定する |

```shell
% export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera_analysis::tokenizer::TokenizerBuilder;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // 設定ファイルからトークナイザーの設定を読み込む
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
