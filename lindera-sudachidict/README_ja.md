# Lindera SudachiDict

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-sudachidict.svg)](https://crates.io/crates/lindera-sudachidict)

## 辞書バージョン

このリポジトリには [SudachiDict](https://github.com/lindera/sudachidict) 20260723（small + core + notcore）が含まれています。

## 辞書フォーマット

SudachiDict の辞書フォーマットと品詞タグの詳細については [SudachiDict のドキュメント](https://github.com/WorksApplications/SudachiDict)を参照してください。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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

> **注意:** 見出し（display surface）がインデックス 4（品詞カラムの前）に
> あるため、トークン詳細の先頭は IPADIC や UniDic のような品詞ではなく
> 見出しになります。詳細の先頭を品詞タグとして位置ベースで読むトークン
> フィルターはこの辞書では期待通りにマッチしません。
> `token.get("part_of_speech")` のようなスキーマベースのアクセスは正しく
> 動作します。詳細は
> [lindera/lindera#997](https://github.com/lindera/lindera/issues/997) を
> 参照してください。

## ユーザー辞書フォーマット (CSV)

### シンプル版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 読み | Reading | |

### 詳細版

詳細版は上記の辞書フォーマット（19 カラム）に従います。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0-18 | - | 辞書フォーマットと同じ | |
| 19 | - | - | 19 以降は自由に拡張できます。 |

## Sudachi との挙動の違い

この辞書は SudachiDict の語彙とラティスの挙動を再現しますが、Sudachi
エンジンのプラグイン（カタカナ・数値の結合、入力正規化、A/B/C 分割
モード）は再現しません。詳細は Lindera ドキュメントの
[SudachiDict ページ](https://lindera.github.io/docs/sudachidict.html)を
参照してください。

## API リファレンス

API リファレンスは以下の URL を参照してください:

- [lindera-sudachidict](https://docs.rs/lindera-sudachidict)
