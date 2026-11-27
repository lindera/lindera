# Build

This page describes how to build the JUMAN dictionary from source files.

## Build system dictionary

The source is `mecab-jumandic/` in the repository of MeCab. Download the archive of the commit the crate builds from, and build the dictionary:

```shell
# Download the repository of MeCab and extract mecab-jumandic
% curl -L -o /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz \
  "https://github.com/taku910/mecab/archive/61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz"
% tar zxvf /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz -C /tmp \
  mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic

# Build the dictionary
% lindera build \
  --src /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic \
  --dest /tmp/lindera-jumandic-7.0-20130310 \
  --metadata ./lindera-jumandic/metadata.json
```

The archive is 45 MiB, and its SHA-256 is `7ad44f987ae0b7fd345c72a4b67e14dbe0f4bd1669d247a8aaeb0f15218a3fd1`, which the build script of the crate checks. The built dictionary is 125 MB, with 751,179 words.

## The six rows that are skipped

The build warns of six rows:

```text
[WARN  lindera_dictionary::builder::prefix_dictionary] Skipping a lexicon entry: /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic/AuxV.csv: line 588: it is not UTF-8 (invalid utf-8: invalid UTF-8 in field 0 near byte index 3)
```

Lines 588 to 593 of `AuxV.csv`, six auxiliary verbs (助動詞, 無活用型), end a character after two of its three bytes, in the surface, the base form and the reading. The released archive of mecab-jumandic 7.0-20130310 has the same bytes: the rows were broken when the dictionary was made, and what they were is not known. MeCab takes a row as bytes, so its dictionary has the six words, with surfaces no text has.

A dictionary in UTF-8 is not built from a row that is not UTF-8: the build fails, which is how a wrong `encoding` is found. The `metadata.json` of this dictionary asks to skip such rows instead, with `"skip_invalid_encoding": true`.

`skip_invalid_encoding` is `false` unless it is set. Set it in the `metadata.json` of a dictionary of your own only when its source is known to have such rows: with it, a wrong `encoding` drops rows with a warning instead of failing. For an encoding other than UTF-8, the rows it skips are those with bytes that have no character in the encoding, which without it get U+FFFD in their place.

## Build user dictionary

Build a user dictionary from a CSV file:

```shell
% lindera build \
  --src ./resources/user_dict/jumandic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-jumandic/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the JUMAN dictionary directly into the binary:

```shell
cargo build --features=embed-jumandic
```

This allows using `embedded://jumandic` as the dictionary path without external dictionary files. The build script of `lindera-jumandic` downloads the archive above and builds the dictionary.

## Training

Training a dictionary of this form with `lindera train` has not been tried: there is no corpus with JUMAN's parts of speech to train on here.
