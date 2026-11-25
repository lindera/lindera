# Architecture

## Module Structure

```text
lindera-dependency/src/
├── lib.rs                # Public API re-exports
├── morpheme.rs           # Morpheme trait, DictionaryMorpheme
├── sentence.rs           # Sentence, Bunsetsu, ArcType, Violation
├── lattice.rs            # Reading and writing the lattice format of CaboCha
├── schema.rs             # Schema: columns, head word and function word
├── schema/
│   ├── pattern.rs        # Pattern over the morphemes of a bunsetsu
│   └── symbols.rs        # Marks, Symbols: punctuation and brackets
├── chunker.rs            # Chunker
├── chunker/
│   ├── columns.rs        # Column: what a template reads from a morpheme
│   ├── template.rs       # Template in the format of CRF++
│   ├── model.rs          # ChunkerData, ChunkerModel: the table of weights
│   ├── decode.rs         # Viterbi over the two labels
│   └── train.rs          # Training with lindera-crf (train)
├── model.rs              # Model, ModelBuilder: the model file of a dictionary
├── model/
│   └── table.rs          # A table from strings to values, read in place
├── parser.rs             # Parser
├── parser/
│   ├── features.rs       # Group, FeatureSet: the features of a decision
│   ├── shift_reduce.rs   # The loop over the bunsetsu
│   ├── classifier.rs     # Classifier, LinearClassifier
│   ├── model.rs          # ParserModel, ParserPart: the weights, in memory and on a file
│   ├── train.rs          # Training: dual coordinate descent, passive-aggressive (train)
│   └── oracle.rs         # Training instances from an answer
├── corpus.rs             # Training data from a dependency corpus (train)
├── corpus/
│   ├── conllu.rs         # Reading CoNLL-U
│   ├── convert.rs        # Word dependencies to bunsetsu dependencies
│   └── project.rs        # Bunsetsu onto the morphemes of a dictionary
├── evaluation.rs         # Chunking and dependency accuracy (train)
├── error.rs              # Error types
├── bytes.rs              # Helpers for the bytes of a model
└── bin/
    ├── lindera-dependency.rs   # The command: parse, chunk (cli)
    └── lindera-dependency/
        └── train.rs            # The subcommands of training and evaluation (train)
```

## Key Components

### Morpheme

A morpheme is a surface and the feature columns of its dictionary. `Morpheme` is a trait, so the crate depends on no tokenizer and a type of your own can be analyzed:

```rust
use lindera_dependency::Morpheme;

struct Word {
    surface: &'static str,
    features: [&'static str; 2],
}

impl Morpheme for Word {
    fn surface(&self) -> &str {
        self.surface
    }

    fn feature(&self, index: usize) -> Option<&str> {
        self.features.get(index).copied()
    }
}

let word = Word { surface: "本", features: ["名詞", "一般"] };
assert_eq!(word.feature(1), Some("一般"));
assert_eq!(word.features(), "名詞,一般");
```

What a column means depends on the dictionary: in IPADIC, column 0 is the part of speech and column 6 the base form. The crate reads columns by role through a [schema](./head_and_function_word.md), not by number.

### Sentence / Bunsetsu

A `Sentence` holds its morphemes and the `Bunsetsu` that group them. A bunsetsu is a range of morphemes with:

- its head: the index of the bunsetsu it depends on, or `None` for the root;
- the type of the arc to the head (`ArcType`): `D` for a plain dependency, `P` for coordination, `A` for apposition, `I` for incomplete coordination;
- optionally the positions of its head word and of its function word, and a score.

`Sentence::violations` reports the ways a sentence is not the tree the parsers assume: one root that is the last bunsetsu, every head to the right of its bunsetsu, and no crossing arcs.

### The lattice format

The `lattice` module reads and writes the format of `cabocha -f1`, which the tools of CaboCha and of J.DepP exchange. A bunsetsu line starts with `*`: the id of the bunsetsu, the id of its head followed by the arc type (`-1` for the root), then optionally the positions of the head word and the function word and a score. A morpheme line is the surface, a tab and the feature string. `EOS` ends a sentence.

```rust
use lindera_dependency::Morpheme;
use lindera_dependency::lattice;

// 彼は | 本を | 読んだ。 as `cabocha -f1` writes it.
let text = "\
* 0 2D 0/1 -1.500000
彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1 -1.500000
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 -1D 0/1 0.000000
読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。\t記号,句点,*,*,*,*,。,。,。
EOS
";
let sentences = lattice::parse(text)?;
let sentence = &sentences[0];
for bunsetsu in &sentence.bunsetsu {
    let surface: String = sentence.morphemes[bunsetsu.morphemes.clone()]
        .iter()
        .map(|morpheme| morpheme.surface())
        .collect();
    println!("{surface} -> {:?} ({})", bunsetsu.head, bunsetsu.arc_type);
}
assert!(sentence.violations().is_empty());

// Writing gives the text back.
let mut output = Vec::new();
lattice::write(&mut output, sentence)?;
assert_eq!(output, text.as_bytes());
```

```text
彼は -> Some(2) (D)
本を -> Some(2) (D)
読んだ。 -> None (D)
```

A sentence without bunsetsu lines is a sentence of morphemes only, as a tokenizer writes it in the MeCab format. A score is written with six decimals, as CaboCha writes it.

### Schema

A `Schema` holds what is known about a dictionary as data: which columns hold the part of speech, the conjugation and the base form, which morphemes are function words and which are particles, and how symbols are recognized. See [Head and Function Word](./head_and_function_word.md).

### Chunker / ChunkerModel

A `Chunker` segments morphemes into bunsetsu with a table from feature string to weights, and with the schema, the columns and the template the table was trained with. It reads them through the `ChunkerData` trait: from a `ChunkerModel` in memory, which is what training gives, or from the chunker part of a model file. See [Chunking](./chunking.md).

### Model / ModelBuilder

A `Model` is the model file of a dictionary, read in place from bytes it owns or borrows: the name of the dictionary, its schema, and the trained parts, the chunker and the parser. `Model::analyze` takes a sentence from morphemes to dependencies. `ModelBuilder` writes a file. See [Model File](./model_file.md).

### Parser / Classifier

A `Parser` finds the head of each bunsetsu with shift-reduce. The features of a decision are strings made through the schema; a `FeatureSet` says which groups of them are made; a `Classifier` scores them, by adding the weights of the features and of their pairs. The classifier of a model file is a `ParserPart`, which reads the weights on the bytes of the file; `ParserModel` holds them in memory, as training gives them; `LinearClassifier` takes weights made by hand. `parser::instances` gives the training instances from a sentence with its answer, and `parser::train` trains on them. See [Parsing](./parsing.md).

### The command

With the `cli` feature the crate builds the `lindera-dependency` command, which tokenizes with `lindera-segmenter` and has no training code. The `train` feature adds the subcommands that make training data, train and evaluate:

| Subcommand | Feature | What it does | Page |
| --- | --- | --- | --- |
| `parse` | `cli` | Parses plain text, or input that is tokenized or segmented already, and writes the lattice format | [Command Line](./command_line.md) |
| `chunk` | `cli` | Segments the morphemes of a file again with a model file | [Chunking](./chunking.md) |
| `convert` | `train` | Makes training data from a CoNLL-U file with bunsetsu labels | [Training Data](./training_data.md) |
| `evaluate` | `train` | Compares an output with the answer | [Evaluation](./evaluation.md) |
| `split` | `train` | Writes the folds of a cross-validation | [Evaluation](./evaluation.md) |
| `train-chunker` | `train` | Trains the chunker and writes a model file | [Chunking](./chunking.md) |
| `instances` | `train` | Writes the training instances of the parser | [Parsing](./parsing.md) |
| `train-parser` | `train` | Trains the parser and writes a model file, with the chunker of another one | [Parsing](./parsing.md) |

The command is in this crate, not in `lindera-cli`: it writes the lattice format and holds the training, which a user of the tokenizer does not need. `lindera-cli` runs the parser as a [token filter](./token_filter.md).

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| (none) | The types, the lattice format, the schema, the model file, the chunker and the parser. No dependency; builds for `wasm32-unknown-unknown` | Yes |
| `cli` | The `lindera-dependency` command with `parse` and `chunk`, which tokenizes with `lindera-segmenter`. No training code | No |
| `train` | The `corpus` and `evaluation` modules, the training of the chunker with `lindera-crf` and of the parser, and their subcommands (implies `cli`) | No |
| `embed-ipadic` | Embeds IPADIC in the command, for `--dict embedded://ipadic` (implies `cli`) | No |
| `embed-model-ipadic` | Embeds the [model for IPADIC](./model_file.md#embedding-it), for `embedded::model("ipadic")` and `--model embedded://ipadic`. A build dependency, and none of the library | No |

The crate depends on `lindera-segmenter` at most, never on `lindera` or `lindera-analysis`: the token filter makes `lindera-analysis` depend on this crate, behind its `dependency` feature.
