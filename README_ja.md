# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

Rust で実装された形態素解析ライブラリです。本プロジェクトは [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs) からフォークされました。

Lindera は、さまざまな Rust アプリケーションに対して、簡単にインストールでき、簡潔な API を提供するライブラリの構築を目指しています。

## ドキュメント

- [英語版ドキュメント](https://lindera.github.io/lindera/)
- [日本語版ドキュメント](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = "5"
```

> **注記:** v4 からアップグレードする場合は[移行ガイド](https://lindera.github.io/lindera/ja/migration_v4_to_v5.html)を参照してください。

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードできます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、読み込み時にパスを指定してください。

## Python バインディング

Lindera は Python バインディングも提供しています。pip でインストールできます:

```bash
pip install lindera
```

詳細は [lindera-python](lindera-python/) ディレクトリを参照してください。

## Node.js バインディング

Lindera は Node.js バインディングも提供しています。npm でインストールできます:

```bash
npm install lindera
```

詳細は [lindera-nodejs](lindera-nodejs/) ディレクトリを参照してください。

## Ruby バインディング

Lindera は Ruby バインディングも提供していますが、RubyGems にはまだ公開されていません。ソースからビルドしてください。詳細は [lindera-ruby](lindera-ruby/) ディレクトリを参照してください。

## PHP バインディング

Lindera は PHP バインディングも提供していますが、Packagist にはまだ公開されていません。ソースからビルドしてください。詳細は [lindera-php](lindera-php/) ディレクトリを参照してください。

## WebAssembly バインディング

Lindera は WebAssembly バインディングも提供しています。npm でインストールできます:

```bash
npm install lindera-wasm
```

パッケージは `web` ターゲットでビルドされており、ブラウザからネイティブ ES モジュールとして直接利用できるほか、モダンなバンドラー（Vite、Webpack 5 など）からも利用できます。

詳細とデモアプリケーションは [lindera-wasm](lindera-wasm/) ディレクトリを参照してください。

## ライセンス

MIT
