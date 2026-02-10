# Lindera UniDic

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-unidic.svg)](https://crates.io/crates/lindera-unidic)

## Dictionary version

This repository contains [unidic-mecab](https://github.com/lindera-morphology/unidic-mecab).

## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Part-of-speech | |
| 5 | 品詞中分類 | Part-of-speech subcategory 1 | |
| 6 | 品詞小分類 | Part-of-speech subcategory 2 | |
| 7 | 品詞細分類 | Part-of-speech subcategory 3 | |
| 8 | 活用型 | Conjugation type | |
| 9 | 活用形 | Conjugation form | |
| 10 | 語彙素読み | Reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthographic surface form | |
| 13 | 発音形出現形 | Phonological surface form | |
| 14 | 書字形基本形 | Orthographic base form | |
| 15 | 発音形基本形 | Phonological base form | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Initial mutation type | |
| 18 | 語頭変化形 | Initial mutation form | |
| 19 | 語末変化型 | Final mutation type | |
| 20 | 語末変化形 | Final mutation form | |

## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 語彙素読み | Reading | |

### Detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Part-of-speech | |
| 5 | 品詞中分類 | Part-of-speech subcategory 1 | |
| 6 | 品詞小分類 | Part-of-speech subcategory 2 | |
| 7 | 品詞細分類 | Part-of-speech subcategory 3 | |
| 8 | 活用型 | Conjugation type | |
| 9 | 活用形 | Conjugation form | |
| 10 | 語彙素読み | Reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthographic surface form | |
| 13 | 発音形出現形 | Phonological surface form | |
| 14 | 書字形基本形 | Orthographic base form | |
| 15 | 発音形基本形 | Phonological base form | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Initial mutation type | |
| 18 | 語頭変化形 | Initial mutation form | |
| 19 | 語末変化型 | Final mutation type | |
| 20 | 語末変化形 | Final mutation form | |
| 21 | - | - | After 21, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-unidic](https://docs.rs/lindera-unidic)
