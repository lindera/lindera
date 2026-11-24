# Model File

A model file holds what one dictionary needs to be analyzed: the name of the dictionary, its [schema](./head_and_function_word.md), and the trained parts: the [chunker](./chunking.md) and the [parser](./parsing.md).

A model belongs to one dictionary. It depends on the feature columns of the dictionary and on how the dictionary segments a text, so the file says which dictionary it is for, and one file serves one dictionary.

No model is distributed yet. [`train-chunker`](./chunking.md#training-a-model) writes a model file with the chunker, and [`train-parser`](./parsing.md#training-a-model) one with the parser, or with both.

## Reading a model

`Model` reads a file in place. It is generic over what holds the bytes, and copies none of its tables:

```rust
use lindera_dependency::model::Model;

// Bytes read from a file: the model owns them.
let model = Model::new(std::fs::read("ipadic.model")?)?;

assert_eq!(model.dictionary(), "ipadic");
let chunker = model.chunker().expect("the file has a chunker");
let parser = model.parser().expect("the file has a parser");
```

| What holds the bytes | How |
| --- | --- |
| A file read into memory | `Model::new(std::fs::read(path)?)` gives a `Model<Vec<u8>>` |
| A file embedded in the program | `Model::new(include_bytes!("ipadic.model").as_slice())` gives a `Model<&'static [u8]>` |
| A memory map | `Model::new(map)` for any type that implements `AsRef<[u8]>`, such as the map of the `memmap2` crate. This crate has no dependency and maps nothing itself |

The whole file is checked once, in `Model::new`: bytes that are not a model file of this version give an `Error::Model`, and nothing fails or panics afterwards. No number is read through a pointer, so the bytes can sit at any address, aligned or not. The same code runs on `wasm32-unknown-unknown`.

`Model::chunker` gives a `Chunker` that reads its weights on the bytes of the model, or `None` when the file has no chunker part; `Model::parser` gives a `Parser` the same way. `Model::analyze` chunks and parses a sentence in one call. A `Model` is `Send + Sync` when its bytes are, and one model serves several threads.

## Checking the dictionary

A model used on the morphemes of another dictionary gives wrong answers without an error, unless it is checked. There are two checks.

**The name.** What knows the dictionary of its morphemes compares its name with the one of the model:

```rust
model.check_dictionary("ipadic")?;
assert!(model.check_dictionary("unidic").is_err());
```

```text
the model is for the dictionary "ipadic", and the morphemes are of "unidic"
```

**The number of feature columns.** A morpheme with fewer feature columns than the schema of the model reads is refused when it is chunked or parsed, with an `Error::Dictionary`. The schema of IPADIC reads seven columns: four levels of part of speech, the conjugation type, the conjugation form and the base form.

```rust
use lindera_dependency::lattice::LatticeMorpheme;

// Three feature columns: not a morpheme of IPADIC.
let morphemes = [LatticeMorpheme::new("彼", "名詞,代名詞,一般")];
let error = chunker.chunk(&morphemes).unwrap_err();
println!("{error}");
```

```text
the morpheme 1 ("彼") has fewer than the 7 feature columns the schema reads: it is not of the dictionary of the model
```

The second check does not find morphemes of another dictionary that are wide enough: the morphemes of UniDic have more columns than IPADIC, and mean something else. Only the name tells those apart, so a caller that knows its dictionary should check the name.

## Writing a model

`ModelBuilder` writes a file from the name of the dictionary, its schema and the parts. The chunker part comes from a `ChunkerModel` and the parser part from a `ParserModel`, which are what training gives ([of the chunker](./chunking.md#training-a-model), [of the parser](./parsing.md#training-a-model)). This chunker is made by hand:

```rust
use std::collections::HashMap;

use lindera_dependency::chunker::{ChunkerModel, Column, Template};
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::Schema;

// A chunker made by hand, with one weight: a morpheme after a particle
// starts a bunsetsu.
let mut features = HashMap::new();
features.insert("U11:助詞-係助詞".to_string(), [1.0, 0.0]);
let transitions = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.5], [0.0, 0.0, 0.5]];
let chunker = ChunkerModel::new(
    Schema::ipadic(),
    vec![Column::Surface, Column::PartOfSpeech],
    Template::cabocha(),
    transitions,
    features,
)?;

let bytes = ModelBuilder::new("ipadic", Schema::ipadic())?
    .chunker(&chunker)?
    .to_bytes();
std::fs::write("chunker.model", &bytes)?;

let model = Model::new(bytes)?;
assert_eq!(model.dictionary(), "ipadic");
```

The same parts give the same bytes. A part trained with another schema than the one of the file is refused.

`ModelBuilder::parser` takes a `ParserModel` and how its weights are written (`Quantization::F32`, `I16` or `I8`). `ModelBuilder::from_model` starts from the name, the schema and the parts of a file that exists, to add a part to it; `train-parser --chunker` does this with the chunker.

## The format

Version 1. Every number is little endian. A string is its length in bytes as a `u32`, followed by its UTF-8.

| Field | Type | Content |
| --- | --- | --- |
| Magic number | 8 bytes | `LDMODEL` and a zero byte |
| Version | `u32` | 1 |
| Dictionary | string | The name of the dictionary, not empty |
| Schema | | The columns of the part of speech, of the conjugation and of the base form; the patterns of the function words, of what is not a head, of the particles and of the function words taken by their surface; the symbols |
| Number of parts | `u32` | |
| For each part | `u32`, `u64`, `u64` | Its kind, its offset from the first byte of the file, its length |
| The parts | | One after the other, in the order of the list, up to the end of the file |

The kind of the chunker part is 1 and the kind of the parser part 2, in this order. A file with a part of another kind, with the same kind twice, with parts that are not one after the other, or with bytes after the last part is refused, and so is a file of another version.

### The chunker part

| Field | Type | Content |
| --- | --- | --- |
| Number of columns | `u32` | |
| Columns | 1 byte each | What the template reads: 0 the surface, 1 the part of speech, 2 the conjugation type, 3 the conjugation form, 4 the base form |
| Template | string | In the format of CRF++ |
| Transitions | 9 × `f64` | As `[from][to]`: 0 is the start of the sentence as `from` and its end as `to`, 1 is `B`, 2 is `I` |
| Feature strings | table | Each with two `f64`: the weight of `B`, then the weight of `I` |

### The parser part

| Field | Type | Content |
| --- | --- | --- |
| Feature set | string | The groups of features, as `distance,source,target,left,right,children,gap,after` |
| Quantization | 1 byte | How a weight is written: 0 as an `f32`, 1 as an `i16`, 2 as an `i8` |
| Pairs | 1 byte | 0 for a weight of its own for each pair, 1 for a table the pairs are hashed into |
| Bias | `f64` | |
| Scale | `f64` | What a weight as it is written is multiplied by; above 0, and 1 for `f32` |
| Feature strings | table | Each with its weight; with hashed pairs, followed by its key as a `u32` |
| Weights of the pairs | | See below |

The score of a decision is the bias, plus the scale times the sum of the weights, as they are written, of its features and of their pairs. The number of a feature is its index in the table.

With **hashed pairs**: the number of bits as a `u32` (1 to 30), then 2^bits weights. The key of a feature is the hash of its string, the one of a table. The place of a pair is taken from the two keys, the smaller in the upper half of a 64-bit number: that number is mixed (XOR with itself shifted right by 30, times `0xbf58476d1ce4e5b9`, XOR shifted by 27, times `0x94d049bb133111eb`), and its highest bits are the place. Pairs that fall on the same place share its weight.

With **a weight for each pair**: the number of pairs as a `u64`; for each feature and once more, as `u32`, where its pairs start (those in which it has the smaller number); the larger number of each pair as a `u32`, sorted for each feature; the weight of each pair.

### A table

A table maps strings to values of one width, and is looked up where it is in the file.

| Field | Type | Content |
| --- | --- | --- |
| Number of strings | `u32` | |
| Width of a value | `u32` | In bytes |
| Number of slots | `u32` | A power of two above the number of strings |
| Offsets | (number + 1) × `u32` | Where each string starts in the strings below; the last is their length |
| Strings | | UTF-8, one after the other, sorted by their bytes, each once |
| Values | number × width | In the order of the strings |
| Slots | slots × `u32` | A hash table over the strings |

A slot is 0 when it is empty, and otherwise one more than the index of a string. A string starts at the slot of its hash and goes to the next free slot after it, the first slot following the last. The hash is the 64-bit FNV-1a of the bytes of the string, folded to 32 bits (the high half XOR the low half), modulo the number of slots. The strings are put in by their order, and a table is written with at least two slots per string.

The strings are sorted so that the same strings always give the same bytes; they are found through the slots.

## Size and speed

For the default model, the chunker and the parser trained on the train file of UD Japanese-GSD with IPADIC, measured on the 542 sentences of the test file:

| | |
| --- | --- |
| Size of the file, with both parts | 6,258,548 bytes |
| Of which the chunker, in a file of its own | 261,851 bytes |
| Time to read it (`Model::new`, with every check) | about 2.7 ms |
| Chunking | about 20,400 sentences per second |
| Parsing the bunsetsu of the answer | about 16,700 sentences per second |
| Chunking and parsing | about 8,700 sentences per second |

The table of the pairs, with 2^22 weights of one byte, is 4.2 MB of the file; the rest is mostly the 62,380 feature strings of the parser.

How the weights of the pairs are kept decides the size and the speed, not the accuracy (see [How the defaults were chosen](./parsing.md#how-the-defaults-were-chosen)):

| Pairs | Weights | Size of the file | Parsing, sentences per second |
| --- | --- | --- | --- |
| Hashed into 2^22 weights | 1 byte | 6,258,548 bytes | about 16,700 |
| Hashed into 2^22 weights | 4 bytes | 19,028,600 bytes | about 13,400 |
| A weight for each pair | 1 byte | 24,514,588 bytes | about 2,700 |
| A weight for each pair | 4 bytes | 38,162,161 bytes | about 2,700 |

The chunker alone, trained on the same data (4,325 feature strings), on the 7,024 sentences of the train file:

| | |
| --- | --- |
| Size of the file | 261,851 bytes |
| Time to read it (`Model::new`, with every check) | about 0.2 ms |
| Chunking with the chunker of the file | about 16,896 sentences per second |
| Chunking with the same chunker in memory, in a hash table of the standard library | about 18,223 sentences per second |

Measured on one core of an AMD Ryzen AI Max+ 395, with Rust 1.100.0; [`scripts/dependency/model_speed`](https://github.com/lindera/lindera/tree/main/scripts/dependency/model_speed) and `scripts/dependency/parser_experiments.sh speed` measure it again.
