# Lindera ko-dic Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

ko-dic dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Install

```shell script
% cargo install lindera-ko-dic-builder
```


## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```

### Build small binary

You can reduce the size of the dictionary by using the "compress" feature flag.  
Instead, it can only be used with Lindera, which supports compression.

This repo example is this.

```sh
% cargo build --release --features compress
```

It also depends on liblzma to compress the dictionary. Please install the dependent packages as follows:

```text
% sudo apt install liblzma-dev
```


## Dictionary version

This repository contains [mecab-ko-dic-2.1.1-20180720](https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/).


## Building a dictionary

Building a dictionary with `lindera-ko-dic` command:

```shell script
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera-ko-dic-builder -s /tmp/mecab-ko-dic-2.1.1-20180720 -d /tmp/lindera-ko-dic-2.1.1-20180720
```


## Dictionary format

Information about the dictionary format and part-of-speech tags used by mecab-ko-dic id documented in [this Google Spreadsheet](https://docs.google.com/spreadsheets/d/1-9blXKjtjeKZqsf4NzHeYJCrr49-nXeRF6D80udfcwY/edit#gid=589544265), linked to from mecab-ko-dic's [repository readme](https://bitbucket.org/eunjeon/mecab-ko-dic/src/master/README.md).

Note how ko-dic has one less feature column than NAIST JDIC, and has an altogether different set of information (e.g. doesn't provide the "original form" of the word).

The tags are a slight modification of those specified by 세종 (Sejong), whatever that is. The mappings from Sejong to mecab-ko-dic's tag names are given in tab `태그 v2.0` on the above-linked spreadsheet.

The dictionary format is specified fully (in Korean) in tab `사전 형식 v2.0` of the spreadsheet. Any blank values default to `*`.

| Index | Name (Korean) | Name (English) | Notes |
| --- | --- | --- | --- |
| 0 | 품사 태그 | part-of-speech tag | See `태그 v2.0` tab on spreadsheet  |
| 1 | 의미 부류 | meaning | (too few examples for me to be sure) |
| 2 | 종성 유무 | presence or absence | `T` for true; `F` for false; else `*` |
| 3 | 읽기 | reading | usually matches surface, but may differ for foreign words e.g. Chinese character words |
| 4 | 타입 | type | One of: `Inflect` (활용); `Compound` (복합명사); or `Preanalysis` (기분석) |
| 5 | 첫번째 품사 | first part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `VV` |
| 6 | 마지막 품사 | last part-of-speech | e.g. given a part-of-speech tag of "VV+EM+VX+EP", would return `EP` |
| 7 | 표현 | expression | `활용, 복합명사, 기분석이 어떻게 구성되는지 알려주는 필드` – Fields that tell how usage, compound nouns, and key analysis are organized |


## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "하네다공항한정토트백" | lindera -d /tmp/lindera-ko-dic-2.1.1-20180720
```

```text
하네다  NNP,인명,F,하네다,*,*,*,*
공항    NNG,장소,T,공항,*,*,*,*
한정    NNG,*,T,한정,*,*,*,*
토트백  NNG,*,T,토트백,Compound,*,*,토트/NNP/인명+백/NNG/*
EOS
```

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-ko-dic-builder" target="_blank">lindera-ko-dic-builder</a>
