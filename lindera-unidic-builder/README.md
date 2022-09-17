# Lindera UniDic Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

UniDic builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Install

```shell script
% cargo install lindera-unidic-builder
```


## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```


## Dictionary version

This project supports UniDic 2.1.2.
See [detail of UniDic](https://clrd.ninjal.ac.jp/unidic/) .


## Building a dictionary

Building a dictionary with `lindera-unidic-builder` command:

```shell script
% curl -l -o /tmp/unidic-mecab-2.1.2_src.zip "https://clrd.ninjal.ac.jp/unidic_archive/cwj/2.1.2/unidic-mecab-2.1.2_src.zip"
% unzip /tmp/unidic-mecab-2.1.2_src.zip -d /tmp
% lindera-unidic-builder -s /tmp/unidic-mecab-2.1.2_src -d /tmp/lindera-unidic-2.1.2
```


## Building a user dictionary

Building a dictionary with `lindera-unidic-builder` command:

```shell script
% lindera-unidic-builder -S ./resources/simple_userdic.csv -D ./resources/unidic_userdic.bin
```


## Dictionary format

Refer to the [manual](ftp://ftp.jaist.ac.jp/pub/sourceforge.jp/unidic/57618/unidic-mecab.pdf) for details on the unidic-mecab dictionary format and part-of-speech tags.

| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 左文脈ID | Left context ID |
| 2 | 右文脈ID | Right context ID |
| 3 | コスト | Cost |
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


## User dictionary format (CSV)

Simple version
| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 品詞大分類 | Major POS classification | |
| 2 | 語彙素読み | Lexeme reading | |

Detailed version
| Index | Name (Japanese) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 表層形 | Surface |
| 1 | 左文脈ID | Left context ID |
| 2 | 右文脈ID | Right context ID |
| 3 | コスト | Cost |
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


## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "羽田空港限定トートバッグ" | lindera -k unidic -d /tmp/lindera-unidic-2.1.2
```

```text
羽田    名詞,固有名詞,人名,姓,*,*,ハタ,ハタ,羽田,ハタ,羽田,ハタ,固,*,*,*,*
空港    名詞,普通名詞,一般,*,*,*,クウコウ,空港,空港,クーコー,空港,クーコー,漢,*,*,*,*
限定    名詞,普通名詞,サ変可能,*,*,*,ゲンテイ,限定,限定,ゲンテー,限定,ゲンテー,漢,*,*,*,*
トート  名詞,普通名詞,一般,*,*,*,トート,トート,トート,トート,トート,トート,外,*,*,*,*
バッグ  名詞,普通名詞,一般,*,*,*,バッグ,バッグ-bag,バッグ,バッグ,バッグ,バッグ,外,*,*,*,*
EOS
```

## Tokenizing text using UniDic dictionary and produced binary user dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera -k unidic -u ./resources/unidic_userdic.bin -t binary
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,トウキョウスカイツリー,*,*,*,*,*,*,*,*,*,*
の      助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*
最寄り  名詞,普通名詞,一般,*,*,*,モヨリ,最寄り,最寄り,モヨリ,最寄り,モヨリ,和,*,*,*,*
駅      名詞,普通名詞,一般,*,*,*,エキ,駅,駅,エキ,駅,エキ,漢,*,*,*,*
は      助詞,係助詞,*,*,*,*,ハ,は,は,ワ,は,ワ,和,*,*,*,*
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,トウキョウスカイツリーエキ,*,*,*,*,*,*,*,*,*,*
です    助動詞,*,*,*,助動詞-デス,終止形-一般,デス,です,です,デス,です,デス,和,*,*,*,*
EOS
```

You can use other user dictionary (e.g. IPADIC) with UniDic.
But, note that the detailed information of the words will be others one.

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-unidic-builder" target="_blank">Lindera UniDic Builder</a>
