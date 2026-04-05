# Lindera PHP サンプル

## 事前準備

- PHP >= 8.1
- Rust ツールチェイン
- Composer（`lindera-php/` で `composer install` を実行）
- `ext-php-rs` 互換環境（[ext-php-rs ドキュメント](https://github.com/davidcole1340/ext-php-rs)を参照）

## ビルド

リポジトリのルート（`lindera/`）から PHP 拡張をビルドします:

```bash
# IPADIC 辞書を埋め込んでビルド（debug）
cargo build -p lindera-php --features embed-ipadic

# 学習機能を含めてビルド（debug）
cargo build -p lindera-php --features embed-ipadic,train

# リリースビルド
cargo build -p lindera-php --release --all-features
```

拡張は `target/debug/liblindera_php.so`（リリースビルドの場合は `target/release/liblindera_php.so`）に生成されます。

## サンプル

`lindera-php/` ディレクトリから実行します:

```bash
cd lindera-php
```

### 基本的なトークナイズ

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize.php
```

埋め込み IPADIC 辞書を使用し、"normal" モードで日本語テキストをトークナイズします。

### Decompose モード

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_decompose.php
```

"decompose" モードでトークナイズし、複合形態素を分解します。

### フィルター付きトークナイズ

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_filters.php
```

`TokenizerBuilder` API を使用し、文字フィルター（`unicode_normalize`、`japanese_iteration_mark`、`mapping`）とトークンフィルター（`japanese_katakana_stem`、`japanese_stop_tags`、`lowercase`、`japanese_base_form`）を適用するデモです。

### ユーザー辞書

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_userdict.php
```

標準辞書に加えて、`resources/ipadic_simple_userdic.csv` からカスタムユーザー辞書を読み込みます。

### ソースからの辞書ビルド

```bash
php -d extension=../target/debug/liblindera_php.so examples/build_ipadic.php
```

mecab-ipadic のソース tarball をダウンロードし、Lindera 辞書をビルドします。

### 学習とエクスポート（`train` feature が必要）

```bash
# 先に train feature を有効にしてビルド（リポジトリのルートから）
cargo build -p lindera-php --features embed-ipadic,train

php -d extension=../target/debug/liblindera_php.so examples/train_and_export.php
```

サンプルコーパスから CRF モデルを学習し、辞書ファイルをエクスポートします。

### Web アプリケーション

```bash
php -d extension=../target/debug/liblindera_php.so -S localhost:8080 examples/tokenize_app.php
```

ブラウザで <http://localhost:8080> を開きます。モード選択（normal/decompose）付きのインタラクティブな形態素解析 Web UI を提供します。
