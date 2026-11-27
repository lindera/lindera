# Lindera JUMAN 辞書

## 辞書バージョン

このクレートは、[MeCab のリポジトリ](https://github.com/taku910/mecab/tree/master/mecab-jumandic)にある mecab-jumandic 7.0-20130310 をビルドします。

## 辞書フォーマット

品詞、活用型、活用形は [JUMAN](https://nlp.ist.i.kyoto-u.ac.jp/?JUMAN) のもので、IPADIC や UniDic のものとは違います。

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Part-of-speech | 名詞、動詞、形容詞、判定詞、助動詞、助詞、接尾辞、特殊 など |
| 5 | 品詞細分類 | Part-of-speech subcategory 1 | 普通名詞、格助詞、句点 など。動詞は `*` |
| 6 | 活用型 | Conjugation type | 子音動詞マ行、母音動詞、ナ形容詞 など |
| 7 | 活用形 | Conjugation form | 基本形、タ形、基本連用形 など |
| 8 | 原形 | Base form | |
| 9 | 読み | Reading | ひらがな |
| 10 | 意味情報 | Semantic information | JUMAN の意味情報。`代表表記:本/ほん 漢字読み:音 カテゴリ:人工物-その他;抽象物`、`連語`、または `*` |

スキーマのフィールド名は、`part_of_speech`、`part_of_speech_subcategory_1`、`conjugation_type`、`conjugation_form`、`base_form`、`reading`、`semantic_information` です。JSON の出力と、フィールドを名前で指定するトークンフィルターで使う名前です。

未知語は、原形から後ろが `*` になります。

ソースには、同じ表層が、同じコスト・同じ文脈 ID で複数あり、読みや意味情報だけが違うものがあります。は は `Postp.csv` に `*`、`Rengo.csv` に `連語` であり、一 は ひと と いち の両方があります。Lindera はそのうちの 1 つを採り、MeCab は別のものを採ることがあります。[使用例](./examples.md)を参照してください。

## ユーザー辞書フォーマット (CSV)

### 簡易版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞 | Part-of-speech | |
| 2 | 読み | Reading | |

この簡易スキーマに含まれないフィールド (`base_form` や `semantic_information` など) には、辞書の `default_field_value` (`*`) が設定されます。

### 詳細版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Part-of-speech | |
| 5 | 品詞細分類 | Part-of-speech subcategory 1 | |
| 6 | 活用型 | Conjugation type | |
| 7 | 活用形 | Conjugation form | |
| 8 | 原形 | Base form | |
| 9 | 読み | Reading | |
| 10 | 意味情報 | Semantic information | |
| 11 | - | - | 11 以降は自由に拡張可能です。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-jumandic](https://docs.rs/lindera-jumandic)
