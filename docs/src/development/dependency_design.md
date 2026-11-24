# Design Notes: Dependency Analysis

These are the design notes of `lindera-dependency`, the crate for Japanese bunsetsu dependency analysis ([#1417](https://github.com/lindera/lindera/issues/1417)). They record what was decided and why, what the reference implementations do, and every number the decisions rest on, with the way to get it again.

The crate is under development and not published. How to use what exists is in the [Lindera Dependency](../lindera-dependency.md) chapter and in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md); this page is for the people who change it.

**Who updates this page**: the pull request that changes a decision or a number changes this page with it, in English and in Japanese.

## Purpose

A dependency analysis groups the morphemes of a sentence into bunsetsu (文節) and finds, for each bunsetsu, the bunsetsu it depends on.

```text
彼は 本を 読んだ。
彼は ──────┐
本を ──────┤
読んだ。 ◀──┘   (root)
```

The established tools, CaboCha and J.DepP, are written in C++ and run on the output of MeCab. `lindera-dependency` does the same in Rust, on the tokens of Lindera or on any sequence of morphemes.

The uses in view are the ones where throughput and ease of embedding matter more than the last point of accuracy: indexing pairs of bunsetsu for search, extracting (subject, object, predicate) triples for retrieval corpora and knowledge bases, and rule matching.

Not in the first release: named entities, predicate-argument structure, the type of coordination, neural models. Korean is considered only after the Japanese model is out and on request; Chinese is out of scope.

## Decisions

Each decision has the issue that made it. A decision is changed by a pull request that also changes this table.

| Decision | Why | Issue |
| --- | --- | --- |
| The crate is `lindera-dependency`, a member of this workspace. Chunking, dependency parsing and, later, named entities live in this one crate. | Lindera keeps its bindings, command and documentation in one repository; a model is released on its own schedule, which the model file absorbs, not the crate. | #1417 |
| The core takes a sequence of morphemes (a surface and feature columns) and depends on no tokenizer. | A type outside Lindera can be analyzed, and the default build has no dependency and builds for `wasm32-unknown-unknown`. | #1417, #1418 |
| The crate depends on `lindera-segmenter` at most, behind a feature, never on `lindera` or `lindera-analysis`. | The token filter will make `lindera-analysis` depend on this crate; the other direction would be a cycle. | #1419 |
| The bunsetsu are those of UD Japanese-GSD (NINJAL's standard, as in BCCWJ-DepPara), not those of the Kyoto corpus. | The training data is UD Japanese-GSD, which has the bunsetsu boundaries and a license that allows a model to be released. | #1417 |
| Coordination is in the form of the Kyoto corpus and CaboCha: each conjunct depends on the next, with the arc type `P`. | UD attaches later conjuncts to the first, with an arc to the left, which the parsers cannot produce. | #1417, #1419 |
| The first model is for IPADIC; UniDic follows. A model belongs to one dictionary. | A model depends on the part-of-speech columns and on the segmentation of its dictionary. The baselines of CaboCha and of J.DepP were measured with IPADIC, so the numbers can be compared. | #1417 |
| Chunking is a conditional random field over `B` and `I`, trained with `lindera-crf`. | It is what CaboCha does, and its chunking is better than J.DepP's independent decisions (see the baselines). | #1417, #1421 |
| Chunking inference has a Viterbi of its own over a table of weights, and does not use `lindera-crf`. | `search_best_path` of `lindera-crf` needs every label of an edge registered in the model, through `&mut self`, so it does not fit sentences it has not seen; and the default build stays without a dependency. | #1417, #1421 |
| Parsing is shift-reduce with a linear binary classifier over features and pairs of features. | Both reference implementations do this; it is linear in the length of the sentence. | #1417 |
| What is known about a dictionary is in one value, the schema, as data: the columns by role, the function word patterns, the symbols. | The chunker, the parser and the output share it, and a second dictionary is a second value, not a second code path. | #1421, #1422 |
| The head of a bunsetsu is CaboCha's (the last content word), not J.DepP's. | It is the word to take for a bunsetsu when indexing or extracting; J.DepP lets an auxiliary be the head. | #1422 |
| A model holds what makes its features: the schema, the columns, the template; for the parser, the feature set. | Training and inference cannot disagree. | #1421, #1422, #1436 |
| The features of the parser are strings with CaboCha's names (`SHS:`, `DFP0:`, `GCASE:`, `a:`), made by one function for parsing and for training instances. | A model of CaboCha can be converted without renaming its features; a feature of training cannot differ from the one of parsing. | #1436 |
| Which groups of features a model uses is data (a feature set), not code. | Feature sets are compared without a change of code, as templates of the chunker are. | #1436 |
| The particles, and the function words taken by their surface as the feature of a child, are patterns of the schema. | The parser names no part of speech and no column. The schema is in the bytes of a model. | #1436 |
| The classifier is behind an interface in two steps: a number for a feature string, a score for the numbers of a decision. | The strings of a bunsetsu are looked up once per sentence, as in CaboCha; how weights are stored can change without the parser. | #1436 |
| A dictionary has one model file: the name of the dictionary, its schema once, and the trained parts (the chunker, the parser). | A user needs one file for a dictionary, not two, and the parts cannot disagree on the schema. A model is released on its own schedule, so the file has a version of its own. | #1437 |
| A model file is read in place, by a type that is generic over what holds the bytes, and is checked once, when it is opened. | Bytes embedded in a program, a file read into memory and a memory map of the caller are one code path, without a dependency and on `wasm32-unknown-unknown`. After the check nothing fails or panics. No number is read through a pointer, so the alignment of the bytes does not matter, as for `matrix.mtx` in #1364. | #1437 |
| A table of strings in a model file is stored sorted, with a hash table over it. | The strings are found in the time of a hash, where they are; sorted, the same strings always give the same bytes. A binary search over the sorted strings alone was measured and is slower (see the numbers). | #1437 |
| The dictionary of a model is checked twice: by its name, by the caller that knows the dictionary of its morphemes; and by the number of feature columns, always. | A morpheme with fewer columns than the schema reads cannot be of the dictionary, and is refused instead of analyzed. Morphemes of another dictionary that are wide enough cannot be told by their shape; only the name tells them. | #1437 |
| The chunker-only file of before the model file (magic number `LDCHUNK`) is not read any more. | The crate is not published and no model is distributed; a second reader would be kept for no file that exists. | #1437 |
| Training data holds no head word and function word positions; what reads it computes them from the schema. | A change of the rule cannot leave stale positions in a file. | #1422 |
| Settings are compared by cross-validation on the train file; the dev file chooses between close settings; the test file and PUD are for the reported result only. | Dev and test are small: the 95% interval of an accuracy near 90% is about ±1 point on them. | #1420 |
| The accuracy targets are the ones under "Targets" below. | They are the baselines measured on the same data. | #1417 |
| A token filter writes its result as four string columns at the end of `Token::details`: bunsetsu id, head bunsetsu id (-1 for the root), position in the bunsetsu, score. | It needs no change of `Token`. See the open questions. | #1417 |
| The crate will read the models CaboCha distributes through a converter, after the first release. Those models are not redistributed. | Feature extraction is separate from decoding for this. | #1417, #1421 |

## The reference implementations

Read from the sources: CaboCha 0.69 (taku910/cabocha, commit 96e100a) and J.DepP 2022-03-18.

- **Parsing.** Both parse with shift-reduce (Sassano 2004): one binary decision, "does the bunsetsu on top of the stack depend on the current one", linear time, on the assumptions that a head is always to the right and that arcs do not cross.
- **Classifier.** Both classify over first-order features and pairs of them. CaboCha's "polynomial kernel SVM" is expanded into a linear model over features and feature pairs, for training and for inference.
- **Chunking.** CaboCha chunks with a CRF (CRF++, labels `B` and `I`) over two columns per morpheme: the normalized surface, and the part of speech as its levels joined by `-`. J.DepP decides each morpheme boundary independently. In CaboCha the first morpheme always starts a bunsetsu.
- **Head and function word** (`src/selector.cpp`, `src/selector_pat.h`). `findHead` starts both at the first morpheme of the bunsetsu, scans from the left, and overwrites the function word position when the feature string starts with a function word pattern, and the head position when it starts with none of the head patterns. For IPA and UniDic, a head after the function word moves the function word to it. For IPA the function word patterns are 助詞, 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立, 形容詞,接尾; those and 記号 are not heads.
- **Punctuation and brackets** are matched on the normalized surface: 。 and 、, ten opening and ten closing brackets.
- **Evaluation** (`src/eval.cpp`). Chunking is precision, recall and F over bunsetsu with the same start and length. Dependency accuracy is counted on the sentences whose segmentation is the one of the answer, at three levels: every bunsetsu, every one but the last of the sentence, every one but the last two.
- **Projection** (`tools/kc2cabocha.pl`). CaboCha's IPA model was trained on the Kyoto corpus tokenized again with IPADIC: bunsetsu are laid over the new morphemes by character offsets, and merged where a boundary falls inside a morpheme.
- CaboCha reads the conjugation form at "number of part-of-speech levels + 1". For UniDic that is the fourth part-of-speech column, so its UniDic model has no conjugation form. Column numbers are stated per dictionary here, not copied.

## What exists, and why it has this shape

### Types and the lattice format ([#1418](https://github.com/lindera/lindera/issues/1418))

- `Morpheme` is a trait: a surface and feature columns by index. `Sentence` holds morphemes and `Bunsetsu`, each with its head, arc type (`D`, `P`, `A`, `I`), the positions of its head word and function word, and a score.
- The lattice format of CaboCha (`cabocha -f1`) is read and written, since it is what CaboCha's and J.DepP's tools exchange. The arc type is kept; CaboCha discards it on reading.
- `Sentence::violations` reports the ways a sentence is not the tree the parsers assume: one root that is the last bunsetsu, every head to the right, no crossing arcs. The corpus conversion and the evaluation share it.

### Training data ([#1419](https://github.com/lindera/lindera/issues/1419))

`lindera-dependency convert` makes training data from UD Japanese-GSD and UD Japanese-PUD in two steps.

1. **Word dependencies to bunsetsu dependencies.** A word with `BunsetuBILabel=B` starts a bunsetsu; the word whose head is outside the bunsetsu gives the head of the bunsetsu; coordination is put back (`a → b`, `b → c`, `c → h`, with `P`); a sentence that is not a tree is dropped.
2. **Projection onto the morphemes of the dictionary.** The text is tokenized with the dictionary and the default settings of Lindera, as the parser will be used. Bunsetsu are laid over the morphemes by character offsets; where a boundary falls inside a morpheme, the bunsetsu are merged up to the next boundary that fits, and the merged one takes the head of the last one, as `kc2cabocha.pl` does.

The command is a subcommand of the crate's own binary, behind the `train` feature: `lindera-cli` is published and cannot depend on a crate that is not.

### Evaluation ([#1420](https://github.com/lindera/lindera/issues/1420))

`lindera-dependency evaluate` counts what `cabocha-system-eval` and J.DepP count, so that numbers can be set next to theirs. Level 1 (every bunsetsu but the last of each sentence) is the number J.DepP reports and the headline number here. End to end counts, over the bunsetsu of the answer but the last of each sentence, those for which the output has a bunsetsu with the same span whose head has the span of the head in the answer.

Two outputs are compared in pairs with McNemar's exact test (binomial, two-sided), not the chi-square approximation: the discordant counts are small. `lindera-dependency split` writes the folds of a cross-validation as "sentence `i` is in the test file of fold `i mod k`", without random numbers, so the folds are the same everywhere.

### Chunker ([#1421](https://github.com/lindera/lindera/issues/1421))

- **Feature extraction is separate from decoding.** Columns are read from a morpheme through the schema, a template in the format of CRF++ expands them into feature strings, and the decoder sees only feature strings and a table of weights. An extractor with CaboCha's strings can later feed the same decoder.
- **Training** gives `lindera-crf` one edge per label at each morpheme, the gold one first, and only `B` at the first morpheme. The weights that are not 0 are copied out of the trained model. The decoder sums in the order `search_best_path` does and breaks ties the same way; it gives the same labels on every sentence of dev and test.
- **The defaults** were chosen by 10-fold cross-validation on the train file: the surface, the part of speech and the conjugation form; CaboCha's template with five lines for the conjugation form; L1 with the coefficient 0.5. It is the smallest of the settings that do not differ significantly from the best one. The conjugation form helps; the conjugation type does not; the base form adds nothing to the conjugation form; L1 is better than L2 with a model about fifty times smaller.

### Schema, head and function word ([#1422](https://github.com/lindera/lindera/issues/1422))

- A `Schema` names the columns of the part of speech, the conjugation type, the conjugation form and the base form, and holds the function word patterns, the patterns of what is not a head, and how symbols, commas, periods and brackets are recognized. A pattern is a prefix of the part-of-speech levels, optionally with base forms and with a pattern for the morpheme before it in the bunsetsu.
- `Schema::positions` follows `findHead` of CaboCha.
- **Symbols are recognized by the surface as well as by the part of speech.** IPADIC has no half-width symbols: `,` is an unknown word and comes out as 名詞,サ変接続.
- **The schema of IPADIC adds to CaboCha's patterns** what the agreement with the content head of UD Japanese-GSD supports: する and できる are function words; 名詞,非自立, the suffixes 的 and さ, and a verb right after a particle are not heads. The largest systematic disagreement of CaboCha's rule is the verbal noun with する (利用 / し / た), where it makes し the head.
- The function word is defined by the schema alone. UD Japanese-GSD has no label that means the same: its `SYN_HEAD` matches neither "the last morpheme that is not a symbol" nor "the morpheme after the head".

### Parser ([#1436](https://github.com/lindera/lindera/issues/1436))

- **The loop** is `parseShiftReduce` of CaboCha and `_parseLinear` of J.DepP: a stack of the bunsetsu without a head, each following bunsetsu as the target, the top of the stack as the source. An arc to the last bunsetsu is not asked. The loop takes the decision as a function, so the classifier and the answer of a sentence run the same code.
- **The features** are CaboCha's (`src/selector.cpp`, `src/dep.cpp`), in seven groups: the distance, the source, the target, the bunsetsu before the source, the one after the target, the bunsetsu already attached, and the gap. They are made once for each bunsetsu and put together for a decision, generic over what a string is turned into: the number of the classifier when parsing, the string itself for training instances.
- **Differences from CaboCha**: the surface is not normalized, and punctuation and brackets are those of the schema; the head and the function word are those of the schema of #1422; a bunsetsu without a known head is not supported; an arc that is not asked has the score 0, where CaboCha writes the score of the decision before.
- **J.DepP's features** (`jdepp/pdep.cc`) differ: four bunsetsu by the first morpheme, the head, the tail and the last one; whether two heads have the same surface; the balance of brackets; the particles of the bunsetsu in between that compete for the target; nothing of the bunsetsu already attached. What only J.DepP has is left for the comparison of feature sets.
- **`LinearClassifier`** adds a bias, the weights of the features and the weights of their pairs, from hash tables. It is the least that parses with weights made by hand; the storage of trained weights is decided with the parser part of the model file.
- **Training instances** are the decisions the loop asks when the answer decides, each with its features and its answer, as in CaboCha's training mode. The answer gives the tree of every sentence of the training data back.

### Model file ([#1437](https://github.com/lindera/lindera/issues/1437))

- **One file per dictionary.** It starts with a magic number and a version, names its dictionary, holds the schema, then a list of parts and the parts. The chunker is the first part; the parser will be the second. A part can be absent: `train-chunker` writes a file with the chunker only.
- **Read in place.** `Model<B>` holds any `B: AsRef<[u8]>` and keeps, besides the bytes, only what is small and structured: the name, the schema, the columns and the template of the chunker, its nine transition weights, and where its table is. The weights of the feature strings stay in the bytes and are read with `from_le_bytes`.
- **Checked once.** `Model::new` reads every field, every offset and every string, and looks every string up through its slots. What is read afterwards goes through checked slices, so a model that was made cannot panic; a test changes every byte of a file in turn to hold this.
- **The chunker reads through a trait** (`ChunkerData`), as the parser reads its classifier: the model in memory that training gives, and the part of a file, are two implementations, and the decoder is the same code. They give the same labels on every sentence of dev, test and PUD.
- **The table of strings** is the piece the parser will share for the numbers of its features. It is sorted and has slots, a hash table with open addressing over the 64-bit FNV-1a of a string, with at least two slots per string. Most feature strings of a sentence are not in a model (L1 leaves 4,325 of them), so what matters is how fast a string is not found: with at most half of the slots taken, the search ends at an empty slot after one or two steps.
- **`Chunker::chunk` and `Parser::parse` return an error** for a morpheme with fewer feature columns than the schema reads. Before, such a morpheme was analyzed with empty columns.
- **Not done here**: the parser part, quantization (the weights of the chunker are `f64`, so that it gives the labels of `lindera-crf`), and a checksum. A file that was cut is found by its lengths; a changed weight is not found.

## What is to come

| Part | Notes | Issue |
| --- | --- | --- |
| Training of the parser, and its part of the model file | The algorithm and the feature set chosen by cross-validation; how the weights of the pairs are stored; quantization; size and speed. The model file itself and the check of the dictionary are done. | [#1437](https://github.com/lindera/lindera/issues/1437) |
| Token filter | And its use from the configuration file and the bindings. | [#1438](https://github.com/lindera/lindera/issues/1438) |
| Command | Output compatible with `cabocha -f1`. | [#1439](https://github.com/lindera/lindera/issues/1439) |
| IPADIC model | Training, release, documentation, license notes. The crate is published with it. | [#1440](https://github.com/lindera/lindera/issues/1440) |
| UniDic model | A second schema. | [#1441](https://github.com/lindera/lindera/issues/1441) |
| Converter for the models of CaboCha | The normalization table of the surface, the reader of a CRF++ text model, and the expansion of the support vectors of the parser model. | [#1442](https://github.com/lindera/lindera/issues/1442) |
| The baselines of CaboCha and J.DepP, measured again | The scripts that build and train them. | [#1433](https://github.com/lindera/lindera/issues/1433) |

### Open questions

- **How the filter is named in a configuration.** Use from the configuration file, the command and the bindings needs a change in `lindera-analysis`: `TokenFilterLoader::load_from_value` is a fixed `match` on the `kind`. Either the filter lives in `lindera-analysis` behind a feature and depends on the core of this crate, or `lindera-analysis` gets a registry of filters.
- **Typed fields for the result on `Token`**, instead of string columns in `details`, which `as_value()` does not show.
- **Bunsetsu ids restart at 0 in every sentence**, so they collide when one input holds several sentences; a fifth column with the sentence number would solve it.
- **The training algorithm of the parser** (passive-aggressive as in J.DepP, or dual coordinate descent as in CaboCha) and how the weights are stored (feature hashing or an exact table). Both are to be measured.
- **More training data** under a usable license.

## Targets

On UD Japanese-GSD with IPADIC, from the baselines below:

| | dev | test |
| --- | --- | --- |
| Chunking F | 96.8 or more | 96.8 or more |
| Dependency accuracy with the bunsetsu of the answer (level 1) | 89.0 or more | 87.8 or more |
| End-to-end dependency accuracy | 85.0 or more | 84.3 or more |

The dependency target is J.DepP less one point; the other two are CaboCha's numbers. The same numbers on UD Japanese-PUD are reported. Chunking is met: 96.85 and 96.92 (#1421).

Besides the accuracy: inference builds for `wasm32-unknown-unknown`; the parser runs as a token filter and as a command; the size of the model and the throughput are measured and documented; the model is released with the provenance and the license of its training data.

## Numbers

Every number the decisions rest on, with where it comes from. The data is UD Japanese-GSD c662dc9 and UD Japanese-PUD cf69c5c, tokenized with the IPADIC of this repository.

"Script" means a script of [`scripts/dependency`](https://github.com/lindera/lindera/tree/main/scripts/dependency), whose README has the commands. **"Prototype, lost" means the number came from prototypes outside this repository that no longer exist: it cannot be reproduced with what is here**, and is to be measured again before a decision is changed on its strength.

### Training data

| Number | Value | From |
| --- | --- | --- |
| Bunsetsu of GSD with exactly one word whose head is outside | 65,966 of 65,966 | Script: `conversion_stats.py` |
| Sentences of GSD kept after coordination is put back | 8,072 of 8,100 (train 7,024, dev 506, test 542) | Script: `conversion_stats.py`; `convert` |
| Sentences of GSD train kept without putting coordination back | 6,549 of 7,050 (92.89%) | Script: `conversion_stats.py` |
| Sentences of PUD kept | 994 of 1,000 | Script: `conversion_stats.py`; `convert` |
| Bunsetsu boundaries of GSD that are morpheme boundaries | 99.20% (56,980 of 57,439) | `convert` (`convert_corpora.sh`), the three files together |
| Bunsetsu of GSD lost by merging | 0.70% (459 of 65,511) | `convert`, the three files together |
| Bunsetsu written (train, dev, test, PUD) | 56,396, 4,147, 4,509, 9,843 | `convert` |
| The lattice files of an independent implementation | The same lines as `convert` writes | Script: `projection_stats.py`, `compare_outputs.py` |

### Baselines

| Number | Value | From |
| --- | --- | --- |
| "Every bunsetsu depends on the next", level 1 | dev 64.10% (2,334 / 3,641), test 62.77% (2,490 / 3,967), PUD 60.87% (5,386 / 8,849) | `evaluate`; script: `baseline_next.py` |
| CaboCha: chunking F, dependency with the bunsetsu of the answer, end to end | dev 96.82 / 89.84 / 85.03, test 96.80 / 88.51 / 84.25, PUD 97.80 / 88.64 / 85.65 | Prototype, lost |
| J.DepP: the same | dev 95.93 / 89.98 / 83.80, test 95.37 / 88.76 / 82.40 | Prototype, lost |
| Arcs on which CaboCha and J.DepP differ | 5 of 3,641 on dev, 10 of 3,967 on test | Prototype, lost |
| Gain of dependency accuracy per doubling of the training set | About 0.8 point, not levelling off at 7,024 sentences | Prototype, lost |
| CRF++ and `lindera-crf` with CaboCha's template, L2 0.25: chunking F | 96.79 / 96.77 on dev / test | Prototype, lost; `chunker_experiments.sh table` gives 96.79 / 96.80 with `lindera-crf` |

The baselines of CaboCha and J.DepP were trained on the GSD train file, on the morphemes of Lindera 7.0.0 with IPADIC. CaboCha's chunker regularization (`-c 4`) was chosen on dev; J.DepP ran with the options its page recommends, untuned.

The baseline on PUD was 5,385 in #1417 and is 5,386 here: in one sentence of PUD (`w01040101`) a bunsetsu has two words whose head is outside it, and the conversion takes the last one.

### Chunker

| Number | Value | From |
| --- | --- | --- |
| Chunking F with the defaults | dev 96.85, test 96.92, PUD 98.04 | Script: `chunker_experiments.sh default` |
| Size of the default model | 4,325 feature strings, 8,656 weights | Script: `chunker_experiments.sh default` |
| F over the 10 folds of train, CaboCha's columns, L1 0.25 / 0.5 / 1.0 | 96.83 / 96.76 / 96.62 | Script: `chunker_experiments.sh cv` |
| The same, with the conjugation form (13 lines) | 96.98 / 96.94 / 96.86 | Script: `chunker_experiments.sh cv` |
| The same, with the conjugation form (5 lines, the default template) | 96.95 / 96.93 / 96.82 | Script: `chunker_experiments.sh cv` |
| CaboCha's columns against the default, by sentence | 74 / 107, p = 0.0171 | Script: `chunker_experiments.sh compare` |
| The best setting against the default, by sentence | 72 / 55, p = 0.1554 | Script: `chunker_experiments.sh compare` |
| Weights with L1 and with L2, CaboCha's template, 0.25 | 10,800 and 603,822 | Script: `chunker_experiments.sh table` |
| The decoder against `search_best_path` | The same labels on the 1,048 sentences of dev and test | The ignored test of `lindera-dependency/src/chunker/train.rs` |
| Bunsetsu boundaries that fit, per set of break characters | — | Prototype, lost |
| Decoding with a CRF++ text model against CaboCha's output | — | Prototype, lost |

The README of the crate has the whole table of the cross-validation and the ten pairs.

### Head word

| Number | Value | From |
| --- | --- | --- |
| Head word on the content head of GSD, CaboCha's rule for IPA | train 85.43%, dev 86.30%, test 84.61%, PUD 86.70% | Script: `head_variants/` |
| The same, `Schema::ipadic()` | train 96.55% (54,449 / 56,396), dev 96.96%, test 96.92%, PUD 97.46% | `convert`; script: `head_variants/` |
| Every suffix (名詞,接尾) left out of the heads | train 87.02% | Script: `head_rules.py table` |
| Counters (名詞,接尾,助数詞) left out of the heads | train 93.45% | Script: `head_rules.py table` |
| 名詞,非自立,副詞可能 kept as a head | train 96.60%, dev 97.13%, test 96.83% | Script: `head_rules.py table` |
| UniDic on GSD's own tags: CaboCha's pattern, with 非自立可能, with compound function words | train 83.89%, 96.17%, 98.06% | Prototype, lost |
| `SYN_HEAD` of GSD against "last morpheme that is not a symbol" and "the morpheme after the head" | 87–88% and 81–82% | Prototype, lost |

### Parser

| Number | Value | From |
| --- | --- | --- |
| Sentences whose tree comes back when the answer decides | train 7,024 of 7,024, dev 506 of 506, test 542 of 542, PUD 994 of 994 | Script: `parser_instances.sh` |
| Training instances, CaboCha's feature set | train 66,000 (33,512 positive), dev 4,881 (2,474), test 5,377 (2,707), PUD 12,739 (6,077) | Script: `parser_instances.sh` |

### Model file

The default chunker model, on one core of an AMD Ryzen AI Max+ 395, with Rust 1.100.0. Chunking is measured on the 7,024 sentences of the train file, the best of seven rounds.

| Number | Value | From |
| --- | --- | --- |
| Model file with the default chunker | 261,851 bytes, 4,325 feature strings; the same bytes when trained twice; F unchanged: 96.85, 96.92, 98.04 | Script: `chunker_experiments.sh default` |
| The chunker on a model file against the chunker-only file of before | The same lines in the outputs for dev, test and PUD | `chunk` before and after #1437, compared with `cmp`; not kept as a script |
| Time to read the file, with every check | 228 microseconds | Script: `model_speed/` |
| Chunking with the model in memory (a hash table of the standard library) | 18,223 sentences per second | Script: `model_speed/` |
| Chunking with the model on the bytes of the file | 16,896 sentences per second | Script: `model_speed/` |
| The same, with a binary search over the sorted strings and no slots | 11,103 sentences per second, against 18,274 in memory in the same run; 196,311 bytes | A variant of the code of #1437, not kept |
| The same, with three slots for two strings, modulo their number | 14,821 sentences per second, against 18,326 in memory in the same run; 222,267 bytes | A variant of the code of #1437, not kept |

### Lindera

| Fact | From |
| --- | --- |
| `search_best_path` of `lindera-crf` needs every edge label registered in the model (`feature_provider()` takes `&mut self`) | Read in `lindera-crf/src/model.rs`; #1421 |
| In Lindera 7.0.0 the details of an unknown word are padded with `*` to the width of the schema of the dictionary | Read in `lindera-segmenter/src/token.rs`; the morphemes of the converted data have 9 columns |
| A filter receives `Token::details` as `None`; columns pushed past the schema show in `details()`, `details_iter()` and `get_detail(i)`, not in `as_value()` | Prototype, lost |
| A token filter of another crate can be added from Rust (`Tokenizer::append_token_filter`), not by `kind` | Prototype, lost; `TokenFilterLoader::load_from_value` is a fixed `match` |

## Order of work

| Issue | What | State |
| --- | --- | --- |
| [#1418](https://github.com/lindera/lindera/issues/1418) | The crate, the core types and the lattice format | Done |
| [#1419](https://github.com/lindera/lindera/issues/1419) | Training data from UD Japanese-GSD | Done |
| [#1420](https://github.com/lindera/lindera/issues/1420) | Evaluation | Done |
| [#1421](https://github.com/lindera/lindera/issues/1421) | Chunker | Done |
| [#1422](https://github.com/lindera/lindera/issues/1422) | IPADIC schema, head and function word | Done |
| [#1423](https://github.com/lindera/lindera/issues/1423) | These notes and the measurement scripts | Done |
| [#1434](https://github.com/lindera/lindera/issues/1434) | The chapter of the documentation | Done |
| [#1436](https://github.com/lindera/lindera/issues/1436) | Dependency parser: the loop, the features, the training instances | Done |
| [#1433](https://github.com/lindera/lindera/issues/1433) | The baselines of CaboCha and J.DepP, measured again | Open |
| [#1437](https://github.com/lindera/lindera/issues/1437) | The model file (done), and the training of the parser | In progress |
| [#1438](https://github.com/lindera/lindera/issues/1438) | Token filter | Open |
| [#1439](https://github.com/lindera/lindera/issues/1439) | Command | Open |
| [#1440](https://github.com/lindera/lindera/issues/1440) | IPADIC model, and the publication of the crate | Open |
| [#1441](https://github.com/lindera/lindera/issues/1441) | UniDic model | Open |
| [#1442](https://github.com/lindera/lindera/issues/1442) | Converter for the models of CaboCha | Open |

## References

- CaboCha: <https://github.com/taku910/cabocha> (`src/dep.cpp`, `src/chunker.cpp`, `src/selector.cpp`, `src/svm_learn.cpp`, `src/eval.cpp`, `tools/kc2cabocha.pl`); 3-clause BSD or LGPL
- J.DepP: <https://www.tkl.iis.u-tokyo.ac.jp/~ynaga/jdepp/>
- Sassano, "Linear-Time Dependency Analysis for Japanese", COLING 2004
- UD Japanese-GSD: <https://github.com/UniversalDependencies/UD_Japanese-GSD> (CC BY-SA 4.0); UD Japanese-PUD: <https://github.com/UniversalDependencies/UD_Japanese-PUD> (CC BY-SA 3.0)
