# Lindera ko-dic

## 辞書バージョン

このリポジトリは [mecab-ko-dic](https://github.com/lindera-morphology/mecab-ko-dic) を含んでいます。

## 辞書フォーマット

mecab-ko-dicで使用されている辞書フォーマットと品詞タグに関する情報は、mecab-ko-dicの [リポジトリreadme](https://bitbucket.org/eunjeon/mecab-ko-dic/src/master/README.md) からリンクされている [Googleスプレッドシート](https://docs.google.com/spreadsheets/d/1-9blXKjtjeKZqsf4NzHeYJCrr49-nXeRF6D80udfcwY/edit#gid=589544265) にドキュメント化されています。

ko-dicはNAIST JDICよりも機能列が1つ少なく、全く異なる情報セットを持っています（例：「原形」を提供していません）。

タグはSejongによって指定されたものを少し修正したものです。Sejongからmecab-ko-dicのタグ名へのマッピングは、上記スプレッドシートの `태그 v2.0` タブに記載されています。

辞書フォーマットは、スプレッドシートの `사전 형식 v2.0` タブに（韓国語で）完全に指定されています。空の値は `*` がデフォルトとなります。

| インデックス | 名前 (韓国語) | 名前 (英語) | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | Part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | Meaning | |
| 6 | 종성 유무 | Presence or absence | `T` (真)、`F` (偽)、その他は `*` |
| 7 | 읽기 | Reading | 通常は表層形と一致しますが、漢字語などの外来語では異なる場合があります |
| 8 | 타입 | Type | 次のいずれか: `Inflect` (활용)、`Compound` (복합명사)、または `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | First part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`VV` を返します |
| 10 | 마지막 품사 | Last part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`EP` を返します |
| 11 | 표현 | Expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – 活用、複合名詞、既分析がどのように構成されているかを示すフィールド |

## ユーザー辞書フォーマット (CSV)

### シンプル版 (Simple version)

| インデックス | 名前 (韓国語) | 名前 (英語) | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 2 | 읽기 | reading | 通常は表層形と一致しますが、漢字語などの外来語では異なる場合があります |

### 詳細版 (Detailed version)

| インデックス | 名前 (韓国語) | 名前 (英語) | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | meaning | |
| 6 | 종성 유무 | presence or absence | `T` (真)、`F` (偽)、その他は `*` |
| 7 | 읽기 | reading | 通常は表層形と一致しますが、漢字語などの外来語では異なる場合があります |
| 8 | 타입 | type | 次のいずれか: `Inflect` (활용)、`Compound` (복합명사)、または `Preanalysis` (기분석) |
| 9 | 첫번째 품사 | first part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`VV` を返します |
| 10 | 마지막 품사 | last part-of-speech | 例: 品詞タグが "VV+EM+VX+EP" の場合、`EP` を返します |
| 11 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – 活用、複合名詞、既分析がどのように構成されているかを示すフィールド |
| 12 | - | - | 12以降は自由に拡張可能です。 |

## APIリファレンス

APIリファレンスは以下で公開されています：

- [lindera-ko-dic](https://docs.rs/lindera-ko-dic)
