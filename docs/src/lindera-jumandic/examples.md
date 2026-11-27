# Examples

This page shows tokenization examples using the JUMAN dictionary.

## Tokenize with external JUMAN dictionary

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-jumandic-7.0-20130310
```

```text
日本	名詞,地名,*,*,日本,にほん,代表表記:日本/にほん 地名:国
語	名詞,普通名詞,*,*,語,ご,代表表記:語/ご 漢字読み:音 カテゴリ:抽象物
の	助詞,接続助詞,*,*,の,の,*
形態	名詞,普通名詞,*,*,形態,けいたい,代表表記:形態/けいたい カテゴリ:形・模様
素	名詞,普通名詞,*,*,素,そ,代表表記:素/そ 漢字読み:音 カテゴリ:抽象物
解析	名詞,サ変名詞,*,*,解析,かいせき,代表表記:解析/かいせき カテゴリ:抽象物 ドメイン:教育・学習;科学・技術
を	助詞,格助詞,*,*,を,を,*
行う	動詞,*,子音動詞ワ行,基本形,行う,おこなう,代表表記:行う/おこなう
こと	名詞,形式名詞,*,*,こと,こと,*
が	助詞,格助詞,*,*,が,が,*
でき	動詞,*,母音動詞,基本連用形,できる,でき,代表表記:出来る/できる
ます	接尾辞,動詞性接尾辞,動詞性接尾辞ます型,基本形,ます,ます,代表表記:ます/ます
。	特殊,句点,*,*,。,。,連語
EOS
```

## Tokenize with embedded JUMAN dictionary

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://jumandic
```

```text
日本	名詞,地名,*,*,日本,にほん,代表表記:日本/にほん 地名:国
語	名詞,普通名詞,*,*,語,ご,代表表記:語/ご 漢字読み:音 カテゴリ:抽象物
の	助詞,接続助詞,*,*,の,の,*
形態	名詞,普通名詞,*,*,形態,けいたい,代表表記:形態/けいたい カテゴリ:形・模様
素	名詞,普通名詞,*,*,素,そ,代表表記:素/そ 漢字読み:音 カテゴリ:抽象物
解析	名詞,サ変名詞,*,*,解析,かいせき,代表表記:解析/かいせき カテゴリ:抽象物 ドメイン:教育・学習;科学・技術
を	助詞,格助詞,*,*,を,を,*
行う	動詞,*,子音動詞ワ行,基本形,行う,おこなう,代表表記:行う/おこなう
こと	名詞,形式名詞,*,*,こと,こと,*
が	助詞,格助詞,*,*,が,が,*
でき	動詞,*,母音動詞,基本連用形,できる,でき,代表表記:出来る/できる
ます	接尾辞,動詞性接尾辞,動詞性接尾辞ます型,基本形,ます,ます,代表表記:ます/ます
。	特殊,句点,*,*,。,。,連語
EOS
```

NOTE: To include the JUMAN dictionary in the binary, you must build with the `--features=embed-jumandic` option.

## Tokenize with user dictionary (CSV format)

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict embedded://jumandic \
  --user-dict ./resources/user_dict/jumandic_simple_userdic.csv
```

```text
東京スカイツリー	カスタム名詞,*,*,*,*,とうきょうすかいつりー,*
の	助詞,接続助詞,*,*,の,の,*
最寄り	名詞,普通名詞,*,*,最寄り,もより,代表表記:最寄り/もより カテゴリ:場所-その他
駅	名詞,普通名詞,*,*,駅,えき,代表表記:駅/えき 漢字読み:音 地名末尾 カテゴリ:場所-施設 ドメイン:交通
は	助詞,副助詞,*,*,は,は,*
とうきょうスカイツリー駅	カスタム名詞,*,*,*,*,とうきょうすかいつりーえき,*
です	判定詞,*,判定詞,デス列基本形,だ,です,*
EOS
```

## Agreement with MeCab

`scripts/dependency/mecab_morphemes.sh` tokenizes the same text with MeCab and mecab-jumandic, built from the same source, and with `lindera tokenize`. The text is the 9,100 sentences of UD Japanese-GSD and UD Japanese-PUD.

| File | Same segmentation | Also the same six fields up to the reading | Also the same semantic information |
| --- | --- | --- | --- |
| GSD dev | 99.61% (505 / 507) | 90.93% (461 / 507) | 2.96% (15 / 507) |
| GSD test | 99.63% (541 / 543) | 93.74% (509 / 543) | 4.05% (22 / 543) |
| PUD | 100% (1,000 / 1,000) | 91.30% (913 / 1,000) | 0% (0 / 1,000) |
| GSD train | 99.79% (7,035 / 7,050) | 90.30% (6,366 / 7,050) | 2.81% (198 / 7,050) |

**The segmentation** differs in 19 sentences:

- 17 at a `・` between digits (`66・0`). mecab-jumandic makes `・` a character of numbers, and MeCab reads `66・0` as one number. Lindera lets no unknown word cross a break character, which is `・` by default: with `lindera tokenize --disable-break-characters` these 17 sentences are the same, and the segmentation is the same in 9,098 sentences of 9,100.
- One at a run of katakana of 28 characters, which MeCab does not take as one unknown word (it groups a character with 24 more at most, `max-grouping-size`), and Lindera does.
- One at a run of symbols, `%、「`: MeCab takes the three as one unknown word, and Lindera `%、` and the bracket.

**The fields** differ where the source has the same surface twice with the same cost and the same context IDs. Then the two analyses cost the same, and Lindera and MeCab do not take the same one:

- **the reading** in 6% to 10% of the sentences: 一 is ひと for MeCab and いち for Lindera, 国 is こく and くに;
- **the semantic information** in nearly every sentence: a particle such as は is in `Postp.csv` with `*` and in `Rengo.csv` with `連語`, and MeCab gives `連語` where Lindera gives `*`.

The surface, the part of speech, the conjugation and the base form of a word are not touched by it.
