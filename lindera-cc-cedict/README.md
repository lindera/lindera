# Lindera CC-CE-DICT

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-cc-cedict.svg)](https://crates.io/crates/lindera-cc-cedict)

## Dictionary version

This repository contains [CC-CEDICT-MeCab](https://github.com/lindera/CC-CEDICT-MeCab).

## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
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
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Major POS classification | |
| 2 | 併音 | pinyin | |

### Detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | POS | |
| 5 | 词类1 | POS subcategory 1 | |
| 6 | 词类2 | POS subcategory 2 | |
| 7 | 词类3 | POS subcategory 3 | |
| 8 | 併音 | pinyin | |
| 9 | 繁体字 | traditional | |
| 10 | 簡体字 | simplified | |
| 11 | 定义 | definition | |
| 12 | - | - | After 12, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-cc-cedict](https://docs.rs/lindera-cc-cedict)
