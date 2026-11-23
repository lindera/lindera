# lindera-dependency

Japanese bunsetsu dependency analysis for [Lindera](https://github.com/lindera/lindera).

A dependency analysis groups the morphemes of a sentence into bunsetsu (文節)
and finds, for each bunsetsu, the bunsetsu it depends on.

This crate is under development and is not published yet. So far it holds
what the analysis is built on, and its first step:

- the `Morpheme` trait: a surface and the feature columns of its dictionary;
  the crate depends on no tokenizer
- the `Sentence` and `Bunsetsu` types
- the reading and writing of the lattice format of CaboCha (`cabocha -f1`)
- a check that a sentence is the tree the parsers assume
- the `Schema` of a dictionary: which feature column holds what
- the chunker, which segments the morphemes of a sentence into bunsetsu

The parser, which finds the head of each bunsetsu, is still to come.

In its default build it has no dependency, and builds for
`wasm32-unknown-unknown`.

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
`train` feature alone is then enough. For each sentence, the command:

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
and the arc type only. A report goes to the standard error. For the train
file of UD Japanese-GSD with IPADIC:

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
```

A sentence dropped for two causes counts for both. The arcs of type `P` are
counted before the bunsetsu are laid over the morphemes, and again in what
is written: a merged bunsetsu takes the arc of the last one merged.

### Corpora

The corpora are not part of this repository. The numbers here were taken at
these commits, with the IPADIC that Lindera embeds (`embed-ipadic`):

| Corpus | Commit | File | Sentences kept | Bunsetsu | Morphemes |
| --- | --- | --- | --- | --- | --- |
| [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) | c662dc9 | train | 7,024 of 7,050 | 56,396 | 162,456 |
| | | dev | 506 of 507 | 4,147 | 11,934 |
| | | test | 542 of 543 | 4,509 | 12,591 |
| [UD Japanese-PUD](https://github.com/UniversalDependencies/UD_Japanese-PUD) | cf69c5c | test | 994 of 1,000 | 9,843 | 27,728 |

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
  trained model is not settled, and this project makes no claim either way.

## Chunking

The chunker segments the morphemes of a sentence into bunsetsu. It decides,
for every morpheme, whether it starts a bunsetsu (`B`) or continues one
(`I`), with a conditional random field over the two labels, as CaboCha does.

```rust
use lindera_dependency::chunker::{Chunker, ChunkerModel};

let model = ChunkerModel::from_bytes(&std::fs::read("chunker.model")?)?;
let chunker = Chunker::new(model);
// The ranges of the bunsetsu, as indices into the morphemes.
let bunsetsu = chunker.chunk(&morphemes);
```

Chunking is in the default build and needs no other crate: the model is a
table from feature string to the weights of `B` and `I`, with the weights of
the transitions, and the labels are found by Viterbi. `Chunker::chunk` takes
`&self`, and a chunker is `Send + Sync`. The first morpheme always starts a
bunsetsu, and an empty sentence has none.

The feature strings are made in two steps that can each be replaced. The
columns of a morpheme (its surface, its part of speech, its conjugation
form, ...) are read through the `Schema` of the dictionary, which says which
feature column holds what; `Schema::ipadic()` is the one of IPADIC. A
template in the format of CRF++ then expands them. A model holds the schema,
the columns and the template it was trained with.

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
bytes: 195322
```

| | F | Precision | Recall | Sentences with every bunsetsu right |
| --- | --- | --- | --- | --- |
| UD Japanese-GSD dev | 96.85 | 97.02% (4,009 / 4,132) | 96.67% (4,009 / 4,147) | 432 / 506 |
| UD Japanese-GSD test | 96.92 | 96.85% (4,373 / 4,515) | 96.98% (4,373 / 4,509) | 459 / 542 |
| UD Japanese-PUD | 98.04 | 98.05% (9,649 / 9,841) | 98.03% (9,649 / 9,843) | 885 / 994 |

The bytes of the model are a format of this crate, with weights as `f64`; it
is not the file format of a released model.

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
