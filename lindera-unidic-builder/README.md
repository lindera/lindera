# Lindera UniDic Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

UniDic builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Dictionary version

This project supports UniDic 2.1.2.
See [detail of UniDic](https://clrd.ninjal.ac.jp/unidic/) .


## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 左文脈ID | Left context ID |
| 2 | 右文脈ID | Right context ID |
| 3 | コスト | Cost |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification  | |
| 8 | 活用型 | Conjugation form | |
| 9 | 活用形 | Conjugation type | |
| 10 | 語彙素読み | Lexeme reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthography appearance type | |
| 13 | 発音形出現形 | Pronunciation appearance type | |
| 14 | 書字形基本形 | Orthography basic type | |
| 15 | 発音形基本形 | Pronunciation basic type | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Prefix of a word form | |
| 18 | 語頭変化形 | Prefix of a word type | |
| 19 | 語末変化型 | Suffix of a word form  | |
| 20 | 語末変化形 | Suffix of a word type  | |


## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 品詞大分類 | Major POS classification | |
| 2 | 語彙素読み | Lexeme reading | |

### Detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 左文脈ID | Left context ID |
| 2 | 右文脈ID | Right context ID |
| 3 | コスト | Cost |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification  | |
| 8 | 活用型 | Conjugation form | |
| 9 | 活用形 | Conjugation type | |
| 10 | 語彙素読み | Lexeme reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthography appearance type | |
| 13 | 発音形出現形 | Pronunciation appearance type | |
| 14 | 書字形基本形 | Orthography basic type | |
| 15 | 発音形基本形 | Pronunciation basic type | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Prefix of a word form | |
| 18 | 語頭変化形 | Prefix of a word type | |
| 19 | 語末変化型 | Suffix of a word form  | |
| 20 | 語末変化形 | Suffix of a word type  | |
| 21 | - | - | After 21, it can be freely expanded. |


## How to use IPADIC dictionary

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-unidic-builder" target="_blank">Lindera UniDic Builder</a>
