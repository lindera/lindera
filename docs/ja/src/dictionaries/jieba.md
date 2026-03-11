# Lindera Jieba

## 辞書バージョン

このリポジトリには [mecab-jieba](https://github.com/lindera/mecab-jieba) が含まれています。

## 辞書フォーマット

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
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

### シンプル版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Part-of-speech | |
| 2 | 併音 | Pinyin | |

### 詳細版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
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
| 9 | - | - | 9以降は自由に拡張可能。 |

## APIリファレンス

APIリファレンスは以下のURLで参照できます:

- [lindera-jieba](https://docs.rs/lindera-jieba)
