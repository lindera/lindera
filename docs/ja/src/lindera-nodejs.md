# Lindera Node.js

Lindera Node.js は、[NAPI-RS](https://napi.rs/) を使用して構築された Lindera 形態素解析エンジンの Node.js バインディングです。Node.js `^20.17.0 || ^22.13.0 || >= 23.5.0` をサポートし、Lindera の高性能なトークナイズ機能を Node.js エコシステムに提供します。

## 特徴

- **多言語対応**: 日本語（IPADIC、IPADIC NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）のテキストをトークナイズ
- **テキスト処理パイプライン**: 文字フィルタとトークンフィルタを組み合わせて、柔軟な前処理・後処理が可能
- **CRF ベースの辞書学習**: アノテーション付きコーパスからカスタム形態素解析モデルを学習（`train` feature が必要）
- **複数のトークナイズモード**: 解析粒度に応じた Normal モードと Decompose モード
- **N-best トークナイズ**: コスト順にランク付けされた複数のトークナイズ候補を取得
- **ユーザー辞書**: システム辞書をカスタム語彙で拡張
- **TypeScript サポート**: 完全な型定義を同梱

## ドキュメント

- [インストール](./lindera-nodejs/installation.md) -- 前提条件、ビルド手順、feature フラグ
- [クイックスタート](./lindera-nodejs/quickstart.md) -- 最小限の使用例
- [Tokenizer API](./lindera-nodejs/tokenizer_api.md) -- `TokenizerBuilder`、`Tokenizer`、`Token` クラスリファレンス
- [辞書管理](./lindera-nodejs/dictionary_management.md) -- 辞書の読み込み、ビルド、管理
- [テキスト処理パイプライン](./lindera-nodejs/text_processing_pipeline.md) -- 文字フィルタとトークンフィルタ
- [学習](./lindera-nodejs/training.md) -- カスタム CRF モデルの学習と辞書のエクスポート

## エラー

失敗した関数・メソッドは `Error` を投げます。`message` は何が起きたかを示し、`code` は失敗の種類を表します。

| `code` | 失敗 | 例 |
| --- | --- | --- |
| `StringExpected` や `ArrayExpected` など、期待した型 | 型の違う引数 | `loadDictionary(123)`、`new Schema("a")` |
| `InvalidArg` | 期待したクラスのインスタンスでない引数、設定の範囲外の数値 | `setMode("normal")`、`new Metadata({ defaultLeftContextId: 2 ** 32 + 5 })` |
| `GenericFailure` | ほかの多くの失敗と、panic（後述） | `loadDictionary("/nonexistent/dictionary")`、`Mode.fromStr("Normal")` |

Lindera の Rust のコードの panic（Lindera のバグです）も、呼んだ関数・メソッドから `Error` として投げられ、Node.js は動き続けます。v6.2.0 までは、panic で Node.js が `fatal runtime error: failed to initiate panic, error 5, aborting` を出して abort していました（[#1314](https://github.com/lindera/lindera/issues/1314)）。panic の `Error` の `code` は、ほかの多くの失敗と同じ `GenericFailure` で、`message` は panic のメッセージです。Rust は、panic が起きたソースコードの位置とメッセージを標準エラーにも書きます。

メソッドが panic したオブジェクトは、panic の前にメソッドが変えた状態を持っていることがあるので、使い続けずに作り直してください。panic は、メッセージと原因になった入力を添えて [GitHub Issues](https://github.com/lindera/lindera/issues) に報告してください。
