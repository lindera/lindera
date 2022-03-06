# Lindera CLI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A morphological analysis command-line interface for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

## Install

You can install binary via cargo as follows:

```shell script
% cargo install lindera-cli
```

Alternatively, you can download a binary from the following release page:

- https://github.com/lindera-morphology/lindera-cli/releases

## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```

### Build with IPADIC

The "ipadic" feature flag allows Lindera to include IPADIC. 

```shell script
% cargo build --release --features=ipadic
```

### Build with UniDic

The "unidic" feature flag allows Lindera to include UniDic. 

```shell script
% cargo build --release --features=unidic
```

### Build small binary

You can reduce the size of the binary containing the lindera by using the "compress" feature flag.  
Instead, you will be penalized for the execution time of the program.

```shell script
% cargo build --release --features=compress
```

It also depends on liblzma to compress the dictionary. Please install the dependent packages as follows:

```shell script
% sudo apt install liblzma-dev
```


## Usage

### Prepare the dictionary

If you have not built binaries with built-in dictionaries, you will need to prepare the dictionaries.
For more information on preparing dictionaries, see the following link:

- <a href="https://github.com/lindera-morphology/lindera/tree/main/lindera-ipadic-builder" target="_blank">Lindera IPADIC Builder</a>
- <a href="https://github.com/lindera-morphology/lindera/tree/main/lindera-unidic-builder" target="_blank">Lindera UniDic Builder</a>
- <a href="https://github.com/lindera-morphology/lindera-ko-dic-builder" target="_blank">Lindera ko-dic Builder</a>
- <a href="https://github.com/lindera-morphology/lindera-cc-cedict-builder" target="_blank">Lindera cc-cedict Builder</a>


#### External dictionary

For example, text can be tokenized using a prepared dictionary as follows:

```shell script
% echo "羽田空港限定トートバッグ" | lindera -t local -d /tmp/lindera-ipadic-2.7.0-20070801
```

```text
羽田空港        名詞,固有名詞,一般,*,*,*,羽田空港,ハネダクウコウ,ハネダクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

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

### Self-contained dictionary

If you had a built-in IPADIC, it is also possible to switch to the self-contained dictionary and tokenize.

#### IPADIC

The following example uses the self-contained IPADIC to tokenize:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -t ipadic
```

```text
関西国際空港	名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定	名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ	UNK
EOS
```

#### UniDic

If UniDic were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -t unidic
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

### User dictionary

Lindera supports two types of user dictionaries, one in CSV format and the other in binary format.

#### CSV format

This will parse the given CSV file at runtime, build a dictionary, and then run the text tokenization.

```shell script
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera -t ipadic -D ./resources/userdic.csv
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```

#### Binary format

This will read the given pre-built user dictionary file and perform text tokenization.
Please check the repository of each dictionary builder for the configuration of the user dictionary binary files.

```shell script
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera -t ipadic -D ./resources/userdic.bin -t bin
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```

### Tokenize mode

Lindera provides two tokenization modes: `normal` and `decompose`.

`normal` mode tokenizes faithfully based on words registered in the dictionary. (Default):

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -t ipadic -m normal
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

`decopose` mode tokenizes a compound noun words additionally:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera -t ipadic -m decompose
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
% echo "お待ちしております。" | lindera -t ipadic -O mecab
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
% echo "お待ちしております。" | lindera -t ipadic -O wakati
```

```text
お待ち し て おり ます 。
```

`json` outputs the token information in JSON format:

```shell script
% echo "お待ちしております。" | lindera -t ipadic -O json
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
$ docker pull linderamorphology/lindera:latest
```

See https://hub.docker.com/r/linderamorphology/lindera-cli/tags

### Start on Docker

Running a Lindera CLI on Docker like so:

```shell script
$ echo "羽田空港限定トートバッグ" | docker run --rm -i --name lindera linderamorphology/lindera:latest
```

```text
羽田空港        名詞,固有名詞,一般,*,*,*,羽田空港,ハネダクウコウ,ハネダクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
EOS
```


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-cli" target="_blank">lindera-cli</a>
