# Lindera CC-CE-DICT

## 辞書バージョン

このリポジトリは [CC-CEDICT-MeCab](https://github.com/lindera/CC-CEDICT-MeCab) を含んでいます。

## 辞書フォーマット

辞書フォーマットと品詞タグの詳細については [マニュアル](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) を参照してください。

| インデックス | 名前 (中国語) | 名前 (英語) | 備考 |
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

## ユーザー辞書フォーマット (CSV)

### シンプル版 (Simple version)

| インデックス | 名前 (中国語) | 名前 (英語) | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Part-of-speech | |
| 2 | 併音 | Pinyin | |

### 詳細版 (Detailed version)

| インデックス | 名前 (中国語) | 名前 (英語) | 備考 |
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
| 12 | - | - | 12以降は自由に拡張可能です。 |

## APIリファレンス

APIリファレンスは以下で公開されています：

- [lindera-cc-cedict](https://docs.rs/lindera-cc-cedict)
