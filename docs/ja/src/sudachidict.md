# SudachiDict（コミュニティレシピ）

このページでは、[SudachiDict](https://github.com/WorksApplications/SudachiDict)
— Sudachi が使用する、活発にメンテナンスされている日本語辞書 — を、汎用辞書ビルダーと
メタデータファイルのみを使って Lindera 辞書としてビルドする方法を説明します。
エンジン側の変更は不要です。

関連 Issue: [lindera/lindera#487](https://github.com/lindera/lindera/issues/487)

> [!NOTE]
> このレシピで生成される辞書は**語彙とラティスの挙動**が Sudachi と一致しますが、
> Sudachi のエンジンプラグインは再現しません。利用する前に、下記の
> [Sudachi との挙動の違い](#sudachi-との挙動の違い) を確認してください。

## なぜ SudachiDict か

Lindera が同梱する MeCab フォーマットの辞書（IPADIC、UniDic 2.1.2）は、語彙の更新が
何年も前に止まっています。SudachiDict は年に数回更新されており、MeCab 互換の raw CSV
フォーマットで配布されているため、汎用ビルダーでそのままコンパイルできます。
`20260723` リリースでは:

- `令和` が単一の固有名詞になります（IPADIC と UniDic 2.1.2 は `令|和` に分割します）
- `スマホ`、`テレワーク`、`推し活`、`コロナ禍` が語彙に含まれます
- 1905 年の 321K 文字の小説では、未知語トークンの割合が 2.00%（IPADIC）から
  0.51% に下がります

## ソースファイル

SudachiDict の raw 辞書ファイル（最新の日付は
[一覧](http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/)
を確認してください）:

```shell
% mkdir -p /tmp/sudachidict-src
% cd /tmp/sudachidict-src
% for f in small_lex core_lex notcore_lex; do
    curl -LO "http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/20260723/${f}.zip"
    unzip -o "${f}.zip" && rm "${f}.zip"
  done
```

連接コスト行列。SudachiDict 自身のビルドがダウンロードするファイルで、UniDic 2.1.2 の
行列と同一です（SudachiDict のエントリは UniDic の文脈 ID を使用しています）:

```shell
% curl -LO "https://d2ej7fkh96fzlu.cloudfront.net/sudachidict-raw/matrix.def.zip"
% unzip -o matrix.def.zip && rm matrix.def.zip
```

文字種定義と未知語定義は UniDic 2.1.2 のソース（`lindera-unidic` がビルドに使う
アーカイブと同じもの）から取得します:

```shell
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp
% cp /tmp/unidic-mecab-2.1.2/char.def /tmp/unidic-mecab-2.1.2/unk.def .
```

## unk.def を SudachiDict のカラムレイアウトに揃える

SudachiDict の辞書行は 19 カラムで、カラム 4（0 始まり）は表示用の表層形（display
surface）です。そのため形態素の詳細情報はカラム 5 から始まります。一方 UniDic の
`unk.def` は品詞がカラム 4 にあります。プレースホルダの表示カラムを挿入して、
未知語の詳細情報が辞書語と同じインデックスに来るようにします:

```shell
% awk -F, 'BEGIN{OFS=","} {out=$1 OFS $2 OFS $3 OFS $4 OFS "*"; for(i=5;i<=NF;i++) out=out OFS $i; print out}' unk.def > unk.def.tmp && mv unk.def.tmp unk.def
```

## メタデータ

`sudachidict-metadata.json` として保存します。先頭 4 フィールドはビルダーの予約
カラムで、残りは SudachiDict のカラムを順に列挙したものです:

```json
{
  "name": "sudachidict",
  "encoding": "UTF-8",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": true,
  "skip_invalid_cost_or_id": true,
  "normalize_details": false,
  "dictionary_schema": {
    "fields": [
      "surface",
      "left_context_id",
      "right_context_id",
      "cost",
      "display_surface",
      "part_of_speech",
      "part_of_speech_subcategory_1",
      "part_of_speech_subcategory_2",
      "part_of_speech_subcategory_3",
      "conjugation_type",
      "conjugation_form",
      "reading",
      "normalized_form",
      "dictionary_form_id",
      "split_mode",
      "split_a",
      "split_b",
      "word_structure",
      "synonym_group_ids"
    ]
  },
  "user_dictionary_schema": { "fields": ["surface", "part_of_speech", "reading"] }
}
```

## ビルドと確認

```shell
% lindera build \
  --src /tmp/sudachidict-src \
  --dest /tmp/lindera-sudachidict \
  --metadata ./sudachidict-metadata.json
```

small + core + notcore（2026-07-23）のビルドは Lindera v6 で約 10 秒、生成される辞書は
約 570MB です。なお、Lindera のリリース間でオンディスク辞書フォーマットのバージョンが
変わった場合、ビルド済み辞書は再ビルドが必要です
（[v5 から v6 への移行](./migration_v5_to_v6.md) を参照）。

```shell
% echo "令和五年に始まった。推し活が楽しい。" | lindera tokenize --dict /tmp/lindera-sudachidict
令和	令和,名詞,固有名詞,一般,*,*,*,レイワ,令和,*,A,*,*,*,*
五	五,名詞,数詞,*,*,*,*,ゴ,五,*,A,*,*,*,017040
年	年,名詞,普通名詞,助数詞可能,*,*,*,ネン,年,*,A,*,*,*,*
...
推し活	推し活,名詞,普通名詞,一般,*,*,*,オシカツ,推し活,*,A,*,*,*,*
```

詳細情報には SudachiDict の追加カラム（正規化形、分割参照、同義語グループ ID）が
含まれるため、後段の処理から利用できます。

> [!NOTE]
> `display_surface` が品詞カラムの前にあるため、`details[0]` は IPADIC や UniDic の
> ような品詞ではなく表示用表層形になります。details の先頭を品詞タグとして位置ベースで
> 読むトークンフィルタ（`japanese_stop_tags`、`japanese_keep_tags`、
> `japanese_compound_word`）は、この辞書では期待どおりにマッチしません。
> `token.get("part_of_speech")` のようなスキーマ経由のアクセスは正しく動作します。

## Sudachi との挙動の違い

ラティス層は等価であることを検証済みです: 行列ファイルは SudachiDict の公式ビルドが
使用するものと byte 単位で同一（sha256）であり、Sudachi のパス書き換え・入力正規化
プラグインを無効にすると、Sudachi エンジンはこの辞書を使う Lindera と同じ最小コスト
パスを生成します。残る違いは辞書データではなく Sudachi の*エンジンプラグイン*です:

| Sudachi の機能 | 効果 | Lindera での状況 |
| --- | --- | --- |
| `JoinKatakanaOovPlugin` | カタカナ連続を結合するパス書き換え（例: よりコストの低い `サブ`+`スク` のパスではなく `サブスク`） | 未対応 |
| `JoinNumericPlugin` | 数値列の結合 | 未対応 |
| `DefaultInputTextPlugin` | 検索前の NFKC + 小文字化による正規化（例: 半角の `AI` が正規化済みエントリにヒット） | Lindera の character filter（`unicode_normalize`）で部分的に代替可能 |
| A/B/C 分割モード | `split_a` / `split_b` 参照による辞書エントリの分割 | 未適用。エントリは分割せずそのまま使用されます。分割参照は詳細情報に保持されるため、後処理で適用できます |

## ライセンス

SudachiDict は Apache-2.0 です。`matrix.def`、`char.def`、`unk.def` は UniDic の一部です
（BSD/GPL/LGPL のトリプルライセンス —
[SudachiDict の LEGAL notice](https://github.com/WorksApplications/SudachiDict/blob/develop/LEGAL)
を参照）。このレシピでビルドした辞書は両方を含みます。再配布の前にライセンスを
確認してください。
