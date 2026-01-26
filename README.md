# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project is forked from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Documentation

- [English Documentation](https://lindera.github.io/lindera/)
- [Japanese Documentation (日本語ドキュメント)](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = { version = "2.0.0", features = ["embed-ipadic"] }
```

## Python Bindings

Lindera also provides Python bindings. You can install it via pip:

```bash
pip install lindera-python
```

For more details, see the [lindera-python](lindera-python/) directory.

## License

MIT
