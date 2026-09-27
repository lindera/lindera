# Lindera Analysis

Lindera Analysisは、`lindera-segmenter`クレートが提供する形態素セグメンター（Segmenter）の上に、Lucene流のテキスト解析チェーンを重ねるクレートです。通常は[`lindera`](./lindera.md)ファサード経由で`lindera::analysis`として使います（デフォルトで有効な`analysis` feature）。文字フィルタ、`Segmenter`、トークンフィルタを1つの`Tokenizer`パイプラインとして組み合わせ、Rustコードから組み立てることも、YAML設定ファイルだけで組み立てることもできます。

## 主な特徴

- **文字フィルタ**: セグメンテーション前に入力テキストを変換し、バイトオフセットは元のテキストに対して自動的に補正される
- **トークンフィルタ**: Segmenterが生成したトークンの変換・結合・除去・並べ替えを行う
- **`Tokenizer` / `TokenizerBuilder`**: Rustコード、またはYAMLファイル（`LINDERA_CONFIG_PATH`）から解析パイプライン全体を組み立てる
- 日本語・韓国語・汎用のテキスト正規化をカバーする豊富な組み込みフィルタ

## 依存関係

ファサード経由（推奨）:

```toml
[dependencies]
lindera = "7"
```

```rust
use lindera::analysis::tokenizer::Tokenizer;
```

直接依存にする場合。同じクレートなので、どちらのパスも同じ型を指します。`lindera`と`lindera-analysis`のメジャーバージョンは揃えてください（[v6 から v7 への移行](./migration_v6_to_v7.md#lindera-と-lindera-analysis-は同時に上げる)を参照）:

```toml
[dependencies]
lindera = "7"
lindera-analysis = "7"
```

```rust
use lindera_analysis::tokenizer::Tokenizer;
```

## 目次

- [設定](./lindera-analysis/configuration.md) -- `Tokenizer`のYAML設定ファイル形式
- [フィルタ](./lindera-analysis/filters.md) -- すべての文字フィルタ・トークンフィルタのリファレンス
- [アーキテクチャ](./lindera-analysis/architecture.md) -- 内部構造と主要コンポーネント
- [APIリファレンス](./lindera-analysis/api_reference.md) -- APIドキュメント
