# CaboCha's Models

[CaboCha](https://taku910.github.io/cabocha/) comes with models trained on the Kyoto University Text Corpus 4.0. They cannot be trained again without the corpus, and their bunsetsu are those of the Kyoto corpus, not those of UD Japanese-GSD, which the models of this crate follow. `lindera dependency convert-cabocha` converts such a model into a [model file](./model_file.md), and the analysis with the converted model is the one CaboCha gives.

## The terms of the models

**A converted model is limited to research and personal use, as the model it comes from. Convert the files of your own copy of CaboCha, and do not distribute the result.** This crate, its releases and its builds hold no model of CaboCha.

The license of CaboCha, the LGPL or the BSD license, does not cover its models. Its page says (in Japanese):

> 付属のモデルファイル(model 以下のファイル)にはBSDライセンス は適用されません。BSDライセンス は, 付属のプログラムや学習ツールのソース・コード, ドキュメントのみに適用されます。
>
> 付属のモデルファイルは, 株式会社毎日新聞社の御厚意により 配布されている毎日新聞記事CD-ROM版を基に学習, 作成されたものです。付属のモデルファイルは, 毎日新聞データ使用許諾に関する覚書に記載の利用条件内で その使用及び, 複製，改変，頒布が認められます. そのため配布しているモデルをそのままの形で使うことは, 研究目的, 個人利用に限られます.

That is: the BSD license applies to the programs, the training tools and the documentation only. The models were made from the Mainichi Shimbun article CD-ROM, and may be used, copied, modified and distributed within the terms of the memorandum on the use of that data; using the distributed models as they are is therefore limited to research and personal use. A model trained on a corpus of your own has no such limit: see [Chunking](./chunking.md) and [Parsing](./parsing.md) for training one.

## Which models

| Model of CaboCha | Files | |
| --- | --- | --- |
| IPA | `chunk.ipa.txt`, `dep.ipa.txt` | Converted, for the dictionary `ipadic` |
| JUMAN | `chunk.juman.txt`, `dep.juman.txt` | Converted, for the dictionary `jumandic`, the [JUMAN dictionary](../lindera-jumandic.md) |
| UniDic | `chunk.unidic.txt`, `dep.unidic.txt` | Converted, for the dictionary `unidic`, or for `unidic-cwj` or `unidic-csj` with `--dictionary` |
| Named entities | `ne.*.txt` | Not converted |

The files are the text models in `model/` of CaboCha's source, in UTF-8. The compiled models of an installation (`dep.ipa.model`) are not read: they depend on the machine.

The two files have to be those of one model. The part-of-speech set is read in the header of the parser model (`posset: IPA`), and the chunker model does not say which it is for: `chunk.ipa.txt` with `dep.juman.txt` converts without an error, into a model that chunks wrongly.

## Converting

The command is in the default build of `lindera`, which has the `train` feature:

```sh
cargo install lindera-cli --features embed-ipadic
```

```sh
lindera dependency convert-cabocha \
    --chunker cabocha/model/chunk.ipa.txt \
    --parser cabocha/model/dep.ipa.txt \
    --output cabocha-ipa.model
```

```text
dictionary: ipadic
chunker feature strings: 339749
support vectors: 107801 (54345 positive)
parser feature strings: 122541
pairs: 3727492 of 10141653 kept, 3713127 with a weight
unit: 1.9819337637727585e-7
bytes: 51809440
seconds: 3.6
The models of CaboCha are limited to research and personal use, and so is this file: do not distribute it.
```

The file is a model for `ipadic`, read by `parse`, by the [token filter](./token_filter.md) and by `Model::new` as any other, in the default build:

```sh
echo "太郎は花子が読んでいる本を次郎に渡した。" \
    | lindera dependency parse --dict embedded://ipadic --model cabocha-ipa.model
```

```text
* 0 5D 0/1 0.000000
太郎	名詞,固有名詞,人名,名,*,*,太郎,タロウ,タロー
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1 1.700175
花子	名詞,固有名詞,人名,名,*,*,花子,ハナコ,ハナコ
が	助詞,格助詞,一般,*,*,*,が,ガ,ガ
* 2 3D 0/2 1.825021
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
で	助詞,接続助詞,*,*,*,*,で,デ,デ
いる	動詞,非自立,*,*,一段,基本形,いる,イル,イル
* 3 5D 0/1 0.000000
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 4 5D 1/2 0.000000
次	名詞,一般,*,*,*,*,次,ツギ,ツギ
郎	名詞,一般,*,*,*,*,郎,ロウ,ロー
に	助詞,格助詞,一般,*,*,*,に,ニ,ニ
* 5 -1D 0/1 0.000000
渡し	動詞,自立,*,*,五段・サ行,連用形,渡す,ワタシ,ワタシ
た	助動詞,*,*,*,特殊・タ,基本形,た,タ,タ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

`cabocha -f1 -n0` writes the same for this sentence, but for one number: the score of an arc to the last bunsetsu, for which no decision is made. CaboCha writes there the score of the decision before (`-0.742128`), and `parse` 0.

From Rust, `lindera_dependency::cabocha::convert` takes the content of the two files and gives the two models and what it did.

### The model for JUMAN

The model for JUMAN is the one CaboCha's page gives the highest accuracy for: it was trained on the Kyoto corpus as it is, where the other two were trained on the corpus tokenized again. It is converted in the same way:

```sh
lindera dependency convert-cabocha \
    --chunker cabocha/model/chunk.juman.txt \
    --parser cabocha/model/dep.juman.txt \
    --output cabocha-juman.model
```

```text
dictionary: jumandic
chunker feature strings: 330800
support vectors: 113652 (57639 positive)
parser feature strings: 149674
pairs: 3687535 of 10578633 kept, 3676410 with a weight
unit: 2.277111690318634e-7
bytes: 52011417
seconds: 4.0
The models of CaboCha are limited to research and personal use, and so is this file: do not distribute it.
```

The file is a model for `jumandic`. `lindera-dependency` embeds no dictionary but IPADIC, so `--dict` takes the directory of the JUMAN dictionary, which `lindera download jumandic` installs, from 7.1.0 on, and [`lindera build`](../lindera-jumandic/build.md) makes from the source:

```sh
echo "太郎は花子が読んでいる本を次郎に渡した。" \
    | lindera dependency parse --dict jumandic --model cabocha-juman.model
```

```text
* 0 5D 0/1 0.000000
太郎	名詞,人名,*,*,太郎,たろう,人名:日本:名:45:0.00106
は	助詞,副助詞,*,*,は,は,*
* 1 2D 0/1 1.847622
花子	名詞,人名,*,*,花子,はなこ,人名:日本:名:912:0.00019
が	助詞,格助詞,*,*,が,が,*
* 2 3D 0/1 1.407350
読んで	動詞,*,子音動詞マ行,タ系連用テ形,読む,よんで,代表表記:読む/よむ
いる	接尾辞,動詞性接尾辞,母音動詞,基本形,いる,いる,連語
* 3 5D 0/1 0.000000
本	名詞,普通名詞,*,*,本,ほん,代表表記:本/ほん 漢字読み:音 カテゴリ:人工物-その他;抽象物
を	助詞,格助詞,*,*,を,を,*
* 4 5D 0/1 0.000000
次郎	名詞,人名,*,*,次郎,じろう,人名:日本:名:135:0.00068
に	助詞,格助詞,*,*,に,に,*
* 5 -1D 0/0 0.000000
渡した	動詞,*,子音動詞サ行,タ形,渡す,わたした,代表表記:渡す/わたす 付属動詞候補（基本） 自他動詞:自:渡る/わたる
。	特殊,句点,*,*,。,。,連語
EOS
```

`cabocha -f1 -n0 -P JUMAN`, with MeCab and mecab-jumandic, writes the same bunsetsu, heads, positions and scores for this sentence, with `-0.468695` where no decision is made. Its lines differ in the last column of the particles, `連語` where Lindera has `*`: the two take another one of two entries of the same cost ([Examples](../lindera-jumandic/examples.md#agreement-with-mecab)), in a column the model does not read.

### The model for UniDic

The model for UniDic is converted in the same way:

```sh
lindera dependency convert-cabocha \
    --chunker cabocha/model/chunk.unidic.txt \
    --parser cabocha/model/dep.unidic.txt \
    --output cabocha-unidic.model
```

```text
dictionary: unidic
chunker feature strings: 322586
support vectors: 118666 (60095 positive)
parser feature strings: 121120
pairs: 3360320 of 8881051 kept, 3350964 with a weight
unit: 2.2718941750099475e-7
bytes: 47727972
seconds: 3.0
The models of CaboCha are limited to research and personal use, and so is this file: do not distribute it.
```

The file is a model for `unidic`, the dictionary built from unidic-mecab 2.1.2. Lindera has three UniDic dictionaries, which have the columns a model reads at the same places, and a model file is for one dictionary by its name: `--dictionary unidic-cwj` or `--dictionary unidic-csj` makes the model one for another of them. CaboCha's page does not say which UniDic its model was trained with. `--dict` takes the directory of the dictionary, which `lindera download unidic` installs:

```sh
echo "太郎は花子が読んでいる本を次郎に渡した。" \
    | lindera dependency parse --dict unidic --model cabocha-unidic.model
```

```text
* 0 5D 0/1 0.000000
太郎	名詞,固有名詞,人名,名,*,*,タロウ,タロウ,太郎,タロー,太郎,タロー,固,*,*,*,*
は	助詞,係助詞,*,*,*,*,ハ,は,は,ワ,は,ワ,和,*,*,*,*
* 1 2D 0/1 1.426111
花子	名詞,固有名詞,人名,名,*,*,ハナコ,ハナコ,花子,ハナコ,花子,ハナコ,固,*,*,*,*
が	助詞,格助詞,*,*,*,*,ガ,が,が,ガ,が,ガ,和,*,*,*,*
* 2 3D 2/2 1.388999
読ん	動詞,一般,*,*,五段-マ行,連用形-撥音便,ヨム,読む,読ん,ヨン,読む,ヨム,和,*,*,*,*
で	助詞,接続助詞,*,*,*,*,テ,て,で,デ,で,デ,和,*,*,*,*
いる	動詞,非自立可能,*,*,上一段-ア行,連体形-一般,イル,居る,いる,イル,いる,イル,和,*,*,*,*
* 3 5D 0/1 0.000000
本	名詞,普通名詞,一般,*,*,*,ホン,本,本,ホン,本,ホン,漢,ホ濁,基本形,*,*
を	助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*
* 4 5D 0/1 0.000000
次郎	名詞,固有名詞,人名,名,*,*,ジロウ,ジロウ,次郎,ジロー,次郎,ジロー,固,*,*,*,*
に	助詞,格助詞,*,*,*,*,ニ,に,に,ニ,に,ニ,和,*,*,*,*
* 5 -1D 0/1 0.000000
渡し	動詞,一般,*,*,五段-サ行,連用形-一般,ワタス,渡す,渡し,ワタシ,渡す,ワタス,和,*,*,*,*
た	助動詞,*,*,*,助動詞-タ,終止形-一般,タ,た,た,タ,た,タ,和,*,*,*,*
。	補助記号,句点,*,*,*,*,,。,。,,。,,記号,*,*,*,*
EOS
```

CaboCha, given these morphemes (`cabocha -I1 -f1 -n0 -P UNIDIC`), writes the same bunsetsu, heads, positions and scores, with `-0.882864` where no decision is made. 読ん / で / いる has `2/2`: いる is a 動詞,非自立可能, which CaboCha's rule for UniDic takes as the head word.

## What is converted

**The chunker** is a CRF++ model in text. Its template, its feature strings and its weights go into the chunker part as they are, with the weights rounded to 32-bit floats, as CRF++ keeps them in the compiled model CaboCha reads.

**The parser** is the support vectors of a kernel of degree 2: the score of a decision is `Σ α (1 + s)^2` over the support vectors, where `s` is the number of features a support vector shares with the decision. It expands, without error, into a bias, a weight for each feature and a weight for each pair of features, which is what a parser part holds. CaboCha does not parse with that: `cabocha-model-index` compiles the text model and approximates, and the conversion does the same steps, so that a decision adds up the same whole numbers:

- a pair of two of the 3,000 most frequent features is always kept;
- another pair is kept when it is in 2 support vectors or more, and its weight is `sigma × positives / all` or more, or `-sigma × negatives / all` or less, with `sigma` 0.0001;
- the weights are added in 32-bit floats, divided by the largest absolute weight over 1,048,575, and truncated to whole numbers. The model file keeps the whole numbers, unquantized, and that divisor as the scale of its weights.

| Option | Default | |
| --- | --- | --- |
| `--sigma` | 0.0001 | `-s` of `cabocha-model-index` |
| `--min-support` | 2 | `-n` |
| `--frequent-features` | 3000 | `-F` |

The defaults are those of `cabocha-model-index`, with which CaboCha's own build compiles its models. `--sigma 0 --min-support 1` keeps every pair, which is the kernel itself, and not what CaboCha parses with:

| Model | | Pairs kept | Pairs with a weight | File | Conversion |
| --- | --- | --- | --- | --- | --- |
| IPA | The defaults | 3,727,492 of 10,141,653 | 3,713,127 | 51.8 MB | 3.6 s |
| IPA | `--sigma 0 --min-support 1` | 10,141,653 | 9,988,000 | 102.0 MB | 4.5 s |
| JUMAN | The defaults | 3,687,535 of 10,578,633 | 3,676,410 | 52.0 MB | 4.0 s |
| JUMAN | `--sigma 0 --min-support 1` | 10,578,633 | 10,450,076 | 106.2 MB | 4.9 s |
| UniDic | The defaults | 3,360,320 of 8,881,051 | 3,350,964 | 47.7 MB | 3.0 s |
| UniDic | `--sigma 0 --min-support 1` | 8,881,051 | 8,782,456 | 91.2 MB | 3.7 s |

**What makes the features** is data of the model file, and CaboCha's for a converted model:

- the surface is normalized by CaboCha's table before it becomes a feature or is matched as a mark: the half-width katakana in full width, the full-width letters, digits and signs in ASCII (`ＡＢＣ` is `ABC`, `ｶﾞ` is `ガ`);
- for IPA, the function words are 助詞, 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立 and 形容詞,接尾, and the head is the last morpheme that is neither one of them nor a 記号. It is not the schema `ipadic` of this crate, which has more patterns ([Head and Function Word](./head_and_function_word.md)): in 利用 / し / た the head is し here, and 利用 there;
- for JUMAN, the function word is the last morpheme that is not a 特殊, and the head the last that is not a 特殊, a 助詞 or a 接尾辞. The two are found apart, which is another form of the rule ([Head and Function Word](./head_and_function_word.md#the-rule)): a bunsetsu of one noun has the noun as both. The part of speech has two levels, and the conjugation form is the fourth column;
- for UniDic, the function words are 助詞, 助動詞, 接尾辞,形容詞的, 接尾辞,形状詞的 and 接尾辞,動詞的, and the head is the last morpheme that is neither one of them nor a 空白, a 補助記号 or a 記号. CaboCha reads another number of levels of the part of speech in its chunker and in its parser: the chunker four, and the parser two, with the column after the next as the conjugation form. In UniDic that column is the fourth level of the part of speech (一般, 名, 国 and 姓 in the model), not a conjugation form; the model was trained with it, and gives CaboCha's analysis only when it is read in the same way. So the schema has four levels and the fourth column as the conjugation form, and the feature set of the parser is held to two levels (`pos-levels=2`, see [Parsing](./parsing.md#the-feature-set));
- the punctuation is `。` and `、`, and the brackets are those of CaboCha, each matched as the whole surface.

## Agreement with CaboCha

`scripts/dependency/cabocha_models.sh` converts the model for IPA, or with `--posset juman` or `--posset unidic` the one for JUMAN or for UniDic, and gives the same input to CaboCha 0.69 (96e100a, built by `scripts/dependency/build_baselines.sh`, with the model as its own build compiled it) and to `lindera dependency parse` with the converted model. The input is the 9,066 sentences of UD Japanese-GSD and UD Japanese-PUD, as a fixed text: their answers are not used.

On the same morphemes, those of Lindera with IPADIC, everything is the same:

| File | Sentences with the same bunsetsu | Bunsetsu with the same head | Bunsetsu with the same head word and function word |
| --- | --- | --- | --- |
| GSD dev | 100% (506 / 506) | 100% (3,740 / 3,740) | 100% (4,246 / 4,246) |
| GSD test | 100% (542 / 542) | 100% (4,059 / 4,059) | 100% (4,601 / 4,601) |
| PUD | 100% (994 / 994) | 100% (9,076 / 9,076) | 100% (10,070 / 10,070) |
| GSD train | 100% (7,024 / 7,024) | 100% (50,718 / 50,718) | 100% (57,742 / 57,742) |

The scores of the decisions are the same to the six decimals CaboCha writes.

With `--sigma 0 --min-support 1` the bunsetsu are the same and 99.65% to 99.79% of the heads (3,727 / 3,740, 4,048 / 4,059, 9,057 / 9,076 and 50,573 / 50,718): CaboCha's approximation changes about one head in 360 (188 of 67,593).

From text, CaboCha tokenizes with MeCab and mecab-ipadic, and `parse` with Lindera and its IPADIC, which is built from the mecab-ipadic source. The two give the same morphemes for 91% to 98% of the sentences, and those sentences are analyzed the same:

| File | Sentences with the same morphemes | Of which analyzed the same |
| --- | --- | --- |
| GSD dev | 97.04% (491 / 506) | 100% (491 / 491) |
| GSD test | 98.15% (532 / 542) | 100% (532 / 532) |
| PUD | 91.15% (906 / 994) | 100% (906 / 906) |
| GSD train | 96.48% (6,777 / 7,024) | 100% (6,777 / 6,777) |

The other sentences are tokenized differently, nearly all of them at a katakana word that is not in the dictionary (332 of 335 on PUD and GSD train): MeCab gives `コリ・シューマン` as one word where Lindera gives three, and `メトロ` / `ポリ` / `タン` where Lindera gives `メトロポリタン`. That is a difference of the tokenizers, not of the models.

### With the model for JUMAN

The input is the 9,100 sentences of the same files, and the morphemes are those of MeCab with mecab-jumandic. Everything is the same:

| File | Sentences with the same bunsetsu | Bunsetsu with the same head | Bunsetsu with the same head word and function word |
| --- | --- | --- | --- |
| GSD dev | 100% (507 / 507) | 100% (3,870 / 3,870) | 100% (4,377 / 4,377) |
| GSD test | 100% (543 / 543) | 100% (4,194 / 4,194) | 100% (4,737 / 4,737) |
| PUD | 100% (1,000 / 1,000) | 100% (9,430 / 9,430) | 100% (10,430 / 10,430) |
| GSD train | 100% (7,050 / 7,050) | 100% (52,811 / 52,811) | 100% (59,861 / 59,861) |

The scores are the same to six decimals here too. With `--sigma 0 --min-support 1` the bunsetsu are the same and 99.63% to 99.77% of the heads (3,861 / 3,870, 4,181 / 4,194, 9,395 / 9,430 and 52,648 / 52,811): one head in 320 (220 of 70,305).

From text, CaboCha tokenizes with MeCab and mecab-jumandic, and Lindera with its JUMAN dictionary, built from the same source (`lindera tokenize`, whose morphemes `parse` is given). The morphemes are the same in nearly every sentence, in the five columns the model reads, and those sentences are analyzed the same:

| File | Sentences with the same morphemes | Of which analyzed the same |
| --- | --- | --- |
| GSD dev | 99.61% (505 / 507) | 100% (505 / 505) |
| GSD test | 99.63% (541 / 543) | 100% (541 / 541) |
| PUD | 100% (1,000 / 1,000) | 100% (1,000 / 1,000) |
| GSD train | 99.79% (7,035 / 7,050) | 100% (7,035 / 7,035) |

The 19 other sentences are tokenized differently, 17 of them at a `・` between digits, which `lindera tokenize --disable-break-characters` reads as MeCab does ([Examples](../lindera-jumandic/examples.md#agreement-with-mecab)).

### With the model for UniDic

The input is the 9,066 sentences with the morphemes of Lindera and a UniDic dictionary, and the model is converted for that dictionary. Everything is the same, with each of the three:

| Dictionary | File | Sentences with the same bunsetsu | Bunsetsu with the same head | Bunsetsu with the same head word and function word |
| --- | --- | --- | --- | --- |
| `unidic` | GSD dev | 100% (506 / 506) | 100% (3,846 / 3,846) | 100% (4,352 / 4,352) |
| `unidic` | GSD test | 100% (542 / 542) | 100% (4,169 / 4,169) | 100% (4,711 / 4,711) |
| `unidic` | PUD | 100% (994 / 994) | 100% (9,333 / 9,333) | 100% (10,327 / 10,327) |
| `unidic` | GSD train | 100% (7,024 / 7,024) | 100% (52,300 / 52,300) | 100% (59,324 / 59,324) |
| `unidic-cwj` | GSD dev | 100% (506 / 506) | 100% (3,852 / 3,852) | 100% (4,358 / 4,358) |
| `unidic-cwj` | GSD test | 100% (542 / 542) | 100% (4,172 / 4,172) | 100% (4,714 / 4,714) |
| `unidic-cwj` | PUD | 100% (994 / 994) | 100% (9,335 / 9,335) | 100% (10,329 / 10,329) |
| `unidic-cwj` | GSD train | 100% (7,024 / 7,024) | 100% (52,271 / 52,271) | 100% (59,295 / 59,295) |
| `unidic-csj` | GSD dev | 100% (506 / 506) | 100% (3,850 / 3,850) | 100% (4,356 / 4,356) |
| `unidic-csj` | GSD test | 100% (542 / 542) | 100% (4,176 / 4,176) | 100% (4,718 / 4,718) |
| `unidic-csj` | PUD | 100% (994 / 994) | 100% (9,336 / 9,336) | 100% (10,330 / 10,330) |
| `unidic-csj` | GSD train | 100% (7,024 / 7,024) | 100% (52,244 / 52,244) | 100% (59,268 / 59,268) |

The scores are the same to six decimals. With `--sigma 0 --min-support 1` and `unidic`, the bunsetsu are the same and 99.45% to 99.71% of the heads (3,835 / 3,846, 4,146 / 4,169, 9,299 / 9,333 and 52,106 / 52,300): one head in 266 (262 of 69,648).

The agreement from text is not measured for UniDic: CaboCha tokenizes text with MeCab, and `build_baselines.sh` builds no UniDic dictionary for MeCab.

## Limits

- **Speed.** `parse` takes 3.03 s for the 7,024 sentences of GSD train from their morphemes, and CaboCha 0.92 s. The other two models are no slower: in a run where those two took 2.98 s and 0.87 s, the model for JUMAN took 2.60 s and CaboCha 0.77 s for the 7,050 sentences, and the model for UniDic 2.60 s and CaboCha 0.77 s for the 7,024. A converted model keeps each pair with a weight of its own, where the models of this crate hash the pairs into a table: see [Model File](./model_file.md) for their speed.
- **The score of an arc to the last bunsetsu** is 0, as with every model of this crate.
- **A long bunsetsu.** CaboCha keeps at most 128 features and 2,048 bytes of feature strings for a bunsetsu, and this crate all of them. The 76,659 bunsetsu of the text above have at most 40 features and 882 bytes, the 79,405 of the morphemes of mecab-jumandic at most 42 and 818, and the 78,714 of those of `unidic` at most 40 and 953, so the limit is not met there; a bunsetsu beyond it would differ.
- **The chunker** is asked for the best path that starts with `B`; CaboCha takes the best path and then makes the first morpheme start a bunsetsu. CRF++ adds the weights of a morpheme in a 32-bit float, and this crate in 64 bits. Neither changed a bunsetsu of the text above.
- **The accuracy** of CaboCha's models is not measured here: the Kyoto corpus is not available, and an accuracy on UD Japanese-GSD would compare two standards of bunsetsu.
- **Not verified**: CaboCha built with another compiler or system than GCC 15.2 on Linux (x86_64).
