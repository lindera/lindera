# lindera-dependency

Japanese bunsetsu dependency analysis for [Lindera](https://github.com/lindera/lindera).

A dependency analysis groups the morphemes of a sentence into bunsetsu (文節)
and finds, for each bunsetsu, the bunsetsu it depends on.

The crate is on crates.io from Lindera 7.1.0 on, and each release of
Lindera publishes a model for IPADIC and one for each of the three UniDic
dictionaries with it. It holds:

- the `Morpheme` trait: a surface and the feature columns of its dictionary;
  the crate depends on no tokenizer
- the `Sentence` and `Bunsetsu` types
- the reading and writing of the lattice format of CaboCha (`cabocha -f1`)
- a check that a sentence is the tree the parsers assume
- the `Schema` of a dictionary: which feature column holds what, which
  morphemes are function words and symbols, and from them the head and
  the function word of a bunsetsu
- the chunker, which segments the morphemes of a sentence into bunsetsu
- the model file of a dictionary, which is read in place
- the parser, which finds the head of each bunsetsu, with its features
  and the training of its classifier
- the `lindera-dependency` command, which parses plain text and writes
  the lattice format

The token filter `japanese_dependency` of Lindera runs this analysis in a
tokenizer. It is in `lindera-analysis`, behind its `dependency` feature.

A release of Lindera publishes the model for IPADIC, from 7.1.0 on, on
the page of its version: `lindera-dependency-ipadic-<version>.zip`. It is
built by
[`scripts/dependency/build_model.sh`](../scripts/dependency/build_model.sh),
from the train file of UD Japanese-GSD, and is under the MIT license. The
feature `embed-model-ipadic` embeds it in a program. The same is published
for `unidic`, `unidic-cwj` and `unidic-csj`, with the name of the dictionary
in place of `ipadic`: `lindera-dependency-unidic-cwj-<version>.zip` and
`embed-model-unidic-cwj`, for UniDic CWJ. The chunker and the parser can
also be trained with the commands of the crate, in a few seconds each.

The documentation has a chapter on the crate, in
[English](../docs/src/lindera-dependency.md) and in
[Japanese](../docs/ja/src/lindera-dependency.md).
Why the crate has the shape it has, and the numbers behind it, are in the
design notes
([English](../docs/src/development/dependency_design.md),
[Japanese](../docs/ja/src/development/dependency_design.md)). The scripts
that give the numbers are in
[`scripts/dependency`](../scripts/dependency/README.md).

In its default build it has no dependency, and builds for
`wasm32-unknown-unknown`.

## Command line

With the `cli` feature the crate builds the `lindera-dependency` command,
which has no training code. `parse` reads plain text, one sentence per
line, tokenizes it with a Lindera dictionary, and writes the bunsetsu and
their dependencies in the lattice format, as `cabocha -f1` does. It needs a
model file with a chunker and a parser, trained [as below](#training-the-parser).

```shell
cargo build --release -p lindera-dependency --features cli,embed-ipadic
echo "彼は赤い花を見た。" | target/release/lindera-dependency parse \
    --model ipadic.model --dict embedded://ipadic
```

```text
* 0 3D 0/1 0.000000
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/0 0.704616
赤い	形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ
* 2 3D 0/1 0.000000
花	名詞,一般,*,*,*,*,花,ハナ,ハナ
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 3 -1D 0/1 0.000000
見	動詞,自立,*,*,一段,連用形,見る,ミ,ミ
た	助動詞,*,*,*,特殊・タ,基本形,た,タ,タ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

The dictionary that is loaded has to be the one the model names. With
`--input-layer morphemes` or `--input-layer bunsetsu` the input is in the
lattice format, tokenized or segmented already. An empty line gives `EOS`
alone. The chapter of the documentation has the options, and what is kept
of CaboCha's. On the text of the test file of UD Japanese-GSD the command
writes about 6,600 sentences per second
(`scripts/dependency/parse_speed.sh`).

## Training data

The parser is trained on bunsetsu dependencies, on the morphemes of the
dictionary it runs with. The `lindera-dependency` command, behind the `train`
feature, makes such data from a corpus that has word dependencies and
bunsetsu boundaries: a CoNLL-U file with `BunsetuBILabel` in its last column,
as UD Japanese-GSD and UD Japanese-PUD have it.

```shell
cargo run --release -p lindera-dependency --features embed-ipadic -- convert \
    --dict embedded://ipadic \
    --input ja_gsd-ud-train.conllu \
    --output train.lattice
```

`--dict` also takes the directory of a dictionary built for Lindera; the
`train` feature alone is then enough. The schema is the one of the
dictionary, by the name in its metadata: there are schemas for `ipadic`,
`unidic`, `unidic-cwj` and `unidic-csj`, and another dictionary is refused.
The commands that train read no dictionary: `train-chunker`, `train-parser`
and `instances` take its name with `--dictionary-name`, and `ipadic` when
it is left out. For each sentence, the command:

1. turns the word dependencies into bunsetsu dependencies. Coordination is
   put back in the form of the Kyoto corpus and CaboCha: each conjunct
   depends on the next one, with an arc of type `P`;
2. tokenizes the text of the sentence with the dictionary, with the default
   settings of Lindera, and lays the bunsetsu over the morphemes. Where a
   bunsetsu boundary falls inside a morpheme, the bunsetsu are merged up to
   the next boundary that fits, and the merged bunsetsu takes the head of
   the last one, as `tools/kc2cabocha.pl` of CaboCha does;
3. leaves the sentence out unless it has one root that is its last bunsetsu,
   every head to the right, and no crossing arcs.

The output is in the lattice format, with bunsetsu lines that hold the head
and the arc type only: the positions of the head word and of the function
word are not written, since what reads the data computes them from its
schema. A report goes to the standard error. For the train file of UD
Japanese-GSD with IPADIC:

```text
ja_gsd-ud-train.conllu
sentences kept: 7024 of 7050 (99.63%)
sentences dropped: 26
  bunsetsu that cannot be made: 0
  root: 0
  arc to the left: 3
  crossing arcs: 24
  text that does not match: 0
bunsetsu with more than one word that leaves it: 0
arcs of type P: 517 of 49764 (1.04%)
bunsetsu boundaries on a morpheme boundary: 49372 of 49764 (99.21%)
sentences with merged bunsetsu: 372 of 7024 (5.30%)
bunsetsu merged: 392 of 56788 (0.69%)
bunsetsu written: 56396
arcs of type P written: 489 of 49372 (0.99%)
morphemes written: 162456
head words on the content head of the corpus: 54449 of 56396 (96.55%)
  275: 名詞-一般 is the head word, to the right of 名詞-一般
  240: 名詞-接尾-一般 is the head word, to the right of 名詞-一般
  90: 名詞-接尾-助数詞 is the head word, to the right of 名詞-数
  76: 名詞-一般 is the head word, to the right of 接頭詞-名詞接続
  51: 動詞-自立 is the head word, to the right of 名詞-一般
  46: 動詞-自立 is the head word, to the right of 動詞-自立
  46: 名詞-数 is the head word, to the left of 助動詞
  36: 名詞-一般 is the head word, to the left of 助詞-格助詞-連語
  35: 名詞-一般 is the head word, to the right of 名詞-固有名詞-人名-名
  34: 名詞-接尾-助数詞 is the head word, to the left of 名詞-非自立-副詞可能
```

A sentence dropped for two causes counts for both. The arcs of type `P` are
counted before the bunsetsu are laid over the morphemes, and again in what
is written: a merged bunsetsu takes the arc of the last one merged. The
last lines compare the head word of the schema of the dictionary with the
content
head of the corpus; see [Head and function word](#head-and-function-word).

### Corpora

The corpora are not part of this repository. The numbers here were taken at
these commits, with the IPADIC that Lindera embeds (`embed-ipadic`):

| Corpus | Commit | File | Sentences kept | Bunsetsu | Morphemes |
| --- | --- | --- | --- | --- | --- |
| [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) | c662dc9 | train | 7,024 of 7,050 | 56,396 | 162,456 |
| | | dev | 506 of 507 | 4,147 | 11,934 |
| | | test | 542 of 543 | 4,509 | 12,591 |
| [UD Japanese-PUD](https://github.com/UniversalDependencies/UD_Japanese-PUD) | cf69c5c | test | 994 of 1,000 | 9,843 | 27,728 |

The sentences kept are the same with every dictionary. The bunsetsu and the
morphemes are not: UD Japanese-GSD is segmented in the short units of
UniDic, so with a UniDic dictionary fewer bunsetsu boundaries fall inside a
morpheme, and fewer bunsetsu are merged. With the dictionaries of the
release 7.0.0 of Lindera:

| Dictionary | File | Bunsetsu boundaries on a morpheme boundary | Bunsetsu merged | Bunsetsu | Morphemes |
| --- | --- | --- | --- | --- | --- |
| IPADIC | GSD train | 99.21% (49,372 / 49,764) | 392 | 56,396 | 162,456 |
| | GSD dev | 99.16% (3,641 / 3,672) | 31 | 4,147 | 11,934 |
| | GSD test | 99.10% (3,967 / 4,003) | 36 | 4,509 | 12,591 |
| | PUD | 99.45% (8,849 / 8,898) | 49 | 9,843 | 27,728 |
| `unidic` | GSD train | 99.87% (49,700 / 49,764) | 64 | 56,724 | 172,976 |
| | GSD dev | 99.89% (3,668 / 3,672) | 4 | 4,174 | 12,709 |
| | GSD test | 99.80% (3,995 / 4,003) | 8 | 4,537 | 13,294 |
| | PUD | 99.98% (8,896 / 8,898) | 2 | 9,890 | 29,571 |
| `unidic-cwj` | GSD train | 99.95% (49,739 / 49,764) | 25 | 56,763 | 172,196 |
| | GSD dev | 99.95% (3,670 / 3,672) | 2 | 4,176 | 12,657 |
| | GSD test | 99.98% (4,002 / 4,003) | 1 | 4,544 | 13,254 |
| | PUD | 99.97% (8,895 / 8,898) | 3 | 9,889 | 29,413 |
| `unidic-csj` | GSD train | 99.94% (49,735 / 49,764) | 29 | 56,759 | 173,184 |
| | GSD dev | 99.95% (3,670 / 3,672) | 2 | 4,176 | 12,723 |
| | GSD test | 99.95% (4,001 / 4,003) | 2 | 4,543 | 13,299 |
| | PUD | 99.97% (8,895 / 8,898) | 3 | 9,889 | 29,545 |

The three UniDic dictionaries do not give the same morphemes: on the
sentences of GSD train, `unidic` and `unidic-cwj` segment 85.88% alike and
tag 66.06% alike, `unidic-cwj` and `unidic-csj` 88.10% and 73.22%. Training
data is made for one dictionary, and so is a model.

### Licenses of the corpora

UD Japanese-GSD is licensed under CC BY-SA 4.0 and UD Japanese-PUD under
CC BY-SA 3.0 (`LICENSE.txt` in each repository). This is how this project
handles them; it is not legal advice.

- The corpora, and data converted from them, are not committed to this
  repository.
- Converted data is adapted from the annotations of the corpus. If you share
  it, share it under the license of the corpus, name the corpus and say that
  the data was converted.
- A model trained on converted data is released with the corpus, its version
  and its license stated. Whether the share-alike condition extends to a
  trained model is not settled by a court. This project takes its model,
  which holds weights and none of the annotation, not to be an adaptation
  of the corpus, and distributes it under the MIT license: see
  [License of the model](../docs/src/lindera-dependency/model_file.md#license-of-the-model).

## Chunking

The chunker segments the morphemes of a sentence into bunsetsu. It decides,
for every morpheme, whether it starts a bunsetsu (`B`) or continues one
(`I`), with a conditional random field over the two labels, as CaboCha does.

```rust
use lindera_dependency::model::Model;

let model = Model::new(std::fs::read("chunker.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");
// The ranges of the bunsetsu, as indices into the morphemes.
let bunsetsu = chunker.chunk(&morphemes)?;
```

Chunking is in the default build and needs no other crate: the model is a
table from feature string to the weights of `B` and `I`, with the weights of
the transitions, and the labels are found by Viterbi. `Chunker::chunk` takes
`&self`, and a chunker is `Send + Sync`. The first morpheme always starts a
bunsetsu, and an empty sentence has none. A morpheme with fewer feature
columns than the schema of the model reads is refused with an error: it is
not of the dictionary of the model.

The feature strings are made in two steps that can each be replaced. The
columns of a morpheme (its surface, its part of speech, its conjugation
form, ...) are read through the `Schema` of the dictionary, which says which
feature column holds what; `Schema::ipadic()` is the one of IPADIC and
`Schema::unidic()` the one of UniDic. A
template in the format of CRF++ then expands them. A model file holds the
schema, the columns and the template the chunker was trained with; the
schema is stored whole, with its patterns for the head and the function
word.

### Training

Training is behind the `train` feature and uses `lindera-crf`:

```shell
cargo run --release -p lindera-dependency --features train -- train-chunker \
    --input train.lattice --output chunker.model
cargo run --release -p lindera-dependency --features train -- chunk \
    --model chunker.model --input dev.lattice --output output.lattice
```

`chunk` segments the morphemes of its input again, for `evaluate` to compare
with the answer. On UD Japanese-GSD converted as above with IPADIC,
`train-chunker` without an option takes a few seconds and writes:

```text
ja_gsd-ud-train.lattice: 7024 sentences
iterations: 126
feature strings: 4325
weights: 8656
bytes: 261851
```

| | F | Precision | Recall | Sentences with every bunsetsu right |
| --- | --- | --- | --- | --- |
| UD Japanese-GSD dev | 96.85 | 97.02% (4,009 / 4,132) | 96.67% (4,009 / 4,147) | 432 / 506 |
| UD Japanese-GSD test | 96.92 | 96.85% (4,373 / 4,515) | 96.98% (4,373 / 4,509) | 459 / 542 |
| UD Japanese-PUD | 98.04 | 98.05% (9,649 / 9,841) | 98.03% (9,649 / 9,843) | 885 / 994 |

The output is a model file, described in the next section.

### The model file

A model file holds what one dictionary needs to be analyzed: the name of the
dictionary, its schema, and the trained parts: the chunker and the parser.
`Model` reads it in place, from whatever holds the bytes, and copies none of its
tables:

```rust
use lindera_dependency::model::Model;

// Read from a file: a `Model<Vec<u8>>`.
let model = Model::new(std::fs::read("chunker.model")?)?;
// Embedded in the program: a `Model<&'static [u8]>`.
let model = Model::new(include_bytes!("chunker.model").as_slice())?;

assert_eq!(model.dictionary(), "ipadic");
// For a caller that knows the dictionary of its morphemes.
model.check_dictionary("ipadic")?;
```

Any type that implements `AsRef<[u8]>` holds the bytes, a memory map of the
caller as well; the crate itself maps nothing and has no dependency. The
file is checked once, in `Model::new`: bytes that are not a model file of
this version are an error, and nothing fails or panics afterwards. No number
is read through a pointer, so the bytes can sit at any address, and the same
code runs on `wasm32-unknown-unknown`.

A model belongs to one dictionary, and used on the morphemes of another it
gives wrong answers without an error. `Model::check_dictionary` compares the
name of the dictionary, and a morpheme with fewer feature columns than the
schema reads is refused when it is analyzed. Morphemes of another dictionary
that are wide enough are told apart by the name only.

The format is version 1: a magic number (`LDMODEL`), the version, the name of
the dictionary, the schema, a list of the parts, and the parts. A table of
strings is stored sorted, with a hash table over it, and is looked up where
it is. The
[documentation](../docs/src/lindera-dependency/model_file.md) has every
field. `ModelBuilder` writes a file. The chunker-only bytes that
`train-chunker` wrote before the model file (magic number `LDCHUNK`) are no
longer read: train the model again.

### How the defaults were chosen

The defaults are three columns (the surface, the part of speech as its
levels joined by `-`, the conjugation form), the template of CaboCha's
chunker with five lines for the conjugation form, and L1 regularization with
the coefficient 0.5. They come from a 10-fold cross-validation on the train
file (7,024 sentences, 56,396 bunsetsu), as the section on the evaluation
asks. F over the 10 folds:

| Columns | L1 0.25 | L1 0.5 | L1 1.0 | L2 0.25 | L2 0.5 | L2 1.0 |
| --- | --- | --- | --- | --- | --- | --- |
| Surface and part of speech (CaboCha) | 96.83 | 96.76 | 96.62 | 96.66 | 96.65 | 96.56 |
| With the conjugation form | 96.98 | 96.94 | 96.86 | 96.82 | 96.79 | 96.73 |
| With the conjugation form, five lines (the default) | 96.95 | 96.93 | 96.82 | 96.79 | 96.77 | 96.70 |
| With the conjugation type | 96.82 | 96.78 | 96.68 | 96.75 | 96.69 | 96.64 |
| With the base form | 96.95 | 96.94 | 96.82 | 96.82 | 96.80 | 96.74 |

A column is added with the lines CaboCha's template for named entities has
for its third column (13 lines), or with five: the column for the morpheme,
the one before and the one after, and the column with the part of speech
for the morpheme and the one before. With L1 and the coefficients 0.1 and
0.15, F is 96.82 and 96.83 for CaboCha's columns and 96.98 and 96.97 with
the conjugation form; with the conjugation form and the base form together
it is 96.99, 97.02 and 97.00 for 0.1, 0.15 and 0.25.

Pairs of settings, by the segmentation of each sentence (`evaluate
--compare`):

| First | Second | First only / second only | p |
| --- | --- | --- | --- |
| CaboCha's columns, L1 0.25 | With the conjugation form, L1 0.25 | 52 / 102 | 0.0001 |
| CaboCha's columns, L1 0.25 | With the conjugation type, L1 0.25 | 65 / 70 | 0.7308 |
| CaboCha's columns, L1 0.25 | With the base form, L1 0.25 | 82 / 121 | 0.0075 |
| With the conjugation form, L1 0.25 | With the base form too, L1 0.25 | 40 / 47 | 0.5203 |
| With the conjugation form, L1 0.25 | The same, L1 0.5 | 49 / 37 | 0.2354 |
| With the conjugation form, L1 0.25 | The same, L1 1.0 | 123 / 80 | 0.0031 |
| With the conjugation form, L1 0.25 | The same, L2 0.25 | 151 / 102 | 0.0025 |
| With the conjugation form, L1 0.25 | The default | 72 / 55 | 0.1554 |
| CaboCha's columns, L1 0.25 | The default | 74 / 107 | 0.0171 |
| The default | The same, L1 1.0 | 91 / 52 | 0.0014 |

- The conjugation form helps, and the conjugation type does not. The base
  form helps too, and adds nothing to the conjugation form.
- L1 is better than L2, with a model fifty times smaller.
- The coefficients 0.1 to 0.5 do not differ; 1.0 is worse.

Among the settings that do not differ from the best one, the default is the
smallest: 8,656 weights, against 10,934 for the 13 lines and the coefficient
0.25. The dev file does not tell them apart (F from 96.78 to 96.85).

### The settings for UniDic

For `unidic`, `unidic-cwj` and `unidic-csj` the chunker is trained with four
columns (the surface, the part of speech, the conjugation form, the base
form), the template `Template::unidic()` and L1 regularization with the
coefficient 0.25. `train-chunker --dictionary-name <name>` takes them, and
`TrainOptions::for_dictionary` gives them. They come from the same
cross-validation, run again with UniDic CWJ (7,024 sentences, 56,763
bunsetsu). F over the 10 folds:

| Columns | L1 0.25 | L1 0.5 | L1 1.0 | L2 0.25 | L2 0.5 | L2 1.0 |
| --- | --- | --- | --- | --- | --- | --- |
| Surface and part of speech (CaboCha) | 97.06 | 96.97 | 96.81 | 96.93 | 96.90 | 96.84 |
| With the conjugation form | 97.11 | 97.03 | 96.90 | 97.03 | 96.98 | 96.92 |
| With the conjugation form, five lines (the settings for IPADIC with L1 0.5) | 97.13 | 97.02 | 96.87 | 97.00 | 96.95 | 96.89 |
| With the conjugation type | 97.09 | 97.01 | 96.82 | 96.99 | 96.95 | 96.91 |
| With the base form | 97.25 | 97.17 | 97.09 | 97.21 | 97.19 | 97.16 |
| With the base form, five lines | 97.12 | 97.05 | 96.95 | 97.11 | 97.09 | 97.05 |

With L1 and the coefficients 0.1 and 0.15, F is 97.08 and 97.05 for
CaboCha's columns, 97.15 and 97.15 with the conjugation form, 97.29 and 97.24
with the base form; with the conjugation form and the base form together it
is 97.22, 97.22 and 97.22 for 0.1, 0.15 and 0.25.

| First | Second | First only / second only | p |
| --- | --- | --- | --- |
| CaboCha's columns, L1 0.25 | With the base form, L1 0.25 | 81 / 149 | < 0.0001 |
| With the conjugation form, L1 0.25 | With the base form too, L1 0.25 | 41 / 80 | 0.0005 |
| The settings for IPADIC | With the base form, L1 0.25 | 107 / 185 | < 0.0001 |
| The settings for IPADIC | The settings for UniDic | 59 / 131 | < 0.0001 |
| With the base form, L1 0.25 | The settings for UniDic | 128 / 122 | 0.7519 |
| With the base form, L1 0.25 | The same, L1 0.1 | 25 / 33 | 0.3581 |
| With the base form, L1 0.25 | The same, L1 0.5 | 53 / 25 | 0.0020 |
| With the base form, L1 0.25 | The same, L2 0.25 | 118 / 112 | 0.7417 |
| With the base form, L1 0.25 | The same, five lines | 121 / 79 | 0.0036 |

- The base form helps, more than the conjugation form: the base form of
  UniDic is the lexeme (語彙素), the same for the ways a word is written.
  With IPADIC it added nothing to the conjugation form.
- The coefficient 0.25 is better than 0.5, and does not differ from 0.1.
  L1 and L2 do not differ, and L1 gives the smaller model.
- The base form with the 13 lines and the two columns together do not
  differ. The settings are the smaller of the two: 11,823 weights per fold
  against 18,174.

The settings hold with the two other dictionaries (`chunker_experiments.sh
--dictionary <name> check`); F with the settings for IPADIC and with those
for UniDic:

| Dictionary | The settings for IPADIC | The settings for UniDic | First only / second only | p |
| --- | --- | --- | --- | --- |
| `unidic` | 97.02 | 97.15 | 75 / 122 | 0.0010 |
| `unidic-cwj` | 97.02 | 97.22 | 59 / 131 | < 0.0001 |
| `unidic-csj` | 96.94 | 97.08 | 74 / 131 | 0.0001 |

The files that are not trained on do not tell the two settings apart; F
with the settings for IPADIC and with those for UniDic, and McNemar's test
over the sentences:

| Dictionary | UD Japanese-GSD dev | UD Japanese-GSD test | UD Japanese-PUD |
| --- | --- | --- | --- |
| `unidic` | 96.89 and 97.03 (p 0.09) | 97.08 and 97.24 (p 0.82) | 98.02 and 98.00 (p 1.00) |
| `unidic-cwj` | 96.63 and 96.63 (p 1.00) | 96.85 and 97.02 (p 0.50) | 97.96 and 97.89 (p 0.70) |
| `unidic-csj` | 96.60 and 96.60 (p 0.69) | 96.79 and 96.95 (p 0.48) | 97.94 and 97.78 (p 0.34) |

UD Japanese-PUD is a little lower with the settings for UniDic. The choice
follows the cross-validation, as the one for IPADIC does.

## Head and function word

Each bunsetsu has a head (主辞), its last content word, and a function word
(機能語), its last particle or auxiliary. The parser builds most of its
features from the two, the lattice format has their positions on the `*`
line, and the head is the word to take for a bunsetsu when indexing pairs
or extracting triples.

Which morphemes they are depends on the dictionary, and so does the column
that holds the part of speech or the base form. A `Schema` holds all of it
as data, and one piece of code applies it:

```rust
use lindera_dependency::Schema;

let schema = Schema::ipadic();
// The positions of the head word and of the function word, counted from
// the first morpheme of the bunsetsu: (0, 2) for 読ん / で / いる / 。
let positions = schema.positions(&morphemes[bunsetsu.clone()]);
// Punctuation and brackets, for the features of the parser.
let ends_with_comma = morphemes.last().is_some_and(|last| schema.is_comma(last));
```

The rule is the one of `findHead` of CaboCha. Both positions start at the
first morpheme of the bunsetsu. From the left, a morpheme that matches a
function word pattern becomes the function word; a morpheme that matches
none of them, is not a symbol and is not in the list of what is not a head
becomes the head. A head after the function word becomes the function word
too. A bunsetsu of symbols only has both at its first morpheme.

A pattern is a prefix of the part-of-speech levels, optionally with the base
forms the morpheme may have and with a pattern for the morpheme before it in
the bunsetsu. A symbol is recognized by its part of speech or by its
surface: IPADIC has no half-width symbols, so `,` is an unknown word and
comes out as 名詞,サ変接続. Commas (`、` `,` `，`), periods (`。` `.` `．`),
opening brackets and closing brackets are kinds of their own
(`Schema::is_comma` and the like), in full width and half width. A morpheme
with fewer feature columns than the schema names matches no pattern on the
missing ones; nothing panics.

### The schema of IPADIC

`Schema::ipadic()` starts from the patterns of CaboCha for IPA and adds what
the measurement below supports:

| | Patterns |
| --- | --- |
| Function words | 助詞, 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立, 形容詞,接尾 (CaboCha); する, できる and 出来る (動詞,自立) |
| Particles | 助詞 (CaboCha) |
| Function words taken by their surface as a child | 助詞, 副詞, 連体詞, 接続詞 (CaboCha) |
| Not heads | 記号 (CaboCha) and symbols by their surface; 名詞,非自立 (こと, の, よう); the suffixes 的 (名詞,接尾,形容動詞語幹) and さ (名詞,接尾,特殊); a 動詞,自立 right after a 助詞 (こと / に / なっ / た, かも / しれ / ない) |

The head is CaboCha's, the content word, and not J.DepP's, which lets an
auxiliary be the head. The function word is defined by the schema alone: UD
Japanese-GSD has no label that means the same.

### The schema of UniDic

`Schema::unidic()` is the schema of the three UniDic dictionaries of
Lindera, `unidic` (unidic-mecab 2.1.2), `unidic-cwj` and `unidic-csj`: the
columns it reads are at the same places in the three. The base form is the
語彙素, in the eighth column, so the base form of こと is 事.
`Schema::for_dictionary` gives the schema of a dictionary by its name.

| | Patterns |
| --- | --- |
| Function words | 助詞, 助動詞, 接尾辞,形容詞的, 接尾辞,形状詞的, 接尾辞,動詞的 (CaboCha); 動詞,非自立可能 and 形容詞,非自立可能 |
| Particles | 助詞 |
| Function words taken by their surface as a child | 助詞, 副詞, 連体詞, 接続詞 |
| Not heads | 空白, 補助記号 and 記号; a 動詞 right after に, と or を (事故 / に / よっ / て); 助動詞語幹 (よう, そう, みたい); a formal noun (こと, ため, もの, 際, うえ, ところ) right after a 動詞, a 助動詞 or a 形容詞; a nominal suffix (接尾辞,名詞的) right after a 形容詞 (大き / さ) |

### Agreement with UD Japanese-GSD

UD Japanese-GSD marks the content head of each bunsetsu
(`BunsetuPositionType=SEM_HEAD`, or `ROOT` in the bunsetsu of the root).
`convert` counts the bunsetsu whose head word is the morpheme that holds the
first character of that word, and its report ends with the count and the
most frequent disagreements, by part of speech. For the corpora converted
as above with IPADIC, each line is the one before with one addition; the
last line is `Schema::ipadic()`:

| Rule | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| The rule of CaboCha for IPA | 85.43% | 86.30% | 84.61% | 86.70% |
| And symbols by their surface | 87.19% | 88.14% | 87.40% | 86.70% |
| And する, できる are function words | 94.13% | 94.43% | 93.52% | 94.27% |
| And 名詞,非自立 is not a head | 95.31% | 95.66% | 95.48% | 95.61% |
| And the suffix 的 is not a head | 95.81% | 96.24% | 96.01% | 96.51% |
| And the suffix さ is not a head | 95.89% | 96.43% | 96.32% | 96.65% |
| And a verb after a particle is not a head | 96.55% (54,449 / 56,396) | 96.96% (4,021 / 4,147) | 96.92% (4,370 / 4,509) | 97.46% (9,593 / 9,843) |

The additions were chosen on the train file. The largest is the verbal noun
with する (利用 / し / た): CaboCha's rule makes し the head, the corpus the
noun. `%` and `°` are not symbols: they are units, which the corpus has as
the head (50 / %).

What remains is not a matter of one part of speech. Of the 1,947
disagreements in train, the most frequent kinds are a compound noun whose
head in the corpus is not its last noun, often because IPADIC splits a word
the corpus has as one (霊 / 芝: 275 with 名詞,一般 for both; 本 / 作: 240
with 名詞,接尾,一般 as the head word after 名詞,一般), a number with a
counter where the corpus has the number (2 / 人: 90), and a prefix the
corpus has as the head (同 / 法: 76).

Two variants were measured on the same data, with a script outside the
repository, and not taken. Leaving every suffix (名詞,接尾) out of the heads
lowers the agreement by about 9 points: the corpus has a suffix such as 化
or 性 as the head. Keeping 名詞,非自立,副詞可能 (後, 間) as a head changes
train by +0.05 points, dev by +0.17 and test by -0.09, which is not a
systematic difference.

For UniDic the same measurement, with UniDic CWJ; the last line is
`Schema::unidic()`:

| Rule | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| The rule of CaboCha for UniDic | 82.50% | 82.81% | 83.96% | 81.75% |
| And 非自立可能 is a function word | 94.73% | 94.64% | 94.70% | 94.47% |
| And a verb after に, と or を is not a head | 96.71% | 96.53% | 96.43% | 96.73% |
| And 助動詞語幹 is not a head | 97.19% | 97.05% | 97.10% | 97.15% |
| And a formal noun after a predicate is not a head | 97.63% | 97.49% | 97.69% | 98.04% |
| And a nominal suffix after an adjective is not a head | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |

The same schema with each of the three dictionaries:

| Dictionary | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| `unidic` | 97.52% (55,320 / 56,724) | 97.60% (4,074 / 4,174) | 97.86% (4,440 / 4,537) | 97.84% (9,676 / 9,890) |
| `unidic-cwj` | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |
| `unidic-csj` | 97.55% (55,366 / 56,759) | 97.49% (4,071 / 4,176) | 97.91% (4,448 / 4,543) | 97.91% (9,682 / 9,889) |

非自立可能 is the largest step: UniDic gives it to する, いる, ある and なる,
which the pattern of CaboCha takes as head words. Three changes were
measured and not taken (`head_variants/`, GSD train). A nominal suffix that
is never a head gives 91.14% (51,736 / 56,763), and one that is not a head
after a number 96.75% (54,917 / 56,763): the corpus has such a suffix as
the head. A letter of the Latin alphabet, which UniDic tags 記号,文字, taken
out of the symbols gives 97.50% (55,346 / 56,763).

One disagreement is not expressed by the schema. The corpus has a percent
sign after a number as the head (3 / %, 65 bunsetsu of GSD train); UniDic
tags it 補助記号,一般, as it tags ・, and a schema takes a symbol by its part
of speech or by its characters, not by the two together. Most of what is
left comes from words of the corpus that the dictionary splits (a number in
digits, a name in katakana): the content head is taken at the morpheme that
holds its first character, and the head word is the last one.

## Parsing

The parser finds the head of each bunsetsu. It goes through the bunsetsu
from left to right with a stack (shift-reduce), and asks one question at a
time: does this bunsetsu, the source, depend on that one, the target? This
is what CaboCha (`parseShiftReduce`) and J.DepP (`_parseLinear`) do. Every
head is to the right of its bunsetsu, no two arcs cross, and the last
bunsetsu is the root; the number of decisions is under twice the number of
bunsetsu.

With a model file that has both parts, a sentence goes from morphemes to
dependencies:

```rust
use lindera_dependency::model::Model;

let model = Model::new(std::fs::read("ipadic.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");
let parser = model.parser().expect("the file has a parser");
// The bunsetsu with their heads, the positions of the head word and of
// the function word, and the scores.
let bunsetsu = parser.parse(&morphemes, &chunker.chunk(&morphemes)?)?;
```

`Model::analyze` does the two steps in one call. A parser can also be made
from its parts, here with a classifier made by hand:

```rust
use std::collections::HashMap;

use lindera_dependency::parser::{FeatureSet, LinearClassifier, Parser};
use lindera_dependency::Schema;

// One weight: a bunsetsu that ends in を depends on the next one.
let pairs = HashMap::from([(("SFS:を".to_string(), "DIST:1".to_string()), 2.0)]);
let classifier = LinearClassifier::new(-1.0, HashMap::new(), pairs)?;
let parser = Parser::new(Schema::ipadic(), FeatureSet::cabocha(), classifier);
// The bunsetsu with their heads, their head words and function words, and
// the scores; `ranges` is what `Chunker::chunk` gives.
let bunsetsu = parser.parse(&morphemes, &ranges)?;
```

Parsing is in the default build and needs no other crate. It is in three
parts:

- **The features of a decision** are strings, with the names CaboCha gives
  them, so that a model of CaboCha can be converted without renaming them.
  They are made through the `Schema`: nothing in the parser names a part of
  speech or a column.
- **The classifier** (`Classifier`) gives a number to each feature string
  it knows, once for each bunsetsu of a sentence, and scores the numbers of
  a decision; the source depends on the target when the score is above 0.
  `LinearClassifier` adds a bias, the weight of each feature and the weight
  of each pair of features, which is what the polynomial kernel of degree 2
  of CaboCha and of J.DepP comes to once it is expanded.
- **The loop** asks the decisions. With a classifier it parses; with the
  answer of a sentence it gives the training instances of a classifier.

### The features

| Group | Features |
| --- | --- |
| `distance` | The number of bunsetsu from the source to the target: `DIST:1`, `DIST:2-5`, `DIST:6-` |
| `source`, `target` | The bunsetsu, with `S` or `D` in front: the surface, the levels of the part of speech up to the first `*` and the conjugation form of its head (`SHS:彼`, `SHP0:名詞`), its function word (`SFS:は`), its first and its last morpheme (`SLS:`, `SRS:`); its punctuation (`SPUNC:、`) and its brackets (`SOB:「`, `SOB:1`, `SCB:」`, `SCB:1`); each of its particles (`SCASE:は`); first or last bunsetsu of the sentence (`SBOS:1`, `SEOS:1`) |
| `left` | The surface of the function word of the bunsetsu before the source (`LF:`) |
| `right` | The surfaces of the first morpheme, the head and the function word of the bunsetsu after the target (`RL:`, `RH:`, `RF:`) |
| `children` | Each bunsetsu already attached to the source (`a:`) and to the target (`A:`), by its function word: its surface for a word of `Schema::lexicalized`, or else its conjugation form, or else its part of speech |
| `gap` | The bunsetsu in between: their punctuation (`GPUNC:、`), their brackets (`GOB:「`, `GCB:」`), the function word of each when it is a particle (`GCASE:を`); and whether brackets open or close in between (`GNB:1`, `GOB:1`, `GCB:1`, `GBB:1`) |

The features of a decision are sorted, and each is there once. The
`children` features are the only ones that depend on the decisions made
before.

One group is not CaboCha's: `after`, the morpheme after the head of the
source and of the target (`SNS:し`, `SNP0:動詞`, `DNS:こと`). CaboCha's head
is the last word of a bunsetsu that is not a function word; the head of the
schema is the content word, which can be the word before it (利用 in
利用 / し / た), and what follows the head says what the bunsetsu is as a
whole.

Which groups a model uses is data: a `FeatureSet`, written as
`distance,source,target,left,right,children,gap,after`, which is the
default. `FeatureSet::cabocha()` is the same without `after`.

Where the features differ from CaboCha's:

- The surface is not normalized. Punctuation and brackets are those of the
  schema, which recognizes them by part of speech and by surface, in both
  widths.
- The head and the function word are those of `Schema::positions`, which
  has more patterns for IPADIC than CaboCha (see above).
- A bunsetsu whose head is not known (CaboCha's partial training) is not
  supported.
- An arc to the last bunsetsu is not asked, and has the score 0. CaboCha
  writes the score of the decision before.

J.DepP has features CaboCha does not have: whether the heads of two bunsetsu
have the same surface, and the balance of brackets instead of their
presence. They are not here: the targets were met without them.

### Training instances

`parser::instances` runs the loop on a sentence whose heads are known, with
the answer deciding, as CaboCha does in its training mode. Each decision is
one instance: its features, and whether the source depends on the target.
The `children` features are those of the answer so far. A sentence whose
tree the parser cannot make is an error.

The `instances` command writes them for a file in the lattice format, with
the schema of IPADIC, or of the dictionary that `--dictionary-name` names:

```shell
cargo run --release -p lindera-dependency --features train -- instances \
    --input train.lattice --output train.instances
```

Each line is `+1` or `-1` and the feature strings of one instance, separated
by tabs; an empty line ends each sentence. `--features` takes a feature set.
The command stops at a sentence whose tree does not come back when its
answer decides. On the training data made as above, every tree comes back:

| File | Sentences | Bunsetsu | Instances | Positive | Negative |
| --- | --- | --- | --- | --- | --- |
| UD Japanese-GSD train | 7,024 | 56,396 | 66,000 | 33,512 | 32,488 |
| UD Japanese-GSD dev | 506 | 4,147 | 4,881 | 2,474 | 2,407 |
| UD Japanese-GSD test | 542 | 4,509 | 5,377 | 2,707 | 2,670 |
| UD Japanese-PUD | 994 | 9,843 | 12,739 | 6,077 | 6,662 |

`scripts/dependency/parser_instances.sh` gives these numbers.

### Training the parser

Training is behind the `train` feature. `train-parser` trains the parser and
writes a model file; with `--chunker`, the chunker of another model file
goes into it, which makes the whole model of the dictionary. `parse` with
`--input-layer` finds the heads of a file in the lattice format: with
`bunsetsu` on the bunsetsu of the file, with `morphemes` on those of the
chunker of the model, which is the analysis from the morphemes.

```shell
cargo run --release -p lindera-dependency --features train -- train-parser \
    --input train.lattice --chunker chunker.model --output ipadic.model
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer bunsetsu --model ipadic.model --input dev.lattice --output output.lattice
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer morphemes --model ipadic.model --input dev.lattice --output output.lattice
```

On UD Japanese-GSD converted as above with IPADIC, `train-parser` without an
option but `--chunker` takes about three seconds and writes:

```text
train.lattice: 7024 sentences
features: distance,source,target,left,right,children,gap,after
instances: 66000
passes: 19
updates: 21374
feature strings: 62380
pair weights: 2762034
bytes: 6258548
```

A last line, `seconds:`, has the time of the training; it is left out here.
The same data gives the same bytes.
The dependency accuracy counts every bunsetsu but the last of each sentence
(level 1 of `evaluate`):

| | With the bunsetsu of the answer | From the morphemes | Chunking F |
| --- | --- | --- | --- |
| UD Japanese-GSD dev | 90.11% (3,281 / 3,641) | 85.39% (3,109 / 3,641) | 96.85 |
| UD Japanese-GSD test | 88.53% (3,512 / 3,967) | 84.47% (3,351 / 3,967) | 96.92 |
| UD Japanese-PUD | 88.81% (7,859 / 8,849) | 86.19% (7,627 / 8,849) | 98.04 |

The targets of the project were 89.0 on dev and 87.8 on test with the
bunsetsu of the answer, and 85.0 and 84.3 from the morphemes: the numbers of
CaboCha and of J.DepP trained on the same data, which are on record and
cannot be measured again with what is in this repository
([#1433](https://github.com/lindera/lindera/issues/1433)).

The algorithms are those of the reference implementations: dual coordinate
descent on a support vector machine, as CaboCha (`--algorithm dcd`, the
default), and passive-aggressive, as J.DepP (`--algorithm pa`). Both learn a
bias, a weight for each feature and a weight for each pair of features. The
weights of the pairs are kept in a table of a fixed size that the pairs are
hashed into (`--pairs hashed`, the default, with `--bits`), or each pair has
a weight of its own (`--pairs exact`). The weights are written as one byte
each (`--quantization i8`, the default), two (`i16`) or four (`f32`).

### How the defaults of the parser were chosen

By a 10-fold cross-validation on the train file, on the bunsetsu of the
answer; `scripts/dependency/parser_experiments.sh` gives every number of
this section. The times are of one fold, with ten folds trained at the same
time on an AMD Ryzen AI Max+ 395; the bytes are of a file with the parser
only. Two settings are compared by the arcs each has right, with McNemar's
exact test.

**The algorithm**, with CaboCha's feature set, a weight for every pair, and
weights of 4 bytes:

| Setting | Level 1 | Sentences with every head right | Seconds | Passes | Features | Pair weights | Bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Dual coordinate descent, cost 0.0005 | 88.11% (43,500 / 49,372) | 3,783 | 5.7 | 12 | 54,688 | 3,135,018 | 26,886,481 |
| Dual coordinate descent, cost 0.001 | 88.79% (43,836 / 49,372) | 3,899 | 6.0 | 15 | 56,725 | 3,297,056 | 28,231,766 |
| Dual coordinate descent, cost 0.0015 | 88.96% (43,922 / 49,372) | 3,923 | 6.3 | 17 | 58,558 | 3,432,038 | 29,355,452 |
| Dual coordinate descent, cost 0.002 | 89.01% (43,946 / 49,372) | 3,914 | 6.7 | 20 | 59,938 | 3,535,001 | 30,212,226 |
| Dual coordinate descent, cost 0.003 | 88.91% (43,899 / 49,372) | 3,906 | 7.3 | 24 | 61,539 | 3,653,850 | 31,201,312 |
| Dual coordinate descent, cost 0.005 | 88.76% (43,823 / 49,372) | 3,863 | 8.0 | 31 | 62,480 | 3,726,424 | 31,804,541 |
| Dual coordinate descent, cost 0.01 | 88.72% (43,801 / 49,372) | 3,853 | 8.6 | 39 | 62,576 | 3,735,836 | 31,882,156 |
| Passive-aggressive, cost 0.0005, 10 passes | 88.45% (43,668 / 49,372) | 3,809 | 5.6 | 10 | 53,582 | 3,104,648 | 26,617,796 |
| Passive-aggressive, cost 0.0005, 10 passes, averaged | 88.82% (43,854 / 49,372) | 3,898 | 5.9 | 10 | 53,582 | 3,147,880 | 26,963,648 |
| Passive-aggressive, cost 0.0005, 30 passes | 88.76% (43,824 / 49,372) | 3,874 | 7.9 | 30 | 55,121 | 3,227,054 | 27,633,847 |
| Passive-aggressive, cost 0.0005, 30 passes, averaged | 88.88% (43,881 / 49,372) | 3,895 | 8.2 | 30 | 55,121 | 3,263,721 | 27,927,186 |
| Passive-aggressive, cost 0.001, 10 passes | 88.28% (43,584 / 49,372) | 3,803 | 5.5 | 10 | 51,403 | 2,925,451 | 25,131,780 |
| Passive-aggressive, cost 0.001, 10 passes, averaged | 88.84% (43,860 / 49,372) | 3,894 | 5.7 | 10 | 51,403 | 2,983,805 | 25,598,612 |
| Passive-aggressive, cost 0.001, 30 passes | 88.70% (43,793 / 49,372) | 3,844 | 7.8 | 30 | 52,380 | 3,004,641 | 25,788,568 |
| Passive-aggressive, cost 0.001, 30 passes, averaged | 88.81% (43,847 / 49,372) | 3,873 | 8.1 | 30 | 52,380 | 3,056,919 | 26,206,791 |
| Passive-aggressive, cost 0.005, 10 passes | 88.16% (43,525 / 49,372) | 3,773 | 5.5 | 10 | 46,052 | 2,513,558 | 21,708,784 |
| Passive-aggressive, cost 0.005, 10 passes, averaged | 88.79% (43,839 / 49,372) | 3,899 | 5.6 | 10 | 46,052 | 2,599,012 | 22,392,420 |
| Passive-aggressive, cost 0.005, 30 passes | 88.33% (43,610 / 49,372) | 3,800 | 7.7 | 30 | 47,006 | 2,588,431 | 22,330,574 |
| Passive-aggressive, cost 0.005, 30 passes, averaged | 88.83% (43,859 / 49,372) | 3,892 | 7.9 | 30 | 47,006 | 2,667,775 | 22,965,330 |

**The feature set**, with dual coordinate descent and the cost 0.0015:

| Setting | Level 1 | Sentences with every head right | Seconds | Passes | Features | Pair weights | Bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CaboCha's set | 88.96% (43,922 / 49,372) | 3,923 | 6.3 | 17 | 58,558 | 3,432,038 | 29,355,452 |
| without `distance` | 88.94% (43,913 / 49,372) | 3,937 | 6.1 | 17 | 58,762 | 3,354,436 | 28,739,586 |
| without `left` | 88.93% (43,908 / 49,372) | 3,931 | 6.0 | 16 | 56,697 | 3,216,520 | 27,588,583 |
| without `right` | 88.53% (43,710 / 49,372) | 3,862 | 4.7 | 19 | 39,771 | 2,135,701 | 18,556,090 |
| without `children` | 88.87% (43,875 / 49,372) | 3,919 | 5.9 | 17 | 57,724 | 3,201,095 | 27,487,690 |
| without `gap` | 88.75% (43,816 / 49,372) | 3,892 | 5.9 | 16 | 58,241 | 3,254,439 | 27,926,979 |
| with `after`: every group | 89.19% (44,036 / 49,372) | 3,971 | 8.8 | 19 | 59,558 | 4,168,732 | 35,272,777 |
| every group but `children` | 89.19% (44,033 / 49,372) | 3,962 | 7.9 | 19 | 58,748 | 3,914,896 | 33,222,359 |

**Which pairs get a weight**, with every group of features:

| Setting | Level 1 | Sentences with every head right | Seconds | Passes | Features | Pair weights | Bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A weight for every pair | 89.19% (44,036 / 49,372) | 3,971 | 8.8 | 19 | 59,558 | 4,168,732 | 35,272,777 |
| Pairs in 2 decisions or more | 89.18% (44,030 / 49,372) | 3,969 | 6.5 | 22 | 56,801 | 2,146,806 | 19,030,939 |
| Pairs in 3 decisions or more | 89.19% (44,033 / 49,372) | 3,983 | 5.4 | 24 | 55,372 | 1,310,821 | 12,308,881 |
| Pairs in 5 decisions or more | 89.19% (44,037 / 49,372) | 3,968 | 4.5 | 27 | 54,191 | 693,778 | 7,344,380 |
| Pairs in 10 decisions or more | 89.09% (43,986 / 49,372) | 3,952 | 4.1 | 29 | 53,171 | 322,354 | 4,348,554 |
| Pairs with a weight of 0.0005 or more | 89.19% (44,036 / 49,372) | 3,968 | 8.7 | 19 | 58,556 | 2,867,684 | 24,840,466 |
| Pairs with a weight of 0.001 or more | 89.17% (44,026 / 49,372) | 3,965 | 8.7 | 19 | 58,556 | 2,262,789 | 20,001,306 |
| Pairs with a weight of 0.002 or more | 89.24% (44,058 / 49,372) | 3,968 | 8.5 | 19 | 58,556 | 1,385,094 | 12,979,746 |
| Features in 2 decisions or more | 89.19% (44,034 / 49,372) | 3,981 | 7.7 | 20 | 46,441 | 3,628,293 | 30,629,600 |
| Features in 3 decisions or more | 89.21% (44,047 / 49,372) | 3,988 | 7.0 | 22 | 32,930 | 2,990,849 | 25,153,259 |
| Features in 5 decisions or more | 89.23% (44,055 / 49,372) | 3,978 | 6.0 | 25 | 19,470 | 2,230,914 | 18,548,709 |

**A table the pairs are hashed into**, with every group of features:

| Setting | Level 1 | Sentences with every head right | Seconds | Passes | Features | Pair weights | Bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Hashed into 2^16 weights | 88.81% (43,849 / 49,372) | 3,907 | 2.5 | 23 | 56,446 | 65,536 | 2,110,085 |
| Hashed into 2^18 weights | 89.14% (44,012 / 49,372) | 3,962 | 2.5 | 20 | 57,762 | 262,144 | 2,928,046 |
| Hashed into 2^20 weights | 89.15% (44,016 / 49,372) | 3,936 | 3.3 | 20 | 58,286 | 1,029,470 | 6,086,670 |
| Hashed into 2^21 weights | 89.19% (44,035 / 49,372) | 3,958 | 4.0 | 19 | 58,433 | 1,812,151 | 10,284,534 |
| Hashed into 2^22 weights | 89.21% (44,045 / 49,372) | 3,974 | 4.4 | 19 | 58,480 | 2,645,711 | 18,674,476 |
| Hashed into 2^23 weights | 89.16% (44,020 / 49,372) | 3,965 | 4.8 | 20 | 58,492 | 3,288,365 | 35,451,774 |
| Hashed into 2^24 weights | 89.22% (44,048 / 49,372) | 3,966 | 5.0 | 19 | 58,500 | 3,691,986 | 69,006,448 |

**The width of a weight**, with the pairs hashed into 2^22 weights:

| Setting | Level 1 | Sentences with every head right | Seconds | Passes | Features | Pair weights | Bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2^22 weights of 4 bytes | 89.21% (44,045 / 49,372) | 3,974 | 4.4 | 19 | 58,480 | 2,645,711 | 18,674,476 |
| 2^22 weights of 2 bytes | 89.21% (44,045 / 49,372) | 3,974 | 4.4 | 19 | 58,480 | 2,645,711 | 10,168,908 |
| 2^22 weights of 1 byte | 89.21% (44,043 / 49,372) | 3,971 | 4.4 | 19 | 58,480 | 2,645,711 | 5,916,123 |

**Pairs of settings**:

| First | Second | Arcs only the first has right | Only the second | p |
| --- | --- | --- | --- | --- |
| Cost 0.0015 (88.96) | cost 0.001 (88.79) | 338 | 252 | 0.0005 |
| Cost 0.0015 (88.96) | cost 0.003 (88.91) | 484 | 461 | 0.4742 |
| Dual coordinate descent (88.96) | passive-aggressive, averaged (88.88) | 559 | 518 | 0.2229 |
| CaboCha's set (88.96) | without `right` (88.53) | 703 | 491 | 0.0000 |
| CaboCha's set (88.96) | without `gap` (88.75) | 347 | 241 | 0.0000 |
| CaboCha's set (88.96) | without `children` (88.87) | 344 | 297 | 0.0692 |
| CaboCha's set (88.96) | with `after` (89.19) | 523 | 637 | 0.0009 |
| Every group (89.19) | every group but `children` (89.19) | 357 | 354 | 0.9402 |
| A weight for every pair (89.19) | pairs in 3 decisions or more (89.19) | 228 | 225 | 0.9251 |
| A weight for every pair (89.19) | features in 3 decisions or more (89.21) | 147 | 158 | 0.5670 |
| A weight for every pair (89.19) | pairs with a weight of 0.002 or more (89.24) | 91 | 113 | 0.1413 |
| A weight for every pair (89.19) | hashed into 2^18 (89.14) | 438 | 414 | 0.4307 |
| A weight for every pair (89.19) | hashed into 2^20 (89.15) | 227 | 207 | 0.3618 |
| A weight for every pair (89.19) | hashed into 2^21 (89.19) | 144 | 143 | 1.0000 |
| A weight for every pair (89.19) | hashed into 2^22 (89.21) | 111 | 120 | 0.5987 |
| Hashed into 2^22 (89.21) | hashed into 2^20 (89.15) | 193 | 164 | 0.1383 |
| Weights of 4 bytes (89.21) | of 2 bytes (89.21) | 0 | 0 | 1.0000 |
| Weights of 4 bytes (89.21) | of 1 byte (89.21) | 18 | 16 | 0.8642 |

What follows from it:

- The two algorithms do not differ, in the same time. Dual coordinate
  descent is the default: it ends by itself, and passive-aggressive needs a
  number of passes to be chosen. The cost 0.0015 is CaboCha's default.
- The group `after` is better than CaboCha's set. Of CaboCha's groups,
  `right` matters most, then `gap`; `children`, `left` and `distance` do
  not change the result significantly.
- Leaving out rare pairs or rare features makes the model smaller and does
  not change the result. So does hashing the pairs, down to 2^18 weights.
- Among the tables the cross-validation does not tell apart, the dev file
  chooses, with the model trained on the whole train file:

| Table | Level 1 on dev | From the morphemes | Bytes |
| --- | --- | --- | --- |
| 2^18 weights of 1 byte | 89.92% (3,274 / 3,641) | 85.28% (3,105 / 3,641) | 2,307,488 |
| 2^20 weights of 1 byte | 89.81% (3,270 / 3,641) | 85.17% (3,101 / 3,641) | 3,106,593 |
| 2^21 weights of 1 byte | 90.03% (3,278 / 3,641) | 85.33% (3,107 / 3,641) | 4,157,297 |
| 2^22 weights of 1 byte | 90.11% (3,281 / 3,641) | 85.39% (3,109 / 3,641) | 6,258,548 |

- Weights of 2 bytes give the same output as weights of 4 bytes, and weights
  of 1 byte do not differ:

| Weights | Level 1 on dev | Level 1 on test | Bytes |
| --- | --- | --- | --- |
| 4 bytes (`f32`) | 90.06% (3,279 / 3,641) | 88.53% (3,512 / 3,967) | 19,028,600 |
| 2 bytes (`i16`) | 90.06% (3,279 / 3,641) | 88.53% (3,512 / 3,967) | 10,515,232 |
| 1 byte (`i8`) | 90.11% (3,281 / 3,641) | 88.53% (3,512 / 3,967) | 6,258,548 |

- The table decides the speed. On the 542 sentences of the test file, on one
  core (`scripts/dependency/parser_experiments.sh speed`):

| Pairs | Weights | Bytes of the file | Reading, microseconds | Parsing, sentences per second | Chunking and parsing |
| --- | --- | --- | --- | --- | --- |
| Hashed into 2^22 weights (the default) | 1 byte | 6,258,548 | 2,732 | 16,678 | 8,718 |
| Hashed into 2^22 weights | 4 bytes | 19,028,600 | 7,202 | 13,375 | 7,368 |
| A weight for every pair | 1 byte | 24,514,588 | 5,314 | 2,741 | 2,426 |
| A weight for every pair | 4 bytes | 38,162,161 | 9,952 | 2,679 | 2,314 |

With CaboCha's feature set and the other defaults, the result is lower:

| | With the bunsetsu of the answer | From the morphemes | Chunking F |
| --- | --- | --- | --- |
| UD Japanese-GSD dev | 89.15% (3,246 / 3,641) | 84.45% (3,075 / 3,641) | 96.85 |
| UD Japanese-GSD test | 88.30% (3,503 / 3,967) | 84.30% (3,344 / 3,967) | 96.92 |
| UD Japanese-PUD | 88.33% (7,816 / 8,849) | 85.59% (7,574 / 8,849) | 98.04 |

## Evaluation

`lindera-dependency evaluate`, also behind the `train` feature, compares an
output with the answer, both in the lattice format with the same sentences
in the same order:

```shell
cargo run --release -p lindera-dependency --features train -- evaluate \
    --gold dev.lattice \
    --system output.lattice
```

It counts what CaboCha's `cabocha-system-eval` and J.DepP count, so that the
numbers can be set next to theirs. Every ratio comes with its numerator and
its denominator; `--json` writes the same as JSON.

- **Chunking**: precision, recall and F over bunsetsu. A bunsetsu of the
  output is right when the answer has a bunsetsu that starts at the same
  morpheme and has the same length.
- **Dependency**, on the sentences whose segmentation is the one of the
  answer (all of them when the parser was given the bunsetsu of the answer):
  the bunsetsu whose head is right. Level 0 counts every bunsetsu, level 1
  every one but the last of its sentence, level 2 every one but the last two.
  Level 1 is the number J.DepP reports, and the headline number here.
- **End to end**: over the bunsetsu of the answer but the last of each
  sentence, the ones for which the output has a bunsetsu with the same span,
  whose head has the span of the head in the answer.

The arc type (`D`, `P`) is not compared. A sentence whose morphemes differ
between the two files is left out of every count and reported with its
number, and a sentence without bunsetsu in the answer is not counted.

For "every bunsetsu depends on the next", the baseline any parser has to
beat, on the dev file of UD Japanese-GSD converted as above:

```text
sentences: 506
left out, different morphemes: 0
not counted, no bunsetsu: 0
chunking precision: 100.00% (4147/4147)
chunking recall: 100.00% (4147/4147)
chunking F: 100.00%
chunking sentences: 100.00% (506/506)
dependency level 0: 68.48% (2840/4147)
dependency level 1: 64.10% (2334/3641)
dependency level 2: 58.43% (1837/3144)
dependency sentences: 16.21% (82/506)
end to end: 64.10% (2334/3641)
end to end sentences: 16.21% (82/506)
```

The same baseline at level 1 is 62.77% (2,490 / 3,967) on the test file and
60.87% (5,386 / 8,849) on UD Japanese-PUD.

### Comparing two outputs

With `--compare`, a second output is evaluated too, and the two are paired:
each segmentation of a sentence, each end-to-end arc and each sentence is
right in both, in the first only, in the second only or in neither. The
p-value is the one of McNemar's exact test, two-sided, on the two discordant
counts. The segmentations compare two chunkers, whatever the heads.

```shell
cargo run --release -p lindera-dependency --features train -- evaluate \
    --gold dev.lattice --system first.lattice --compare second.lattice
```

For two chunkers on the 10 folds of the train file (the template of CaboCha,
then the default one):

```text
paired segmentations: both 5908, first only 74, second only 107, neither 935, McNemar p = 0.0171
paired arcs: both 29235, first only 117, second only 164, neither 19856, McNemar p = 0.0060
paired sentences: both 887, first only 4, second only 12, neither 6121, McNemar p = 0.0768
```

### Which data for which question

The dev and the test file are small: 3,641 and 3,967 arcs. For an accuracy
near 90%, the 95% interval of one such number is about ±1 point, so a
difference under a point cannot be told on them alone.

- **Features and settings are compared by cross-validation on the train
  file**, which gives about 49,000 arcs (about ±0.3 point), and with the
  paired test above.
- **The dev file chooses between settings.**
- **The test file and UD Japanese-PUD are for the reported result only.**

`lindera-dependency split` writes the folds. Sentence `i` of the input,
from 0, is in the test file of fold `i mod k` and in the training file of
every other fold, so the folds are the same wherever the command runs:

```shell
cargo run --release -p lindera-dependency --features train -- split \
    --input train.lattice --folds 10 --output-dir folds
```

This writes `fold0.test.lattice` and `fold0.train.lattice` to
`fold9.test.lattice` and `fold9.train.lattice`.

## License

MIT
