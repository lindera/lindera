# アーキテクチャ

## モジュール構成

```text
lindera-analysis/src/
├── lib.rs                          # パブリックAPIの再エクスポート、CLIフラグ解析ヘルパー
├── character_filter.rs              # CharacterFilter trait、OffsetMapping、CharacterFilterLoader
├── character_filter/
│   ├── unicode_normalize.rs         # Unicode正規化（NFC/NFD/NFKC/NFKD）
│   ├── japanese_iteration_mark.rs   # 日本語の踊り字（繰り返し記号）正規化
│   ├── mapping.rs                   # マッピングによるテキスト置換
│   └── regex.rs                     # 正規表現によるテキスト置換
├── token_filter.rs                  # TokenFilter trait、TokenFilterLoader
├── token_filter/
│   ├── japanese_base_form.rs
│   ├── japanese_compound_word.rs
│   ├── japanese_kana.rs
│   ├── japanese_katakana_stem.rs
│   ├── japanese_keep_tags.rs
│   ├── japanese_number.rs
│   ├── japanese_reading_form.rs
│   ├── japanese_stop_tags.rs
│   ├── keep_words.rs
│   ├── korean_keep_tags.rs
│   ├── korean_reading_form.rs
│   ├── korean_stop_tags.rs
│   ├── length.rs
│   ├── lowercase.rs
│   ├── mapping.rs
│   ├── remove_diacritical_mark.rs
│   ├── stop_words.rs
│   ├── tags.rs                      # keep/stop系タグフィルタが共有するヘルパー（非公開）
│   └── uppercase.rs
└── tokenizer.rs                      # Tokenizer、TokenizerBuilder
```

## 主要コンポーネント

### CharacterFilter

セグメンテーション前にテキストを前処理するフィルタのtraitです。各実装は`name()`と、`text`をその場で書き換えて実施した変換内容を`OffsetMapping`として返す`apply(&self, text: &mut String) -> LinderaResult<OffsetMapping>`を提供します。

`OffsetMapping`（`Transformation`レコードのリストから構築される）により、複数のフィルタを順に適用した後でも、`Tokenizer`はフィルタ後のテキストに対して計算されたトークンのバイトオフセットを、元の入力テキストにおけるバイトオフセットへ変換できます。`BoxCharacterFilter`は任意の`CharacterFilter`実装をボックス化・クローン可能なトレイトオブジェクトとしてラップし、`CharacterFilterLoader`は`kind`文字列と`serde_json::Value`の引数からフィルタを構築します（YAML設定の読み込みとCLIフラグ解析の両方で利用されます）。

### TokenFilter

Segmenterが生成したトークンを後処理するフィルタのtraitです。各実装は`name()`と、トークンをその場で変換・結合・並べ替え・除去する`apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()>`を提供します。`BoxTokenFilter`は任意の`TokenFilter`実装をボックス化・クローン可能なトレイトオブジェクトとしてラップし、`TokenFilterLoader`は`CharacterFilterLoader`と同様に、`kind`文字列と`serde_json::Value`の引数からフィルタを構築します。

### Tokenizer / TokenizerBuilder

`Tokenizer`は、文字フィルタ、[`lindera::segmenter::Segmenter`](../lindera/segmenter.md)、トークンフィルタを1つの解析パイプラインとして組み合わせます。`tokenize`を呼び出すと、入力テキストに文字フィルタを適用し、フィルタ後のテキストをセグメンテーションし、得られたトークンにトークンフィルタを適用したうえで、記録済みの`OffsetMapping`を使って各トークンのバイトオフセットを元のテキストに対する値へ補正します。

`TokenizerBuilder`は`TokenizerConfig`（`serde_json::Value`）から`Tokenizer`を組み立てます。この設定はプログラムから直接構築することも、YAMLファイルから読み込むこと（`TokenizerBuilder::from_file`、または環境変数`LINDERA_CONFIG_PATH`経由で自動的に読み込む`TokenizerBuilder::new`）も、`set_segmenter_mode`・`set_segmenter_dictionary`・`append_character_filter`・`append_token_filter`で段階的に組み立てることもできます。YAMLファイルの形式は[設定](./configuration.md)を、フィルタの完全なリファレンスは[フィルタ](./filters.md)を参照してください。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `embed-ipadic` | IPADIC辞書をバイナリに埋め込む（`lindera/embed-ipadic`へ委譲） | No |
| `embed-ipadic-neologd` | IPADIC-NEologd辞書をバイナリに埋め込む（`lindera/embed-ipadic-neologd`へ委譲） | No |
| `embed-unidic` | UniDic辞書をバイナリに埋め込む（`lindera/embed-unidic`へ委譲） | No |
| `embed-ko-dic` | ko-dic辞書をバイナリに埋め込む（`lindera/embed-ko-dic`へ委譲） | No |
| `embed-cc-cedict` | CC-CEDICT辞書をバイナリに埋め込む（`lindera/embed-cc-cedict`へ委譲） | No |
| `embed-jieba` | Jieba辞書をバイナリに埋め込む（`lindera/embed-jieba`へ委譲） | No |
