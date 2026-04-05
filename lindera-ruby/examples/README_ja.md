# Lindera Ruby サンプル

## 事前準備

- Ruby >= 3.1
- Rust ツールチェイン
- Bundler (`gem install bundler`)

## ビルド

`lindera-ruby/` ディレクトリから実行します:

```bash
cd lindera-ruby
bundle install

# IPADIC 辞書を埋め込んでビルド
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile

# 学習機能を含めてビルド
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

コンパイル済みの拡張は Rake により `lib/lindera/` に配置されます。

## サンプル

ビルド後、`lindera-ruby/` ディレクトリから実行します:

### 基本的なトークナイズ

```bash
bundle exec ruby examples/tokenize.rb
```

埋め込み IPADIC 辞書を使用し、"normal" モードで日本語テキストをトークナイズします。

### Decompose モード

```bash
bundle exec ruby examples/tokenize_with_decompose.rb
```

"decompose" モードでトークナイズし、複合形態素を分解します。

### フィルター付きトークナイズ

```bash
bundle exec ruby examples/tokenize_with_filters.rb
```

`TokenizerBuilder` API を使用し、文字フィルター（`unicode_normalize`、`japanese_iteration_mark`、`mapping`）とトークンフィルター（`japanese_katakana_stem`、`japanese_stop_tags`、`lowercase`、`japanese_base_form`）を適用するデモです。

### ユーザー辞書

```bash
bundle exec ruby examples/tokenize_with_userdict.rb
```

標準辞書に加えて、`resources/ipadic_simple_userdic.csv` からカスタムユーザー辞書を読み込みます。

### ソースからの辞書ビルド

```bash
bundle exec ruby examples/build_ipadic.rb
```

mecab-ipadic のソース tarball をダウンロードし、Lindera 辞書をビルドします。`tar` コマンドが必要です。

### 学習とエクスポート（`train` feature が必要）

```bash
# 先に train feature を有効にしてビルド
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile

bundle exec ruby examples/train_and_export.rb
```

サンプルコーパスから CRF モデルを学習し、辞書ファイルをエクスポートします。
