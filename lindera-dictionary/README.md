# Lindera Core

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-core.svg)](https://crates.io/crates/lindera-core)

A morphological analysis core library for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

This package contains dictionary structures and the viterbi algorithm.

## Dictionary format

### IPADIC

This repository uses [mecab-ipadic](https://github.com/lindera-morphology/mecab-ipadic).

#### IPADIC dictionary format

Refer to the [manual](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf) for details on the IPADIC dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Major POS classification | |
| 5 | 品詞細分類1 | Middle POS classification | |
| 6 | 品詞細分類2 | Small POS classification | |
| 7 | 品詞細分類3 | Fine POS classification | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |

#### IPADIC user dictionary format (CSV)

##### IPADIC user dictionary simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | Major POS classification | |
| 2 | 読み | Reading | |

##### IPADIC user dictionary detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | POS | |
| 5 | 品詞細分類1 | POS subcategory 1 | |
| 6 | 品詞細分類2 | POS subcategory 2 | |
| 7 | 品詞細分類3 | POS subcategory 3 | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |
| 13 | - | - | After 13, it can be freely expanded. |


### IPADIC NEologd

This repository uses [mecab-ipadic-neologd](https://github.com/lindera-morphology/mecab-ipadic-neologd).

#### IPADIC NEologd dictionary format

Refer to the [manual](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf) for details on the IPADIC dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | Major POS classification | |
| 5 | 品詞細分類1 | Middle POS classification | |
| 6 | 品詞細分類2 | Small POS classification | |
| 7 | 品詞細分類3 | Fine POS classification | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |

#### IPADIC NEologd user dictionary format (CSV)

##### IPADIC NEologd user dictionary simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | Major POS classification | |
| 2 | 読み | Reading | |

##### IPADIC NEologd user dictionary detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞 | POS | |
| 5 | 品詞細分類1 | POS subcategory 1 | |
| 6 | 品詞細分類2 | POS subcategory 2 | |
| 7 | 品詞細分類3 | POS subcategory 3 | |
| 8 | 活用形 | Conjugation type | |
| 9 | 活用型 | Conjugation form | |
| 10 | 原形 | Base form | |
| 11 | 読み | Reading | |
| 12 | 発音 | Pronunciation | |
| 13 | - | - | After 13, it can be freely expanded. |

### UniDic

This repository uses [unidic-mecab](https://github.com/lindera-morphology/unidic-mecab).

#### UniDic dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification  | |
| 8 | 活用型 | Conjugation form | |
| 9 | 活用形 | Conjugation type | |
| 10 | 語彙素読み | Lexeme reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthography appearance type | |
| 13 | 発音形出現形 | Pronunciation appearance type | |
| 14 | 書字形基本形 | Orthography basic type | |
| 15 | 発音形基本形 | Pronunciation basic type | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Prefix of a word form | |
| 18 | 語頭変化形 | Prefix of a word type | |
| 19 | 語末変化型 | Suffix of a word form  | |
| 20 | 語末変化形 | Suffix of a word type  | |

#### UniDic user dictionary format (CSV)

##### UniDic user dictionary simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Major POS classification | |
| 2 | 語彙素読み | Lexeme reading | |

##### UniDic user dictionary detailed version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification  | |
| 8 | 活用型 | Conjugation form | |
| 9 | 活用形 | Conjugation type | |
| 10 | 語彙素読み | Lexeme reading | |
| 11 | 語彙素（語彙素表記 + 語彙素細分類） | Lexeme | |
| 12 | 書字形出現形 | Orthography appearance type | |
| 13 | 発音形出現形 | Pronunciation appearance type | |
| 14 | 書字形基本形 | Orthography basic type | |
| 15 | 発音形基本形 | Pronunciation basic type | |
| 16 | 語種 | Word type | |
| 17 | 語頭変化型 | Prefix of a word form | |
| 18 | 語頭変化形 | Prefix of a word type | |
| 19 | 語末変化型 | Suffix of a word form  | |
| 20 | 語末変化形 | Suffix of a word type  | |
| 21 | - | - | After 21, it can be freely expanded. |

### ko-dic

This repository uses [mecab-ko-dic](https://github.com/lindera-morphology/mecab-ko-dic).

#### ko-dic dictionary format

Information about the dictionary format and part-of-speech tags used by mecab-ko-dic id documented in [this Google Spreadsheet](https://docs.google.com/spreadsheets/d/1-9blXKjtjeKZqsf4NzHeYJCrr49-nXeRF6D80udfcwY/edit#gid=589544265), linked to from mecab-ko-dic's [repository readme](https://bitbucket.org/eunjeon/mecab-ko-dic/src/master/README.md).

Note how ko-dic has one less feature column than NAIST JDIC, and has an altogether different set of information (e.g. doesn't provide the "original form" of the word).

The tags are a slight modification of those specified by 세종 (Sejong), whatever that is. The mappings from Sejong to mecab-ko-dic's tag names are given in tab `태그 v2.0` on the above-linked spreadsheet.

The dictionary format is specified fully (in Korean) in tab `사전 형식 v2.0` of the spreadsheet. Any blank values default to `*`.

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet  |
| 5 | 의미 부류 | meaning | (too few examples for me to be sure) |
| 6 | 종성 유무 | presence or absence | `T` for true; `F` for false; else `*` |
| 7 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |
| 8 | 타입 | type | One of: `Inflect` (활용); `Compound` (복합명사); or `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | first part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `VV` |
| 10 | 마지막 품사 | last part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `EP` |
| 11 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – Fields that tell how usage, compound nouns, and key analysis are organized |

#### ko-dic user dictionary format (CSV)

##### ko-dic user dictionary simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet  |
| 2 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |

##### ko-dic user dictionary detailed version

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet  |
| 5 | 의미 부류 | meaning | (too few examples for me to be sure) |
| 6 | 종성 유무 | presence or absence | `T` for true; `F` for false; else `*` |
| 7 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |
| 8 | 타입 | type | One of: `Inflect` (활용); `Compound` (복합명사); or `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | first part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `VV` |
| 10 | 마지막 품사 | last part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `EP` |
| 11 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – Fields that tell how usage, compound nouns, and key analysis are organized |
| 12 | - | - | After 12, it can be freely expanded. |

### CC-CEDICT

This repository uses [CC-CEDICT-MeCab](https://github.com/lindera/CC-CEDICT-MeCab).

#### CC-CEDICT dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Major POS classification | |
| 5 | 词类1 | Middle POS classification | |
| 6 | 词类2 | Small POS classification | |
| 7 | 词类3 | Fine POS classification | |
| 8 | 併音 | pinyin | |
| 9 | 繁体字 | traditional | |
| 10 | 簡体字 | simplified | |
| 11 | 定义 | definition | |

#### CC-CEDICT user dictionary format (CSV)

##### CC-CEDICT user dictionary simple version

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Major POS classification | |
| 2 | 併音 | pinyin | |

##### CC-CEDICT user dictionary detailed version

| Index | Name (Chinese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | POS | |
| 5 | 词类1 | POS subcategory 1 | |
| 6 | 词类2 | POS subcategory 2 | |
| 7 | 词类3 | POS subcategory 3 | |
| 8 | 併音 | pinyin | |
| 9 | 繁体字 | traditional | |
| 10 | 簡体字 | simplified | |
| 11 | 定义 | definition | |
| 12 | - | - | After 12, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-core](https://docs.rs/lindera-core)
