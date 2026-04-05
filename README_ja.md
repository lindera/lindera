# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

Rust で実装された形態素解析ライブラリです。本プロジェクトは [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs) からフォークされました。

Lindera は、さまざまな Rust アプリケーションに対して、簡単にインストールでき、簡潔な API を提供するライブラリの構築を目指しています。

## ドキュメント

- [英語版ドキュメント](https://lindera.github.io/lindera/)
- [日本語版ドキュメント](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = "3.0.0"
```

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードできます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、読み込み時にパスを指定してください。

## Python バインディング

Lindera は Python バインディングも提供しています。pip でインストールできます:

```bash
pip install lindera-python
```

詳細は [lindera-python](lindera-python/) ディレクトリを参照してください。

## WebAssembly バインディング

Lindera は WebAssembly バインディングも提供しています。npm でインストールできます:

```bash
npm install lindera-wasm
```

詳細とデモアプリケーションは [lindera-wasm](lindera-wasm/) ディレクトリを参照してください。

## ライセンス

MIT
