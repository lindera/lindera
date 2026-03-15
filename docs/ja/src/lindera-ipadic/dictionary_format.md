# Lindera IPADIC

## 辞書バージョン

このリポジトリには [mecab-ipadic](https://github.com/lindera-morphology/mecab-ipadic) が含まれています。

## 辞書フォーマット

IPADIC の辞書フォーマットおよび品詞タグの詳細については、[マニュアル](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf)を参照してください。

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Part-of-speech | |
| 5 | 品詞細分類1 | Part-of-speech subcategory 1 | |
| 6 | 品詞細分類2 | Part-of-speech subcategory 2 | |
| 7 | 品詞細分類3 | Part-of-speech subcategory 3 | |
| 8 | 活用形 | Conjugation form | |
| 9 | 活用型 | Conjugation type | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |

## ユーザー辞書フォーマット (CSV)

### 簡易版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞 | Part-of-speech | |
| 2 | 読み | Reading | |

### 詳細版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Part-of-speech | |
| 5 | 品詞細分類1 | Part-of-speech subcategory 1 | |
| 6 | 品詞細分類2 | Part-of-speech subcategory 2 | |
| 7 | 品詞細分類3 | Part-of-speech subcategory 3 | |
| 8 | 活用形 | Conjugation form | |
| 9 | 活用型 | Conjugation type | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |
| 13 | - | - | 13 以降は自由に拡張可能です。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-ipadic](https://docs.rs/lindera-ipadic)
