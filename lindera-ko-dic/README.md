# Lindera ko-dic

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-ko-dic.svg)](https://crates.io/crates/lindera-ko-dic)

## Dictionary version

This repository contains [mecab-ko-dic](https://github.com/lindera-morphology/mecab-ko-dic).

## Dictionary format

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
| 4 | 품사 태그 | Part-of-speech tag | See `태그 v2.0` tab on spreadsheet |
| 5 | 의미 부류 | Meaning | (too few examples for me to be sure) |
| 6 | 종성 유무 | Presence or absence | `T` for true; `F` for false; else `*` |
| 7 | 읽기 | Reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |
| 8 | 타입 | Type | One of: `Inflect` (활용); `Compound` (복합명사); or `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | First part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `VV` |
| 10 | 마지막 품사 | Last part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `EP` |
| 11 | 표현 | Expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – Fields that tell how usage, compound nouns, and key analysis are organized |

## Left-space penalty

mecab-ko (the MeCab fork mecab-ko-dic is built for) raises the connection cost of a word whose left side is preceded by whitespace when its `pos-id.def` id is listed in `dicrc`:

```text
left-space-penalty-factor = 100,3000,120,6000,172,3000,183,3000,184,3000,185,3000,200,3000,210,6000,220,3000,221,3000,222,3000,230,3000
```

Lindera does not store `pos-id.def` ids, but every listed id corresponds to the entry's first part-of-speech tag (for `Inflect` rows, the `First part-of-speech` column), so the same rules are written by tag. ko-dic ships them in its `metadata.json` (`space_penalty`), so they can be enabled without spelling them out: `Segmenter::space_penalty_from_dictionary()`, `"space_penalty": true` in the segmenter config, or `lindera tokenize --space-penalty`. Explicit rules go through `Segmenter::space_penalty`, a `space_penalty` object, or `--space-penalty-rules`. The penalty is off by default. The shipped rules are present only in a ko-dic built by a Lindera release later than 6.0.0; a ko-dic from the v6.0.0 release has none, so `--space-penalty` fails until the dictionary is rebuilt, while `--space-penalty-rules` works regardless.

| pos-id.def ids | First part-of-speech tags | Cost |
| --- | --- | --- |
| 100, 200 (`Inflect`) | `EC`, `EF`, `EP`, `ETM`, `ETN` | 3000 |
| 172, 230 (`Inflect`) | `VCP` | 3000 |
| 183, 184, 185, 220, 221, 222 (`Inflect`) | `XSA`, `XSN`, `XSV` | 3000 |
| 120, 210 (`Inflect`) | `JC`, `JKB`, `JKC`, `JKG`, `JKO`, `JKQ`, `JKS`, `JKV`, `JX` | 6000 |

```json
{
  "rules": [
    { "pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000 },
    { "pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000 }
  ]
}
```

```shell
echo "서울 시 에서 출발" | lindera tokenize --dict embedded://ko-dic --space-penalty
```

With the penalty, `시` in `서울 시 에서` is read as the noun `NNG` (as mecab-ko does) instead of the ending `EP`. See the Segmenter documentation for the semantics and the remaining differences from mecab-ko's whitespace handling.

## User dictionary format (CSV)

### Simple version

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet |
| 2 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |

### Detailed version

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet |
| 5 | 의미 부류 | meaning | (too few examples for me to be sure) |
| 6 | 종성 유무 | presence or absence | `T` for true; `F` for false; else `*` |
| 7 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |
| 8 | 타입 | type | One of: `Inflect` (활용); `Compound` (복합명사); or `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | first part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `VV` |
| 10 | 마지막 품사 | last part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `EP` |
| 11 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – Fields that tell how usage, compound nouns, and key analysis are organized |
| 12 | - | - | After 12, it can be freely expanded. |

## API reference

The API reference is available. Please see following URL:

- [lindera-ko-dic](https://docs.rs/lindera-ko-dic)
