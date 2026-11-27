# Lindera JUMAN dictionary

Lindera JUMAN dictionary (`lindera-jumandic`) is a Japanese dictionary crate built from mecab-jumandic 7.0-20130310, the dictionary of the morphological analyzer [JUMAN](https://nlp.ist.i.kyoto-u.ac.jp/?JUMAN) in MeCab's format. Its name is `jumandic`.

It has JUMAN's parts of speech (名詞,普通名詞; 動詞; 判定詞; 接尾辞; 特殊,句点), its conjugation types and forms (子音動詞マ行, タ形), readings in hiragana, and JUMAN's semantic information as a seventh field (`代表表記:本/ほん 漢字読み:音 カテゴリ:人工物-その他;抽象物`). A conjugated word with its ending is one word: 読んだ, where IPADIC gives 読ん and だ.

The source is taken from the repository of MeCab ([taku910/mecab](https://github.com/taku910/mecab), `mecab-jumandic/`), under a 3-clause BSD license. It is the dictionary the released archive of that version holds, file for file.

It is also the dictionary of the model CaboCha distributes for JUMAN, which `lindera-dependency` [converts](./lindera-dependency/cabocha_models.md) into a model for bunsetsu dependency analysis.

## Contents

- [Dictionary Format](./lindera-jumandic/dictionary_format.md) -- Field definitions for system and user dictionaries
- [Build](./lindera-jumandic/build.md) -- How to build the dictionary from source, and the six rows that are skipped
- [Examples](./lindera-jumandic/examples.md) -- Tokenization examples, and the agreement with MeCab

## API Reference

- [lindera-jumandic](https://docs.rs/lindera-jumandic)
