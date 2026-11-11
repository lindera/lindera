# Lindera UniDic CWJ

## 辞書バージョン

このクレートは、国立国語研究所のフルパッケージ `unidic-cwj-202512_full.zip` から、[現代書き言葉UniDic](https://clrd.ninjal.ac.jp/unidic/) ver.2025.12 を組み立てます。

## 辞書フォーマット

各項目とその値については、[UniDic の FAQ](https://clrd.ninjal.ac.jp/unidic/faq.html) を参照してください。項目 0 から 20 は [UniDic 2.1.2](../lindera-unidic/dictionary_format.md) と同じで、名前も同じです。

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
| 29 | アクセント結合型 | Accent connection type | `動詞%F2@0,名詞%F1` のようにコンマを含むことがある |
| 30 | アクセント修飾型 | Accent modification type | |
| 31 | 語彙表ID | Lexicon ID | |
| 32 | 語彙素ID | Lemma ID | |

`lindera tokenize` の JSON 出力は、整数として読める項目を JSON の数値で書きます。`lexicon_id` は、JavaScript の数値が正確に表せる上限の 2^53 を超えます。`JSON.parse` で読むと別の値になるので、大きな整数を保つパーサーで読んでください。

## ユーザー辞書フォーマット (CSV)

### 簡易版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 語彙素読み | Reading | |

### 詳細版

上の辞書フォーマットの 33 項目を、同じ順に並べます。インデックス 32 より後は自由に拡張できます。

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-unidic-cwj](https://docs.rs/lindera-unidic-cwj)
