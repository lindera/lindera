# Lindera Dependency

Lindera Dependency（`lindera-dependency`）は、日本語の文節の係り受け解析のためのクレートです。文の形態素を文節にまとめ、文節ごとに、その文節が係る先の文節を求めます。

```text
彼は 本を 読んだ。
彼は ──────┐
本を ──────┤
読んだ。 ◀──┘   (root)
```

## 状態

> このクレートは、Lindera 7.1.0 から crates.io にあります。Lindera のリリースが、一緒に **IPADIC と 3 つの UniDic の辞書のモデル**を公開します。形態素から係り受けまでの解析が一通りできます。学習データ、文節の区切り、解析器、モデルファイル、評価、テキストを受けるコマンドが揃っています。Lindera のトークンフィルターは、`lindera` コマンドとバインディングの既定のビルドに入っています。

[インストール](./lindera-dependency/installation.md)と[クイックスタート](./lindera-dependency/quickstart.md)から始めてください。作業は [#1417](https://github.com/lindera/lindera/issues/1417) で追っています。

## できること

- **文の形態素を文節に区切る**（チャンキング）。条件付き確率場と、重みの表だけのモデルを使います。IPADIC では、UD Japanese-GSD の dev で F 値 96.85、test で 96.92 です。
- **モデルを辞書ごとの 1 つのファイルに持ち、その場で読む**。プログラムに埋め込んだバイト列、ファイルから読んだバイト列、メモリマップのどれでも、表を写さずに読みます。ファイルは自分の辞書の名前を持ち、その辞書のものではありえない形態素は断ります。
- **文節の係り受けを解析する**。CaboCha・J.DepP と同じく、shift-reduce と、素性とその対の上の分類器を使い、その分類器を学習します。IPADIC では、UD Japanese-GSD の dev で、正解の文節なら 90.11% の文節が正しい係り先になり（test は 88.53%）、形態素からなら 85.39% です（test は 84.47%）。
- **コマンドラインでテキストを解析する**。Lindera の辞書で分かち書きし、`cabocha -f1` と同じ形式で書き出します。
- **Lindera のトークンフィルターとして動く**。`japanese_dependency` を、`lindera` コマンド、設定ファイル、Rust、5 つのバインディングから使えます。各トークンの文節と係り先が、JSON の出力と `Token` に入ります。
- **文節の主辞と機能語を求め**、句読点と括弧を見分ける。辞書のスキーマから決めます。最初のスキーマは IPADIC です。
- CaboCha の**ラティス形式**（`cabocha -f1`）**を読み書きし**、文が、解析器の仮定する木であるかを検査する。
- UD Japanese-GSD のような係り受けのコーパスから、解析器を動かす辞書の形態素の上で、**学習データを作る**。
- **CaboCha が配布する 3 つのモデル（IPA・JUMAN・UniDic）を変換し**、CaboCha と同じ解析をするモデルファイルにする。そのモデルの利用は、CaboCha のモデルと同じく、研究目的と個人利用に限られる。
- 文節の区切りと係り受けの精度を、CaboCha・J.DepP と同じ数え方で**評価し**、2 つの出力を対の検定で比べる。

既定のビルドは依存がなく、`wasm32-unknown-unknown` 向けにビルドできます。型、ラティス形式、スキーマ、モデルファイル、文節の区切り、解析器が入っています。コマンドは `cli` feature の裏に、学習と評価は `train` feature の裏にあります。

## 目次

- [インストール](./lindera-dependency/installation.md) -- クレート、トークンフィルターの入ったビルド、モデルを手に入れる 3 つの方法
- [クイックスタート](./lindera-dependency/quickstart.md) -- 公開しているモデルで 1 つの文を解析する。コマンドラインと Rust から
- [アーキテクチャ](./lindera-dependency/architecture.md) -- モジュール、型、ラティス形式、feature フラグ
- [文節の区切り](./lindera-dependency/chunking.md) -- 形態素を文節に区切る、モデルを学習する
- [係り受けの解析](./lindera-dependency/parsing.md) -- 文節の係り先を求める。ループ、素性、モデルの学習、精度
- [モデルファイル](./lindera-dependency/model_file.md) -- 辞書のファイル。その場で読む、辞書を照合する、形式
- [CaboCha のモデル](./lindera-dependency/cabocha_models.md) -- CaboCha が配布するモデルを変換する。利用条件、コマンド、変換するもの、CaboCha との一致
- [コマンドライン](./lindera-dependency/command_line.md) -- `lindera-dependency` コマンドでテキストを解析する。オプション、入力の層、速度
- [トークンフィルター](./lindera-dependency/token_filter.md) -- Lindera のトークナイザーのフィルター `japanese_dependency`。ビルド、トークンに書かれるもの、文、フィルターの順、速度
- [主辞と機能語](./lindera-dependency/head_and_function_word.md) -- 辞書のスキーマと、文節の 2 つの語
- [学習データ](./lindera-dependency/training_data.md) -- UD Japanese-GSD から学習データを作る
- [評価](./lindera-dependency/evaluation.md) -- 精度を数える、2 つの出力を比べる、交差検証

- [API リファレンス](./lindera-dependency/api_reference.md) -- docs.rs にあるクレートのドキュメント

このクレートの決定と、その根拠になった数字は、[設計メモ](./development/dependency_design.md)にあります。
