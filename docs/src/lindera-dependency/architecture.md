# Architecture

## Module Structure

```text
lindera-dependency/src/
├── lib.rs                # Public API re-exports
├── morpheme.rs           # Morpheme trait
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
│   ├── model.rs          # ChunkerModel: the table of weights, and its bytes
│   ├── decode.rs         # Viterbi over the two labels
│   └── train.rs          # Training with lindera-crf (train)
├── corpus.rs             # Training data from a dependency corpus (train)
├── corpus/
│   ├── conllu.rs         # Reading CoNLL-U
│   ├── convert.rs        # Word dependencies to bunsetsu dependencies
│   └── project.rs        # Bunsetsu onto the morphemes of a dictionary
├── evaluation.rs         # Chunking and dependency accuracy (train)
├── error.rs              # Error types
├── bytes.rs              # Helpers for the bytes of a model
└── bin/
    └── lindera-dependency.rs   # The command (train)
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

A `Schema` holds what is known about a dictionary as data: which columns hold the part of speech, the conjugation and the base form, which morphemes are function words, and how symbols are recognized. See [Head and Function Word](./head_and_function_word.md).

### Chunker / ChunkerModel

A `Chunker` segments morphemes into bunsetsu with a `ChunkerModel`: a table from feature string to weights, with the schema, the columns and the template it was trained with. See [Chunking](./chunking.md).

### The command

With the `train` feature the crate builds the `lindera-dependency` command, which tokenizes with `lindera-segmenter`:

| Subcommand | What it does | Page |
| --- | --- | --- |
| `convert` | Makes training data from a CoNLL-U file with bunsetsu labels | [Training Data](./training_data.md) |
| `evaluate` | Compares an output with the answer | [Evaluation](./evaluation.md) |
| `split` | Writes the folds of a cross-validation | [Evaluation](./evaluation.md) |
| `train-chunker` | Trains the model of the chunker | [Chunking](./chunking.md) |
| `chunk` | Segments the morphemes of a file again with a model | [Chunking](./chunking.md) |

The command is in this crate, not in `lindera-cli`: `lindera-cli` is published, and cannot depend on a crate that is not.

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| (none) | The types, the lattice format, the schema and the chunker. No dependency; builds for `wasm32-unknown-unknown` | Yes |
| `train` | The `corpus` and `evaluation` modules, the training of the chunker with `lindera-crf`, and the `lindera-dependency` command, which tokenizes with `lindera-segmenter` | No |
| `embed-ipadic` | Embeds IPADIC in the command, for `--dict embedded://ipadic` (implies `train`) | No |

The crate depends on `lindera-segmenter` at most, never on `lindera` or `lindera-analysis`: the token filter that is to come will make `lindera-analysis` depend on this crate.
