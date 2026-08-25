# Lindera SudachiDict

## 辞書バージョン

このリポジトリには [SudachiDict](https://github.com/lindera/sudachidict) 20260723（small + core + notcore レキシコン）が含まれています。

## 辞書フォーマット

辞書フォーマットおよび品詞タグの詳細については、[SudachiDict のドキュメント](https://github.com/WorksApplications/SudachiDict)を参照してください。

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
> 見出し（解析結果表示用）が品詞カラムより前のインデックス 4 に位置するため、
> トークン詳細（details）の先頭は、IPADIC や UniDic のような品詞ではなく見出しに
> なります。先頭の詳細を品詞タグとして位置ベースで読み取るトークンフィルタ
> （`japanese_stop_tags`、`japanese_keep_tags`、`japanese_compound_word`）は、
> この辞書では期待どおりにマッチしません。`token.get("part_of_speech")` のような
> スキーマを認識するアクセスは正しく動作します。詳細は
> [lindera/lindera#997](https://github.com/lindera/lindera/issues/997) を参照してください。

## ユーザー辞書フォーマット (CSV)

### 簡易版

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 読み | Reading | |

### 詳細版

詳細版は上記の辞書フォーマット（19 カラム）に従います。

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0-18 | - | Same as the dictionary format | |
| 19 | - | - | 19 以降は自由に拡張可能です。 |

## API リファレンス

API リファレンスは以下の URL から参照できます:

- [lindera-sudachidict](https://docs.rs/lindera-sudachidict)
