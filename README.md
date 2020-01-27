# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/bayard-search/lindera](https://badges.gitter.im/bayard-search/lindera.svg)](https://gitter.im/bayard-search/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese Morphological Analyzer written in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).


## Building Lindera

### Requirements

The following products are required to build Bayrad:

- Rust >= 1.39.0
- make >= 3.81

### Build

Build Bayard with the following command:

```text
$ make build
```

## Usage

```
$ lindera
すもももももももものうち
すもも  スモモ
も      モ
もも    モモ
も      モ
もも    モモ
の      ノ
うち    ウチ
EOS
```

test test_tokenize ... bench:       7,666 ns/iter (+/- 25,545)  
test test_tokenize ... bench:       5,507 ns/iter (+/- 755)
