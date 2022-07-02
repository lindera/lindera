# Lindera UniDic Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

UniDic builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Install

```shell script
% cargo install lindera-unidic-builder
```


## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```


## Dictionary version

This project supports UniDic 2.1.2.
See [detail of UniDic](https://unidic.ninjal.ac.jp/) .


## Building a dictionary

Building a dictionary with `lindera-unidic` command:

```shell script
% curl -l -o /tmp/unidic-mecab-2.1.2_src.zip "https://ccd.ninjal.ac.jp/unidic_archive/cwj/2.1.2/unidic-mecab-2.1.2_src.zip"
% unzip /tmp/unidic-mecab-2.1.2_src.zip -d /tmp
% lindera-unidic-builder -s /tmp/unidic-mecab-2.1.2_src -d /tmp/lindera-unidic-2.1.2
```


## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 品詞大分類 | | |
| 1 | 品詞中分類 | | |
| 2 | 品詞小分類 | | |
| 3 | 品詞細分類 | | |
| 4 | 活用型 | | |
| 5 | 活用形 | | |
| 6 | 語彙素読み | | |
| 7 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 8 | 書字形出現形 | | |
| 9 | 発音形出現形 | | |
| 10 | 書字形基本形 | | |
| 11 | 発音形基本形 | | |
| 12 | 語種 | | |
| 13 | 語頭変化型 | | |
| 14 | 語頭変化形 | | |
| 15 | 語末変化型 | | |
| 16 | 語末変化形 | | |


## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "羽田空港限定トートバッグ" | lindera -k unidic -d /tmp/lindera-unidic-2.1.2
```

```text
羽田    名詞,固有名詞,人名,姓,*,*,羽田,ハタ,ハタ
空港    名詞,普通名詞,一般,*,*,*,空港,クーコー,クーコー
限定    名詞,普通名詞,サ変可能,*,*,*,限定,ゲンテー,ゲンテー
トート  名詞,普通名詞,一般,*,*,*,トート,トート,トート
バッグ  名詞,普通名詞,一般,*,*,*,バッグ,バッグ,バッグ
EOS
```

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-unidic-builder" target="_blank">Lindera UniDic Builder</a>
