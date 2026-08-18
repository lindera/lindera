# Lindera CLI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-cli.svg)](https://crates.io/crates/lindera-cli)

A morphological analysis command-line interface for [Lindera](https://github.com/lindera-morphology/lindera).

## Documentation

- [English Documentation](https://lindera.github.io/lindera/lindera-cli.html)
- [Japanese Documentation (日本語ドキュメント)](https://lindera.github.io/lindera/ja/lindera-cli.html)

## Install

You can install binary via cargo as follows:

```shell script
% cargo install lindera-cli
```

Alternatively, you can download a binary from the following release page:

- [https://github.com/lindera/lindera/releases](https://github.com/lindera/lindera/releases)

## Dictionary

The easiest way to obtain a dictionary is the `download` subcommand, which installs the pre-built dictionary matching the CLI version under the OS-standard application data directory:

```shell script
% lindera download ipadic
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dict ipadic
```

Available dictionary names: `ipadic`, `ipadic-neologd`, `unidic`, `ko-dic`, `cc-cedict`, `jieba`.

Alternatively, pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`), extract it, and specify the path with the `--dict` option.

## License

MIT
