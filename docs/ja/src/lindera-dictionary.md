# Lindera Dictionary

Lindera Dictionaryは、形態素解析辞書のベースライブラリです。辞書の読み込み、ビルド、Viterbiベースのセグメンテーション、およびCRFベースの学習機能を提供します。

## 主な特徴

- ファイルシステムまたは埋め込みデータからの辞書読み込み
- MeCab形式のCSVソースファイルからの辞書ビルド
- 最適なセグメンテーションのためのViterbiアルゴリズム
- N-bestパス生成（Forward-DP Backward-A*）
- メモリマップドファイルサポート
- CRFベースの辞書学習（`train` feature使用時）

## 目次

- [アーキテクチャ](./lindera-dictionary/architecture.md) -- 内部構造と主要コンポーネント
- [APIリファレンス](./lindera-dictionary/api_reference.md) -- APIドキュメント
