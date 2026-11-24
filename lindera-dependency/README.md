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
- the `Schema` of a dictionary: which feature column holds what, which
  morphemes are function words and symbols, and from them the head and
  the function word of a bunsetsu
- the chunker, which segments the morphemes of a sentence into bunsetsu
- the model file of a dictionary, which is read in place
- the parser, which finds the head of each bunsetsu, with its features
  and the training instances of its classifier

The parser has no trained model yet: it parses with weights given to it,
and the training of those weights is still to come.

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
last lines compare the head word of the schema of IPADIC with the content
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
feature column holds what; `Schema::ipadic()` is the one of IPADIC. A
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
dictionary, its schema, and the trained parts, so far the chunker. `Model`
reads it in place, from whatever holds the bytes, and copies none of its
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

## Parsing

The parser finds the head of each bunsetsu. It goes through the bunsetsu
from left to right with a stack (shift-reduce), and asks one question at a
time: does this bunsetsu, the source, depend on that one, the target? This
is what CaboCha (`parseShiftReduce`) and J.DepP (`_parseLinear`) do. Every
head is to the right of its bunsetsu, no two arcs cross, and the last
bunsetsu is the root; the number of decisions is under twice the number of
bunsetsu.

There is no trained model of the parser yet
([#1437](https://github.com/lindera/lindera/issues/1437)). What is here is
the loop, the features, a classifier that takes weights made by hand or by
an experiment, and the training instances of a classifier.

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

Which groups a model uses is data: a `FeatureSet`, written as
`distance,source,target,left,right,children,gap`. `FeatureSet::cabocha()`
has all of them.

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
presence. They are candidates for the comparison of feature sets
([#1437](https://github.com/lindera/lindera/issues/1437)).

### Training instances

`parser::instances` runs the loop on a sentence whose heads are known, with
the answer deciding, as CaboCha does in its training mode. Each decision is
one instance: its features, and whether the source depends on the target.
The `children` features are those of the answer so far. A sentence whose
tree the parser cannot make is an error.

The `instances` command writes them for a file in the lattice format, with
the schema of IPADIC:

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
