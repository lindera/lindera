# Lindera UniDic CSJ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-unidic-csj.svg)](https://crates.io/crates/lindera-unidic-csj)

## 辞書バージョン

このクレートは、[現代話し言葉UniDic](https://clrd.ninjal.ac.jp/unidic/)（UniDic CSJ）ver.2025.12 を組み立てます。話し言葉を書き起こしたテキストのための辞書で、日本語日常会話コーパス（CEJC）で学習しています。元データは国立国語研究所のフルパッケージ `unidic-csj-202512_full.zip` で、国立国語研究所の公式の URL から取得し、`build.rs` に固定した SHA-256 と照合します。

UniDic CSJ の見出し語と項目は、現代書き言葉UniDic である UniDic CWJ（[`lindera-unidic-cwj`](https://crates.io/crates/lindera-unidic-cwj)）と同じで、単語のコストと連接表が違います。話し言葉の書き起こしには UniDic CSJ を、書き言葉には UniDic CWJ を使ってください。

## 大きさ

UniDic CSJ は、多くの辞書よりずっと大きい辞書です。連接表は 24,314 × 21,609 で、組み立てた辞書は 1.2 GB、そのうち 1,002 MiB が連接表です。`embed-unidic-csj` を使うと、実行ファイルはどれもこの 1.2 GB を含み、このクレートのコンパイルには、UniDic CWJ と同じく 8〜11 GiB のメモリを使います（macOS で計測したビルドでの値）。実行ファイルを小さく保つには、`lindera download unidic-csj` で辞書を取得し、そのディレクトリから読み込んでください。WASM のバインディングは `embed-unidic-csj` を提供しません。

## 辞書フォーマット

各項目とその値については、[UniDic の FAQ](https://clrd.ninjal.ac.jp/unidic/faq.html) を参照してください。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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
| 29 | アクセント結合型 | Accent connection type | `動詞%F2@0,名詞%F1` のようにコンマを含むことがある |
| 30 | アクセント修飾型 | Accent modification type | |
| 31 | 語彙表ID | Lexicon ID | |
| 32 | 語彙素ID | Lemma ID | |

## ユーザー辞書フォーマット（CSV）

### 簡易版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Part-of-speech | |
| 2 | 語彙素読み | Reading | |

### 詳細版

上の辞書フォーマットの 33 項目を、同じ順に並べます。インデックス 32 より後は自由に拡張できます。

## API リファレンス

API リファレンスは次の URL で参照できます。

- [lindera-unidic-csj](https://docs.rs/lindera-unidic-csj)
