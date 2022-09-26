# Lindera IPADIC Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

IPADIC dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).


## Dictionary version

This repository contains [mecab-ipadic-2.7.0-20070801](http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/).


## Dictionary format

Refer to the [manual](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf) for details on the IPADIC dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Major POS classification | |
| 5 | 品詞細分類1 | Middle POS classification | |
| 6 | 品詞細分類2 | Small POS classification | |
| 7 | 品詞細分類3 | Fine POS classification | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |


## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | Major POS classification | |
| 2 | 読み | Reading | |

### Detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | POS | |
| 5 | 品詞細分類1 | POS subcategory 1 | |
| 6 | 品詞細分類2 | POS subcategory 2 | |
| 7 | 品詞細分類3 | POS subcategory 3 | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |
| 13 | - | - | After 13, it can be freely expanded. |


## How to use IPADIC dictionary

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-ipadic-builder" target="_blank">lindera-ipadic-builder</a>
