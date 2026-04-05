# インストール

> [!NOTE]
> lindera-php はまだ Packagist に公開されていません。ソースからビルドする必要があります。

## 前提条件

- **PHP 8.1 以降**
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **Composer** -- PHP の依存関係管理（テスト実行時に必要）

## 辞書の入手

Lindera はパッケージに辞書を同梱していません。ビルド済み辞書を別途入手する必要があります。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手で���ます。辞書アーカイブをダウンロードしてローカルディレクトリに展開してください：

```bash
# 例: IPADIC 辞書のダウンロードと展開
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## ビルド

lindera-php をビルドします：

```bash
cargo build -p lindera-php
```

または、プロジェクトの Makefile を使用します：

```bash
make php-build
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

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 有効 |
| `embed-ipadic` | 日本語辞書（IPADIC）をバイナリに埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）をバイナリに埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）をバイナリに埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）をバイナリに埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）をバイナリに埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）をバイナリに埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書をバイナリに埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
cargo build -p lindera-php --features "train,embed-ipadic,embed-ko-dic"
```

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
php -d extension=target/debug/liblindera_php.so script.php
```
