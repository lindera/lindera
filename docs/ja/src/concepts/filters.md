# フィルタ

Linderaの解析パイプライン（[`lindera-analysis`](../lindera-analysis.md)クレートが提供する`Tokenizer`）には、Segmenterの前後に2つの拡張ポイントがあります。トークナイズ**前**の生テキストを変換する**文字フィルタ**と、トークナイズ**後**のトークン列を変換する**トークンフィルタ**です。

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

## 文字フィルタ

文字フィルタは、入力テキストがSegmenterに渡される前に前処理を行います。全角文字を半角に変換する、Unicode表現を正準化する、日本語の踊り字を展開形に変換するなど、トークナイズの一貫性を高めるためのテキスト正規化に主に使われます。文字フィルタはテキストの長さを変えることがあるため、Linderaはすべての変換を記録し、各トークンのバイトオフセットを元の（フィルタ適用前の）テキストにおける位置へ補正します。

## トークンフィルタ

トークンフィルタは、Segmenterが生成したトークン列に対して後処理を行います。トークンを基本形（辞書形）に置き換える、トークンの表層形をひらがな・カタカナ間で変換する、品詞タグでトークンを除去する、ストップワードを除去するなど、検索や解析のためにトークン列を正規化・削減する目的で主に使われます。

## フィルタの設定方法

文字フィルタ・トークンフィルタのいずれも、`kind`文字列で識別され、フィルタ固有のパラメータを持つJSONの`args`オブジェクトで設定します。フィルタは追加された順序で実行され、追加方法は2通りあります。

- **YAML設定ファイル**: `character_filters`キーと`token_filters`キーの下にフィルタを列挙する。

  ```yaml
  character_filters:
    - kind: unicode_normalize
      args:
        kind: nfkc

  token_filters:
    - kind: japanese_base_form
  ```

- **Rust API**: `append_character_filter`と`append_token_filter`で`Tokenizer`に順番にフィルタを追加する。

  ```rust
  // Character filters run first, transforming the raw input text;
  // token filters run last, transforming the resulting token list.
  tokenizer
      .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
      .append_token_filter(BoxTokenFilter::from(japanese_base_form_filter));
  ```

## 利用可能なフィルタ

Linderaは4種類の文字フィルタと18種類のトークンフィルタを提供しており、日本語・韓国語・汎用的なテキスト正規化をカバーしています。

| カテゴリ | フィルタ |
| --- | --- |
| 文字フィルタ | `unicode_normalize`, `japanese_iteration_mark`, `mapping`, `regex` |
| トークンフィルタ -- 正規化 | `japanese_base_form`, `japanese_reading_form`, `korean_reading_form`, `japanese_kana`, `japanese_katakana_stem`, `japanese_number`, `mapping`, `remove_diacritical_mark`, `lowercase`, `uppercase` |
| トークンフィルタ -- 品詞によるフィルタリング | `japanese_keep_tags`, `japanese_stop_tags`, `korean_keep_tags`, `korean_stop_tags` |
| トークンフィルタ -- 単語によるフィルタリング | `keep_words`, `stop_words` |
| トークンフィルタ -- 構造の変換 | `japanese_compound_word`, `length` |

各フィルタの説明・パラメータ・実行可能なYAML/Rust APIの例は[フィルターリファレンス](../lindera-analysis/filters.md)を参照してください。
