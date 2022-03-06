# Lindera IPADIC Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

IPADIC dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).


## Install

```shell script
% cargo install lindera-ipadic-builder
```


## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```


## Dictionary version

This repository contains [mecab-ipadic-2.7.0-20070801](http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/).


## Building a dictionary

Building a dictionary with `lindera-ipadic-builder` command:

```shell script
% curl -L -o /tmp/mecab-ipadic-2.7.0-20070801.tar.gz "http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/mecab-ipadic-2.7.0-20070801.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20070801.tar.gz -C /tmp
% lindera-ipadic-builder -s /tmp/mecab-ipadic-2.7.0-20070801 -d /tmp/lindera-ipadic-2.7.0-20070801
```


## Building a user dictionary

Building a dictionary with `lindera-userdic-builder` command:

```shell script
% lindera-ipadic-builder -S ./resources/userdic.csv -D ./resources/userdic.bin
```


## Dictionary format

Refer to the [manual](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf) for details on the IPADIC dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 左文脈ID | left-context-id | |
| 2 | 右文脈ID | right-context-id | |
| 3 | コスト | cost | |
| 4 | 品詞 | part-of-speech | |
| 5 | 品詞細分類1 | sub POS 1 | |
| 6 | 品詞細分類2 | sub POS 2 | |
| 7 | 品詞細分類3 | sub POS 3 | |
| 8 | 活用形 | conjugation type | |
| 9 | 活用型 | conjugation form | |
| 10 | 原形 | base form | |
| 11 | 読み | reading | |
| 12 | 発音 | pronunciation | |


## User dictionary format (CSV)

Simple version
| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | part-of-speech | |
| 2 | 読み | reading | |

Detailed version
| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 左文脈ID | left-context-id | |
| 2 | 右文脈ID | right-context-id | |
| 3 | コスト | cost | |
| 4 | 品詞 | part-of-speech | |
| 5 | 品詞細分類1 | sub POS 1 | |
| 6 | 品詞細分類2 | sub POS 2 | |
| 7 | 品詞細分類3 | sub POS 3 | |
| 8 | 活用形 | conjugation type | |
| 9 | 活用型 | conjugation form | |
| 10 | 原形 | base form | |
| 11 | 読み | reading | |
| 12 | 発音 | pronunciation | |


## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "羽田空港限定トートバッグ" | lindera -d /tmp/lindera-ipadic-2.7.0-20070801
```

```text
羽田空港        名詞,固有名詞,一般,*,*,*,羽田空港,ハネダクウコウ,ハネダクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```


## Tokenizing text using default dictionary and produced binary user dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera -D ./resources/userdic.bin -t bin
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-ipadic-builder)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-ipadic-builder" target="_blank">lindera-ipadic-builder</a>
