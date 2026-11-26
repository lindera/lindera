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

The easiest way to obtain a dictionary is the `download` subcommand, which installs the pre-built dictionary matching the CLI version in the dictionaries directory (`LINDERA_DICTIONARIES_DIR`, by default `~/.local/share/lindera/dictionaries` on Linux) and prints its path, which `--dict` takes:

```shell script
% DICT_DIR=$(lindera download ipadic)
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dict "$DICT_DIR"
```

`lindera list` shows the dictionaries in place and those embedded in the binary.

Available dictionary names: `ipadic`, `ipadic-neologd`, `unidic`, `unidic-cwj`, `unidic-csj`, `sudachidict`, `ko-dic`, `cc-cedict`, `jieba`.

`lindera download dependency-ipadic` gets the model of the dependency parser for IPADIC the same way, from 7.1.0 on (`dependency-unidic`, `dependency-unidic-cwj` and `dependency-unidic-csj` the ones for UniDic), and prints the path of the model file, which the token filter `japanese_dependency` takes as `model`.

Alternatively, pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`), extract it, and specify the path with the `--dict` option.

## License

MIT
