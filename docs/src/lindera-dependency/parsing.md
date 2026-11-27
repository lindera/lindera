# Parsing

Parsing finds, for each bunsetsu of a sentence, the bunsetsu it depends on. The `parser` module does it the way CaboCha and J.DepP do: it goes through the bunsetsu from left to right with a stack (shift-reduce), and a classifier answers one question at a time.

Lindera releases the [model for IPADIC](./model_file.md#the-model-for-ipadic) and the [models for UniDic](./model_file.md#the-models-for-unidic). To make one of your own, [train it](#training-a-model) from [training data](./training_data.md): it takes a few seconds.

## Parsing with a model

A [model file](./model_file.md) holds the chunker and the parser of a dictionary. With both, a sentence goes from morphemes to dependencies:

```rust
use std::io;

use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::Model;
use lindera_dependency::Sentence;

let model = Model::new(std::fs::read("ipadic.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");
let parser = model.parser().expect("the file has a parser");

// The morphemes of 彼は赤い花を見た。 as IPADIC gives them.
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("赤い", "形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ"),
    LatticeMorpheme::new("花", "名詞,一般,*,*,*,*,花,ハナ,ハナ"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("見", "動詞,自立,*,*,一段,連用形,見る,ミ,ミ"),
    LatticeMorpheme::new("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];
let ranges = chunker.chunk(&morphemes)?;
let bunsetsu = parser.parse(&morphemes, &ranges)?;

// The result is a sentence the lattice format can hold.
let sentence = Sentence::new(morphemes, bunsetsu)?;
lattice::write(&mut io::stdout(), &sentence)?;
```

<!-- markdownlint-disable MD010 -->
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
<!-- markdownlint-enable MD010 -->

Each bunsetsu has its head, the positions of its head word and of its function word, and the score of the decision that attached it: above 0, and the larger the surer. Here 赤い depends on 花を with the score 0.70. The score is 0 for the root and for a bunsetsu attached to the root, since that decision is not asked. `Model::analyze` does the two steps in one call; for many sentences, take the chunker and the parser once, as above. Both take `&self` and serve several threads.

## How a sentence is parsed

A decision is about two bunsetsu, the **source** and the **target**: does the source depend on the target?

1. A stack holds the bunsetsu that have no head yet. It starts with the first bunsetsu.
2. Each following bunsetsu is taken as the target, and the top of the stack as the source.
3. While the classifier says that the source depends on the target, the source gets its head and leaves the stack, and the next one is looked at.
4. A source that is refused stays on the stack, and the target goes on top of it.
5. The last bunsetsu is the root: whatever is on the stack when it comes depends on it, and the classifier is not asked.

The result is the tree the reference parsers assume: every head is to the right of its bunsetsu, no two arcs cross, and the last bunsetsu is the root. The number of decisions is under twice the number of bunsetsu, so the time is linear in the length of the sentence. A sentence of one or two bunsetsu asks nothing.

For 彼は | 本を | 読んだと | 言う, where 本を depends on 読んだと and the other two on 言う:

| Stack | Target | Decision | Answer |
| --- | --- | --- | --- |
| 彼は | 本を | 彼は → 本を | No: 本を goes on the stack |
| 彼は, 本を | 読んだと | 本を → 読んだと | Yes: 本を leaves the stack |
| 彼は | 読んだと | 彼は → 読んだと | No: 読んだと goes on the stack |
| 彼は, 読んだと | 言う | not asked | Both depend on 言う |

## Using the parser

The parser of a model file is a `Parser` with the classifier of the file. A `Parser` can be made from its parts as well: the [schema](./head_and_function_word.md) of the dictionary, a feature set and a classifier. This classifier is made by hand, with one weight: a bunsetsu that ends in を depends on the next one.

```rust
use std::collections::HashMap;
use std::io;

use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::parser::{FeatureSet, LinearClassifier, Parser};
use lindera_dependency::{Schema, Sentence};

// SFS is the surface of the function word of the source, and DIST:1
// says that the target is the next bunsetsu.
let pairs = HashMap::from([(("SFS:を".to_string(), "DIST:1".to_string()), 2.0)]);
let classifier = LinearClassifier::new(-1.0, HashMap::new(), pairs)?;
let parser = Parser::new(Schema::ipadic(), FeatureSet::cabocha(), classifier);

// 彼は | 本を | 読んだと | 言う
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
    LatticeMorpheme::new("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
    LatticeMorpheme::new("と", "助詞,格助詞,引用,*,*,*,と,ト,ト"),
    LatticeMorpheme::new("言う", "動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ"),
];
let bunsetsu = parser.parse(&morphemes, &[0..2, 2..4, 4..7, 7..8])?;
for (index, one) in bunsetsu.iter().enumerate() {
    println!("{index} -> {:?}, score {:?}", one.head, one.score);
}

// The result is a sentence the lattice format can hold.
let sentence = Sentence::new(morphemes, bunsetsu)?;
lattice::write(&mut io::stdout(), &sentence)?;
```

<!-- markdownlint-disable MD010 -->
```text
0 -> Some(3), score Some(0.0)
1 -> Some(2), score Some(1.0)
2 -> Some(3), score Some(0.0)
3 -> None, score Some(0.0)
* 0 3D 0/1 0.000000
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1 1.000000
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 3D 0/2 0.000000
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ	助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
と	助詞,格助詞,引用,*,*,*,と,ト,ト
* 3 -1D 0/0 0.000000
言う	動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ
EOS
```
<!-- markdownlint-enable MD010 -->

The bunsetsu are given as ranges of morphemes, which is what [`Chunker::chunk`](./chunking.md) returns. Each bunsetsu of the result has its head, the positions of its head word and of its function word from the schema, and the score of the decision that attached it. The score is 0 for the root and for a bunsetsu attached to the root, since that decision is not asked.

## The classifier

A classifier implements the trait `Classifier`, in two steps:

- `id` gives a number to a feature string the classifier knows, and `None` to any other, which the parser then leaves out. The parser asks it once for each feature of each bunsetsu of a sentence.
- `score` scores the numbers of the features of a decision. The source depends on the target when the score is above 0.

A decision looks up no string, as in CaboCha.

Every classifier of the crate scores a bias, plus the weight of each feature of the decision, plus the weight of each pair of them: what the polynomial kernel of degree 2 of CaboCha and of J.DepP comes to once it is expanded.

| Classifier | Where the weights are |
| --- | --- |
| `ParserPart` | On the bytes of a model file; `Model::parser` gives a parser with it |
| `ParserModel` | In memory: what training gives, and what `ModelBuilder` writes |
| `LinearClassifier` | In memory, in hash tables by the names of the features: for weights made by hand |

A trained model keeps the weights of the pairs in a table of a fixed size that the pairs are hashed into, so pairs that fall on the same place share a weight. A weight of its own for every pair is the other choice (`--pairs exact`); see [How the defaults were chosen](#how-the-defaults-were-chosen).

## The features

The features of a decision are strings, with the names CaboCha gives them, and one group CaboCha does not have. Most are made once for each bunsetsu, and a decision puts together those of the bunsetsu it looks at. They are sorted, and each is there once.

| Group | Features | Examples |
| --- | --- | --- |
| `distance` | The number of bunsetsu from the source to the target: 1, 2 to 5, 6 or more | `DIST:1`, `DIST:2-5`, `DIST:6-` |
| `source` | The source: the surface, the levels of the part of speech up to the first `*` and the conjugation form of its head (`H`), its function word (`F`), its first (`L`) and its last morpheme (`R`); its punctuation and its brackets; each of its particles; whether it is the first or the last bunsetsu of the sentence | `SHS:彼`, `SHP0:名詞`, `SFS:は`, `SRF:基本形`, `SPUNC:、`, `SOB:「`, `SOB:1`, `SCASE:は`, `SBOS:1` |
| `target` | The same for the target, with `D` in front | `DHS:読ん`, `DHF:連用タ接続`, `DEOS:1` |
| `left` | The surface of the function word of the bunsetsu before the source | `LF:は` |
| `right` | The surfaces of the first morpheme, the head and the function word of the bunsetsu after the target | `RL:言う`, `RH:言う`, `RF:言う` |
| `children` | Each bunsetsu already attached to the source (`a:`) and to the target (`A:`), by its function word: its surface when it is a particle, an adverb, an adnominal or a conjunction; or else its conjugation form; or else its part of speech | `a:は`, `A:を`, `A:基本形`, `a:名詞-一般` |
| `gap` | The bunsetsu between the source and the target: their punctuation, their brackets, the function word of each when it is a particle; and whether brackets open or close in between | `GPUNC:、`, `GOB:「`, `GCASE:を`, `GNB:1` (neither), `GOB:1`, `GCB:1`, `GBB:1` (both) |
| `after` | The morpheme after the head of the source and of the target (`N`): its surface, the levels of its part of speech and its conjugation form | `SNS:し`, `SNP0:動詞`, `SNF:連用形`, `DNS:こと` |

The `children` features are the only ones that depend on the decisions made before.

The `after` group is not CaboCha's. CaboCha takes the last word of a bunsetsu that is not a function word as its head. The head of a [schema](./head_and_function_word.md) is the content word, which can be the word before that one: 利用 in 利用 / し / た, 読む in 読む / こと / を. What follows the head then says what the bunsetsu is as a whole, a verb or a noun, and the parser needs it: with these features the accuracy is as high as with CaboCha's head, and the head stays the content word.

Nothing in the parser names a part of speech or a column: the head and the function word, the levels of the part of speech, the conjugation form, the particles (`Schema::particles`), the words taken by their surface as a child (`Schema::lexicalized`) and the symbols all come from the schema.

### The feature set

Which groups are made is data, as the template of the chunker is. A `FeatureSet` is written as the names of its groups with commas. The default, `FeatureSet::default()`, has all of them:

```text
distance,source,target,left,right,children,gap,after
```

`FeatureSet::cabocha()` is the same without `after`. A model file holds the feature set of its parser.

A classifier is trained on the features of one set and parses with the same set.

### Differences from CaboCha

| | CaboCha | This crate | Why |
| --- | --- | --- | --- |
| Surface | Normalized by a table (full width and half width) | As the tokenizer gives it, unless the schema has a normalization | The schemas of this crate have none: they recognize a symbol in both widths, by its part of speech and by its surface. A [model converted from CaboCha](./cabocha_models.md) has CaboCha's table |
| Punctuation and brackets | 。 and 、, ten opening and ten closing brackets, by the normalized surface | Those of the schema, which also has `,` `.` `，` `．` and more brackets | IPADIC gives a half-width symbol as a noun. A model converted from CaboCha has CaboCha's marks and its way to match them |
| Head and function word | `findHead` with the patterns of the tag set | `Schema::positions`, which has more patterns for IPADIC | See [Head and Function Word](./head_and_function_word.md). A model converted from CaboCha has CaboCha's patterns |
| The morpheme after the head | No such feature | The group `after` | The head of the schema is the content word |
| A bunsetsu whose head is not known | Attached to the next one, without an instance (partial training) | Not supported: a bunsetsu without a head is the root | The training data has the head of every bunsetsu |
| Score of an arc to the last bunsetsu | The score of the decision before, which is unrelated | 0 | That decision is not asked |

J.DepP has features CaboCha does not have: whether the heads of two bunsetsu have the same surface (a hint of coordination), and the balance of brackets instead of their presence. They are not here: the accuracy targets were met without them.

## Training instances

A classifier is trained on the decisions the parser would ask. `parser::instances` runs the loop on a sentence whose heads are known, with the answer deciding in the place of a classifier, as CaboCha does in its training mode. Each decision is one instance: its features, and whether the source depends on the target. The `children` features are those of the answer so far.

```rust
use lindera_dependency::lattice;
use lindera_dependency::parser::{self, FeatureSet};
use lindera_dependency::Schema;

// 彼は | 本を | 読んだと | 言う, with 彼は depending on 言う.
let text = "\
* 0 3D
彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 3D
読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
と\t助詞,格助詞,引用,*,*,*,と,ト,ト
* 3 -1D
言う\t動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ
EOS
";
let sentence = &lattice::parse(text)?[0];
let set = FeatureSet::parse("distance,left,children")?;
for instance in parser::instances(&Schema::ipadic(), &set, sentence)? {
    let label = if instance.label { "+1" } else { "-1" };
    println!("{label} {}", instance.features.join(" "));
}
```

```text
-1 DIST:1
+1 DIST:1 LF:は
-1 A:を DIST:2-5
```

These are the three decisions of the table above. In the third, 本を is already attached to the target 読んだと, hence `A:を`.

A sentence whose tree the parser cannot make (a head to the left, crossing arcs, a root that is not the last bunsetsu) is an error. The [conversion](./training_data.md) leaves such sentences out of the training data.

### The command

With the `train` feature, `lindera-dependency instances` writes the instances of a file in the lattice format, with the schema of IPADIC, or of the dictionary that `--dictionary-name` names:

```shell
lindera-dependency instances --input train.lattice --output train.instances
```

Each line is one instance: `+1` or `-1`, then the feature strings, separated by tabs. An empty line ends each sentence, so that a file can be cut into folds between sentences. `--features` takes a feature set; the default is every group.

The report goes to the standard error. For the train file of UD Japanese-GSD:

```text
train.lattice
features: distance,source,target,left,right,children,gap,after
sentences: 7024
bunsetsu: 56396
instances: 66000
positive: 33512
negative: 32488
```

The command stops at a sentence whose tree does not come back when its answer decides. `scripts/dependency/parser_instances.sh` runs it on the four files of the training data:

| File | Sentences | Bunsetsu | Instances | Positive | Negative |
| --- | --- | --- | --- | --- | --- |
| UD Japanese-GSD train | 7,024 | 56,396 | 66,000 | 33,512 | 32,488 |
| UD Japanese-GSD dev | 506 | 4,147 | 4,881 | 2,474 | 2,407 |
| UD Japanese-GSD test | 542 | 4,509 | 5,377 | 2,707 | 2,670 |
| UD Japanese-PUD | 994 | 9,843 | 12,739 | 6,077 | 6,662 |

## Training a model

Training is behind the `train` feature. `train-parser` trains the parser on [training data](./training_data.md), with the schema of IPADIC, or of the dictionary that `--dictionary-name` names, and writes a [model file](./model_file.md). With `--chunker`, the chunker of another model file goes into the output, which is then the whole model of the dictionary; that file has to be for the same dictionary:

```shell
cargo run --release -p lindera-dependency --features train -- train-chunker \
    --input train.lattice --output chunker.model
cargo run --release -p lindera-dependency --features train -- train-parser \
    --input train.lattice --chunker chunker.model --output ipadic.model
```

On the train file of UD Japanese-GSD converted with IPADIC, `train-parser` takes about three seconds and ends with:

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

A last line, `seconds:`, has the time of the training; it is left out here. Training the same data again gives the same bytes.

| Option | Default | Meaning |
| --- | --- | --- |
| `--chunker` | None | A model file whose chunker goes into the output |
| `--algorithm` | `dcd` | `dcd`: dual coordinate descent on a support vector machine, as CaboCha trains. `pa`: passive-aggressive, as J.DepP trains |
| `--cost` | `0.0015` for `dcd`, `0.001` for `pa` | The most one training decision adds to the weights |
| `--iterations` | `10` | For `pa`: the passes over the training decisions |
| `--averaged` | Off | For `pa`: the average of the weights after every decision |
| `--features` | Every group | The [feature set](#the-feature-set) |
| `--pairs` | `hashed` | `hashed`: a table of a fixed size the pairs share. `exact`: a weight for every pair |
| `--bits` | `22` | For `hashed`: the table has 2^bits weights |
| `--min-support` | `1` | For `exact`: the training decisions a pair has to be in to get a weight |
| `--pair-threshold` | `0` | For `exact`: a pair whose weight is closer to 0 is left out |
| `--min-count` | `1` | The training decisions a feature has to be in to be used |
| `--quantization` | `i8` | The weights in the file: `f32` as they are, `i16` or `i8` as whole numbers times a scale |
| `--seed` | `1` | For `dcd`: the seed of the order of the decisions |

[`parse`](./command_line.md) with `--input-layer` finds the heads of a file in the lattice format with a model, for [`evaluate`](./evaluation.md) to compare with the answer:

```shell
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer bunsetsu --model ipadic.model --input dev.lattice --output output.lattice
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer morphemes --model ipadic.model --input dev.lattice --output output.lattice
```

With `bunsetsu` it parses the bunsetsu of the input, which measures the parser alone. With `morphemes` the bunsetsu of the input are not read: the morphemes are segmented by the chunker of the model and then parsed, which is the analysis as it is used. Without `--input-layer` the input is plain text: see [Command Line](./command_line.md).

## Accuracy

With the defaults, trained on the train file of UD Japanese-GSD (commit c662dc9) with IPADIC. The dependency accuracy counts every bunsetsu but the last of each sentence ([level 1](./evaluation.md)).

| | With the bunsetsu of the answer | From the morphemes | Chunking F |
| --- | --- | --- | --- |
| UD Japanese-GSD dev | 90.11% (3,281 / 3,641) | 85.39% (3,109 / 3,641) | 96.85 |
| UD Japanese-GSD test | 88.53% (3,512 / 3,967) | 84.47% (3,351 / 3,967) | 96.92 |
| UD Japanese-PUD | 88.81% (7,859 / 8,849) | 86.19% (7,627 / 8,849) | 98.04 |

The targets of the project, from CaboCha and J.DepP trained on the same data, are 89.1 on dev and 87.7 on test with the bunsetsu of the answer, and 85.0 on dev and 84.3 on test from the morphemes. The two tools are run by [`scripts/dependency/baselines.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) ([#1433](https://github.com/lindera/lindera/issues/1433)): CaboCha gives 89.84 and 88.51 with the bunsetsu of the answer and 85.00 and 84.25 from the morphemes, J.DepP 90.11 and 88.71, and 85.99 and 84.65. J.DepP is ahead from the morphemes because it chunks better (F 97.17 and 97.02): see [#1464](https://github.com/lindera/lindera/issues/1464).

The model file with the chunker and the parser has 6,258,548 bytes. On one core of an AMD Ryzen AI Max+ 395 it is read in about 3 ms, and chunks and parses about 8,700 sentences per second; see [Model File](./model_file.md#size-and-speed).

### With UniDic

The models for the three UniDic dictionaries, each trained on the train file converted with its dictionary, with the [settings of the chunker for UniDic](./chunking.md#with-unidic) and the defaults of `train-parser`. With the bunsetsu of the answer:

| Dictionary | UD Japanese-GSD dev | UD Japanese-GSD test | UD Japanese-PUD |
| --- | --- | --- | --- |
| IPADIC | 90.11% (3,281 / 3,641) | 88.53% (3,512 / 3,967) | 88.81% (7,859 / 8,849) |
| `unidic` | 89.67% (3,289 / 3,668) | 88.69% (3,543 / 3,995) | 88.67% (7,888 / 8,896) |
| `unidic-cwj` | 89.35% (3,279 / 3,670) | 88.91% (3,558 / 4,002) | 88.58% (7,879 / 8,895) |
| `unidic-csj` | 89.75% (3,294 / 3,670) | 88.90% (3,557 / 4,001) | 88.59% (7,880 / 8,895) |

From the morphemes:

| Dictionary | UD Japanese-GSD dev | UD Japanese-GSD test | UD Japanese-PUD |
| --- | --- | --- | --- |
| IPADIC | 85.39% (3,109 / 3,641) | 84.47% (3,351 / 3,967) | 86.19% (7,627 / 8,849) |
| `unidic` | 85.06% (3,120 / 3,668) | 84.88% (3,391 / 3,995) | 85.93% (7,644 / 8,896) |
| `unidic-cwj` | 84.22% (3,091 / 3,670) | 84.71% (3,390 / 4,002) | 85.73% (7,626 / 8,895) |
| `unidic-csj` | 84.58% (3,104 / 3,670) | 84.65% (3,387 / 4,001) | 85.58% (7,612 / 8,895) |

Chunking F:

| Dictionary | UD Japanese-GSD dev | UD Japanese-GSD test | UD Japanese-PUD |
| --- | --- | --- | --- |
| IPADIC | 96.85 | 96.92 | 98.04 |
| `unidic` | 97.03 | 97.24 | 98.00 |
| `unidic-cwj` | 96.63 | 97.02 | 97.89 |
| `unidic-csj` | 96.60 | 96.95 | 97.78 |

The numbers of two dictionaries are not counted on the same bunsetsu: a dictionary that splits a bunsetsu of the corpus merges it with the next, so the answers differ (3,641 arcs on dev with IPADIC, 3,670 with UniDic CWJ). The targets of the project are stated for IPADIC, and none is set for UniDic: the baselines they come from were measured with IPADIC.

UniDic CSJ is a dictionary for spoken language, and the training data is written language; its model is trained and measured as the two others are.

The settings of the parser were not carried over: the cross-validations were run again with each UniDic dictionary, and choose the same settings. Level 1 over the 10 folds of the train file:

| Dictionary | CaboCha's feature set | Every group | p | Hashed into 2^22 weights | And weights of 1 byte |
| --- | --- | --- | --- | --- | --- |
| `unidic` | 88.67% | 89.26% | < 0.0001 | 89.23% | 89.22% |
| `unidic-cwj` | 88.79% | 89.23% | < 0.0001 | 89.23% | 89.25% |
| `unidic-csj` | 88.65% | 89.08% | < 0.0001 | 89.10% | 89.09% |

With UniDic CWJ the cost 0.0015 gives 88.79%, 0.001 gives 88.59% (p 0.0001) and 0.003 gives 88.71% (p 0.2556).

## How the defaults were chosen

By a 10-fold cross-validation on the train file, on the bunsetsu of the answer. Settings that do not differ significantly (McNemar's exact test on the arcs) are taken as equal, and the smaller or faster one is chosen; the dev file chooses between sizes the cross-validation does not tell apart.

| Choice | Level 1 over the 10 folds | What it showed |
| --- | --- | --- |
| Dual coordinate descent, cost 0.0015 | 88.96% | The best cost; 0.003 gives 88.91% (p = 0.47), 0.001 gives 88.79% (p < 0.001) |
| Passive-aggressive, averaged, cost 0.0005, 30 passes | 88.88% | Not different from dual coordinate descent (p = 0.22), in the same time; it needs a number of passes to be chosen, and dual coordinate descent ends by itself |
| CaboCha's feature set without `right` | 88.53% | The group that matters most (p < 0.001); without `gap` 88.75%, without `children` 88.87% |
| With the group `after` | 89.19% | Better than CaboCha's set (p < 0.001) |
| A weight for every pair, in a table of 38 MB | 89.19% | 2,700 sentences per second |
| Pairs hashed into 2^22 weights | 89.21% | Not different (p = 0.60), 16,700 sentences per second; 2^18 to 2^21 do not differ either, and 2^22 is the best of them on dev |
| Weights of 1 byte | 89.21% | Not different from 4 bytes (p = 0.86); weights of 2 bytes give the same output |

The tables with every setting are in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#how-the-defaults-of-the-parser-were-chosen), and [`scripts/dependency/parser_experiments.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) runs them again.
