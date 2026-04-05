# Lindera Jieba

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-jieba.svg)](https://crates.io/crates/lindera-jieba)

## 辞書バージョン

このリポジトリには [mecab-jieba](https://github.com/lindera/mecab-jieba) が含まれています。

## 辞書フォーマット

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
| 5 | 併音 | Pinyin | |
| 6 | 繁体字 | Traditional | |
| 7 | 簡体字 | Simplified | |
| 8 | 定义 | Definition | |
| 9 | - | - | 9 以降は自由に拡張できます。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-jieba](https://docs.rs/lindera-jieba)
