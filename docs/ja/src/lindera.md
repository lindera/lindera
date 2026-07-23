# Lindera ライブラリ

`lindera` クレートは純粋な形態素セグメンターです。辞書クレートを統合し、`Segmenter` API を提供します。デフォルトでは `lindera-analysis`・`lindera-crf`・`lindera-trainer` に依存しません。このセクションでは、セグメンテーション、エラーハンドリング、APIリファレンスについて説明します。

`Tokenizer` や文字フィルタ・トークンフィルタ（`Segmenter` の上に構築されたLucene風の分析チェーン）が必要な場合は、別クレートの[Lindera Analysis](./lindera-analysis.md)（[設定](./lindera-analysis/configuration.md)・[フィルタ](./lindera-analysis/filters.md)ページを含む）を参照してください。

- [Segmenter](./lindera/segmenter.md) - Viterbi アルゴリズムを使用するコアセグメンテーションコンポーネント
- [エラーハンドリング](./lindera/error_handling.md) - エラー型とハンドリングパターン
- [APIリファレンス](./lindera/api_reference.md) - 生成されたAPIドキュメントへのリンク
