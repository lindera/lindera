# Lindera Dependency

Lindera Dependency（`lindera-dependency`）は、日本語の文節の係り受け解析のためのクレートです。文の形態素を文節にまとめ、文節ごとに、その文節が係る先の文節を求めることを目指しています。

```text
彼は 本を 読んだ。
彼は ──────┐
本を ──────┤
読んだ。 ◀──┘   (root)
```

## 状態

> このクレートは**開発中で、crates.io にはまだ公開していません**。**文節の係り先を求める係り受けの解析器は、まだありません**。今あるのは、その手前のすべてです。型、学習データ、評価、文節の区切り、文節の主辞と機能語があります。

いま使うには、[リポジトリ](https://github.com/lindera/lindera)の checkout からビルドします。最初のリリースまでは、インターフェースが変わることがあります。作業は [#1417](https://github.com/lindera/lindera/issues/1417) で追っています。

## できること

- **文の形態素を文節に区切る**（チャンキング）。条件付き確率場と、重みの表だけのモデルを使います。IPADIC では、UD Japanese-GSD の dev で F 値 96.85、test で 96.92 です。
- **文節の主辞と機能語を求め**、句読点と括弧を見分ける。辞書のスキーマから決めます。最初のスキーマは IPADIC です。
- CaboCha の**ラティス形式**（`cabocha -f1`）**を読み書きし**、文が、解析器の仮定する木であるかを検査する。
- UD Japanese-GSD のような係り受けのコーパスから、解析器を動かす辞書の形態素の上で、**学習データを作る**。
- 文節の区切りと係り受けの精度を、CaboCha・J.DepP と同じ数え方で**評価し**、2 つの出力を対の検定で比べる。

既定のビルドは依存がなく、`wasm32-unknown-unknown` 向けにビルドできます。型、ラティス形式、スキーマ、文節の区切りが入っています。コマンド、学習、評価は `train` feature の裏にあります。

## 目次

- [アーキテクチャ](./lindera-dependency/architecture.md) -- モジュール、型、ラティス形式、feature フラグ
- [文節の区切り](./lindera-dependency/chunking.md) -- 形態素を文節に区切る、モデルを学習する
- [主辞と機能語](./lindera-dependency/head_and_function_word.md) -- 辞書のスキーマと、文節の 2 つの語
- [学習データ](./lindera-dependency/training_data.md) -- UD Japanese-GSD から学習データを作る
- [評価](./lindera-dependency/evaluation.md) -- 精度を数える、2 つの出力を比べる、交差検証

API リファレンスのページは、まだありません。クレートが docs.rs に無いためです。API のドキュメントは、`cargo doc -p lindera-dependency --features train --open` で手元にビルドできます。

このクレートの決定と、その根拠になった数字は、[設計メモ](./development/dependency_design.md)にあります。
