# Lindera UniDic CSJ

## Dictionary version

This crate builds [UniDic for contemporary spoken Japanese](https://clrd.ninjal.ac.jp/unidic/) (現代話し言葉UniDic) ver.2025.12, from NINJAL's full package `unidic-csj-202512_full.zip`.

## Dictionary format

Refer to the [UniDic FAQ](https://clrd.ninjal.ac.jp/unidic/faq.html) for the fields and their values. Fields 0 to 20 are those of [UniDic 2.1.2](../lindera-unidic/dictionary_format.md), with the same names, and all 33 are those of [UniDic CWJ](../lindera-unidic-cwj/dictionary_format.md).

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

The JSON output of `lindera tokenize` writes a field that parses as an integer as a JSON number. `lexicon_id` goes above 2^53, the largest integer a JavaScript number holds exactly: `JSON.parse` gives another value, so read the JSON with a parser that keeps big integers.

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

- [lindera-unidic-csj](https://docs.rs/lindera-unidic-csj)
