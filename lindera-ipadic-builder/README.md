# Lindera IPADIC Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

IPADIC dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

## Install

```shell script
% cargo install lindera-ipadic-builder
```

## Build

The following products are required to build:

- Rust >= 1.39.0
- make >= 3.81

```shell script
% cargo build --release
```

## Dictionary version

This repository contains [mecab-ipadic-2.7.0-20070801](http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/).

## Building a dictionary

Building a dictionary with `lindera-ipadic` command:

```shell script
% curl -L -O "http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/mecab-ipadic-2.7.0-20070801.tar.gz"
% tar zxvf ./mecab-ipadic-2.7.0-20070801.tar.gz
% lindera-ipadic ./mecab-ipadic-2.7.0-20070801 ./lindera-ipadic-2.7.0-20070801
```

## Dictionary format

Refer to the [manual](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf) for details on the IPADIC dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 品詞 | part-of-speech | |
| 1 | 品詞細分類1 | sub POS 1 | |
| 2 | 品詞細分類2 | sub POS 2 | |
| 3 | 品詞細分類3 | sub POS 3 | |
| 4 | 活用形 | conjugation type | |
| 5 | 活用型 | conjugation form | |
| 6 | 原形 | base form | |
| 7 | 読み | reading | |
| 8 | 発音 | pronunciation | |

## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "羽田空港限定トートバッグ" | lindera -d ./lindera-ipadic-2.7.0-20070801
```

```text
羽田空港        名詞,固有名詞,一般,*,*,*,羽田空港,ハネダクウコウ,ハネダクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-ipadic-builder)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-ipadic-builder" target="_blank">lindera-ipadic-builder</a>
