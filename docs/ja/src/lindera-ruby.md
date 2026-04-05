# Lindera Ruby

Lindera Ruby は、[Magnus](https://github.com/matsadler/magnus) と [rb-sys](https://github.com/oxidize-rb/rb-sys) を使用して構築された Lindera 形態素解析エンジンの Ruby バインディングです。Ruby 3.1 以降をサポートし、Lindera の高性能なトークナイズ機能を Ruby エコシステムに提供します。

## 特徴

- **多言語対応**: 日本語（IPADIC、IPADIC NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）のテキストをトークナイズ
- **テキスト処理パイプライン**: 文字フィルタとトークンフィルタを組み合わせて、柔軟な前処理・後処理が可能
- **CRF ベースの辞書学習**: アノテーション付きコーパスからカスタム形態素解析モデルを学習（`train` feature が必要）
- **複数のトークナイズモード**: 解析粒度に応じた Normal モードと Decompose モード
- **N-best トークナイズ**: コスト順にランク付けされた複数のトークナイズ候補を取得
- **ユーザー辞書**: システム辞書をカスタム語彙で拡張

## ドキュメント

- [インストール](./lindera-ruby/installation.md) -- 前提条件、ビルド手順、feature フラグ
- [クイックスタート](./lindera-ruby/quickstart.md) -- 最小限の使用例
- [Tokenizer API](./lindera-ruby/tokenizer_api.md) -- `TokenizerBuilder`、`Tokenizer`、`Token` クラスリファレンス
- [辞書管理](./lindera-ruby/dictionary_management.md) -- 辞書の読み込み、ビルド、管理
- [テキスト処理パイプライン](./lindera-ruby/text_processing_pipeline.md) -- 文字フィルタとトークンフィルタ
- [学習](./lindera-ruby/training.md) -- カスタム CRF モデルの学習と辞書のエクスポート
