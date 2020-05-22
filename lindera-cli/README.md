# Lindera CLI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese morphological analysis command-line interface for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

## Install

```shell script
% cargo install lindera-cli
```

## Build

The following products are required to build:

- Rust >= 1.39.0
- make >= 3.81

```shell script
% cargo build --release
```

## Usage

### Basic usage

The CLI already includes IPADIC as the default Japanese dictionary.  
You can easily tokenize the text and see the results as follows:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

### Switching dictionary

It is also possible to switch to the pre-built dictionary data instead of the default dictionary and tokenize.

#### IPADIC

The following example uses the pre-built IPADIC to tokenize:

```shell script
% curl -L -O "https://github.com/lindera-morphology/lindera-ipadic-builder/releases/download/v0.3.2/lindera-ipadic-2.7.0-20070801.tar.bz2"
% tar -xjvf ./lindera-ipadic-2.7.0-20070801.tar.bz2
```

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -d ./lindera-ipadic-2.7.0-20070801
```

```text
関西国際空港	名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定	名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ	UNK
EOS
```

#### IPADIC NEologd

The following example uses the pre-built IPADIC-NEologd to tokenize:

```shell script
% curl -L -O "https://github.com/lindera-morphology/lindera-ipadic-neologd-builder/releases/download/v0.1.2/lindera-ipadic-2.7.0-20070801-neologd-20200130.tar.bz2"
% tar -xjvf ./lindera-ipadic-2.7.0-20070801-neologd-20200130.tar.bz2
```

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -d ./lindera-ipadic-2.7.0-20070801-neologd-20200130
```

```text
関西国際空港	名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定	名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ	名詞,固有名詞,一般,*,*,*,トートバッグ,トートバッグ,トートバッグ
EOS
```

#### UniDic

The following example uses the pre-built UniDic to tokenize:

```shell script
% curl -L -O "https://github.com/lindera-morphology/lindera-unidic-builder/releases/download/v0.3.2/lindera-unidic-2.1.2.tar.bz2"
% tar -xjvf ./lindera-unidic-2.1.2.tar.bz2
```

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -d ./lindera-unidic-2.1.2
```

```text
関西	名詞,固有名詞,地名,一般,*,*,カンサイ,カンサイ,関西,カンサイ,関西,カンサイ,固,*,*,*,*
国際	名詞,普通名詞,一般,*,*,*,コクサイ,国際,国際,コクサイ,国際,コクサイ,漢,*,*,*,*
空港	名詞,普通名詞,一般,*,*,*,クウコウ,空港,空港,クーコー,空港,クーコー,漢,*,*,*,*
限定	名詞,普通名詞,サ変可能,*,*,*,ゲンテイ,限定,限定,ゲンテー,限定,ゲンテー,漢,*,*,*,*
トート	名詞,普通名詞,一般,*,*,*,トート,トート,トート,トート,トート,トート,外,*,*,*,*
バッグ	名詞,普通名詞,一般,*,*,*,バッグ,バッグ-bag,バッグ,バッグ,バッグ,バッグ,外,*,*,*,*
EOS
```

#### ko-dic

The following example uses the pre-built ko-dic to tokenize:

```shell script
% curl -L -O "https://github.com/lindera-morphology/lindera-ko-dic-builder/releases/download/v0.1.0/lindera-ko-dic-2.1.1-20180720.tar.bz2"
% tar -xjvf ./lindera-ko-dic-2.1.1-20180720.tar.bz2
```

```shell script
% echo "하네다공항한정토트백" | lindera -d ./lindera-ko-dic-2.1.1-20180720
```

```text
하네다	NNP,인명,F,하네다,*,*,*,*
공항	NNG,장소,T,공항,*,*,*,*
한정	NNG,*,T,한정,*,*,*,*
토트백	NNG,*,T,토트백,Compound,*,*,토트/NNP/인명+백/NNG/*
EOS
```

Please refer to the following repository for building a dictionary:

- <a href="https://github.com/lindera-morphology/lindera-ipadic-builder" target="_blank">Lindera IPADIC Builder</a>
- <a href="https://github.com/lindera-morphology/lindera-ipadic-neologd-builder" target="_blank">Lindera IPDIC NEologd Builder</a>
- <a href="https://github.com/lindera-morphology/lindera-unidic-builder" target="_blank">Lindera UniDic Builder</a>
- <a href="https://github.com/lindera-morphology/lindera-ko-dic-builder" target="_blank">Lindera ko-dic Builder</a>

### Tokenize mode

Lindera provides two tokenization modes: `normal` and `decompose`.

`normal` mode tokenizes faithfully based on words registered in the dictionary. (Default):

```shell script
% echo "関西国際空港限定トートバッグ" | lindera --mode=normal
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

`decopose` mode tokenizes a compound noun words additionally:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera --mode=decompose
```

```text
関西    名詞,固有名詞,地域,一般,*,*,関西,カンサイ,カンサイ
国際    名詞,一般,*,*,*,*,国際,コクサイ,コクサイ
空港    名詞,一般,*,*,*,*,空港,クウコウ,クーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

### Output format

Lindera provides three output formats: `mecab`, `wakati` and `json`.

`mecab` outputs results in a format like MeCab:

```shell script
% echo "お待ちしております。" | lindera --output=mecab
```

```text
お待ち	名詞,サ変接続,*,*,*,*,お待ち,オマチ,オマチ
し	動詞,自立,*,*,サ変・スル,連用形,する,シ,シ
て	助詞,接続助詞,*,*,*,*,て,テ,テ
おり	動詞,非自立,*,*,五段・ラ行,連用形,おる,オリ,オリ
ます	助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

`wakati` outputs the token text separated by spaces:

```shell script
% echo "お待ちしております。" | lindera --output=wakati
```

```text
お待ち し て おり ます 。
```

`json` outputs the token information in JSON format:

```shell script
% echo "お待ちしております。" | lindera --output=json
```

```json
[
  {
    "text": "お待ち",
    "detail": [
      "名詞",
      "サ変接続",
      "*",
      "*",
      "*",
      "*",
      "お待ち",
      "オマチ",
      "オマチ"
    ]
  },
  {
    "text": "し",
    "detail": [
      "動詞",
      "自立",
      "*",
      "*",
      "サ変・スル",
      "連用形",
      "する",
      "シ",
      "シ"
    ]
  },
  {
    "text": "て",
    "detail": [
      "助詞",
      "接続助詞",
      "*",
      "*",
      "*",
      "*",
      "て",
      "テ",
      "テ"
    ]
  },
  {
    "text": "おり",
    "detail": [
      "動詞",
      "非自立",
      "*",
      "*",
      "五段・ラ行",
      "連用形",
      "おる",
      "オリ",
      "オリ"
    ]
  },
  {
    "text": "ます",
    "detail": [
      "助動詞",
      "*",
      "*",
      "*",
      "特殊・マス",
      "基本形",
      "ます",
      "マス",
      "マス"
    ]
  },
  {
    "text": "。",
    "detail": [
      "記号",
      "句点",
      "*",
      "*",
      "*",
      "*",
      "。",
      "。",
      "。"
    ]
  }
]
```

## Docker

### Build Docker container image

You can build the Docker container image like so:

```shell script
$ make docker-build
```

### Pull Docker container image from docker.io

You can also use the Docker container image already registered in docker.io like so:

```shell script
$ docker pull linderamorphology/lindera-cli:latest
```

See https://hub.docker.com/r/linderamorphology/lindera-cli/tags/

### Start on Docker

Running a Lindera CLI on Docker like so:

```shell script
$ echo "羽田空港限定トートバッグ" | docker run --rm -i --name lindera linderamorphology/lindera-cli:latest
```

You can specify IPADIC dictionary in docker container as follows:

```shell script
$ echo "羽田空港限定トートバッグ" | docker run --rm -i --name lindera linderamorphology/lindera-cli:latest -d /usr/local/lib/lindera/dic/lindera-ipadic-2.7.0-20070801
```

```text
羽田空港        名詞,固有名詞,一般,*,*,*,羽田空港,ハネダクウコウ,ハネダクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
EOS
```

And you can specify UNIDIC dictionary in docker container as follows:

```shell script
$ echo "羽田空港限定トートバッグ" | docker run --rm -i --name lindera linderamorphology/lindera-cli:latest -d /usr/local/lib/lindera/dic/lindera-unidic-2.1.2
```

```text
羽田    名詞,固有名詞,人名,姓,*,*,ハタ,ハタ,羽田,ハタ,羽田,ハタ,固,*,*,*,*
空港    名詞,普通名詞,一般,*,*,*,クウコウ,空港,空港,クーコー,空港,クーコー,漢,*,*,*,*
限定    名詞,普通名詞,サ変可能,*,*,*,ゲンテイ,限定,限定,ゲンテー,限定,ゲンテー,漢,*,*,*,*
トート  名詞,普通名詞,一般,*,*,*,トート,トート,トート,トート,トート,トート,外,*,*,*,*
バッグ  名詞,普通名詞,一般,*,*,*,バッグ,バッグ-bag,バッグ,バッグ,バッグ,バッグ,外,*,*,*,*
EOS
```

In addition, when executing morphological analysis of Korean (specifying ko-dic dictionary), it is run as follows:

```shell script
$ echo "하네다공항한정토트백" | docker run --rm -i --name lindera linderamorphology/lindera-cli:latest -d /usr/local/lib/lindera/dic/lindera-ko-dic-2.1.1-20180720
```

```text
하네다  NNP,인명,F,하네다,*,*,*,*
공항    NNG,장소,T,공항,*,*,*,*
한정    NNG,*,T,한정,*,*,*,*
토트백  NNG,*,T,토트백,Compound,*,*,토트/NNP/인명+백/NNG/*
EOS
```
