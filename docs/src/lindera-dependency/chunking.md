# Chunking

The chunker segments the morphemes of a sentence into bunsetsu. It decides, for every morpheme, whether it starts a bunsetsu (`B`) or continues one (`I`), with a conditional random field over the two labels, as CaboCha does.

Chunking is in the default build and needs no other crate: the model is a table from feature string to the weights of `B` and `I`, with the weights of the transitions, and the labels are found by Viterbi. The table is read where it is in a [model file](./model_file.md). Training is behind the `train` feature.

## Chunking a sentence

```rust
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::Model;
use lindera_dependency::{Bunsetsu, Morpheme, Sentence};

let model = Model::new(std::fs::read("chunker.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");

// The morphemes of 彼は本を読んだ。 as IPADIC gives them. Any type that
// implements `Morpheme` will do.
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
    LatticeMorpheme::new("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];

// The ranges of the bunsetsu, as indices into the morphemes.
let ranges = chunker.chunk(&morphemes)?;
assert_eq!(ranges, [0..2, 2..4, 4..7]);
```

`Chunker::chunk` takes `&self`, and a chunker is `Send + Sync`, so one chunker serves several threads. The first morpheme always starts a bunsetsu, and an empty sentence has none.

The morphemes have to come from the dictionary the model was trained with, tokenized with the same settings: a model belongs to one dictionary. A morpheme with fewer feature columns than the schema of the model reads is refused with an error; see [Checking the dictionary](./model_file.md#checking-the-dictionary).

### With the head word and the function word

A model file holds the [schema](./head_and_function_word.md) of its dictionary, which gives the head word and the function word of each bunsetsu. The following writes the result in the lattice format. The heads are a placeholder here, every bunsetsu depending on the next one; the [parser](./parsing.md) finds them.

```rust
let schema = model.schema();
let count = ranges.len();
let bunsetsu = ranges
    .into_iter()
    .enumerate()
    .map(|(index, range)| {
        let positions = schema.positions(&morphemes[range.clone()]);
        let mut bunsetsu = Bunsetsu::new(range, (index + 1 < count).then_some(index + 1));
        bunsetsu.positions = positions;
        bunsetsu
    })
    .collect();
let sentence = Sentence::new(morphemes, bunsetsu)?;
let mut output = Vec::new();
lattice::write(&mut output, &sentence)?;
print!("{}", String::from_utf8(output)?);
```

```text
* 0 1D 0/1
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 -1D 0/1
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ	助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

## Training a model

The [model for IPADIC](./model_file.md#the-model-for-ipadic) that Lindera releases has a chunker. To train one of your own from [training data](./training_data.md):

```shell
cargo run --release -p lindera-dependency --features train -- train-chunker \
    --input train.lattice --output chunker.model
```

The model is for IPADIC unless `--dictionary-name` names another dictionary (`unidic`, `unidic-cwj` or `unidic-csj`): the name and the [schema](./head_and_function_word.md) of that dictionary then go into the model, and the columns, the template and the coefficient are [the ones for it](#with-unidic).

On the train file of UD Japanese-GSD converted with IPADIC, this takes a few seconds and ends with:

```text
train.lattice: 7024 sentences
iterations: 126
feature strings: 4325
weights: 8656
bytes: 261851
```

The output is a [model file](./model_file.md) for the dictionary `ipadic`, with the chunker as its only part. Training the same data again gives the same bytes. The weights are `f64`. The format may change until the first release.

| Option | Default | Meaning |
| --- | --- | --- |
| `--regularization` | `l1` | `l1` or `l2` |
| `--lambda` | `0.5` | The coefficient of the regularization |
| `--columns` | `surface,pos,cform` | The columns the template reads: `surface`, `pos` (the part of speech as its levels joined by `-`), `ctype` (conjugation type), `cform` (conjugation form), `base` (base form) |
| `--template` | The template of CaboCha's chunker with five lines for a third column | A file in the format of CRF++ |
| `--max-iterations` | `1000` | The largest number of iterations |
| `--threads` | The number of CPUs | The number of threads |

`chunk` segments the morphemes of a file again with a model, for [`evaluate`](./evaluation.md) to compare with the answer:

```shell
cargo run --release -p lindera-dependency --features train -- chunk \
    --model chunker.model --input dev.lattice --output output.lattice
```

It writes every bunsetsu as depending on the next one: it measures the chunker alone. [`parse --input-layer morphemes`](./command_line.md#input-that-is-tokenized-already) segments and parses.

## Accuracy

With the defaults, trained on the train file of UD Japanese-GSD (commit c662dc9) with IPADIC:

| | F | Precision | Recall | Sentences with every bunsetsu right |
| --- | --- | --- | --- | --- |
| UD Japanese-GSD dev | 96.85 | 97.02% (4,009 / 4,132) | 96.67% (4,009 / 4,147) | 432 / 506 |
| UD Japanese-GSD test | 96.92 | 96.85% (4,373 / 4,515) | 96.98% (4,373 / 4,509) | 459 / 542 |
| UD Japanese-PUD | 98.04 | 98.05% (9,649 / 9,841) | 98.03% (9,649 / 9,843) | 885 / 994 |

The defaults were chosen by a 10-fold cross-validation on the train file. The conjugation form helps, the conjugation type does not, and L1 is better than L2 with a model about fifty times smaller. The tables of the cross-validation are in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#how-the-defaults-were-chosen), and [`scripts/dependency`](https://github.com/lindera/lindera/tree/main/scripts/dependency) runs them again.

## With UniDic

`train-chunker --dictionary-name unidic` (or `unidic-cwj`, `unidic-csj`) trains with other settings than for IPADIC: four columns, the surface, the part of speech, the conjugation form and the base form, with `Template::unidic()`, and L1 regularization with the coefficient 0.25. `TrainOptions::for_dictionary` gives the settings of a dictionary. As for IPADIC, an option of the command replaces any of them.

The settings were chosen again by the cross-validation on the train file, with UniDic CWJ, and hold with the two other dictionaries. The base form of UniDic is the lexeme (語彙素), which is the same for the ways a word is written; it helps here, and it did not with IPADIC. F over the 10 folds, the sentences only one of the two settings segments right, and McNemar's test:

| Dictionary | The settings for IPADIC | The settings for UniDic | First only / second only | p |
| --- | --- | --- | --- | --- |
| `unidic` | 97.02 | 97.15 | 75 / 122 | 0.0010 |
| `unidic-cwj` | 97.02 | 97.22 | 59 / 131 | < 0.0001 |
| `unidic-csj` | 96.94 | 97.08 | 74 / 131 | 0.0001 |

On the files that are not trained on, the two settings cannot be told apart: the files are small (506, 542 and 994 sentences). F with the settings for IPADIC and with those for UniDic:

| Dictionary | UD Japanese-GSD dev | UD Japanese-GSD test | UD Japanese-PUD |
| --- | --- | --- | --- |
| `unidic` | 96.89 and 97.03 (p 0.09) | 97.08 and 97.24 (p 0.82) | 98.02 and 98.00 (p 1.00) |
| `unidic-cwj` | 96.63 and 96.63 (p 1.00) | 96.85 and 97.02 (p 0.50) | 97.96 and 97.89 (p 0.70) |
| `unidic-csj` | 96.60 and 96.60 (p 0.69) | 96.79 and 96.95 (p 0.48) | 97.94 and 97.78 (p 0.34) |

UD Japanese-PUD is lower with the settings for UniDic, by a difference that is not significant. The settings are the ones the cross-validation chose, as for IPADIC; the tables of the cross-validation are in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#the-settings-for-unidic).

## How the features are made

The feature strings are made in two steps that can each be replaced:

1. **Columns.** For each morpheme, the columns of the model are read through the schema: the surface, the part of speech, the conjugation form.
2. **Template.** A template in the format of CRF++ expands the columns of the morpheme and of its neighbours into feature strings: `U02:%x[0,0]` is the first column of the morpheme itself, `U11:%x[-1,1]` the second column of the one before. A position outside the sentence gives `_B-1`, `_B+1` and so on.

The decoder sees only the feature strings and the table of weights. `Chunker::features` makes the strings and `Chunker::labels` decodes them, so an extractor of your own can feed the same decoder.
