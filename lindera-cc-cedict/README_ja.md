# Lindera CC-CE-DICT

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-cc-cedict.svg)](https://crates.io/crates/lindera-cc-cedict)

## 辞書バージョン

このリポジトリには [CC-CEDICT-MeCab](https://github.com/lindera/CC-CEDICT-MeCab) が含まれています。

## 辞書フォーマット

unidic-mecab 辞書のフォーマットと品詞タグの詳細については、[マニュアル](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf)を参照してください。

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 词类1 | Part-of-speech subcategory 1 | |
| 6 | 词类2 | Part-of-speech subcategory 2 | |
| 7 | 词类3 | Part-of-speech subcategory 3 | |
| 8 | 併音 | Pinyin | |
| 9 | 繁体字 | Traditional | |
| 10 | 簡体字 | Simplified | |
| 11 | 定义 | Definition | |

## ユーザー辞書フォーマット（CSV）

### 簡易版

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Part-of-speech | |
| 2 | 併音 | Pinyin | |

### 詳細版

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 词类1 | Part-of-speech subcategory 1 | |
| 6 | 词类2 | Part-of-speech subcategory 2 | |
| 7 | 词类3 | Part-of-speech subcategory 3 | |
| 8 | 併音 | Pinyin | |
| 9 | 繁体字 | Traditional | |
| 10 | 簡体字 | Simplified | |
| 11 | 定义 | Definition | |
| 12 | - | - | 12 以降は自由に拡張できます。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-cc-cedict](https://docs.rs/lindera-cc-cedict)
