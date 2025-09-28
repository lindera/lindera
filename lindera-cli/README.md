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

### Build with IPADIC (Japanese dictionary)

The "ipadic" feature flag allows Lindera to include IPADIC.

```shell script
% cargo build --release --features=embedded-ipadic
```

### Build with UniDic (Japanese dictionary)

The "unidic" feature flag allows Lindera to include UniDic.

```shell script
% cargo build --release --features=embedded-unidic
```

### Build with ko-dic (Korean dictionary)

The "ko-dic" feature flag allows Lindera to include ko-dic.

```shell script
% cargo build --release --features=embedded-ko-dic
```

### Build with CC-CEDICT (Chinese dictionary)

The "cc-cedict" feature flag allows Lindera to include CC-CEDICT.

```shell script
% cargo build --release --features=embedded-cc-cedict
```

### Build without dictionaries

To reduce Lindera's binary size, omit the feature flag.
This results in a binary containing only the tokenizer and trainer, as it no longer includes the dictionary.

```shell script
% cargo build --release
```

### Build with all features

```shell script
% cargo build --release --all-features
```

## Build dictionary

Build (compile) a morphological analysis dictionary from source CSV files for use with Lindera.

### Basic build usage

```bash
# Build a system dictionary
lindera build \
  --src /path/to/dictionary/csvs \
  --dest /path/to/output/dictionary \
  --metadata ./lindera-ipadic/metadata.json

# Build a user dictionary
lindera build \
  --src ./user_dict.csv \
  --dest ./user_dictionary \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

### Build parameters

- `--src` / `-s`: Source directory containing dictionary CSV files (or single CSV file for user dictionary)
- `--dest` / `-d`: Destination directory for compiled dictionary output
- `--metadata` / `-m`: Metadata configuration file (metadata.json) that defines dictionary structure
- `--user` / `-u`: Build user dictionary instead of system dictionary (optional flag)

### Dictionary types

#### System dictionary

A full morphological analysis dictionary containing:

- Lexicon entries (word definitions)
- Connection cost matrix
- Unknown word handling rules
- Character type definitions

#### User dictionary

A supplementary dictionary for custom words that works alongside a system dictionary.

### Examples

#### Build IPADIC (Japanese dictionary)

```shell script
# Download and extract IPADIC source files
% curl -L -o /tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/mecab-ipadic-2.7.0-20250920 \
  --dest /tmp/lindera-ipadic-2.7.0-20250920 \
  --metadata ./lindera-ipadic/metadata.json

% ls -al /tmp/lindera-ipadic-2.7.0-20250920
% (cd /tmp && zip -r lindera-ipadic-2.7.0-20250920.zip lindera-ipadic-2.7.0-20250920/)
% tar -czf /tmp/lindera-ipadic-2.7.0-20250920.tar.gz -C /tmp lindera-ipadic-2.7.0-20250920
```

### Build IPADIC NEologd (Japanese dictionary)

```shell script
# Download and extract IPADIC NEologd source files
% curl -L -o /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"
% tar zxvf /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/mecab-ipadic-neologd-0.0.7-20200820 \
  --dest /tmp/lindera-ipadic-neologd-0.0.7-20200820 \
  --metadata ./lindera-ipadic-neologd/metadata.json

% ls -al /tmp/lindera-ipadic-neologd-0.0.7-20200820
% (cd /tmp && zip -r lindera-ipadic-neologd-0.0.7-20200820.zip lindera-ipadic-neologd-0.0.7-20200820/)
% tar -czf /tmp/lindera-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp lindera-ipadic-neologd-0.0.7-20200820
```

### Build UniDic (Japanese dictionary)

```shell script
# Download and extract UniDic source files
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxvf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/unidic-mecab-2.1.2 \
  --dest /tmp/lindera-unidic-2.1.2 \
  --metadata ./lindera-unidic/metadata.json

% ls -al /tmp/lindera-unidic-2.1.2
% (cd /tmp && zip -r lindera-unidic-2.1.2.zip lindera-unidic-2.1.2/)
% tar -czf /tmp/lindera-unidic-2.1.2.tar.gz -C /tmp lindera-unidic-2.1.2
```

### Build CC-CEDICT (Chinese dictionary)

```shell script
# Download and extract CC-CEDICT source files
% curl -L -o /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz "https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
% tar zxvf /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/CC-CEDICT-MeCab-0.1.0-20200409 \
  --dest /tmp/lindera-cc-cedict-0.1.0-20200409 \
  --metadata ./lindera-cc-cedict/metadata.json

% ls -al /tmp/lindera-cc-cedict-0.1.0-20200409
% (cd /tmp && zip -r lindera-cc-cedict-0.1.0-20200409.zip lindera-cc-cedict-0.1.0-20200409/)
% tar -czf /tmp/lindera-cc-cedict-0.1.0-20200409.tar.gz -C /tmp lindera-cc-cedict-0.1.0-20200409
```

### Build ko-dic (Korean dictionary)

```shell script
# Download and extract ko-dic source files
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/mecab-ko-dic-2.1.1-20180720 \
  --dest /tmp/lindera-ko-dic-2.1.1-20180720 \
  --metadata ./lindera-ko-dic/metadata.json

% ls -al /tmp/lindera-ko-dic-2.1.1-20180720
% (cd /tmp && zip -r lindera-ko-dic-2.1.1-20180720.zip lindera-ko-dic-2.1.1-20180720/)
% tar -czf /tmp/lindera-ko-dic-2.1.1-20180720.tar.gz -C /tmp lindera-ko-dic-2.1.1-20180720
```

## Build user dictionary

### Build IPADIC user dictionary (Japanese)

For more details about user dictionary format please refer to the following URL:

- [Lindera IPADIC Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ipadic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/ipadic_simple_userdic.csv \
  --dest ./resources \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

### Build UniDic user dictionary (Japanese)

For more details about user dictionary format please refer to the following URL:

- [Lindera UniDic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-unidic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/unidic_simple_userdic.csv \
  --dest ./resources \
  --metadata ./lindera-unidic/metadata.json \
  --user
```

### Build CC-CEDICT user dictionary (Chinese)

For more details about user dictionary format please refer to the following URL:

- [Lindera CC-CEDICT Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-cc-cedict-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/cc-cedict_simple_userdic.csv \
  --dest ./resources \
  --metadata ./lindera-cc-cedict/metadata.json \
  --user
```

### Build ko-dic user dictionary (Korean)

For more details about user dictionary format please refer to the following URL:

- [Lindera ko-dic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ko-dic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/ko-dic_simple_userdic.csv \
  --dest ./resources \
  --metadata ./lindera-ko-dic/metadata.json \
  --user
```

## Tokenize text

Perform morphological analysis (tokenization) on Japanese, Chinese, or Korean text using various dictionaries.

### Basic tokenization usage

```bash
# Tokenize text using a dictionary directory
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /path/to/dictionary

# Tokenize text using embedded dictionary
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic

# Tokenize with different output format
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic \
  --output json

# Tokenize text from file
lindera tokenize \
  --dict /path/to/dictionary \
  --output wakati \
  input.txt
```

### Tokenization parameters

- `--dict` / `-d`: Dictionary path or URI (required)
  - File path: `/path/to/dictionary`
  - Embedded: `embedded://ipadic`, `embedded://unidic`, etc.
- `--output` / `-o`: Output format (default: mecab)
  - `mecab`: MeCab-compatible format with part-of-speech info
  - `wakati`: Space-separated tokens only
  - `json`: Detailed JSON format with all token information
- `--user-dict` / `-u`: User dictionary path (optional)
- `--mode` / `-m`: Tokenization mode (default: normal)
  - `normal`: Standard tokenization
  - `decompose`: Decompose compound words
- `--char-filter` / `-c`: Character filter configuration (JSON)
- `--token-filter` / `-t`: Token filter configuration (JSON)
- Input file: Optional file path (default: stdin)

### Examples with external dictionaries

#### Tokenize with external IPADIC (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-ipadic-2.7.0-20250920
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

#### Tokenize with external IPADIC Neologd (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-ipadic-neologd-0.0.7-20200820
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

#### Tokenize with external UniDic (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-unidic-2.1.2
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

#### Tokenize with external ko-dic (Korean dictionary)

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize \
  --dict /tmp/lindera-ko-dic-2.1.1-20180720
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

#### Tokenize with external CC-CEDICT (Chinese dictionary)

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-cc-cedict-0.1.0-20200409
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

### Examples with embedded dictionaries

Lindera can include dictionaries directly in the binary when built with specific feature flags. This allows tokenization without external dictionary files.

#### Tokenize with embedded IPADIC (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic
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

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://unidic
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

#### Tokenize with embedded IPADIC NEologd (Japanese dictionary)

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic-neologd
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

#### Tokenize with embedded ko-dic (Korean dictionary)

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize \
  --dict embedded://ko-dic
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

#### Tokenize with embedded CC-CEDICT (Chinese dictionary)

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://cc-cedict
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

### User dictionary examples

Lindera supports user dictionaries to add custom words alongside system dictionaries. User dictionaries can be in CSV or binary format.

#### Use user dictionary (CSV format)

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict embedded://ipadic \
  --user-dict ./resources/ipadic_simple_userdic.csv
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

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict /tmp/lindera-ipadic-2.7.0-20250920 \
  --user-dict ./resources/ipadic_simple_userdic.bin
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

### Tokenization modes

Lindera provides two tokenization modes: `normal` and `decompose`.

#### Normal mode (default)

Tokenizes faithfully based on words registered in the dictionary:

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize \
  --dict embedded://ipadic \
  --mode normal
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

#### Decompose mode

Tokenizes compound noun words additionally:

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize \
  --dict embedded://ipadic \
  --mode decompose
```

```text
関西    名詞,固有名詞,地域,一般,*,*,関西,カンサイ,カンサイ
国際    名詞,一般,*,*,*,*,国際,コクサイ,コクサイ
空港    名詞,一般,*,*,*,*,空港,クウコウ,クーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

### Output formats

Lindera provides three output formats: `mecab`, `wakati` and `json`.

#### MeCab format (default)

Outputs results in MeCab-compatible format with part-of-speech information:

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output mecab
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

#### Wakati format

Outputs only the token text separated by spaces:

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output wakati
```

```text
お待ち し て おり ます 。
```

#### JSON format

Outputs detailed token information in JSON format:

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output json
```

```json
[
  {
    "base_form": "お待ち",
    "byte_end": 9,
    "byte_start": 0,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "名詞",
    "part_of_speech_subcategory_1": "サ変接続",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "オマチ",
    "reading": "オマチ",
    "surface": "お待ち",
    "word_id": 14698
  },
  {
    "base_form": "する",
    "byte_end": 12,
    "byte_start": 9,
    "conjugation_form": "サ変・スル",
    "conjugation_type": "連用形",
    "part_of_speech": "動詞",
    "part_of_speech_subcategory_1": "自立",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "シ",
    "reading": "シ",
    "surface": "し",
    "word_id": 30763
  },
  {
    "base_form": "て",
    "byte_end": 15,
    "byte_start": 12,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "助詞",
    "part_of_speech_subcategory_1": "接続助詞",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "テ",
    "reading": "テ",
    "surface": "て",
    "word_id": 46603
  },
  {
    "base_form": "おる",
    "byte_end": 21,
    "byte_start": 15,
    "conjugation_form": "五段・ラ行",
    "conjugation_type": "連用形",
    "part_of_speech": "動詞",
    "part_of_speech_subcategory_1": "非自立",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "オリ",
    "reading": "オリ",
    "surface": "おり",
    "word_id": 14239
  },
  {
    "base_form": "ます",
    "byte_end": 27,
    "byte_start": 21,
    "conjugation_form": "特殊・マス",
    "conjugation_type": "基本形",
    "part_of_speech": "助動詞",
    "part_of_speech_subcategory_1": "*",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "マス",
    "reading": "マス",
    "surface": "ます",
    "word_id": 68733
  },
  {
    "base_form": "。",
    "byte_end": 30,
    "byte_start": 27,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "記号",
    "part_of_speech_subcategory_1": "句点",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "。",
    "reading": "。",
    "surface": "。",
    "word_id": 101
  }
]
```

## Advanced tokenization

Lindera provides an analytical framework that combines character filters, tokenizers, and token filters for advanced text processing. Filters are configured using JSON.

### Tokenize with character and token filters

```shell
% echo "すもももももももものうち" | lindera tokenize \
  --dict embedded://ipadic \
  --char-filter 'unicode_normalize:{"kind":"nfkc"}' \
  --token-filter 'japanese_keep_tags:{"tags":["名詞,一般"]}'
```

```text
すもも  名詞,一般,*,*,*,*,すもも,スモモ,スモモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
EOS
```

## Train dictionary

Train a new morphological analysis model from annotated corpus data. To use this feature, you must build with the `train` feature flag enabled. (The `train` feature flag is enabled by default.)

### Training parameters

- `--seed` / `-s`: Seed lexicon file (CSV format) to be weighted
- `--corpus` / `-c`: Training corpus (annotated text)
- `--char-def` / `-C`: Character definition file (char.def)
- `--unk-def` / `-u`: Unknown word definition file (unk.def) to be weighted
- `--feature-def` / `-f`: Feature definition file (feature.def)
- `--rewrite-def` / `-r`: Rewrite rule definition file (rewrite.def)
- `--output` / `-o`: Output model file
- `--lambda` / `-l`: L1 regularization (0.0-1.0) (default: 0.01)
- `--max-iterations` / `-i`: Maximum number of iterations for training (default: 100)
- `--max-threads` / `-t`: Maximum number of threads (defaults to CPU core count, auto-adjusted based on dataset size)

### Basic workflow

#### 1. Prepare training files

**Seed lexicon file (seed.csv):**

The seed lexicon file contains initial dictionary entries used for training the CRF model. Each line represents a word entry with comma-separated fields. The specific field structure varies depending on the dictionary format:

- Surface
- Left context ID
- Right context ID
- Word cost
- Part-of-speech tags (multiple fields)
- Base form
- Reading (katakana)
- Pronunciation

Note: The exact field definitions differ between dictionary formats (IPADIC, UniDic, ko-dic, CC-CEDICT). Please refer to each dictionary's format specification for details.

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
```

**Training corpus (corpus.txt):**

The training corpus file contains annotated text data used to train the CRF model. Each line consists of:

- A surface form (word) followed by a tab character
- Comma-separated morphological features (part-of-speech tags, base form, reading, pronunciation)
- Sentences are separated by "EOS" (End Of Sentence) markers

Note: The morphological feature format varies depending on the dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT). Please refer to each dictionary's format specification for details.

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS

形態	名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ
素	名詞,接尾,一般,*,*,*,素,ソ,ソ
解析	名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う	動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
EOS
```

For detailed information about file formats and advanced features, see [TRAINER_README.md](../TRAINER_README.md).

#### 2. Train model

```bash
lindera train \
  --seed ./resources/training/seed.csv \
  --corpus ./resources/training/corpus.txt \
  --unk-def ./resources/training/unk.def \
  --char-def ./resources/training/char.def \
  --feature-def ./resources/training/feature.def \
  --rewrite-def ./resources/training/rewrite.def \
  --output /tmp/lindera/training/model.dat \
  --lambda 0.01 \
  --max-iterations 100
```

#### 3. Training results

The trained model will contain:

- **Existing words**: All seed dictionary records with newly learned weights
- **New words**: Words from the corpus not in the seed dictionary, added with appropriate weights

## Export trained model to dictionary

Export a trained model file to Lindera dictionary format files. This feature requires building with the `train` feature flag enabled.

### Basic export usage

```bash
# Export trained model to dictionary files
lindera export \
  --model /tmp/lindera/training/model.dat \
  --metadata ./resources/training/metadata.json \
  --output /tmp/lindera/training/dictionary
```

### Export parameters

- `--model` / `-m`: Path to the trained model file (.dat format)
- `--output` / `-o`: Directory to output the dictionary files
- `--metadata`: Optional metadata.json file to update with trained model information

### Output files

The export command creates the following dictionary files in the output directory:

- `lex.csv`: Lexicon file with learned weights
- `matrix.def`: Connection cost matrix
- `unk.def`: Unknown word definitions
- `char.def`: Character type definitions
- `metadata.json`: Updated metadata file (if `--metadata` option is provided)

### Complete workflow example

#### 1. Train model

```bash
lindera train \
  --seed ./resources/training/seed.csv \
  --corpus ./resources/training/corpus.txt \
  --unk-def ./resources/training/unk.def \
  --char-def ./resources/training/char.def \
  --feature-def ./resources/training/feature.def \
  --rewrite-def ./resources/training/rewrite.def \
  --output /tmp/lindera/training/model.dat \
  --lambda 0.01 \
  --max-iterations 100
```

#### 2. Export to dictionary format

```bash
lindera export \
  --model /tmp/lindera/training/model.dat \
  --metadata ./resources/training/metadata.json \
  --output /tmp/lindera/training/dictionary
```

#### 3. Build dictionary

```bash
lindera build \
  --src /tmp/lindera/training/dictionary \
  --dest /tmp/lindera/training/compiled_dictionary \
  --metadata /tmp/lindera/training/dictionary/metadata.json
```

#### 4. Use trained dictionary

```bash
echo "これは外国人参政権です。" | lindera tokenize \
  -d /tmp/lindera/training/compiled_dictionary
```

### Metadata update feature

When the `--metadata` option is provided, the export command will:

1. **Read the base metadata.json file** to preserve existing configuration
2. **Update specific fields** with values from the trained model:
   - `default_left_context_id`: Maximum left context ID from trained model
   - `default_right_context_id`: Maximum right context ID from trained model
   - `default_word_cost`: Calculated from feature weight median
   - `model_info`: Training statistics including:
     - `feature_count`: Number of features in the model
     - `label_count`: Number of labels in the model
     - `max_left_context_id`: Maximum left context ID
     - `max_right_context_id`: Maximum right context ID
     - `connection_matrix_size`: Size of connection cost matrix
     - `training_iterations`: Number of training iterations performed
     - `regularization`: L1 regularization parameter used
     - `version`: Model version
     - `updated_at`: Timestamp of when the model was exported

3. **Preserve existing settings** such as:
   - Dictionary name
   - Character encoding settings
   - Schema definitions
   - Other user-defined configuration

This allows you to maintain your base dictionary configuration while incorporating the optimized parameters learned during training.

## API reference

The API reference is available. Please see following URL:

- [lindera-cli](https://docs.rs/lindera-cli)
