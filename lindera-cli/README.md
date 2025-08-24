# Lindera CLI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-cli.svg)](https://crates.io/crates/lindera-cli)

A morphological analysis command-line interface for [Lindera](https://github.com/lindera-morphology/lindera).

## Install

You can install binary via cargo as follows:

```shell script
% cargo install lindera-cli
```

Alternatively, you can download a binary from the following release page:

- [https://github.com/lindera-morphology/lindera/releases](https://github.com/lindera-morphology/lindera/releases)

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

## Build dictionary

### IPADIC (Japanese dictionary)

```shell script
% curl -L -o /tmp/mecab-ipadic-2.7.0-20070801.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20070801.tar.gz -C /tmp
% lindera build ./resources/ipadic_metadata.json /tmp/mecab-ipadic-2.7.0-20070801 /tmp/lindera-ipadic-2.7.0-20070801
% ls -al /tmp/lindera-ipadic-2.7.0-20070801
% (cd /tmp && zip -r lindera-ipadic-2.7.0-20070801.zip lindera-ipadic-2.7.0-20070801/)
% tar -czf /tmp/lindera-ipadic-2.7.0-20070801.tar.gz -C /tmp lindera-ipadic-2.7.0-20070801
```

### CC-CEDICT (Chinese dictionary)

```shell script
% curl -L -o /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz "https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
% tar zxvf /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz -C /tmp
% lindera build ./resources/cc-cedict_metadata.json /tmp/CC-CEDICT-MeCab-0.1.0-20200409 /tmp/lindera-cc-cedict-0.1.0-20200409
% ls -al /tmp/lindera-cc-cedict-0.1.0-20200409
% (cd /tmp && zip -r lindera-cc-cedict-0.1.0-20200409.zip lindera-cc-cedict-0.1.0-20200409/)
% tar -czf /tmp/lindera-cc-cedict-0.1.0-20200409.tar.gz -C /tmp lindera-cc-cedict-0.1.0-20200409
```

### ko-dic (Korean dictionary)

```shell script
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera build ./resources/ko-dic_metadata.json /tmp/mecab-ko-dic-2.1.1-20180720 /tmp/lindera-ko-dic-2.1.1-20180720
% ls -al /tmp/lindera-ko-dic-2.1.1-20180720
% (cd /tmp && zip -r lindera-ko-dic-2.1.1-20180720.zip lindera-ko-dic-2.1.1-20180720/)
% tar -czf /tmp/lindera-ko-dic-2.1.1-20180720.tar.gz -C /tmp lindera-ko-dic-2.1.1-20180720
```

### UniDic (Japanese dictionary)

```shell script
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxvf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp
% lindera build ./resources/unidic_metadata.json /tmp/unidic-mecab-2.1.2 /tmp/lindera-unidic-2.1.2
% ls -al /tmp/lindera-unidic-2.1.2
% (cd /tmp && zip -r lindera-unidic-2.1.2.zip lindera-unidic-2.1.2/)
% tar -czf /tmp/lindera-unidic-2.1.2.tar.gz -C /tmp lindera-unidic-2.1.2
```

### IPADIC NEologd (Japanese dictionary)

```shell script
% curl -L -o /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"
% tar zxvf /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp
% lindera build ./resources/ipadic-neologd_metadata.json /tmp/mecab-ipadic-neologd-0.0.7-20200820 /tmp/lindera-ipadic-neologd-0.0.7-20200820
% ls -al /tmp/lindera-ipadic-neologd-0.0.7-20200820
% (cd /tmp && zip -r lindera-ipadic-neologd-0.0.7-20200820.zip lindera-ipadic-neologd-0.0.7-20200820/)
% tar -czf /tmp/lindera-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp lindera-ipadic-neologd-0.0.7-20200820
```

## Build user dictionary

### Build IPADIC (Japanese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera IPADIC Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ipadic-builder#user-dictionary-format-csv)

```shell
% lindera build --build-user-dictionary ./resources/ipadic_metadata.json ./resources/ipadic_simple_userdic.csv ./resources
```

### Build CC-CEDICT (Chinese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera CC-CEDICT Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-cc-cedict-builder#user-dictionary-format-csv)

```shell
% lindera build --build-user-dictionary ./resources/cc-cedict_metadata.json ./resources/cc-cedict_simple_userdic.csv ./resources
```

### Build ko-dic (Korean dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera ko-dic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ko-dic-builder#user-dictionary-format-csv)

```shell
% lindera build --build-user-dictionary ./resources/ko-dic_metadata.json ./resources/ko-dic_simple_userdic.csv ./resources
```

### Build UniDic (Japanese dictionary)

For more details about user dictionary format please refer to the following URL:

- [Lindera UniDic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-unidic-builder#user-dictionary-format-csv)

```shell
% lindera build --build-user-dictionary ./resources/unidic_metadata.json ./resources/unidic_simple_userdic.csv ./resources
```

## Tokenization

### External dictionary

For example, text can be tokenized using a prepared dictionary as follows:

#### Tokenize with IPADIC (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize /tmp/lindera-ipadic-2.7.0-20070801
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

#### Tokenize with UniDic (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize /tmp/lindera-unidic-2.1.2
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

#### Tokenize with IPADIC Neologd (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize /tmp/lindera-ipadic-neologd-0.0.7-20200820
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### Tokenize ko-dic (Korean dictionary)

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize /tmp/lindera-ko-dic-2.1.1-20180720
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

#### Tokenize with CC-CEDICT (Chinese dictionary)

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize /tmp/lindera-cc-cedict-0.1.0-20200409
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

### Embedded dictionary

If you had a built-in IPADIC, it is also possible to switch to the self-contained dictionary and tokenize.

#### Tokenize with embedded IPADIC (Japanese dictionary)

The following example uses the self-contained IPADIC to tokenize:

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize embedded://ipadic
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

NOTE: To include IPADIC dictionary in the binary, you must build with the `--features=embedded-ipadic` option.

#### Tokenize with embedded UniDic (Japanese dictionary)

If UniDic were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize embedded://unidic
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

NOTE: To include UniDic dictionary in the binary, you must build with the `--features=embedded-unidic` option.

#### Tokenize with self-contained IPADIC NEologd (Japanese dictionary)

If IPADIC NEologd were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize embedded://ipadic-neologd
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

NOTE: To include UniDic dictionary in the binary, you must build with the `--features=embedded-ipadic-neologd` option.

#### Tokenize with self-contained ko-dic (Korean dictionary)

If ko-dic were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize embedded://ko-dic
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

NOTE: To include ko-dic dictionary in the binary, you must build with the `--features=embedded-ko-dic` option.

#### Tokenize with self-contained CC-CEDICT (Chinese dictionary)

If CC-CEDICT were built in, it could also be tokenized by switching to a self-contained dictionary in the same way:

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize embedded://cc-cedict
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

NOTE: To include CC-CEDICT dictionary in the binary, you must build with the `--features=embedded-cc-cedict` option.

### User dictionary

Lindera supports two types of user dictionaries, one in CSV format and the other in binary format.

#### Use user dictionary (CSV format)

This will parse the given CSV file at runtime, build a dictionary, and then run the text tokenization.

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize --user-dictionary-uri=./resources/ipadic_simple_userdic.csv embedded://ipadic
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

#### Use user dictionary (Binary format)

This will read the given pre-built user dictionary file and perform text tokenization.
Please check the repository of each dictionary builder for the configuration of the user dictionary binary files.

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize --user-dictionary-uri=./resources/ipadic_simple_userdic.bin /tmp/lindera-ipadic-2.7.0-20070801/
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

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --mode=normal embedded://ipadic
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

`decompose` mode tokenizes a compound noun words additionally:

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --mode=decompose embedded://ipadic
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

```shell
% echo "お待ちしております。" | lindera tokenize --output-format=mecab embedded://ipadic
```

```text
お待ち  名詞,サ変接続,*,*,*,*,お待ち,オマチ,オマチ
し  動詞,自立,*,*,サ変・スル,連用形,する,シ,シ
て  助詞,接続助詞,*,*,*,*,て,テ,テ
おり  動詞,非自立,*,*,五段・ラ行,連用形,おる,オリ,オリ
ます  助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。  記号,句点,*,*,*,*,。,。,。
EOS
```

`wakati` outputs the token text separated by spaces:

```shell
% echo "お待ちしております。" | lindera tokenize --output-format=wakati embedded://ipadic
```

```text
お待ち し て おり ます 。
```

`json` outputs the token information in JSON format:

```shell
% echo "お待ちしております。" | lindera tokenize --output-format=json embedded://ipadic
```

```json
[
  {
    "conjugation_type": "*",
    "word_id": "14695",
    "part_of_speech_subcategory_2": "*",
    "reading": "オマチ",
    "part_of_speech_subcategory_1": "サ変接続",
    "byte_end": "9",
    "base_form": "お待ち",
    "pronunciation": "オマチ",
    "surface": "お待ち",
    "byte_start": "0",
    "part_of_speech_subcategory_3": "*",
    "conjugation_form": "*",
    "part_of_speech": "名詞"
  },
  {
    "byte_start": "9",
    "conjugation_form": "サ変・スル",
    "conjugation_type": "連用形",
    "base_form": "する",
    "surface": "し",
    "part_of_speech_subcategory_3": "*",
    "byte_end": "12",
    "word_id": "30760",
    "part_of_speech_subcategory_1": "自立",
    "part_of_speech_subcategory_2": "*",
    "pronunciation": "シ",
    "reading": "シ",
    "part_of_speech": "動詞"
  },
  {
    "base_form": "て",
    "surface": "て",
    "pronunciation": "テ",
    "part_of_speech_subcategory_1": "接続助詞",
    "conjugation_type": "*",
    "part_of_speech_subcategory_3": "*",
    "reading": "テ",
    "part_of_speech": "助詞",
    "part_of_speech_subcategory_2": "*",
    "byte_start": "12",
    "byte_end": "15",
    "word_id": "46600",
    "conjugation_form": "*"
  },
  {
    "word_id": "14236",
    "part_of_speech_subcategory_1": "非自立",
    "conjugation_type": "連用形",
    "byte_start": "15",
    "part_of_speech_subcategory_3": "*",
    "part_of_speech": "動詞",
    "surface": "おり",
    "byte_end": "21",
    "base_form": "おる",
    "part_of_speech_subcategory_2": "*",
    "pronunciation": "オリ",
    "reading": "オリ",
    "conjugation_form": "五段・ラ行"
  },
  {
    "pronunciation": "マス",
    "part_of_speech": "助動詞",
    "base_form": "ます",
    "word_id": "68730",
    "part_of_speech_subcategory_1": "*",
    "byte_start": "21",
    "reading": "マス",
    "conjugation_type": "基本形",
    "byte_end": "27",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "conjugation_form": "特殊・マス",
    "surface": "ます"
  },
  {
    "byte_end": "30",
    "byte_start": "27",
    "part_of_speech_subcategory_3": "*",
    "word_id": "98",
    "conjugation_form": "*",
    "conjugation_type": "*",
    "base_form": "。",
    "part_of_speech": "記号",
    "part_of_speech_subcategory_2": "*",
    "reading": "。",
    "pronunciation": "。",
    "surface": "。",
    "part_of_speech_subcategory_1": "句点"
  }
]
```

## Filtering

Lindera introduced an analytical framework.
Combine character filters, tokenizers, and token filters for more advanced text processing.
Describe the character filter and token filter settings used for analysis in JSON.

```shell
% echo "すもももももももものうち" | lindera tokenize --character-filter='unicode_normalize:{"kind":"nfkc"}' --token-filter='japanese_keep_tags:{"tags":["名詞,一般"]}' embedded://ipadic
```

```text
すもも  名詞,一般,*,*,*,*,すもも,スモモ,スモモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
EOS
```

## API reference

The API reference is available. Please see following URL:

- [lindera-cli](https://docs.rs/lindera-cli)
