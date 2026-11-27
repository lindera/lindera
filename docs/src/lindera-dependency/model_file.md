# Model File

A model file holds what one dictionary needs to be analyzed: the name of the dictionary, its [schema](./head_and_function_word.md), and the trained parts: the [chunker](./chunking.md) and the [parser](./parsing.md).

A model belongs to one dictionary. It depends on the feature columns of the dictionary and on how the dictionary segments a text, so the file says which dictionary it is for, and one file serves one dictionary.

A release of Lindera publishes the [model for IPADIC](#the-model-for-ipadic) and the [models for UniDic](#the-models-for-unidic), from 7.1.0 on. [`train-chunker`](./chunking.md#training-a-model) writes a model file with the chunker, and [`train-parser`](./parsing.md#training-a-model) one with the parser, or with both.

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

## The model for IPADIC

The project trains a model for IPADIC, with a chunker and a parser. A release of Lindera builds it with [`build_model.sh`](#building-it-again) and puts it on the page of its version, next to the dictionaries, from 7.1.0 on: `lindera-dependency-ipadic-<version>.zip` in `v<version>`. The model has the version of the release, as the dictionaries have, and belongs to it: a program of Lindera 7.1.0 takes the model of 7.1.0.

[`lindera download dependency-ipadic`](../lindera-cli/commands.md#downloading-the-model-of-the-dependency-parser) installs the archive of the version of the command, and prints the path of the model file.

The archive holds one directory, `lindera-dependency-ipadic`, with three files:

| File | What it is |
| --- | --- |
| `dependency.model` | The model file |
| `model.json` | The name of the model (`dependency-ipadic`), its dictionary (`ipadic`) and the version of Lindera (`lindera_version`) |
| `NOTICE.txt` | The license of the model, what it was made from, and the notice of IPADIC |

| | |
| --- | --- |
| Dictionary | IPADIC (`ipadic`), as the `lindera-ipadic` crate builds it from mecab-ipadic 2.7.0 |
| Training data | The train file of [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) at c662dc9, [converted](./training_data.md) with IPADIC: 7,024 sentences |
| Settings | The defaults of [`train-chunker`](./chunking.md#training-a-model) and of [`train-parser`](./parsing.md#training-a-model) |
| Size | 6,258,548 bytes |
| License | MIT, with a notice: see [License of the model](#license-of-the-model) |

Only the train file is used. The dev and test files of UD Japanese-GSD and UD Japanese-PUD are not seen in training, and no setting was chosen on them, so the [accuracy](./parsing.md#accuracy) measured on them is the accuracy of the file itself.

### Embedding it

The feature `embed-model-ipadic` of `lindera-dependency` embeds the model in the program; `lindera`, `lindera-cli` and the bindings forward it as `embed-dependency-ipadic`. The model is then named as the embedded dictionary is:

```rust
use lindera_dependency::embedded;

// The bytes of the program, read in place.
let model = embedded::model("ipadic")?;
assert_eq!(model.dictionary(), "ipadic");
```

The [command](./command_line.md) takes `--model embedded://ipadic`, and the [token filter](./token_filter.md) `model: "embedded://ipadic"`. A build without the feature answers with an error that names it.

The model is not trained by the build: the training needs the corpus. Where the build takes it from depends on where the crate comes from:

| Build | The model it embeds |
| --- | --- |
| The crate from crates.io | The archive of its release, which the crate pins by SHA-256 in a `prebuilt.json`: the build downloads it and checks it, as a dictionary crate does for its [dictionary](../development/dictionary_release.md) |
| The repository (`main`, a pull request, a git dependency) | The file [`LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`](../getting_started/environment_variables.md#lindera_build_dependency_model_ipadic_file) names. The repository has no `prebuilt.json`, and a build without the variable stops with an error that says so |
| docs.rs | An empty file: `embedded::model` gives an error |

With the variable set, a published crate embeds that file too, instead of the one it pins.

```shell
scripts/dependency/build_model.sh
export LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE=$PWD/scripts/dependency/data/model/lindera-dependency-ipadic/dependency.model
cargo build --release -p lindera-cli --features embed-ipadic,embed-dependency-ipadic
```

### Building it again

One script builds the file from the corpus and the sources of the repository:

```shell
scripts/dependency/build_model.sh
```

It gets UD Japanese-GSD at its fixed commit, builds the `lindera-dependency` command with a named toolchain and the dependencies of `Cargo.lock`, converts the train file, trains the chunker and the parser, and writes the directory of the archive and the archive to `scripts/dependency/data/model`. It takes about 15 seconds after the build. At its end it names the commit of Lindera, and then says what else it used and what it made:

```text
corpus: UD Japanese-GSD c662dc96ed5ee5ac0fc8b5e44261557e431f50e6
toolchain: rustc 1.100.0 (0ef1bc386 2026-11-09)
platform: x86_64-unknown-linux-gnu, glibc 2.43
training data: 5075c95f2518b6af791e6a9ea2bd5f9f63eeaa5b316f3801d8068d71436e5064  train.lattice
chunker: 74762e9b4f23d1dd06a304238006480264e0fea70b7a3a5c9e647c176dad59c3  chunker.model
model: 793b22e6ef2d92abf0fd91f282942acc6ebd568c22b930770ff2ff57573fbe46  lindera-dependency-ipadic/dependency.model (6258548 bytes)
archive: f688c9077d58c3ecd9ad6d770d6074f326ea8ebaccc424319d8dd9886873e715  lindera-dependency-ipadic-7.0.0.zip
```

`--check <sha256>` makes the script fail when the file it built has another SHA-256. The script refuses a tree with uncommitted changes, so that the commit it names is what the file was built from.

The same sources, corpus, toolchain and platform give the same bytes: nothing in the training depends on the time, on the number of threads or on the order of a hash table. Whether another platform gives the same bytes is not known. The training of the chunker calls `exp` and `ln_1p` of the mathematics library of the platform, and their last bit can differ from one library to another. The notice of a file (`NOTICE.txt`) names the platform it was built on. The model of a release is built by the runner of the release, so its bytes can differ from those of another machine; the SHA-256 of its archive is in the `prebuilt.json` of the published crate. The release measures the model it built on dev, test and PUD, and stops when a count is not the one of this documentation ([Dictionary Release](../development/dictionary_release.md#the-model-of-lindera-dependency)).

### License of the model

The model file is distributed under the MIT license, the license of Lindera's code. `NOTICE.txt`, which comes with the file, has the text of the license, an acknowledgement of UD Japanese-GSD and the notice of IPADIC. This is how the project handles the question; it is not legal advice.

**The question.** UD Japanese-GSD is licensed under CC BY-SA 4.0. That license asks that material "derived from or based upon" the licensed material ("Adapted Material") is shared under the same license. Neither the license nor the FAQ of Creative Commons says whether a model trained on a work is such material.

**What the model holds.** The names of features and their weights. A name is made of a part of speech, a conjugation form or the surface of one morpheme, so single words of the training sentences are in the file. No sentence is, and nothing of the annotation: no bunsetsu and no dependency. The model generates no text.

**The position.** The project takes such a model not to be an adaptation of the treebank, so that the share-alike condition does not reach it. The position rests on these:

- The Agency for Cultural Affairs of Japan, [AI と著作権に関する考え方について](https://www.bunka.go.jp/seisaku/bunkashingikai/chosakuken/pdf/94037901_01.pdf) (March 2024): a trained model is in many cases not a reproduction of the works it was trained on; it can be one when it is in a state to generate, with high probability, what resembles them. The document says of itself that it is not legally binding.
- Szkalej and Senftleben, [Mapping the Impact of Share Alike/Copyleft Licensing on Machine Learning and Generative AI](https://www.ivir.nl/publicaties/download/Share-Alike-and-ML.pdf) (2024): "the final artefact – the trained model – is unlikely to constitute adapted material", as long as it holds no protected traces of the works.
- Creative Commons, [Using CC-licensed Works for AI Training](https://creativecommons.org/wp-content/uploads/2025/05/Using-CC-licensed-Works-for-AI-Training.pdf) (2025): releasing a model under the license of its training data is "the most conservative approach", and following it "will almost certainly lead to overcompliance".
- Models trained on text under CC BY-SA are released under permissive licenses: the models of [BERT](https://github.com/google-research/bert), trained on Wikipedia, are under Apache 2.0.

**What is uncertain.** No court has decided whether a trained model is an adaptation of its training data, and others chose the other way: the [Japanese model of spaCy](https://github.com/explosion/spacy-models/releases/tag/ja_core_news_sm-3.8.0), trained on the same treebank, is under CC BY-SA 4.0. If the position turns out to be wrong, the model is distributed under CC BY-SA 4.0 from then on; the file itself does not change.

**What a user does.**

| Use | What it asks |
| --- | --- |
| Analyzing text with the model | Nothing. The project puts no condition on what an analysis gives |
| Distributing the file, or a program that embeds it | `NOTICE.txt` goes with it: the text of the MIT license and the notice of IPADIC |
| Changing the model and distributing the result | The same |

## The models for UniDic

A release of Lindera publishes a model for each of the three UniDic dictionaries too, from 7.1.0 on: `unidic` (unidic-mecab 2.1.2), `unidic-cwj` and `unidic-csj`. What is said above for IPADIC holds with the name of the dictionary in place of `ipadic`:

| | |
| --- | --- |
| Archive | `lindera-dependency-unidic-<version>.zip`, `lindera-dependency-unidic-cwj-<version>.zip`, `lindera-dependency-unidic-csj-<version>.zip`, each with one directory of the same three files |
| Download | `lindera download dependency-unidic`, `dependency-unidic-cwj` or `dependency-unidic-csj` |
| Embedding | `embed-model-unidic`, `embed-model-unidic-cwj` or `embed-model-unidic-csj` of `lindera-dependency`, forwarded as `embed-dependency-unidic` and the like; the model is then `embedded://unidic`, `embedded://unidic-cwj` or `embedded://unidic-csj` |
| File for a build from the repository | [`LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_FILE`](../getting_started/environment_variables.md#lindera_build_dependency_model_ipadic_file), `LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_CWJ_FILE` or `LINDERA_BUILD_DEPENDENCY_MODEL_UNIDIC_CSJ_FILE` |
| Building it again | [`build_model.sh --dictionary <name>`](#building-it-again), with `--dict` for the directory of the dictionary |
| Training data | The train file of UD Japanese-GSD, [converted](./training_data.md) with that dictionary: 7,024 sentences |
| Settings | The ones of [`train-chunker`](./chunking.md#with-unidic) for UniDic, and the defaults of `train-parser` |
| Size | 6,257,148 bytes for `unidic`, 6,258,600 for `unidic-cwj`, 6,267,183 for `unidic-csj` |
| License | MIT, with a notice that carries the notice of the dictionary. UniDic is under the GPL v2.0, the LGPL v2.1 or the BSD license, at the choice of the user |

A model is for one dictionary. The three UniDic dictionaries do not segment and tag alike, and the model for `unidic-cwj` is [refused](#checking-the-dictionary) with the morphemes of `unidic`. UniDic CSJ is a dictionary for spoken language, and the training data is written language.

Nothing embeds UniDic to train a model: `build_model.sh` takes the directory of the dictionary with `--dict`, or installs the one of the release of the version of the workspace with `lindera download`. A release trains each model with the dictionary it has just built, measures it on the files it was not trained on, and stops when a count is not the one of the [documentation](./parsing.md#with-unidic), as it does for IPADIC.

The four models, on the test file of UD Japanese-GSD converted with each dictionary, on one core of an AMD Ryzen AI Max+ 395; the speeds are sentences per second, from one run each, and differ by a few percent from one run to another:

| Dictionary | Size of the file (bytes) | Of which the chunker | Chunking | Parsing the bunsetsu of the answer | Chunking and parsing | From text, with the tokenizer | Loading the dictionary and the model | The token filter, microseconds per token |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| IPADIC | 6,258,548 | 261,851 | 20,500 | 17,500 | 8,900 | 7,000 | 5 ms | 5.3 |
| `unidic` | 6,257,148 | 360,869 | 13,100 | 17,400 | 6,900 | 5,300 | 59 ms | 6.4 |
| `unidic-cwj` | 6,258,600 | 360,019 | 13,400 | 16,400 | 6,900 | 4,500 | 388 ms | 6.9 |
| `unidic-csj` | 6,267,183 | 365,215 | 11,900 | 15,000 | 6,500 | 4,100 | 389 ms | 7.2 |

The chunker for UniDic reads four columns through a template of 36 lines, against three columns and 24 lines for IPADIC, and is slower by about a third. From text, most of the difference is the tokenizer: UniDic CWJ and UniDic CSJ are loaded from a directory of 1.3 GB.

## The format

Version 3. Every number is little endian. A string is its length in bytes as a `u32`, followed by its UTF-8.

| Field | Type | Content |
| --- | --- | --- |
| Magic number | 8 bytes | `LDMODEL` and a zero byte |
| Version | `u32` | 3 |
| Dictionary | string | The name of the dictionary, not empty |
| Schema | | The columns of the part of speech, of the conjugation and of the base form; the patterns of the function words, of what is not a head, of the particles and of the function words taken by their surface; the symbols, with one byte for how a surface is matched against their characters (0: every character of it is one of them; 1: it is one of them, and a mark by its surface is still a word); the normalization of the surface, as the number of its rules and for each two strings, what is replaced and by what, sorted; one byte for the rule of the head word (0: a function word is not the head, and the function word is moved to a head after it; 1: the last morpheme that is not a symbol is the function word, and the two are found apart) |
| Number of parts | `u32` | |
| For each part | `u32`, `u64`, `u64` | Its kind, its offset from the first byte of the file, its length |
| The parts | | One after the other, in the order of the list, up to the end of the file |

The kind of the chunker part is 1 and the kind of the parser part 2, in this order. A file with a part of another kind, with the same kind twice, with parts that are not one after the other, or with bytes after the last part is refused, and so is a file of another version. Version 1 had no rule for the surface of a mark and no normalization, and version 2 no rule of the head word; neither was in a release.

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
