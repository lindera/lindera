# Lindera CC-CEDICT Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

CC-CEDICT dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface |
| 1 | 左语境ID | Left context ID |
| 2 | 右语境ID | Right context ID |
| 3 | 成本 | Cost |
| 4 | 词类 | Major POS classification | |
| 5 | 词类1 | Middle POS classification | |
| 6 | 词类2 | Small POS classification | |
| 7 | 词类3 | Fine POS classification | |
| 8 | 併音 | pinyin | |
| 9 | 繁体字 | traditional | |
| 10 | 簡体字 | simplified | |
| 11 | 定义 | definition | |


## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface |
| 1 | 词类 | Major POS classification | |
| 2 | 併音 | pinyin | |

### Detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface |
| 1 | 左语境ID | Left context ID |
| 2 | 右语境ID | Right context ID |
| 3 | 成本 | Cost |
| 4 | 词类 | POS | |
| 5 | 词类1 | POS subcategory 1 | |
| 6 | 词类2 | POS subcategory 2 | |
| 7 | 词类3 | POS subcategory 3 | |
| 8 | 併音 | pinyin | |
| 9 | 繁体字 | traditional | |
| 10 | 簡体字 | simplified | |
| 11 | 定义 | definition | |
| 12 | - | - | After 12, it can be freely expanded. |


## How to use CC-CEDICT dictionary

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-cc-cedict-builder" target="_blank">lindera-cc-cedict-builder</a>
