# Lindera Jieba

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-jieba.svg)](https://crates.io/crates/lindera-jieba)

## Dictionary version

This repository contains [mecab-jieba](https://github.com/lindera/mecab-jieba).

## Dictionary format

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 併音 | Pinyin | |
| 6 | 繁体字 | Traditional | |
| 7 | 簡体字 | Simplified | |
| 8 | 定义 | Definition | |

## User dictionary format (CSV)

### Simple version

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Part-of-speech | |
| 2 | 併音 | Pinyin | |

### Detailed version

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 併音 | Pinyin | |
| 6 | 繁体字 | Traditional | |
| 7 | 簡体字 | Simplified | |
| 8 | 定义 | Definition | |
| 9 | - | - | After 9, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-jieba](https://docs.rs/lindera-jieba)
