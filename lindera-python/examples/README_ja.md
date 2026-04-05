# Lindera Python サンプル

## 事前準備

- Python >= 3.10
- Rust ツールチェイン
- [Maturin](https://www.maturin.rs/) (`pip install maturin`)

## ビルド

`lindera-python/` ディレクトリから実行します:

```bash
cd lindera-python

# IPADIC 辞書を埋め込んでビルド
maturin develop --features embed-ipadic

# 学習機能を含めてビルド
maturin develop --features embed-ipadic,train

# リリースビルド
maturin develop --release --features embed-ipadic
```

## サンプル

ビルド後、`lindera-python/` ディレクトリから実行します:

### 基本的なトークナイズ

```bash
python examples/tokenize.py
```

埋め込み IPADIC 辞書を使用し、"normal" モードで日本語テキストをトークナイズします。

### Decompose モード

```bash
python examples/tokenize_with_decompose.py
```

"decompose" モードでトークナイズし、複合形態素を分解します。

### フィルター付きトークナイズ

```bash
python examples/tokenize_with_filters.py
```

`TokenizerBuilder` API を使用し、文字フィルター（`unicode_normalize`、`japanese_iteration_mark`、`mapping`）とトークンフィルター（`japanese_katakana_stem`、`japanese_stop_tags`、`lowercase`、`japanese_base_form`）を適用するデモです。

### ユーザー辞書

```bash
python examples/tokenize_with_userdict.py
```

標準辞書に加えて、`resources/ipadic_simple_userdic.csv` からカスタムユーザー辞書を読み込みます。

### ソースからの辞書ビルド

```bash
python examples/build_ipadic.py
```

mecab-ipadic のソース tarball をダウンロードし、Lindera 辞書をビルドします。

### 学習とエクスポート（`train` feature が必要）

```bash
# 先に train feature を有効にしてビルド
maturin develop --features embed-ipadic,train

python examples/train_and_export.py
```

サンプルコーパスから CRF モデルを学習し、辞書ファイルをエクスポートします。
