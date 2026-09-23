# インストール

lindera-php は PHP 拡張です。[Packagist](https://packagist.org/packages/lindera/lindera) に `lindera/lindera` として公開されており、PHP 拡張インストーラーの [PIE](https://github.com/php/pie) でインストールします。拡張名は `lindera` です。

## 前提条件

- **PHP 8.1 以降**（Linux または macOS）。Windows は非対応です
- **PIE** -- [PIE のインストール方法](https://github.com/php/pie/blob/main/docs/usage.md)を参照
- ソースからのビルド（現在 PIE が行う方法）には、**Rust ツールチェーン**（[rustup](https://rustup.rs/)）、**libclang**（Debian/Ubuntu は `libclang-dev`、macOS は Xcode コマンドラインツールまたは `brew install llvm`）、および PHP 拡張の標準的なビルドツール（`autoconf`、`libtool`、`make`）が必要です

## PIE でのインストール

```bash
pie install lindera/lindera
```

PIE がソースを取得し、検出した PHP 向けに拡張をビルドして（別の PHP を対象にする場合は `--with-php-config=/path/to/php-config` を指定）、`lindera.so` を拡張ディレクトリにインストールし有効化します。Composer 自体は `php-ext` パッケージをインストールしないため、`composer require lindera/lindera` ではインストールできません。

> [!NOTE]
> `lindera/lindera` は lindera v6.1.0 以降で Packagist から利用できます。それ以前のバージョンは、以下の手順でソースからビルドしてください。

パッケージには辞書は埋め込まれていません。[辞書の入手](#辞書の入手)を参照してください。

## 辞書の入手

Lindera はパッケージに辞書を同梱していません。ビルド済み辞書を別途入手する必要があります。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手できます。辞書アーカイブをダウンロードしてローカルディレクトリに展開してください：

```bash
# 例: IPADIC 辞書のダウンロードと展開
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## ソースからのビルド

lindera-php をビルドします：

```bash
cargo build -p lindera-php
```

または、プロジェクトの Makefile を使用します：

```bash
make build-lindera-php
```

### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります。デフォルトで有効になっています：

```bash
cargo build -p lindera-php --features train
```

### PHP 拡張の読み込み

ビルド後、`-d extension=` オプションでビルドされた共有ライブラリを指定して PHP を実行します：

```bash
php -d extension=target/debug/liblindera_php.so script.php
```

リリースビルドの場合：

```bash
cargo build -p lindera-php --release
php -d extension=target/release/liblindera_php.so script.php
```

`php.ini` に追加して常に読み込むこともできます：

```ini
extension=/absolute/path/to/liblindera_php.so
```

macOS では cargo は `liblindera_php.dylib` を生成します。どの方法で読み込んでも拡張名は `lindera` です。`php -m` には `lindera` と表示され、`extension_loaded('lindera')` で確認できます。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 有効 |
| `embed-ipadic` | 日本語辞書（IPADIC）をバイナリに埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）をバイナリに埋め込み | 無効 |
| `embed-sudachidict` | 日本語辞書（SudachiDict）をバイナリに埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）をバイナリに埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）をバイナリに埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）をバイナリに埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）をバイナリに埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書をバイナリに埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
cargo build -p lindera-php --features "train,embed-ipadic,embed-ko-dic"
```

PIE はデフォルトの feature（`train` のみ、辞書の埋め込みなし）でビルドします。

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```php
> $dictionary = Lindera\Dictionary::load('embedded://ipadic');
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## インストールの確認

インストール後、PHP で lindera が利用可能であることを確認します：

```php
<?php

$version = Lindera\Dictionary::version();
echo "Lindera version: {$version}\n";
```

実行方法：

```bash
# PIE でインストールした場合
php script.php

# 自分でビルドした場合
php -d extension=target/debug/liblindera_php.so script.php
```

## 静的解析用スタブ

[`lindera-php/stubs/lindera.stubs.php`](https://github.com/lindera/lindera/blob/main/lindera-php/stubs/lindera.stubs.php) は、拡張の全クラスを IDE・PHPStan・Psalm 向けに宣言したスタブです。ビルドした拡張から生成され、内容が古くなるとテストが失敗します。ファイルのコピーを解析ツールに指定してください。拡張がクラスを宣言済みのため、実行時に include してはいけません。

```neon
# phpstan.neon
parameters:
    stubFiles:
        - path/to/lindera.stubs.php
```
