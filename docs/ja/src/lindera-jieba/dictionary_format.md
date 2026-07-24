# Lindera Jieba

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
| 5 | 字符类型 | Character type | |
| 6 | 併音 | Pinyin | |
| 7 | 繁体字 | Traditional | |
| 8 | 簡体字 | Simplified | |
| 9 | 定义 | Definition | |
| 10 | 字符数 | Character count | |
| 11 | 首字符 | First character | |
| 12 | 末字符 | Last character | |
| 13 | 频率等级 | Frequency band | |

## ユーザー辞書フォーマット (CSV)

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
| 5 | 字符类型 | Character type | |
| 6 | 併音 | Pinyin | |
| 7 | 繁体字 | Traditional | |
| 8 | 簡体字 | Simplified | |
| 9 | 定义 | Definition | |
| 10 | 字符数 | Character count | |
| 11 | 首字符 | First character | |
| 12 | 末字符 | Last character | |
| 13 | 频率等级 | Frequency band | |
| 14 | - | - | 14 以降は自由に拡張可能です。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-jieba](https://docs.rs/lindera-jieba)
