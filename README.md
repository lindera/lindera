# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/bayard-search/lindera](https://badges.gitter.im/bayard-search/lindera.svg)](https://gitter.im/bayard-search/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese Morphological Analyzer written in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).


## Building Lindera

### Requirements

The following products are required to build Bayrad:

- Rust >= 1.39.0
- make >= 3.81

### Build

Build Bayard with the following command:

```text
$ make build
```

## Usage

Normal mode:
```
$ echo "関西国際空港限定トートバッグ" | ./bin/lindera tokenize --mode=normal
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

Search mode:
```
$ echo "関西国際空港限定トートバッグ" | ./bin/lindera tokenize --mode=search
関西    名詞,固有名詞,地域,一般,*,*,関西,カンサイ,カンサイ
国際    名詞,一般,*,*,*,*,国際,コクサイ,コクサイ
空港    名詞,一般,*,*,*,*,空港,クウコウ,クーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

test test_tokenize ... bench:       7,666 ns/iter (+/- 25,545)  
test test_tokenize ... bench:       5,507 ns/iter (+/- 755)
