# Lindera SudachiDict

## Dictionary version

This repository contains [SudachiDict](https://github.com/lindera/sudachidict) 20260723 (small + core + notcore lexicons).

## Dictionary format

Refer to the [SudachiDict documentation](https://github.com/WorksApplications/SudachiDict) for details on the dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 見出し（解析結果表示用） | Display surface | |
| 5 | 品詞大分類 | Part-of-speech | |
| 6 | 品詞中分類 | Part-of-speech subcategory 1 | |
| 7 | 品詞小分類 | Part-of-speech subcategory 2 | |
| 8 | 品詞細分類 | Part-of-speech subcategory 3 | |
| 9 | 活用型 | Conjugation type | |
| 10 | 活用形 | Conjugation form | |
| 11 | 読み | Reading | |
| 12 | 正規化表記 | Normalized form | |
| 13 | 辞書形ID | Dictionary form word ID | |
| 14 | 分割タイプ | Split mode | A/B/C |
| 15 | A単位分割情報 | Split references (A) | |
| 16 | B単位分割情報 | Split references (B) | |
| 17 | 語構成 | Word structure | |
| 18 | 同義語グループID | Synonym group IDs | |

> [!NOTE]
> Because the display surface sits at index 4 (before the part-of-speech
> columns), the first token detail is the display surface -- not the
> part-of-speech as in IPADIC or UniDic. Token filters that read the leading
> details positionally as part-of-speech tags (`japanese_stop_tags`,
> `japanese_keep_tags`, `japanese_compound_word`) do not match as expected
> with this dictionary; schema-aware access such as
> `token.get("part_of_speech")` works correctly. See
> [lindera/lindera#997](https://github.com/lindera/lindera/issues/997).

## User dictionary format (CSV)

### Simple version

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 読み | Reading | |

### Detailed version

The detailed version follows the dictionary format above (19 columns).

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0-18 | - | Same as the dictionary format | |
| 19 | - | - | After 19, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-sudachidict](https://docs.rs/lindera-sudachidict)
