# Lindera ライブラリ

`lindera` クレートは Rust API のファサードです。`lindera-segmenter` の形態素セグメンター（`lindera::segmenter`・`lindera::dictionary`・`lindera::mode` など）を再エクスポートし、デフォルトで有効な `analysis` feature により `lindera-analysis` の分析チェーンを `lindera::analysis` として提供します。必要な依存は `lindera = "7"` の 1 行だけで、`default-features = false` にするとセグメンターのみになります。このセクションでは、セグメンテーション、エラーハンドリング、APIリファレンスについて説明します。

`Tokenizer` や文字フィルタ・トークンフィルタ（`Segmenter` の上に構築されたLucene風の分析チェーン）が必要な場合は `lindera::analysis` を使います。[Lindera Analysis](./lindera-analysis.md)（[設定](./lindera-analysis/configuration.md)・[フィルタ](./lindera-analysis/filters.md)ページを含む）を参照してください。

- [Segmenter](./lindera/segmenter.md) - Viterbi アルゴリズムを使用するコアセグメンテーションコンポーネント
- [エラーハンドリング](./lindera/error_handling.md) - エラー型とハンドリングパターン
- [APIリファレンス](./lindera/api_reference.md) - 生成されたAPIドキュメントへのリンク
