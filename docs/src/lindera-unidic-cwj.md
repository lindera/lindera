# Lindera UniDic CWJ

Lindera UniDic CWJ is a Japanese dictionary crate based on [UniDic for contemporary written Japanese](https://clrd.ninjal.ac.jp/unidic/) (現代書き言葉UniDic) ver.2025.12, the successor of UniDic 3.1, distributed by the National Institute for Japanese Language and Linguistics (NINJAL). It has 33 fields per entry, 12 more than [UniDic 2.1.2](./lindera-unidic.md), among them the kana forms, the accent and the lexicon and lemma IDs.

Its source is downloaded from NINJAL's official URL, not from lindera.dev, and checked against the SHA-256 that the crate pins. `lindera-unidic` keeps providing UniDic 2.1.2.

> [!NOTE]
> UniDic CWJ is much larger than the other dictionaries. The built dictionary takes 1.2 GB, 982 MiB of it the connection matrix (24,082 × 21,375). With `embed-unidic-cwj`, every binary carries those 1.2 GB, and compiling the crate takes about 11 GiB of memory (measured on macOS). The WASM binding does not provide `embed-unidic-cwj`. To keep binaries small, download the dictionary with `lindera download unidic-cwj` and load it from its directory.

## Contents

- [Dictionary Format](./lindera-unidic-cwj/dictionary_format.md) -- Field definitions for system and user dictionaries
- [Build](./lindera-unidic-cwj/build.md) -- How to build the dictionary from source
- [Examples](./lindera-unidic-cwj/examples.md) -- Tokenization examples

## API Reference

- [lindera-unidic-cwj](https://docs.rs/lindera-unidic-cwj)
