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

### Tokenize mode

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

### Output format

MeCab format:
```
$ echo "お待ちしております。" | ./bin/lindera tokenize --output=mecab
お待ち	名詞,サ変接続,*,*,*,*,お待ち,オマチ,オマチ
し	動詞,自立,*,*,サ変・スル,連用形,する,シ,シ
て	助詞,接続助詞,*,*,*,*,て,テ,テ
おり	動詞,非自立,*,*,五段・ラ行,連用形,おる,オリ,オリ
ます	助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

Wakati format:
```
$ echo "お待ちしております。" | ./bin/lindera tokenize --output=wakati
お待ち し て おり ます 。
```

JSON format:
```
$ echo "お待ちしております。" | ./bin/lindera tokenize --output=json
[
  {
    "text": "お待ち",
    "detail": {
      "pos_level1": "名詞",
      "pos_level2": "サ変接続",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "*",
      "conjugate_form": "*",
      "base_form": "お待ち",
      "reading": "オマチ",
      "pronunciation": "オマチ"
    }
  },
  {
    "text": "し",
    "detail": {
      "pos_level1": "動詞",
      "pos_level2": "自立",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "サ変・スル",
      "conjugate_form": "連用形",
      "base_form": "する",
      "reading": "シ",
      "pronunciation": "シ"
    }
  },
  {
    "text": "て",
    "detail": {
      "pos_level1": "助詞",
      "pos_level2": "接続助詞",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "*",
      "conjugate_form": "*",
      "base_form": "て",
      "reading": "テ",
      "pronunciation": "テ"
    }
  },
  {
    "text": "おり",
    "detail": {
      "pos_level1": "動詞",
      "pos_level2": "非自立",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "五段・ラ行",
      "conjugate_form": "連用形",
      "base_form": "おる",
      "reading": "オリ",
      "pronunciation": "オリ"
    }
  },
  {
    "text": "ます",
    "detail": {
      "pos_level1": "助動詞",
      "pos_level2": "*",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "特殊・マス",
      "conjugate_form": "基本形",
      "base_form": "ます",
      "reading": "マス",
      "pronunciation": "マス"
    }
  },
  {
    "text": "。",
    "detail": {
      "pos_level1": "記号",
      "pos_level2": "句点",
      "pos_level3": "*",
      "pos_level4": "*",
      "conjugation_type": "*",
      "conjugate_form": "*",
      "base_form": "。",
      "reading": "。",
      "pronunciation": "。"
    }
  }
]
```

If you output result in JSON format, token can be filtering is easily assured by using with jq command.  
For example, folloing command executes:
1. Tokenize a text
2. Filter tokens by part of speech (名詞)
3. Concat the token text with a white space

```
$ echo "すもももももももものうち" | ./bin/lindera tokenize --output=json |
    jq -r '.[] | select (.detail.pos_level1 =="名詞")' |
    jq -s -r '. | map(.text) | join(" ")'
すもも もも もも うち
```

test test_tokenize ... bench:       7,666 ns/iter (+/- 25,545)  
test test_tokenize ... bench:       5,507 ns/iter (+/- 755)
