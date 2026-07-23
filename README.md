# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project is forked from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Documentation

- [English Documentation](https://lindera.github.io/lindera/)
- [Japanese Documentation (日本語ドキュメント)](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = "5.0"
```

> **Note:** v5.0.0 is the next planned release and has not been published to crates.io yet; the
> current published version is `4.0.1`. See the [migration guide](https://lindera.github.io/lindera/migration_v4_to_v5.html)
> for details.

Pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`) and specify the path when loading.

## Python Bindings

Lindera also provides Python bindings. You can install it via pip:

```bash
pip install lindera-python
```

For more details, see the [lindera-python](lindera-python/) directory.

## WebAssembly Bindings

Lindera also provides WebAssembly bindings. You can install it via npm:

```bash
npm install lindera-wasm
```

For more details and a demo application, see the [lindera-wasm](lindera-wasm/) directory.

## License

MIT
