# Lindera CRF

Lindera CRFは、[rucrf](https://github.com/daac-tools/rucrf)からフォークされたConditional Random Fields（CRF）のpure Rust実装です。ラティス構造をサポートしたCRFの学習器と推定器を提供します。

## 主な特徴

- 可変長エッジを持つラティス構造
- L1、L2、およびElastic Net正則化
- マルチスレッド学習
- rkyvによるゼロコピーデシリアライゼーション
- `no_std`サポート（`train` featureなしの場合）

## 目次

- [アーキテクチャ](./lindera-crf/architecture.md) -- 内部構造と主要コンポーネント
- [APIリファレンス](./lindera-crf/api_reference.md) -- APIドキュメント

## rucrfからの変更点

- **シリアライゼーションバックエンド**: ゼロコピーデシリアライゼーションのため、`bincode`から`rkyv`に変更
- **Elastic Net正則化**: L1とL2のペナルティを組み合わせた`Regularization::ElasticNet`を追加
- **Rust 2024 edition**: Rust 2024 editionに更新
- **依存クレートの更新**: `argmin`、`argmin-math`、`hashbrown`などを更新
