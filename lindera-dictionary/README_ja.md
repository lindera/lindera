# Lindera Dictionary

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-dictionary.svg)](https://crates.io/crates/lindera-dictionary)

[Lindera](https://github.com/lindera-morphology/lindera) の形態素解析辞書ライブラリ。

このパッケージは辞書構造と Viterbi アルゴリズムを含みます。

## 辞書フォーマット

### IPADIC

このリポジトリは [mecab-ipadic](https://github.com/lindera-morphology/mecab-ipadic) を使用しています。

#### IPADIC 辞書フォーマット

IPADIC の辞書フォーマットと品詞タグの詳細は[マニュアル](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf)を参照してください。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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

#### IPADIC ユーザー辞書フォーマット（CSV）

##### IPADIC ユーザー辞書 簡易版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | Major POS classification | |
| 2 | 読み | Reading | |

##### IPADIC ユーザー辞書 詳細版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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
| 13 | - | - | 13 以降は自由に拡張可能 |

### IPADIC NEologd

このリポジトリは [mecab-ipadic-neologd](https://github.com/lindera-morphology/mecab-ipadic-neologd) を使用しています。

#### IPADIC NEologd 辞書フォーマット

IPADIC の辞書フォーマットと品詞タグの詳細は[マニュアル](https://ja.osdn.net/projects/ipadic/docs/ipadic-2.7.0-manual-en.pdf/en/1/ipadic-2.7.0-manual-en.pdf.pdf)を参照してください。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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

#### IPADIC NEologd ユーザー辞書フォーマット（CSV）

##### IPADIC NEologd ユーザー辞書 簡易版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | surface | |
| 1 | 品詞 | Major POS classification | |
| 2 | 読み | Reading | |

##### IPADIC NEologd ユーザー辞書 詳細版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
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
| 13 | - | - | 13 以降は自由に拡張可能 |

### UniDic

このリポジトリは [unidic-mecab](https://github.com/lindera-morphology/unidic-mecab) を使用しています。

#### UniDic 辞書フォーマット

unidic-mecab の辞書フォーマットと品詞タグの詳細は[マニュアル](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf)を参照してください。

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification | |
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
| 19 | 語末変化型 | Suffix of a word form | |
| 20 | 語末変化形 | Suffix of a word type | |

#### UniDic ユーザー辞書フォーマット（CSV）

##### UniDic ユーザー辞書 簡易版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 品詞大分類 | Major POS classification | |
| 2 | 語彙素読み | Lexeme reading | |

##### UniDic ユーザー辞書 詳細版

| インデックス | 名前（日本語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface | |
| 1 | 左文脈ID | Left context ID | |
| 2 | 右文脈ID | Right context ID | |
| 3 | コスト | Cost | |
| 4 | 品詞大分類 | Major POS classification | |
| 5 | 品詞中分類 | Middle POS classification | |
| 6 | 品詞小分類 | Small POS classification | |
| 7 | 品詞細分類 | Fine POS classification | |
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
| 19 | 語末変化型 | Suffix of a word form | |
| 20 | 語末変化形 | Suffix of a word type | |
| 21 | - | - | 21 以降は自由に拡張可能 |

### ko-dic

このリポジトリは [mecab-ko-dic](https://github.com/lindera-morphology/mecab-ko-dic) を使用しています。

#### ko-dic 辞書フォーマット

mecab-ko-dic が使用する辞書フォーマットと品詞タグの情報は、mecab-ko-dic の[リポジトリ README](https://bitbucket.org/eunjeon/mecab-ko-dic/src/master/README.md) からリンクされている[この Google スプレッドシート](https://docs.google.com/spreadsheets/d/1-9blXKjtjeKZqsf4NzHeYJCrr49-nXeRF6D80udfcwY/edit#gid=589544265)に記載されています。

ko-dic は NAIST JDIC よりも素性カラムが1つ少なく、全く異なる情報セット（例: 単語の「原形」は提供されない）を持っていることに注意してください。

タグは세종（Sejong）で規定されたものを若干修正したものです。Sejong から mecab-ko-dic のタグ名へのマッピングは、上記リンクのスプレッドシートの `태그 v2.0` タブに記載されています。

辞書フォーマットの完全な仕様（韓国語）はスプレッドシートの `사전 형식 v2.0` タブに記載されています。空白値のデフォルトは `*` です。

| インデックス | 名前（韓国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | meaning | （例が少ないため確証なし） |
| 6 | 종성 유무 | presence or absence | `T` は true、`F` は false、それ以外は `*` |
| 7 | 읽기 | reading | 通常は表層形と一致するが、外来語（例: 漢字語）では異なる場合がある |
| 8 | 타입 | type | `Inflect`（活用）、`Compound`（複合名詞）、`Preanalysis`（既分析）のいずれか |
| 9 | 첫번째 품사 | first part-of-speech | 例: 品詞タグ "VV+EM+VX+EP" の場合、`VV` を返す |
| 10 | 마지막 품사 | last part-of-speech | 例: 品詞タグ "VV+EM+VX+EP" の場合、`EP` を返す |
| 11 | 표현 | expression | 活用、複合名詞、既分析がどのように構成されるかを示すフィールド |

#### ko-dic ユーザー辞書フォーマット（CSV）

##### ko-dic ユーザー辞書 簡易版

| インデックス | 名前（韓国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 2 | 읽기 | reading | 通常は表層形と一致するが、外来語（例: 漢字語）では異なる場合がある |

##### ko-dic ユーザー辞書 詳細版

| インデックス | 名前（韓国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 표면 | Surface | |
| 1 | 왼쪽 문맥 ID | Left context ID | |
| 2 | 오른쪽 문맥 ID | Right context ID | |
| 3 | 비용 | Cost | |
| 4 | 품사 태그 | part-of-speech tag | スプレッドシートの `태그 v2.0` タブを参照 |
| 5 | 의미 부류 | meaning | （例が少ないため確証なし） |
| 6 | 종성 유무 | presence or absence | `T` は true、`F` は false、それ以外は `*` |
| 7 | 읽기 | reading | 通常は表層形と一致するが、外来語（例: 漢字語）では異なる場合がある |
| 8 | 타입 | type | `Inflect`（活用）、`Compound`（複合名詞）、`Preanalysis`（既分析）のいずれか |
| 9 | 첫번째 품사 | first part-of-speech | 例: 品詞タグ "VV+EM+VX+EP" の場合、`VV` を返す |
| 10 | 마지막 품사 | last part-of-speech | 例: 品詞タグ "VV+EM+VX+EP" の場合、`EP` を返す |
| 11 | 표현 | expression | 活用、複合名詞、既分析がどのように構成されるかを示すフィールド |
| 12 | - | - | 12 以降は自由に拡張可能 |

### CC-CEDICT

このリポジトリは [CC-CEDICT-MeCab](https://github.com/lindera/CC-CEDICT-MeCab) を使用しています。

#### CC-CEDICT 辞書フォーマット

unidic-mecab の辞書フォーマットと品詞タグの詳細は[マニュアル](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf)を参照してください。

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
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

#### CC-CEDICT ユーザー辞書フォーマット（CSV）

##### CC-CEDICT ユーザー辞書 簡易版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Major POS classification | |
| 2 | 併音 | pinyin | |

##### CC-CEDICT ユーザー辞書 詳細版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
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
| 12 | - | - | 12 以降は自由に拡張可能 |

### Jieba

このリポジトリは [mecab-jieba](https://github.com/lindera/mecab-jieba) を使用しています。

#### Jieba 辞書フォーマット

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 併音 | Pinyin | |
| 6 | 繁体字 | Traditional | |
| 7 | 簡体字 | Simplified | |
| 8 | 定义 | Definition | |

#### Jieba ユーザー辞書フォーマット（CSV）

##### Jieba ユーザー辞書簡易版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 词类 | Part-of-speech | |
| 2 | 併音 | Pinyin | |

##### Jieba ユーザー辞書詳細版

| インデックス | 名前（中国語） | 名前（英語） | 備考 |
| --- | --- | --- | --- |
| 0 | 表面形式 | Surface | |
| 1 | 左语境ID | Left context ID | |
| 2 | 右语境ID | Right context ID | |
| 3 | 成本 | Cost | |
| 4 | 词类 | Part-of-speech | |
| 5 | 併音 | Pinyin | |
| 6 | 繁体字 | Traditional | |
| 7 | 簡体字 | Simplified | |
| 8 | 定义 | Definition | |
| 9 | - | - | 9 以降は自由に拡張可能 |

## API リファレンス

API リファレンスは以下の URL を参照してください:

- [lindera-dictionary](https://docs.rs/lindera-dictionary)
