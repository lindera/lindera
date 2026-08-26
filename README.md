# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project is forked from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Documentation

- [English Documentation](https://lindera.github.io/lindera/)
- [Japanese Documentation (日本語ドキュメント)](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = "6"
```

> **Note:** Upgrading from v5? See the [migration guide](https://lindera.github.io/lindera/migration_v5_to_v6.html)
> for details.

Pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`) and specify the path when loading.
When using the CLI, `lindera download ipadic` downloads and installs a dictionary automatically.

## Performance

Lindera tokenizes Japanese text (IPADIC) at roughly 10-20 MB/s single-threaded, depending on hardware and whether the Viterbi lattice is reused across calls. Run `make bench` to reproduce the benchmark suite (`lindera/benches/`) on your own machine; see [#875](https://github.com/lindera/lindera/issues/875) for detailed methodology and comparison numbers.

## Python Bindings

Lindera also provides Python bindings. You can install it via pip:

```bash
pip install lindera
```

For more details, see the [lindera-python](lindera-python/) directory.

## Node.js Bindings

Lindera also provides Node.js bindings. You can install it via npm:

```bash
npm install lindera
```

For more details, see the [lindera-nodejs](lindera-nodejs/) directory.

## Ruby Bindings

Lindera also provides Ruby bindings. They are not yet published to RubyGems — build from source; see the [lindera-ruby](lindera-ruby/) directory for instructions.

## PHP Bindings

Lindera also provides PHP bindings. They are not yet published to Packagist — build from source; see the [lindera-php](lindera-php/) directory for instructions.

## WebAssembly Bindings

Lindera also provides WebAssembly bindings. You can install it via npm:

```bash
npm install lindera-wasm
```

The package is built with wasm-pack's `web` target and works in browsers
natively as well as with modern bundlers (Vite, Webpack 5).

For more details and a demo application, see the [lindera-wasm](lindera-wasm/) directory.

## License

MIT
