# CaboCha's Models

[CaboCha](https://taku910.github.io/cabocha/) comes with models trained on the Kyoto University Text Corpus 4.0. They cannot be trained again without the corpus, and their bunsetsu are those of the Kyoto corpus, not those of UD Japanese-GSD, which the models of this crate follow. `lindera-dependency convert-cabocha` converts such a model into a [model file](./model_file.md), and the analysis with the converted model is the one CaboCha gives.

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
| JUMAN | `chunk.juman.txt`, `dep.juman.txt` | Refused: Lindera has no JUMAN dictionary yet ([#1466](https://github.com/lindera/lindera/issues/1466)), and the rule for the head of a bunsetsu is another one |
| UniDic | `chunk.unidic.txt`, `dep.unidic.txt` | Refused: its columns are read in another way, and whether its features agree with the morphemes of Lindera's UniDic is not known yet ([#1442](https://github.com/lindera/lindera/issues/1442)) |
| Named entities | `ne.*.txt` | Not converted |

The files are the text models in `model/` of CaboCha's source, in UTF-8. The compiled models of an installation (`dep.ipa.model`) are not read: they depend on the machine.

## Converting

The command is in the build with the `train` feature:

```sh
cargo install lindera-dependency --features train,embed-ipadic
```

```sh
lindera-dependency convert-cabocha \
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
bytes: 51809439
seconds: 3.6
The models of CaboCha are limited to research and personal use, and so is this file: do not distribute it.
```

The file is a model for `ipadic`, read by `parse`, by the [token filter](./token_filter.md) and by `Model::new` as any other, in the default build:

```sh
echo "太郎は花子が読んでいる本を次郎に渡した。" \
    | lindera-dependency parse --dict embedded://ipadic --model cabocha-ipa.model
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

| | Pairs kept | Pairs with a weight | File | Conversion |
| --- | --- | --- | --- | --- |
| The defaults | 3,727,492 of 10,141,653 | 3,713,127 | 51.8 MB | 3.6 s |
| `--sigma 0 --min-support 1` | 10,141,653 | 9,988,000 | 102.0 MB | 4.5 s |

**What makes the features** is data of the model file, and CaboCha's for a converted model:

- the surface is normalized by CaboCha's table before it becomes a feature or is matched as a mark: the half-width katakana in full width, the full-width letters, digits and signs in ASCII (`ＡＢＣ` is `ABC`, `ｶﾞ` is `ガ`);
- the function words are 助詞, 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立 and 形容詞,接尾, and the head is the last morpheme that is neither one of them nor a 記号. It is not the schema `ipadic` of this crate, which has more patterns ([Head and Function Word](./head_and_function_word.md)): in 利用 / し / た the head is し here, and 利用 there;
- the punctuation is `。` and `、`, and the brackets are those of CaboCha, each matched as the whole surface.

## Agreement with CaboCha

`scripts/dependency/cabocha_models.sh` converts the model for IPA and gives the same input to CaboCha 0.69 (96e100a, built by `scripts/dependency/build_baselines.sh`, with the model as its own build compiled it) and to `lindera-dependency parse` with the converted model. The input is the 9,066 sentences of UD Japanese-GSD and UD Japanese-PUD, as a fixed text: their answers are not used.

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

## Limits

- **Speed.** `parse` takes 3.03 s for the 7,024 sentences of GSD train from their morphemes, and CaboCha 0.92 s. A converted model keeps each pair with a weight of its own, where the models of this crate hash the pairs into a table: see [Model File](./model_file.md) for their speed.
- **The score of an arc to the last bunsetsu** is 0, as with every model of this crate.
- **A long bunsetsu.** CaboCha keeps at most 128 features and 2,048 bytes of feature strings for a bunsetsu, and this crate all of them. The 76,659 bunsetsu of the text above have at most 40 features and 882 bytes, so the limit is not met there; a bunsetsu beyond it would differ.
- **The chunker** is asked for the best path that starts with `B`; CaboCha takes the best path and then makes the first morpheme start a bunsetsu. CRF++ adds the weights of a morpheme in a 32-bit float, and this crate in 64 bits. Neither changed a bunsetsu of the text above.
- **The accuracy** of CaboCha's models is not measured here: the Kyoto corpus is not available, and an accuracy on UD Japanese-GSD would compare two standards of bunsetsu.
- **Not verified**: CaboCha built with another compiler or system than GCC 15.2 on Linux (x86_64).
