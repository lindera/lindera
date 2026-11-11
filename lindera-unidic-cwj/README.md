# Lindera UniDic CWJ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-unidic-cwj.svg)](https://crates.io/crates/lindera-unidic-cwj)

## Dictionary version

This crate builds [UniDic for contemporary written Japanese](https://clrd.ninjal.ac.jp/unidic/) (現代書き言葉UniDic, UniDic CWJ) ver.2025.12, the successor of UniDic 3.1. The source is NINJAL's full package, `unidic-cwj-202512_full.zip`, downloaded from NINJAL's official URL and checked against the SHA-256 pinned in `build.rs`. [`lindera-unidic`](https://crates.io/crates/lindera-unidic) keeps providing UniDic 2.1.2 (unidic-mecab).

## Size

UniDic CWJ is much larger than the other dictionaries: its connection matrix is 24,082 × 21,375, and the built dictionary takes 1.2 GB, 982 MiB of it the matrix. With `embed-unidic-cwj`, every binary carries those 1.2 GB, and compiling this crate takes about 11 GiB of memory (measured on macOS). To keep binaries small, download the dictionary with `lindera download unidic-cwj` and load it from its directory instead. The WASM binding does not provide `embed-unidic-cwj`.

## Dictionary format

Refer to the [UniDic FAQ](https://clrd.ninjal.ac.jp/unidic/faq.html) for the fields and their values.

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
| 21 | 語頭変化結合型 | Initial connection type | |
| 22 | 語末変化結合型 | Final connection type | |
| 23 | 語彙素類 | Lexeme class | |
| 24 | 仮名形出現形 | Kana | |
| 25 | 仮名形基本形 | Kana base | |
| 26 | 語形出現形 | Form | |
| 27 | 語形基本形 | Form base | |
| 28 | アクセント型 | Accent type | |
| 29 | アクセント結合型 | Accent connection type | Can contain commas, such as `動詞%F2@0,名詞%F1` |
| 30 | アクセント修飾型 | Accent modification type | |
| 31 | 語彙表ID | Lexicon ID | |
| 32 | 語彙素ID | Lemma ID | |

## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 語彙素読み | Reading | |

### Detailed version

The 33 fields of the dictionary format above, in the same order. After index 32, it can be freely expanded.

## API reference

The API reference is available. Please see following URL:

- [lindera-unidic-cwj](https://docs.rs/lindera-unidic-cwj)
