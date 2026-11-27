# Lindera JUMAN dictionary

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-jumandic.svg)](https://crates.io/crates/lindera-jumandic)

## Dictionary version

This crate builds mecab-jumandic 7.0-20130310, as [the repository of MeCab](https://github.com/taku910/mecab/tree/master/mecab-jumandic) holds it.

## Dictionary format

The parts of speech, the conjugation types and the conjugation forms are those of [JUMAN](https://nlp.ist.i.kyoto-u.ac.jp/?JUMAN), not those of IPADIC or UniDic.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Part-of-speech | 名詞, 動詞, 形容詞, 判定詞, 助動詞, 助詞, 接尾辞, 特殊 … |
| 5 | 品詞細分類 | Part-of-speech subcategory 1 | 普通名詞, 格助詞, 句点 …; `*` for a verb |
| 6 | 活用型 | Conjugation type | 子音動詞マ行, 母音動詞, ナ形容詞 … |
| 7 | 活用形 | Conjugation form | 基本形, タ形, 基本連用形 … |
| 8 | 原形 | Base form | |
| 9 | 読み | Reading | In hiragana |
| 10 | 意味情報 | Semantic information | JUMAN's: `代表表記:本/ほん 漢字読み:音 カテゴリ:人工物-その他;抽象物`, `連語`, or `*` |

The field names of the schema are `part_of_speech`, `part_of_speech_subcategory_1`, `conjugation_type`, `conjugation_form`, `base_form`, `reading` and `semantic_information`: the names of the JSON output, and of the token filters that take a field by its name.

An unknown word has `*` from the base form on.

The source has the same surface more than once with the same cost and the same context IDs, and another reading or other semantic information: は is in `Postp.csv` with `*` and in `Rengo.csv` with `連語`, and 一 is there as ひと and as いち. Lindera takes one of them and MeCab may take the other: see [Examples](../docs/src/lindera-jumandic/examples.md).

## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞 | Part-of-speech | |
| 2 | 読み | Reading | |

Fields not covered by this simple schema (such as `base_form` and `semantic_information`) are filled with the dictionary's `default_field_value` (`*`).

### Detailed version

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
| 11 | - | - | After 11, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-jumandic](https://docs.rs/lindera-jumandic)
