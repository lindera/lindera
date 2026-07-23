# フィルタ

文字フィルタ（Character Filter）とトークンフィルタ（Token Filter）は、`lindera-analysis`の`Tokenizer`パイプラインにおける前処理・後処理の2つの段階です。

- **文字フィルタ**はセグメンテーションの*前に*入力テキストを変換します。バイトオフセットは自動的に補正されるため、変換後のテキストに対して生成されたトークンでも、元のフィルタ前のテキストにおける位置が正しく報告されます。
- **トークンフィルタ**はSegmenterが生成したトークンのリストをセグメンテーションの*後に*変換します。

どちらのフィルタも設定方法は共通で、`kind`文字列（CLIの`--character-filter` / `--token-filter`フラグでも`kind:{"json": "args"}`という形式で使用されます）と、フィルタ固有のパラメータを持つJSONの`args`オブジェクトで構成されます。

## 文字フィルタ

文字フィルタはYAML設定ファイルの`character_filters`キーで設定します。各エントリは順番に適用され、あるフィルタの出力が次のフィルタの入力になります。

### unicode_normalize

4種類の標準的なUnicode正規化形式のいずれかを使って入力テキストを正規化します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `kind` | string | はい | `nfc`、`nfd`、`nfkc`、`nfkd`のいずれか |

**例:**

```json
{
  "kind": "unicode_normalize",
  "args": {
    "kind": "nfkc"
  }
}
```

### japanese_iteration_mark

日本語の踊り字（繰り返し記号）である`々`、`ゝ`、`ゞ`、`ヽ`、`ヾ`を、それぞれが繰り返す文字に置き換えて正規化します。ひらがな・カタカナの繰り返し記号については、必要に応じて濁点の付与・除去も行います。

**パラメータ:**

| パラメータ | 型 | 必須 | デフォルト | 説明 |
| --- | --- | --- | --- | --- |
| `normalize_kanji` | bool | いいえ | `false` | 漢字の踊り字`々`を正規化する |
| `normalize_kana` | bool | いいえ | `false` | ひらがな・カタカナの踊り字`ゝ`、`ゞ`、`ヽ`、`ヾ`を正規化する |

**例:**

```json
{
  "kind": "japanese_iteration_mark",
  "args": {
    "normalize_kanji": true,
    "normalize_kana": true
  }
}
```

### mapping（文字フィルタ）

`mapping`のキーに一致する部分を対応する値に置き換えます。入力テキスト全体に対して、Aho-Corasickオートマトンによる最長一致検索を行います。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `mapping` | object（string to string） | はい | 置換対象の部分文字列と、その置換先の対応表 |

**例:**

```json
{
  "kind": "mapping",
  "args": {
    "mapping": {
      "リンデラ": "Lindera"
    }
  }
}
```

### regex

正規表現にマッチした箇所をすべて、リテラルな置換文字列で置き換えます。キャプチャグループの内容は置換文字列に展開されません。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `pattern` | string | はい | 正規表現（[`regex`](https://docs.rs/regex)クレートの構文） |
| `replacement` | string | はい | `pattern`にマッチした箇所すべてを置き換えるリテラル文字列 |

**例:**

```json
{
  "kind": "regex",
  "args": {
    "pattern": "\\s{2,}",
    "replacement": " "
  }
}
```

## トークンフィルタ

トークンフィルタはYAML設定ファイルの`token_filters`キーで設定します。各フィルタは、Segmenterが生成したトークンリストに対して順番に適用されます。

### japanese_base_form

トークンの表層形を、辞書の`base_form`（または`orthographic_base_form`）フィールドに登録された原形（辞書形）に置き換えます。動詞・形容詞のレンマ化（見出し語化）として機能します。最初の詳細情報が`UNK`（未知語）のトークンは変更されません。

このフィルタに設定パラメータはありません。

**例:**

```json
{
  "kind": "japanese_base_form"
}
```

### japanese_compound_word

品詞タグが`tags`のいずれかに一致する連続したトークンを、1つの複合語トークンに結合します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\> | はい | 結合対象となるトークンを示す品詞タグ（カンマ区切りで最大4階層） |
| `new_tag` | string | いいえ | 結合後のトークンに付与する品詞タグ。省略した場合は`複合語`が付与される |

**例:**

```json
{
  "kind": "japanese_compound_word",
  "args": {
    "tags": [
      "名詞,数",
      "名詞,接尾,助数詞"
    ],
    "new_tag": "名詞,数"
  }
}
```

### japanese_kana

トークンテキストをひらがなとカタカナの間で相互変換します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `kind` | string | はい | `"hiragana"`はカタカナをひらがなに、`"katakana"`はひらがなをカタカナに変換する |

**例:**

```json
{
  "kind": "japanese_kana",
  "args": {
    "kind": "hiragana"
  }
}
```

### japanese_katakana_stem

カタカナのトークンの末尾にある長音記号（`ー`、U+30FC）を除去します。ただし、トークンの文字数が`min`より大きい場合のみ除去されます。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `min` | 正の整数 | はい | 末尾の長音記号をステミングする対象となる、カタカナトークンの最小文字数 |

**例:**

```json
{
  "kind": "japanese_katakana_stem",
  "args": {
    "min": 3
  }
}
```

### japanese_keep_tags

品詞タグが`tags`のいずれかに一致するトークンのみを保持し、それ以外を除去します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\> | はい | 保持する品詞タグ（カンマ区切りで最大4階層） |

**例:**

```json
{
  "kind": "japanese_keep_tags",
  "args": {
    "tags": [
      "名詞",
      "名詞,一般",
      "名詞,固有名詞"
    ]
  }
}
```

### japanese_number

トークンの表層テキストに含まれる日本語の数値表現（漢数字、大字、全角数字）をアラビア数字に変換します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\>または`null` | いいえ | 変換対象を限定する品詞タグ（カンマ区切りで最大4階層）。省略または`null`の場合はすべてのトークンが変換対象になる |

**例:**

```json
{
  "kind": "japanese_number",
  "args": {
    "tags": [
      "名詞,数"
    ]
  }
}
```

### japanese_reading_form

トークンの表層テキストを、辞書の`reading`フィールドに登録された読み（カタカナ）に置き換えます。最初の詳細情報が`UNK`（未知語）のトークンは変更されません。

このフィルタに設定パラメータはありません。

**例:**

```json
{
  "kind": "japanese_reading_form"
}
```

### japanese_stop_tags

品詞タグが`tags`のいずれかに一致するトークンを除去します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\> | はい | 除去する品詞タグ（カンマ区切りで最大4階層） |

**例:**

```json
{
  "kind": "japanese_stop_tags",
  "args": {
    "tags": [
      "助詞",
      "助動詞",
      "記号"
    ]
  }
}
```

### keep_words

表層テキストが`words`のいずれかに完全一致するトークンのみを保持します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `words` | array\<string\> | はい | 保持する表層形の一覧 |

**例:**

```json
{
  "kind": "keep_words",
  "args": {
    "words": [
      "すもも",
      "もも"
    ]
  }
}
```

### korean_keep_tags

最初の品詞タグが`tags`のいずれかに一致する韓国語トークンのみを保持します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\> | はい | 保持する品詞タグ |

**例:**

```json
{
  "kind": "korean_keep_tags",
  "args": {
    "tags": [
      "NNG"
    ]
  }
}
```

### korean_reading_form

トークンの表層テキストを、辞書の`reading`フィールドに登録された読みに置き換えます。最初の詳細情報が`UNK`（未知語）のトークンは変更されません。

このフィルタに設定パラメータはありません。

**例:**

```json
{
  "kind": "korean_reading_form"
}
```

### korean_stop_tags

最初の品詞タグが`tags`のいずれかに一致する韓国語トークンを除去します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tags` | array\<string\> | はい | 除去する品詞タグ |

**例:**

```json
{
  "kind": "korean_stop_tags",
  "args": {
    "tags": [
      "EP",
      "EF",
      "JKG"
    ]
  }
}
```

### length

表層テキストの文字数が`[min, max]`の範囲に収まるトークンのみを保持します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `min` | 符号なし整数 | いいえ | 最小文字数（この値を含む） |
| `max` | 符号なし整数 | いいえ | 最大文字数（この値を含む） |

**例:**

```json
{
  "kind": "length",
  "args": {
    "min": 2,
    "max": 3
  }
}
```

### lowercase

トークンの表層テキストを小文字に変換します。

このフィルタに設定パラメータはありません。

**例:**

```json
{
  "kind": "lowercase"
}
```

### mapping（トークンフィルタ）

`mapping`のキーに一致する部分を、各トークンの表層テキスト内で対応する値に置き換えます。Aho-Corasickオートマトンによる最長一致検索を使用します。文字フィルタの`mapping`のトークン版に相当します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `mapping` | object（string to string） | はい | 置換対象の部分文字列と、その置換先の対応表 |

**例:**

```json
{
  "kind": "mapping",
  "args": {
    "mapping": {
      "籠": "篭"
    }
  }
}
```

### remove_diacritical_mark

トークンの表層テキストからダイアクリティカルマーク（発音区別符号）を除去し、その後テキストの元のUnicode正規化形式を再適用します。

**パラメータ:**

| パラメータ | 型 | 必須 | デフォルト | 説明 |
| --- | --- | --- | --- | --- |
| `japanese` | bool | いいえ | `false` | 日本語の濁点・半濁点の結合文字（分解済みの濁音・半濁音仮名に含まれるものなど）も除去する |

**例:**

```json
{
  "kind": "remove_diacritical_mark",
  "args": {
    "japanese": false
  }
}
```

### stop_words

表層テキストが`words`のいずれかに完全一致するトークンを除去します。

**パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `words` | array\<string\> | はい | 除去する表層形の一覧 |

**例:**

```json
{
  "kind": "stop_words",
  "args": {
    "words": [
      "も",
      "の"
    ]
  }
}
```

### uppercase

トークンの表層テキストを大文字に変換します。

このフィルタに設定パラメータはありません。

**例:**

```json
{
  "kind": "uppercase"
}
```

## YAML設定

文字フィルタとトークンフィルタは、Segmenterと一緒に1つのYAMLファイルで設定します。ファイル全体の形式は[設定](./configuration.md)を参照してください。関連する部分だけを抜粋すると次のようになります。

```yaml
character_filters:
  - kind: "unicode_normalize"
    args:
      kind: "nfkc"
  - kind: "japanese_iteration_mark"
    args:
      normalize_kanji: true
      normalize_kana: true

token_filters:
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "助詞"
        - "助動詞"
        - "記号"
  - kind: "japanese_katakana_stem"
    args:
      min: 3
  - kind: "lowercase"
  - kind: "length"
    args:
      min: 2
```

## Rust API

文字フィルタとトークンフィルタは、プログラムから作成・適用することもできます。

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::character_filter::BoxCharacterFilter;
use lindera_analysis::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera_analysis::token_filter::BoxTokenFilter;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera_analysis::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
use lindera_analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let mut tokenizer = Tokenizer::new(segmenter);

    // 文字フィルタを追加
    let normalize_filter = UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);
    tokenizer.append_character_filter(BoxCharacterFilter::from(normalize_filter));

    // トークンフィルタを追加
    let stop_tags_filter = JapaneseStopTagsTokenFilter::new(
        vec![
            "助詞".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
        ]
        .into_iter()
        .collect(),
    );
    tokenizer.append_token_filter(BoxTokenFilter::from(stop_tags_filter));

    let katakana_stem_filter =
        JapaneseKatakanaStemTokenFilter::new(std::num::NonZeroUsize::new(3).unwrap());
    tokenizer.append_token_filter(BoxTokenFilter::from(katakana_stem_filter));

    // フィルタを適用してトークナイズ
    let tokens = tokenizer.tokenize("Linderaは形態素解析エンジンです。")?;

    for token in tokens {
        println!(
            "token: {:?}, details: {:?}",
            token.surface, token.details
        );
    }

    Ok(())
}
```

`append_character_filter`と`append_token_filter`メソッドは、フィルタを追加した順番で登録します。文字フィルタはセグメンテーション前のテキストに対して順次適用され、トークンフィルタはセグメンテーション後のトークンリストに対して順次適用されます。
