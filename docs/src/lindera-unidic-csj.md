# Lindera UniDic CSJ

Lindera UniDic CSJ is a Japanese dictionary crate based on [UniDic for contemporary spoken Japanese](https://clrd.ninjal.ac.jp/unidic/) (現代話し言葉UniDic) ver.2025.12, distributed by the National Institute for Japanese Language and Linguistics (NINJAL). It is made for transcriptions of speech: NINJAL trained it on the Corpus of Everyday Japanese Conversation (CEJC).

It has the same words and the same 33 fields per entry as [UniDic CWJ](./lindera-unidic-cwj.md), the UniDic for contemporary written Japanese, with other word costs and another connection matrix, so the two split some text differently. Use UniDic CSJ for transcribed speech and UniDic CWJ for written text.

Its source is downloaded from NINJAL's official URL, not from lindera.dev, and checked against the SHA-256 that the crate pins.

> [!NOTE]
> UniDic CSJ is as large as UniDic CWJ. The built dictionary takes 1.2 GB, 1,002 MiB of it the connection matrix (24,314 × 21,609). With `embed-unidic-csj`, every binary carries those 1.2 GB, and compiling the crate takes as much memory as for UniDic CWJ: 8 to 11 GiB in the builds measured on macOS. The WASM binding does not provide `embed-unidic-csj`. To keep binaries small, download the dictionary with `lindera download unidic-csj` and load it from its directory.

## Contents

- [Dictionary Format](./lindera-unidic-csj/dictionary_format.md) -- Field definitions for system and user dictionaries
- [Build](./lindera-unidic-csj/build.md) -- How to build the dictionary from source
- [Examples](./lindera-unidic-csj/examples.md) -- Tokenization examples

## API Reference

- [lindera-unidic-csj](https://docs.rs/lindera-unidic-csj)
