# Lindera Trainer

Lindera Trainerは、CRFベースの辞書学習パイプライン（`lindera train`）を実装するクレートです。アノテーション付きコーパスと種辞書から学習済みモデルを生成し、そのモデルをMeCab形式の辞書ソースファイルにエクスポートしてバイナリ辞書としてビルドできるようにします。内部では`lindera-dictionary`のランタイム型と`lindera-crf`のCRFコアを利用しています。このクレートは直接利用することも、`lindera`ファサードの`train` feature経由（`lindera::dictionary::trainer`として再エクスポート）で利用することもできます。

## 主な特徴

- `lindera-crf`によるCRFベースの重み学習（L1、L2、Elastic Net正則化に対応）
- MeCab互換の素性テンプレート解析（`feature.def`: `%F[n]`、`%L[n]`、`%R[n]`、`%w`、`%u`、`%l`、`%r`、およびそれぞれの`?`付きオプション形式）
- MeCab互換の3セクション形式による素性書き換え（`rewrite.def`: ユニグラム／左文脈／右文脈の書き換えルール）
- 辞書形式に依存しない設計: `surface,left_id,right_id,cost,feature...`の列構成に従う辞書であれば任意のもの（IPADIC、UniDic、ko-dic、CC-CEDICTなど）を扱える
- `char.def`の文字カテゴリに基づく未知語の自動カテゴリ分類
- 学習されたCRFの重みから連接コスト行列を生成し、MeCab互換のコスト変換（`tocost`）を適用
- ゼロコピーの`rkyv`バイナリ形式によるモデルのシリアライズ（読み込み時はレガシーJSON形式へのフォールバックにも対応）
- Lindera／MeCab辞書ソースファイル（`lex.csv`、`matrix.def`、`unk.def`、`char.def`、`feature.def`、`rewrite.def`、`left-id.def`、`right-id.def`）の直接エクスポート

## 目次

- [アーキテクチャ](./lindera-trainer/architecture.md) -- 内部構造と主要コンポーネント
- [APIリファレンス](./lindera-trainer/api_reference.md) -- APIドキュメント
