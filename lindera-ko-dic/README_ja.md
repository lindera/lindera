# Lindera ko-dic

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-ko-dic.svg)](https://crates.io/crates/lindera-ko-dic)

## 辞書バージョン

このリポジトリには [mecab-ko-dic](https://github.com/lindera-morphology/mecab-ko-dic) が含まれています。

## 辞書フォーマット

mecab-ko-dic で使用される辞書フォーマットと品詞タグの情報は、mecab-ko-dic の[リポジトリ README](https://bitbucket.org/eunjeon/mecab-ko-dic/src/master/README.md) からリンクされている[この Google スプレッドシート](https://docs.google.com/spreadsheets/d/1-9blXKjtjeKZqsf4NzHeYJCrr49-nXeRF6D80udfcwY/edit#gid=589544265)に記載されています。

ko-dic は NAIST JDIC よりも素性カラムが 1 つ少なく、まったく異なる情報セットを持っていることに注意してください（例: 単語の「原形」は提供されません）。

タグは세종（Sejong）で規定されたものを若干修正したものです。Sejong から mecab-ko-dic のタグ名へのマッピングは、上記スプレッドシートの `태그 v2.0` タブに記載されています。

辞書フォーマットの完全な仕様（韓国語）は、スプレッドシートの `사전 형식 v2.0` タブに記載されています。空白の値はデフォルトで `*` になります。

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | Part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | Meaning | （例が少ないため確定的ではありません） |
| 6 | 종성 유무 | Presence or absence | `T` は true、`F` は false、それ以外は `*` |
| 7 | 읽기 | Reading | 通常は表層形と一致しますが、外来語（例: 漢字語）では異なる場合があります |
| 8 | 타입 | Type | `Inflect`（活用）、`Compound`（복합명사）、`Preanalysis`（기분석）のいずれか |
| 9 | 첫번째 품사 | First part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`VV` を返します |
| 10 | 마지막 품사 | Last part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`EP` を返します |
| 11 | 표현 | Expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` -- 活用・複合名詞・基分析がどのように構成されるかを示すフィールド |

## 左側空白ペナルティ

mecab-ko（mecab-ko-dic が対象とする MeCab のフォーク）は、語の左側に空白がある場合、その語の `pos-id.def` の ID が `dicrc` に列挙されていれば連接コストを加算します:

```text
left-space-penalty-factor = 100,3000,120,6000,172,3000,183,3000,184,3000,185,3000,200,3000,210,6000,220,3000,221,3000,222,3000,230,3000
```

Lindera は `pos-id.def` の ID を保持しませんが、列挙されている ID はすべてエントリの先頭品詞タグ（`Inflect` 行では「最初の品詞」列）に対応するため、同じルールをタグで記述し、`Segmenter::space_penalty`、セグメンター設定の `space_penalty` キー、または `lindera tokenize --space-penalty-rules` で有効にできます。デフォルトではオフです。

| pos-id.def の ID | 先頭品詞タグ | コスト |
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
echo "서울 시 에서 출발" | lindera tokenize --dict embedded://ko-dic \
  --space-penalty-rules '{"rules":[{"pos":["EC","EF","EP","ETM","ETN","VCP","XSA","XSN","XSV"],"cost":3000},{"pos":["JC","JKB","JKC","JKG","JKO","JKQ","JKS","JKV","JX"],"cost":6000}]}'
```

ペナルティを有効にすると、`서울 시 에서` の `시` は語尾 `EP` ではなく（mecab-ko と同様に）名詞 `NNG` と解析されます。意味論と、mecab-ko の空白処理との残る差異については Segmenter のドキュメントを参照してください。

## ユーザー辞書フォーマット（CSV）

### 簡易版

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 2 | 읽기 | reading | 通常は表層形と一致しますが、外来語（例: 漢字語）では異なる場合があります |

### 詳細版

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | meaning | （例が少ないため確定的ではありません） |
| 6 | 종성 유무 | presence or absence | `T` は true、`F` は false、それ以外は `*` |
| 7 | 읽기 | reading | 通常は表層形と一致しますが、外来語（例: 漢字語）では異なる場合があります |
| 8 | 타입 | type | `Inflect`（活用）、`Compound`（복합명사）、`Preanalysis`（기분석）のいずれか |
| 9 | 첫번째 품사 | first part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`VV` を返します |
| 10 | 마지막 품사 | last part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`EP` を返します |
| 11 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` -- 活用・複合名詞・基分析がどのように構成されるかを示すフィールド |
| 12 | - | - | 12 以降は自由に拡張できます。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-ko-dic](https://docs.rs/lindera-ko-dic)
