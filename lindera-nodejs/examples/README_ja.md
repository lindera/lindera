# Lindera Node.js サンプル

## 事前準備

- Node.js (v18以上)
- Rust ツールチェイン
- NAPI CLI (`npm install -g @napi-rs/cli`)

以下のコマンドは、特に記載がない限りリポジトリのルート（`lindera/`）で実行してください。

## ビルド

npm 依存パッケージをインストールし、ネイティブモジュールをビルドします:

```bash
cd lindera-nodejs
npm install

# IPADIC 辞書を埋め込んでビルド（debug）
npx napi build --platform -p lindera-nodejs --features embed-ipadic

# 学習機能を含めてビルド（debug）
npx napi build --platform -p lindera-nodejs --features embed-ipadic,train

# リリースビルド
npx napi build --platform --release -p lindera-nodejs --features embed-ipadic
```

## サンプル

ビルド後、`lindera-nodejs/` ディレクトリから実行します:

```bash
cd lindera-nodejs
```

### 基本的なトークナイズ

```bash
node examples/tokenize.js
```

埋め込み IPADIC 辞書を使用し、"normal" モードで日本語テキストをトークナイズします。

### Decompose モード

```bash
node examples/tokenize_with_decompose.js
```

"decompose" モードでトークナイズし、複合形態素を分解します。

### フィルター付きトークナイズ

```bash
node examples/tokenize_with_filters.js
```

`TokenizerBuilder` API を使用し、文字フィルター（`unicode_normalize`、`japanese_iteration_mark`、`mapping`）とトークンフィルター（`japanese_katakana_stem`、`japanese_stop_tags`、`lowercase`、`japanese_base_form`）を適用するデモです。

### ユーザー辞書

```bash
node examples/tokenize_with_userdict.js
```

標準辞書に加えて、`resources/ipadic_simple_userdic.csv` からカスタムユーザー辞書を読み込みます。

### ソースからの辞書ビルド

```bash
node examples/build_ipadic.js
```

mecab-ipadic のソース tarball をダウンロードし、Lindera 辞書をビルドします。`tar` コマンドが必要です。

### 学習とエクスポート（`train` feature が必要）

```bash
# 先に train feature を有効にしてビルド
npx napi build --platform -p lindera-nodejs --features embed-ipadic,train

node examples/train_and_export.js
```

サンプルコーパスから CRF モデルを学習し、辞書ファイルをエクスポートします。
