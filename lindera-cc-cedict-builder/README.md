# Lindera CC-CEDICT Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

CC-CEDICT dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera).


## Install

```shell script
% cargo install lindera-cc-cedict-builder
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


## Building a dictionary

Downloading a dictionary source from [CC-CEDICT-MeCab](https://github.com/ueda-keisuke/CC-CEDICT-MeCab):

```shell script
% curl -L -o /tmp/CC-CEDICT-MeCab.zip https://github.com/ueda-keisuke/CC-CEDICT-MeCab/archive/refs/heads/master.zip
% unzip /tmp/CC-CEDICT-MeCab.zip -d /tmp
% lindera-cc-cedict-builder -s /tmp/CC-CEDICT-MeCab-master -d /tmp/lindera-cc-cedict
```


## Dictionary format


## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```shell script
% echo "它可以进行日语和汉语的语态分析" | lindera -d /tmp/lindera-cc-cedict
```

```text
它      *,*,*,*,ta1,它,它,it/
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
日语    *,*,*,*,Ri4 yu3,日語,日语,Japanese language/
和      *,*,*,*,he2,龢,和,old variant of 和[he2]/harmonious/
汉语    *,*,*,*,Han4 yu3,漢語,汉语,Chinese language/CL:門|门[men2]/
的      *,*,*,*,di4,的,的,aim/clear/
语态    *,*,*,*,yu3 tai4,語態,语态,voice (grammar)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
EOS
```

For more details about `lindera` command, please refer to the following URL:

- [Lindera CLI](https://github.com/lindera-morphology/lindera/lindera-cli)


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-cc-cedict-builder" target="_blank">lindera-cc-cedict-builder</a>
