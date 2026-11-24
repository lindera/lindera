# Parsing

Parsing finds, for each bunsetsu of a sentence, the bunsetsu it depends on. The `parser` module does it the way CaboCha and J.DepP do: it goes through the bunsetsu from left to right with a stack (shift-reduce), and a classifier answers one question at a time.

> The crate has **no trained model of the parser yet**. What is here is the loop, the features, a classifier that takes weights made by hand or by an experiment, and the training instances of a classifier. Training and the model file are tracked in [#1437](https://github.com/lindera/lindera/issues/1437).

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

A `Parser` takes the [schema](./head_and_function_word.md) of the dictionary, a feature set and a classifier. This classifier is made by hand, with one weight: a bunsetsu that ends in を depends on the next one.

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

`LinearClassifier` is the classifier of the crate for now. Its score is a bias, plus the weight of each feature of the decision, plus the weight of each pair of them: what the polynomial kernel of degree 2 of CaboCha and of J.DepP comes to once it is expanded. It keeps its weights in hash tables in memory, which is enough to parse with weights made by hand or by an experiment.

## The features

The features of a decision are strings, with the names CaboCha gives them. Most are made once for each bunsetsu, and a decision puts together those of the bunsetsu it looks at. They are sorted, and each is there once.

| Group | Features | Examples |
| --- | --- | --- |
| `distance` | The number of bunsetsu from the source to the target: 1, 2 to 5, 6 or more | `DIST:1`, `DIST:2-5`, `DIST:6-` |
| `source` | The source: the surface, the levels of the part of speech up to the first `*` and the conjugation form of its head (`H`), its function word (`F`), its first (`L`) and its last morpheme (`R`); its punctuation and its brackets; each of its particles; whether it is the first or the last bunsetsu of the sentence | `SHS:彼`, `SHP0:名詞`, `SFS:は`, `SRF:基本形`, `SPUNC:、`, `SOB:「`, `SOB:1`, `SCASE:は`, `SBOS:1` |
| `target` | The same for the target, with `D` in front | `DHS:読ん`, `DHF:連用タ接続`, `DEOS:1` |
| `left` | The surface of the function word of the bunsetsu before the source | `LF:は` |
| `right` | The surfaces of the first morpheme, the head and the function word of the bunsetsu after the target | `RL:言う`, `RH:言う`, `RF:言う` |
| `children` | Each bunsetsu already attached to the source (`a:`) and to the target (`A:`), by its function word: its surface when it is a particle, an adverb, an adnominal or a conjunction; or else its conjugation form; or else its part of speech | `a:は`, `A:を`, `A:基本形`, `a:名詞-一般` |
| `gap` | The bunsetsu between the source and the target: their punctuation, their brackets, the function word of each when it is a particle; and whether brackets open or close in between | `GPUNC:、`, `GOB:「`, `GCASE:を`, `GNB:1` (neither), `GOB:1`, `GCB:1`, `GBB:1` (both) |

The `children` features are the only ones that depend on the decisions made before.

Nothing in the parser names a part of speech or a column: the head and the function word, the levels of the part of speech, the conjugation form, the particles (`Schema::particles`), the words taken by their surface as a child (`Schema::lexicalized`) and the symbols all come from the schema.

### The feature set

Which groups are made is data, as the template of the chunker is. A `FeatureSet` is written as the names of its groups with commas, and `FeatureSet::cabocha()` has all of them:

```text
distance,source,target,left,right,children,gap
```

A classifier is trained on the features of one set and parses with the same set.

### Differences from CaboCha

| | CaboCha | This crate | Why |
| --- | --- | --- | --- |
| Surface | Normalized by a table (full width and half width) | As the tokenizer gives it | The schema recognizes a symbol in both widths, by its part of speech and by its surface |
| Punctuation and brackets | 。 and 、, ten opening and ten closing brackets, by the normalized surface | Those of the schema, which also has `,` `.` `，` `．` and more brackets | IPADIC gives a half-width symbol as a noun |
| Head and function word | `findHead` with the patterns of the tag set | `Schema::positions`, which has more patterns for IPADIC | See [Head and Function Word](./head_and_function_word.md) |
| A bunsetsu whose head is not known | Attached to the next one, without an instance (partial training) | Not supported: a bunsetsu without a head is the root | The training data has the head of every bunsetsu |
| Score of an arc to the last bunsetsu | The score of the decision before, which is unrelated | 0 | That decision is not asked |

J.DepP has features CaboCha does not have: whether the heads of two bunsetsu have the same surface (a hint of coordination), and the balance of brackets instead of their presence. They are candidates for the comparison of feature sets in [#1437](https://github.com/lindera/lindera/issues/1437).

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

With the `train` feature, `lindera-dependency instances` writes the instances of a file in the lattice format, with the schema of IPADIC:

```shell
lindera-dependency instances --input train.lattice --output train.instances
```

Each line is one instance: `+1` or `-1`, then the feature strings, separated by tabs. An empty line ends each sentence, so that a file can be cut into folds between sentences. `--features` takes a feature set; the default is the one of CaboCha.

The report goes to the standard error. For the train file of UD Japanese-GSD:

```text
train.lattice
features: distance,source,target,left,right,children,gap
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
