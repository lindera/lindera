# Lindera CLI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A morphological analysis command-line interface for [Lindera](https://github.com/lindera-morphology/lindera).


## Install

You can install binary via cargo as follows:

```shell script
% cargo install lindera-cli
```

Alternatively, you can download a binary from the following release page:

- https://github.com/lindera-morphology/lindera/releases


## Build

The following products are required to build:

- Rust >= 1.46.0

```shell script
% cargo build --release
```

### Build with IPADIC (Japanese dictionary)

The "ipadic" feature flag allows Lindera to include IPADIC. 

```shell script
% cargo build --release --features=ipadic
```

### Build with UniDic (Japanese dictionary)

The "unidic" feature flag allows Lindera to include UniDic. 

```shell script
% cargo build --release --features=unidic
```

### Build with ko-dic (Korean dictionary)

The "ko-dic" feature flag allows Lindera to include ko-dic. 

```shell script
% cargo build --release --features=ko-dic
```

### Build with CC-CEDICT (Chinese dictionary)

The "cc-cedict" feature flag allows Lindera to include CC-CEDICT. 

```shell script
% cargo build --release --features=cc-cedict
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


## Build dictionary

### IPADIC (Japanese dictionary)

```shell script
% curl -L -o /tmp/mecab-ipadic-2.7.0-20070801.tar.gz "http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/mecab-ipadic-2.7.0-20070801.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20070801.tar.gz -C /tmp
% lindera build --dic-type=ipadic /tmp/mecab-ipadic-2.7.0-20070801 /tmp/lindera-ipadic-2.7.0-20070801
```

### CC-CEDICT (Chinese dictionary)

```shell script
% curl -L -o /tmp/CC-CEDICT-MeCab.zip https://github.com/ueda-keisuke/CC-CEDICT-MeCab/archive/refs/heads/master.zip
% unzip /tmp/CC-CEDICT-MeCab.zip -d /tmp
% lindera build --dic-type=cc-cedict /tmp/CC-CEDICT-MeCab-master /tmp/lindera-cc-cedict
```

### ko-dic (Korean dictionary)

```shell script
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera build --dic-type=ko-dic /tmp/mecab-ko-dic-2.1.1-20180720 /tmp/lindera-ko-dic-2.1.1-20180720
```

### UniDic (Japanese dictionary)

```shell script
% curl -l -o /tmp/unidic-mecab-2.1.2_src.zip "https://clrd.ninjal.ac.jp/unidic_archive/cwj/2.1.2/unidic-mecab-2.1.2_src.zip"
% unzip /tmp/unidic-mecab-2.1.2_src.zip -d /tmp
% lindera build --dic-type=unidic /tmp/unidic-mecab-2.1.2_src /tmp/lindera-unidic-2.1.2
```


## Build user dictionary

### IPADIC (Japanese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera IPADIC Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ipadic-builder#user-dictionary-format-csv)

```
% lindera build --build-user-dic --dic-type=ipadic ./resources/ipadic_simple_userdic.csv ./resources
```

### CC-CEDICT (Chinese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera CC-CEDICT Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-cc-cedict-builder#user-dictionary-format-csv)
```
% lindera build --build-user-dic --dic-type=cc-cedict ./resources/cc-cedict_simple_userdic.csv ./resources
```

### ko-dic (Korean dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera ko-dic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ko-dic-builder#user-dictionary-format-csv)

For more details about user dictionary format please refer to the following URL:

- [Lindera ko-dic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ko-dic-builder#user-dictionary-format-csv)

```
% lindera build --build-user-dic --dic-type=ko-dic ./resources/ko-dic_simple_userdic.csv ./resources
```

### UniDic (Japanese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera UniDic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-unidic-builder#user-dictionary-format-csv)

```
% lindera build --build-user-dic --dic-type=unidic ./resources/unidic_simple_userdic.csv ./resources
```


## Tokenization

### External dictionary

For example, text can be tokenized using a prepared dictionary as follows:

#### IPADIC (Japanese dictionary)

```shell script
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dic-type=unidic --dic-dir=/tmp/lindera-ipadic-2.7.0-20070801
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### UniDic (Japanese dictionary)

```shell script
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dic-type=ipadic --dic-dir=/tmp/lindera-unidic-2.1.2
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```


### Self-contained dictionary

If you had a built-in IPADIC, it is also possible to switch to the self-contained dictionary and tokenize.

#### Use default dictionary

The CLI already includes IPADIC as the default Japanese dictionary.  
You can easily tokenize the text and see the results as follows:

```shell script
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize
```

```text
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### IPADIC (Japanese dictionary)

The following example uses the self-contained IPADIC to tokenize:

```shell script
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dic-type=ipadic
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### UniDic (Japanese dictionary)

If UniDic were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell script
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize --dic-type=unidic
```

```text
日本    名詞,固有名詞,地名,国,*,*,ニッポン,日本,日本,ニッポン,日本,ニッポン,固,*,*,*,*
語      名詞,普通名詞,一般,*,*,*,ゴ,語,語,ゴ,語,ゴ,漢,*,*,*,*
の      助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*
形態    名詞,普通名詞,一般,*,*,*,ケイタイ,形態,形態,ケータイ,形態,ケータイ,漢,*,*,*,*
素      接尾辞,名詞的,一般,*,*,*,ソ,素,素,ソ,素,ソ,漢,*,*,*,*
解析    名詞,普通名詞,サ変可能,*,*,*,カイセキ,解析,解析,カイセキ,解析,カイセキ,漢,*,*,*,*
を      助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*
行う    動詞,一般,*,*,五段-ワア行,連体形-一般,オコナウ,行う,行う,オコナウ,行う,オコナウ,和,*,*,*,*
こと    名詞,普通名詞,一般,*,*,*,コト,事,こと,コト,こと,コト,和,コ濁,基本形,*,*
が      助詞,格助詞,*,*,*,*,ガ,が,が,ガ,が,ガ,和,*,*,*,*
でき    動詞,非自立可能,*,*,上一段-カ行,連用形-一般,デキル,出来る,でき,デキ,できる,デキル,和,*,*,*,*
ます    助動詞,*,*,*,助動詞-マス,終止形-一般,マス,ます,ます,マス,ます,マス,和,*,*,*,*
。      補助記号,句点,*,*,*,*,,。,。,,。,,記号,*,*,*,*
EOS
```

#### ko-dic (Korean dictionary)

If ko-dic were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell script
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize --dic-type=ko-dic
```

```text
한국어  NNG,*,F,한국어,Compound,*,*,한국/NNG/*+어/NNG/*
의      JKG,*,F,의,*,*,*,*
형태    NNG,*,F,형태,*,*,*,*
해석    NNG,행위,T,해석,*,*,*,*
을      JKO,*,T,을,*,*,*,*
실시    NNG,행위,F,실시,*,*,*,*
할      VV+ETM,*,T,할,Inflect,VV,ETM,하/VV/*+ᆯ/ETM/*
수      NNG,*,F,수,*,*,*,*
있      VX,*,T,있,*,*,*,*
습니다  EF,*,F,습니다,*,*,*,*
.       UNK
EOS
```

#### CC-CEDICT (Chinese dictionary)

If CC-CEDICT were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell script
% echo "可以进行中文形态学分析。" | lindera tokenize --dic-type=cc-cedict
```

```text
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
中文    *,*,*,*,Zhong1 wen2,中文,中文,Chinese language/
形态学  *,*,*,*,xing2 tai4 xue2,形態學,形态学,morphology (in biology or linguistics)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
。      UNK
EOS
```


### User dictionary

Lindera supports two types of user dictionaries, one in CSV format and the other in binary format.

#### CSV format

This will parse the given CSV file at runtime, build a dictionary, and then run the text tokenization.

```shell script
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize --dic-type=ipadic --user-dictionary=./resources/simple_userdic.csv
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
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize --dic-type=ipadic --user-dictionary=./resources/ipadic_userdic.bin
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
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dic-type=ipadic --mode=normal
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

`decopose` mode tokenizes a compound noun words additionally:

```shell script
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dic-type=ipadic --mode=decompose
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
% echo "お待ちしております。" | lindera tokenize --dic-type=ipadic --output-format=mecab
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
% echo "お待ちしております。" | lindera tokenize --dic-type=ipadic --output-format=wakati
```

```text
お待ち し て おり ます 。
```

`json` outputs the token information in JSON format:

```shell script
% echo "お待ちしております。" | lindera tokenize --dic-type=ipadic --output-format=json
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
