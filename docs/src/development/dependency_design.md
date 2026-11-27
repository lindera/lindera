# Design Notes: Dependency Analysis

These are the design notes of `lindera-dependency`, the crate for Japanese bunsetsu dependency analysis ([#1417](https://github.com/lindera/lindera/issues/1417)). They record what was decided and why, what the reference implementations do, and every number the decisions rest on, with the way to get it again.

The crate is published from Lindera 7.1.0 on. How to use it is in the [Lindera Dependency](../lindera-dependency.md) chapter and in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md); this page is for the people who change it.

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
| The crate depends on `lindera-segmenter` at most, behind a feature, never on `lindera` or `lindera-analysis`. | The token filter makes `lindera-analysis` depend on this crate; the other direction would be a cycle. | #1419 |
| The bunsetsu are those of UD Japanese-GSD (NINJAL's standard, as in BCCWJ-DepPara), not those of the Kyoto corpus. | The training data is UD Japanese-GSD, which has the bunsetsu boundaries and a license that allows a model to be released. | #1417 |
| Coordination is in the form of the Kyoto corpus and CaboCha: each conjunct depends on the next, with the arc type `P`. | UD attaches later conjuncts to the first, with an arc to the left, which the parsers cannot produce. | #1417, #1419 |
| The first model is for IPADIC; UniDic follows. A model belongs to one dictionary. | A model depends on the part-of-speech columns and on the segmentation of its dictionary. The baselines of CaboCha and of J.DepP were measured with IPADIC, so the numbers can be compared. | #1417 |
| A model for each of the three UniDic dictionaries of Lindera (`unidic`, `unidic-cwj`, `unidic-csj`), with one schema. | The three do not segment and tag alike: on the sentences of GSD train, two of them agree on every morpheme and its part of speech for 63% to 73% of the sentences. A model is checked against the name of its dictionary. The columns a schema reads are at the same places in the three. | [#1441](https://github.com/lindera/lindera/issues/1441) |
| The chunker for UniDic has settings of its own: the base form as a fourth column, and L1 with the coefficient 0.25. The settings of the parser are the same as for IPADIC. | The cross-validation on the train file chooses them: the base form of UniDic, the lexeme, helps where the one of IPADIC did not. The files that are not trained on do not tell the two settings apart, and UD Japanese-PUD is a little lower with the new ones; the choice follows the cross-validation, as for IPADIC. For the parser the cross-validations were run again and choose what they chose for IPADIC. | [#1441](https://github.com/lindera/lindera/issues/1441) |
| No accuracy target is set for UniDic. The numbers are reported next to those of IPADIC. | The targets come from baselines measured with IPADIC, and the numbers of two dictionaries are not counted on the same bunsetsu. | [#1441](https://github.com/lindera/lindera/issues/1441) |
| Chunking is a conditional random field over `B` and `I`, trained with `lindera-crf`. | It is what CaboCha does, whose format and models the crate follows. It is not the better chunker of the two baselines: J.DepP, which decides each boundary by itself with a kernel classifier, chunks better when both are measured (see the baselines). Where that gain comes from is [#1464](https://github.com/lindera/lindera/issues/1464). | #1417, #1421, [#1433](https://github.com/lindera/lindera/issues/1433) |
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
| The parser is trained by dual coordinate descent on a support vector machine with the hinge loss, as CaboCha, with CaboCha's cost of 0.0015. Passive-aggressive, as J.DepP, is an option. | The two do not differ in a cross-validation (88.96% and 88.88%, p = 0.22) and take the same time. Dual coordinate descent ends by itself; passive-aggressive needs a number of passes to be chosen. | #1437 |
| The weights of the pairs of features are in a table of 2^22 weights that the pairs are hashed into. A weight of its own for every pair is an option. | The accuracy does not differ (p = 0.60), parsing is six times faster, and the file is a sixth. Sizes from 2^18 do not differ in the cross-validation; 2^22 is the best of them on dev. | #1437 |
| The place of a hashed pair comes from the hashes of the two feature strings, not from their numbers in a model. | A model can leave features out without moving the pairs of the others, and training and inference cannot disagree on a numbering. | #1437 |
| The weights of the parser are written as one byte each, a whole number times a scale. | The accuracy does not differ from weights of four bytes (p = 0.86); weights of two bytes give the same output. The file is a third, and parsing is faster. | #1437 |
| The features of the parser are CaboCha's and one group more: the morpheme after the head of the source and of the target. The head of a bunsetsu stays the content word of the schema. | With the head of the schema, CaboCha's features lose what CaboCha's head gave them: whether a bunsetsu is a verb or a noun as a whole (利用 / し / た, 読む / こと / を). The group gives it back: 89.19% against 88.96% in the cross-validation (p < 0.001). The output keeps the content word as the head. | #1437 |
| A model is chosen by the rule of #1420, and what does not differ is taken as equal: among equal settings the smaller or faster one is the default. | The differences between good settings are under the noise of the data; size and speed are not. | #1437 |
| The chunker-only file of before the model file (magic number `LDCHUNK`) is not read any more. | The crate was not published then and no model was distributed; a second reader would have been kept for no file that existed. | #1437 |
| Training data holds no head word and function word positions; what reads it computes them from the schema. | A change of the rule cannot leave stale positions in a file. | #1422 |
| Settings are compared by cross-validation on the train file; the dev file chooses between close settings; the test file and PUD are for the reported result only. | Dev and test are small: the 95% interval of an accuracy near 90% is about ±1 point on them. | #1420 |
| The accuracy targets are the ones under "Targets" below. | They are the baselines measured on the same data. | #1417 |
| The targets keep their definition after the baselines were measured again: CaboCha's numbers for the chunking and for the whole analysis, J.DepP less one point for the dependency accuracy. | J.DepP, measured, is better than the numbers that were on record, and ahead of the chunking and of the whole analysis here. The targets say what the crate set out to reach, a chunker as good as CaboCha's and a parser close to J.DepP's; raising them to the better baseline when it is found to be better is another goal, and is [#1464](https://github.com/lindera/lindera/issues/1464). | [#1433](https://github.com/lindera/lindera/issues/1433) |
| A token filter writes its result in a field of `Token`, read with `Token::dependency`: the sentence, the bunsetsu, its head, what the token is in its bunsetsu, the score. `Token::details` and the MeCab output do not change. This replaces four string columns at the end of the details. | The details stay the features of the dictionary, for every output and every later filter, and `Token::as_value` shows the result, which it did not show of columns past the schema. It breaks code that builds a `Token` with a struct expression: `Token` is `#[non_exhaustive]` from here on, so the next field breaks nothing. | #1438 |
| The filter is in `lindera-analysis`, behind its `dependency` feature, and its `kind` is `japanese_dependency`. | The configuration file, the command and the bindings all make a filter with `TokenFilterLoader::load_from_value`, a fixed `match`: one arm serves them all. A registry of filters would be a second way to name one. | #1438 |
| The filter cuts a text into sentences, at `。`, `！` and `？` outside brackets, and numbers the bunsetsu from 0 in each; the result holds the number of the sentence. | A model is trained on sentences, and the lattice format numbers bunsetsu in a sentence. | #1438 |
| The filter refuses the tokens of another dictionary than the one of the model, and tokens an earlier filter removed or joined. | Both give a wrong analysis without failing. | #1438 |
| The crate will read the models CaboCha distributes through a converter, after the first release. Those models are not redistributed. | Feature extraction is separate from decoding for this. | #1417, #1421 |
| A converted model reproduces what CaboCha parses with, not the kernel it was trained with: the pruning of `cabocha-model-index` with its defaults, and its whole numbers. The settings are options. | The goal is the analysis CaboCha gives. With them every decision of 9,066 sentences is the same; the kernel itself changes one head in 360. | [#1442](https://github.com/lindera/lindera/issues/1442) |
| What makes the features of a model is data of its schema, down to the normalization of the surface and the way a mark is matched. | A model of another program is only right with that program's features, and a schema in code would be one schema for a dictionary. The file is version 2 for it. | [#1442](https://github.com/lindera/lindera/issues/1442) |
| The three models of CaboCha are to be converted, and a JUMAN dictionary is added to Lindera for the one of JUMAN. | Decided by the maintainer on 2026-11-27. The model for JUMAN is the one trained on the Kyoto corpus directly. | [#1442](https://github.com/lindera/lindera/issues/1442), [#1466](https://github.com/lindera/lindera/issues/1466) |
| The command for text is in the binary of this crate, in a build without the training code: the `cli` feature holds the command, `train` adds the training to it. The `lindera` command follows with the filter and the publication. | A user of the command does not get `lindera-crf`. `lindera-cli` is published and cannot depend on a crate that is not, and an output format of `lindera tokenize` needs the token filter. | #1439 |
| Of CaboCha's options the command keeps the lattice output (`-f1`) and the three input layers (`-I`), as `--input-layer`. The tree, XML and CoNLL outputs, the output layer and the named entities are not kept. | The lattice format is what other tools read, and its writer exists. The layers are what `parse` and `parse --chunk` did, as one option. The morphemes alone and the bunsetsu alone have commands of their own. | #1439 |
| The command reads, parses and writes one sentence at a time, and stops at the first sentence that fails, with what it wrote before. | It stands in a pipe, and its memory does not grow with the input. | #1439 |
| The model for IPADIC is distributed under the MIT license, with an acknowledgement of UD Japanese-GSD and the notice of IPADIC. | The model holds names of features and weights, and none of the annotation: the project takes it not to be an adaptation of the treebank. No court has decided the question; the [model file page](../lindera-dependency/model_file.md#license-of-the-model) says what the position rests on. With one license for the code and the model, a program that embeds the model is distributed as one with an embedded dictionary is. | #1440 |
| The models for UniDic are released, embedded and downloaded as the one for IPADIC, under the same license, each with the notice of its dictionary. A release trains them with the dictionaries it has just built, read from a directory. | One mechanism for every model: a feature, a variable, a pin and an archive for each dictionary. Embedding UniDic CWJ or UniDic CSJ to train takes 8 to 11 GiB of memory to compile; reading the directory takes none of it, and the dictionary is the one of the same release. | [#1441](https://github.com/lindera/lindera/issues/1441) |
| The released model is trained on the train file of UD Japanese-GSD only. | The numbers on dev, test and PUD are then the numbers of the file that is released, and anyone can take them again. | #1440 |
| A released model has the version of Lindera, and is released as a built dictionary is: a release builds it with one script, puts its archive on the page of its version, and writes a `prebuilt.json` into the crate just before it publishes it. A build from the repository embeds a file it is given. | One version for everything a release holds: nobody has to know which model fits which release. The model cannot be trained by a build script, as a dictionary is built, so a build without a pin takes a file. Decided on 2026-11-25, in place of a version of its own and a release with a tag of its own. | #1440 |
| The release that first has the crate is 7.1.0, with the break of `Token` stated in the migration guide and in the release notes. | Decided by the maintainer. The break touches code that builds a `Token` with a struct expression or destructures all of its fields, not code that reads the tokens of a tokenizer. | #1440 |
| The `lindera` command and the five bindings have the `dependency` feature in their default builds, and no embedded model; the libraries `lindera` and `lindera-analysis` do not have the feature by default. | What is distributed has to have the filter, or a user of a package would have to build it from source. The model is data, as a dictionary is: it is downloaded and given by its path, and nobody carries 6 MB of a model they do not use. The command grows by 134 kB (1.5%). A library leaves the choice to its user. | #1440 |

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

The command is a subcommand of the crate's own binary, behind the `train` feature (the binary itself is behind `cli` since #1439): `lindera-cli` is published and cannot depend on a crate that is not.

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
- **The schema of UniDic is a second value, measured in the same way.** To the pattern of CaboCha for UniDic it adds 非自立可能 as a function word, and as what is not a head: a verb right after に, と or を, a 助動詞語幹, a formal noun right after a predicate, and a nominal suffix right after an adjective. No code of the chunker or of the parser changed for it. One thing is not expressed: the corpus has a percent sign after a number as the head, UniDic tags it as it tags other symbols (補助記号,一般), and a schema takes a symbol by its part of speech or by its characters, not by the two together.
- The function word is defined by the schema alone. UD Japanese-GSD has no label that means the same: its `SYN_HEAD` matches neither "the last morpheme that is not a symbol" nor "the morpheme after the head".

### Parser ([#1436](https://github.com/lindera/lindera/issues/1436))

- **The loop** is `parseShiftReduce` of CaboCha and `_parseLinear` of J.DepP: a stack of the bunsetsu without a head, each following bunsetsu as the target, the top of the stack as the source. An arc to the last bunsetsu is not asked. The loop takes the decision as a function, so the classifier and the answer of a sentence run the same code.
- **The features** are CaboCha's (`src/selector.cpp`, `src/dep.cpp`), in seven groups: the distance, the source, the target, the bunsetsu before the source, the one after the target, the bunsetsu already attached, and the gap. They are made once for each bunsetsu and put together for a decision, generic over what a string is turned into: the number of the classifier when parsing, the string itself for training instances.
- **Differences from CaboCha**, with the schemas of this crate: the surface is not normalized, and punctuation and brackets are those of the schema; the head and the function word are those of the schema of #1422; a bunsetsu without a known head is not supported; an arc that is not asked has the score 0, where CaboCha writes the score of the decision before. The first two are data of the schema since #1442, and a model converted from CaboCha has CaboCha's.
- **J.DepP's features** (`jdepp/pdep.cc`) differ: four bunsetsu by the first morpheme, the head, the tail and the last one; whether two heads have the same surface; the balance of brackets; the particles of the bunsetsu in between that compete for the target; nothing of the bunsetsu already attached. What only J.DepP has was not tried: the targets were met without it.
- **`LinearClassifier`** adds a bias, the weights of the features and the weights of their pairs, from hash tables. It is the least that parses with weights made by hand; a trained model has its own (see below).
- **Training instances** are the decisions the loop asks when the answer decides, each with its features and its answer, as in CaboCha's training mode. The answer gives the tree of every sentence of the training data back.

### Model file ([#1437](https://github.com/lindera/lindera/issues/1437))

- **One file per dictionary.** It starts with a magic number and a version, names its dictionary, holds the schema, then a list of parts and the parts. The chunker is the first part; the parser will be the second. A part can be absent: `train-chunker` writes a file with the chunker only.
- **Read in place.** `Model<B>` holds any `B: AsRef<[u8]>` and keeps, besides the bytes, only what is small and structured: the name, the schema, the columns and the template of the chunker, its nine transition weights, and where its table is. The weights of the feature strings stay in the bytes and are read with `from_le_bytes`.
- **Checked once.** `Model::new` reads every field, every offset and every string, and looks every string up through its slots. What is read afterwards goes through checked slices, so a model that was made cannot panic; a test changes every byte of a file in turn to hold this.
- **The chunker reads through a trait** (`ChunkerData`), as the parser reads its classifier: the model in memory that training gives, and the part of a file, are two implementations, and the decoder is the same code. They give the same labels on every sentence of dev, test and PUD.
- **The table of strings** is the piece the parser will share for the numbers of its features. It is sorted and has slots, a hash table with open addressing over the 64-bit FNV-1a of a string, with at least two slots per string. Most feature strings of a sentence are not in a model (L1 leaves 4,325 of them), so what matters is how fast a string is not found: with at most half of the slots taken, the search ends at an empty slot after one or two steps.
- **`Chunker::chunk` and `Parser::parse` return an error** for a morpheme with fewer feature columns than the schema reads. Before, such a morpheme was analyzed with empty columns.
- **Not done**: quantization of the chunker (its weights are `f64`, so that it gives the labels of `lindera-crf`, and it is 4% of a model file), and a checksum. A file that was cut is found by its lengths; a changed weight is not found.

### Training of the parser, and its part of the model file ([#1437](https://github.com/lindera/lindera/issues/1437))

- **One space, two algorithms.** Both reference trainers learn the same linear model: a bias, a weight for each feature and a weight for each pair, which is the kernel `(1 + s)^2` expanded; a step `d` adds `d`, `3 d` and `2 d`. CaboCha does not solve the kernel problem with a kernel either (`src/svm_learn.cpp`). The two algorithms here differ in the step only, and where the weight of a pair is kept is one function under both.
- **Dual coordinate descent** follows `solveParameters` of CaboCha: the hinge loss, the box of the cost, shrinking, a random order at every pass, and the stop at a projected gradient of 0.1. The kernel of a decision with itself counts a place that several of its pairs share by the square of their number, so that the step is exact with hashed pairs as well. The order comes from a generator of the crate (SplitMix64) with a seed, so the same data gives the same model.
- **Passive-aggressive** follows opal (PA-I): the step is the loss over the number of features, no larger than the cost. With the pairs, a step moves the score by about the number of features times too much, so the cost has to bind: good costs are near 0.0005, where CaboCha's is 0.0015. The average of the weights is kept as opal keeps it.
- **Training takes seconds, not minutes**: 66,000 decisions with 37 features each on average have about 680 pairs each, and a pass touches them once. The whole train file is learned in about three seconds on one core.
- **The parser part of a file** holds the feature set, the bias, the scale of the quantization, the table of the feature strings with their weights, and the weights of the pairs: a table of `2^bits` weights, or the pairs of each feature sorted by the other feature. The classifier of a file reads them where they are; with weights of four bytes it gives the score of the model in memory to the last bit.
- **A weight for every pair is slow as it is stored here**: each pair of a decision is a binary search in the pairs of one of its features. CaboCha keeps the pairs of the 3,000 most frequent features in a triangular array and the rest in a double array; that was not built, since hashing gives the same accuracy.
- **The head of the schema and the features.** The content head of #1422 is the better word to index, and the worse one to parse with: with CaboCha's rule for the head and CaboCha's features, the cross-validation gives 89.22%; with the head of the schema, 88.96%. The morpheme after the head gives the difference back (89.19%), and with it the two rules do not differ.
- **`train-parser --chunker`** copies the chunker part of another file as its bytes are (`ModelBuilder::from_model`), so that the chunker and the parser are trained apart and end in one file.

### Command ([#1439](https://github.com/lindera/lindera/issues/1439))

- **`parse` is the command of a user**: plain text in, one sentence per line, and the lattice format out. It was the subcommand that parsed a lattice file for `evaluate`; a second subcommand for text would have been a second entry to the same analysis. What it did is `--input-layer bunsetsu` and `--input-layer morphemes` now.
- **The text is tokenized as the training data is**: `Mode::Normal` and no user dictionary, the settings of `convert`. Another segmentation would not be the one the model was trained on.
- **The name of the dictionary that was loaded is compared with the name in the model file** (`Model::check_dictionary` of #1437), which is the check the model file was given its name for. With a lattice input there is no dictionary, and the number of feature columns is what is checked.
- **The features are split in two**: `cli` brings `clap` and `lindera-segmenter`, `train` adds `lindera-crf` and `serde_json`. `scripts/check_dependency_crate.sh` checks that the build with `cli` has no `lindera-crf`, and a job of CI tests the command in that build.
- **`DictionaryMorpheme`**, the morpheme made from a token, moved from the `corpus` module, which is behind `train`, to the default build.
- **CaboCha's tools read the output** ([#1433](https://github.com/lindera/lindera/issues/1433)): `cabocha-system-eval` counts on the output of `parse` what `evaluate` counts, and `cabocha -I 2` takes it as its input with bunsetsu.

### Token filter ([#1438](https://github.com/lindera/lindera/issues/1438))

- **One arm of a `match` makes the filter everywhere.** `TokenFilterLoader::load_from_value` is what the configuration file, the `lindera` command and the bindings call. The arm is there with the `dependency` feature; without it, another arm refuses the `kind` and names the feature, where the build would have said that it knows no such filter. `scripts/check_dependency_crate.sh` checks that the builds without the feature do not link this crate.
- **The field of `Token` is a slot that frees its value in a cold function.** With a plain `Option<Box<_>>` field, the drop of a token was no longer inlined where tokens are dropped, and tokenizing without the filter took 0.73% more instructions with IPADIC. The slot tests for a value inline and calls a function for the rest: at most 0.35% on 15 workloads. A token grows by one pointer.
- **The half-width period ends no sentence.** The periods of the schema are `。`, `．` and `.`; with them the filter cut 4 of the 543 lines of the test file of UD Japanese-GSD, all at a number (`4.20`, `1.5km`) or at `...`. The full-width `．` ends a sentence unless a digit follows. `！` and `？` end one too: in IPADIC they are symbols, not periods.
- **What the filter can see of an earlier filter is the positions.** The tokenizer numbers its tokens from 0 without a gap, a filter that removes tokens leaves a gap, and one that joins them a token of more than one position. A rewritten surface or detail, and tokens removed from the end, cannot be seen; the documentation says to put the filter first.
- **The tokens are read where they are.** A morpheme of the filter borrows the surface and the details of its token. The model is behind an `Arc`, shared by the clones of the filter, which a tokenizer makes when it is cloned.
- **The filter gives what the command gives**: on the 543 sentences of the test file of UD Japanese-GSD, every token has the bunsetsu, the head, the role and the score of `lindera-dependency parse` with the same model.
- **The bindings give the dependency as an object of its own on a token**, and nothing without the filter: `None` in Python, `nil` in Ruby, `null` in PHP, and no property in Node.js and WebAssembly, where a token is a plain object and stays the object it was. The head of the last bunsetsu is null in every binding, as in the JSON of the command. `TokenView` of `lindera-binding` carries it to all five.
- **Node.js makes the names of the six properties for the first token that has a dependency**, in the handle scope that token is made in, and again after the scope is renewed: a call without the filter makes the objects it made before, with the calls it made before.
- **WebAssembly takes the bytes of a model**, with `appendDependencyFilter`: there is no path to read. The builder of `lindera-binding` takes a filter that was made in Rust for it, and runs it where it was appended among the filters given by `kind`.
- **Not verified**: a dictionary other than IPADIC, for which there is no schema yet.

### Converter for the models of CaboCha ([#1442](https://github.com/lindera/lindera/issues/1442))

- **The module `cabocha`**, behind `train`: a reader of the CRF++ text model, a reader of the parser text model, the expansion of the support vectors, CaboCha's schema for IPA, and `convert`. The command is `convert-cabocha`. Reading a converted model needs nothing of it: the default build reads it as any model.
- **The expansion follows `FastSVMModel::compile` step by step**: the weights added in 32-bit floats in the order of the support vectors, the count of a pair in a byte, the pairs of two frequent features kept and the others by their count and their weight, the unit as the largest weight over 1,048,575, and the truncation. A decision then adds up the whole numbers CaboCha adds up, and its sign cannot differ by a rounding.
- **The weights stay whole numbers.** A parser model has a unit, what the sum of its weights is multiplied by, and the bias is not in it; the file writes it as the scale of unquantized weights, which was always 1 before. A converted model is not quantized: `i8` would change its decisions.
- **`Schema` got two things**: `Normalization`, a table applied with the longest match, used for the surface column of the chunker, for the surfaces of the features and for matching a mark; and `SurfaceRule`, how a surface is matched against the characters of a mark. CaboCha matches the whole surface (`((` is not a bracket), and a mark by its surface is still a word for the head rule: an unknown `(` is a noun for IPADIC, and can be a head. The schemas of this crate keep the other rule, and no normalization.
- **The table is CaboCha's file**, `src/normalizer.rule`, included as it is with its notice (`NOTICE.txt`). It is part of the software, under the BSD license, not of the models.
- **As in CaboCha, a mark is matched after the normalization**, so `｛`, `｝`, `’` and `”`, which the table turns into ASCII, are not brackets, though CaboCha's patterns list them.
- **The chunker takes the weights as 32-bit floats**, which is what CRF++ keeps in the compiled model CaboCha reads.
- **Measured** (`cabocha_models.sh`): on the morphemes of Lindera for the 9,066 sentences of UD Japanese-GSD and PUD, the same bunsetsu, heads, head words and function words as CaboCha 0.69 in every sentence, and the same scores to six decimals. 3,727,492 of 10,141,653 pairs are kept, 51.8 MB, 3.6 s to convert. From text, 91% to 98% of the sentences have the morphemes MeCab gives; the others differ at katakana words that are not in the dictionary.
- **Slower than CaboCha**: 3.03 s against 0.92 s for 7,024 sentences. CaboCha has a dense table for the pairs of frequent features; a model file looks every pair up in the sorted pairs of a feature.
- **Not done**: the models for JUMAN ([#1466](https://github.com/lindera/lindera/issues/1466) first) and for UniDic. JUMAN needs the head rule without the move of the function word and a negated pattern; UniDic two numbers of levels of the part of speech.

## What is to come

| Part | Notes | Issue |
| --- | --- | --- |
| IPADIC model | Done: the model is built, measured and published by each release, and the crate is published with it. The first release with both is 7.1.0. | [#1440](https://github.com/lindera/lindera/issues/1440) |
| The models of CaboCha for JUMAN and UniDic | The model for IPA is converted. JUMAN needs a dictionary ([#1466](https://github.com/lindera/lindera/issues/1466)) and its head rule as data; UniDic needs to be checked against the morphemes of Lindera's UniDic first. | [#1442](https://github.com/lindera/lindera/issues/1442) |
| A JUMAN dictionary | `mecab-jumandic` as a dictionary of Lindera. | [#1466](https://github.com/lindera/lindera/issues/1466) |
| The chunking of J.DepP | Where it gains over a conditional random field, and whether the chunker can gain the same. | [#1464](https://github.com/lindera/lindera/issues/1464) |

### Open questions

- **The steps of a release that run on a tag only** have not run yet: the upload of the archive of the model, its pin in the crate, the build that checks the pin against the release, and the publication of the crate. They run for the first time with 7.1.0.
- **A line break ends no sentence in the token filter**: it is not a token. A text of lines without a period is one sentence there, unless what calls the tokenizer cuts it.
- **More training data** under a usable license.

## Targets

On UD Japanese-GSD with IPADIC, from the baselines below:

| | dev | test |
| --- | --- | --- |
| Chunking F | 96.8 or more | 96.8 or more |
| Dependency accuracy with the bunsetsu of the answer (level 1) | 89.1 or more | 87.7 or more |
| End-to-end dependency accuracy | 85.0 or more | 84.3 or more |

The dependency target is J.DepP less one point; the other two are CaboCha's numbers. The same numbers on UD Japanese-PUD are reported. All three are met with IPADIC: chunking 96.85 and 96.92 (#1421), dependency with the bunsetsu of the answer 90.11 and 88.53, end to end 85.39 and 84.47 (#1437). The baselines were measured again in #1433, from scripts in the repository; with the numbers that were on record before, the dependency target was 89.0 and 87.8. J.DepP as measured is ahead of `lindera-dependency` in the chunking (97.17 and 97.02) and in the whole analysis (85.99 and 84.65), which the targets do not ask for: see the decision above, and #1464. No target is set for UniDic (#1441): its numbers are reported next to these.

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
| CaboCha: chunking F, dependency with the bunsetsu of the answer, end to end | dev 96.79 / 89.84 (3,271 / 3,641) / 85.00, test 96.80 / 88.51 (3,511 / 3,967) / 84.25, PUD 97.81 / 88.65 (7,845 / 8,849) / 85.69 | `evaluate`; script: `baselines.sh` |
| J.DepP: the same | dev 97.17 / 90.11 (3,281 / 3,641) / 85.99, test 97.02 / 88.71 (3,519 / 3,967) / 84.65, PUD 98.04 / 88.98 (7,874 / 8,849) / 86.45 | `evaluate`; script: `baselines.sh` |
| The same two, on record before they were measured again | CaboCha: dev 96.82 / 89.84 / 85.03, test 96.80 / 88.51 / 84.25, PUD 97.80 / 88.64 / 85.65. J.DepP: dev 95.93 / 89.98 / 83.80, test 95.37 / 88.76 / 82.40 | Prototype, lost |
| The cost of CaboCha's chunker (`-c`) 0.5, 1, 2, 4, 8, 16: chunking F on dev, and over a 10-fold cross-validation of train | dev 96.66, 96.64, 96.68, 96.79, 96.82, 96.86; cross-validation 96.45, 96.57, 96.63, 96.66, 96.66, 96.67 | Script: `cabocha_chunker_cost.sh` |
| `cabocha-system-eval` on the outputs of CaboCha and of `lindera-dependency parse` | The counts of `evaluate`: for CaboCha level 1 3,271 / 3,641 and chunking 4,008 / 4,135 and 4,008 / 4,147 on dev; for `parse` 3,281 / 3,641, and 4,009 / 4,132 and 4,009 / 4,147 | Script: `baselines.sh`; the second by hand |
| Arcs on which CaboCha and J.DepP differ, with the bunsetsu of the answer | 127 of 3,641 on dev, 196 of 3,967 on test, 424 of 8,849 on PUD | `evaluate`; script: `baselines.sh`. On record before: 5 and 10, which is the difference of their counts of right arcs, not the arcs that differ |
| Gain of dependency accuracy per doubling of the training set | About 0.8 point, not levelling off at 7,024 sentences | Prototype, lost |
| CRF++ and `lindera-crf` with CaboCha's template, L2 0.25: chunking F | 96.79 / 96.77 on dev / test | Prototype, lost; `chunker_experiments.sh table` gives 96.79 / 96.80 with `lindera-crf` |

The baselines of CaboCha and J.DepP are trained on the GSD train file, on the morphemes of Lindera with IPADIC (`convert`), by `baselines.sh`, with the tools `build_baselines.sh` builds: CaboCha at 96e100a, with CRF++ at c603ea6 and MeCab at 61b90ba, and J.DepP 2022-03-18.

- **CaboCha**: `cabocha-learn -e chunk -c 4` and `cabocha-learn -e dep`, with its other defaults and the part-of-speech set IPA. The cost of the chunker was chosen on dev when the baselines were first measured, and is kept: dev rises a little with the cost, and a cross-validation is level from 4 on. CaboCha gives the numbers that were on record, within 0.04.
- **J.DepP**: the steps and the options of its own `make model` for its default classifier (a polynomial kernel of degree 2), untuned. It is better than the numbers that were on record, by 1.2 to 1.7 points of chunking F and 2.2 points from the morphemes. What J.DepP reports of itself agrees with `evaluate`. The setting of the lost measurement is not known: a linear classifier with the options of its `configure` gives 96.36 / 95.24 for the chunking and 86.98 / 86.44 at level 1, and J.DepP without options 94.34 / 94.53 and 83.93 / 83.44, neither the numbers on record.
- **J.DepP takes a line by its first character**: a morpheme whose surface starts with `*` is read as a bunsetsu line, which stops it, and one that starts with `#` is dropped as a comment. The corpus has such morphemes, so J.DepP is given them with the full-width character, and the morphemes of its output are taken from its input (`jdepp_format.py`).

**J.DepP is ahead of CaboCha and of `lindera-dependency` in the chunking, and through it in the whole analysis**; with the bunsetsu of the answer the three parsers are within 0.4 point of each other. The crate chunks as CaboCha does, and the notes said that this was the better of the two: measured, it is not. See #1464.

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
| UniDic through the output of Lindera (UniDic CWJ): CaboCha's pattern, with 非自立可能, with a verb after に, と or を, with 助動詞語幹, with formal nouns, `Schema::unidic()` | train 82.50%, 94.73%, 96.71%, 97.19%, 97.63%, 97.70% (55,460 / 56,763); the schema on dev 97.61%, test 97.95%, PUD 98.16% | Script: `head_variants/` |
| The same, `Schema::unidic()` with `unidic` and with `unidic-csj` | train 97.52% and 97.55% | Script: `head_variants/` |
| Bunsetsu boundaries of GSD train on a morpheme boundary: IPADIC, `unidic`, `unidic-cwj`, `unidic-csj` | 99.21%, 99.87%, 99.95%, 99.94% | Script: `convert_corpora.sh` |
| Sentences of GSD train with the same morphemes, and with the same part of speech too: `unidic` and `unidic-cwj`, `unidic` and `unidic-csj`, `unidic-cwj` and `unidic-csj` | 85.88% and 66.06%, 86.33% and 62.64%, 88.10% and 73.22% | Script: `compare_dictionaries.py` |
| Chunker with UniDic, cross-validation F: the settings for IPADIC and the settings for UniDic, with `unidic`, `unidic-cwj`, `unidic-csj` | 97.02 and 97.15 (p 0.0010), 97.02 and 97.22 (p < 0.0001), 96.94 and 97.08 (p 0.0001) | Script: `chunker_experiments.sh --dictionary <name> check` |
| Models for UniDic, chunking F on dev, test, PUD | `unidic` 97.03, 97.24, 98.00; `unidic-cwj` 96.63, 97.02, 97.89; `unidic-csj` 96.60, 96.95, 97.78 | Script: `build_model.sh --dictionary <name>`, `model_accuracy.py` |
| The same, level 1 | `unidic` 89.67%, 88.69%, 88.67%; `unidic-cwj` 89.35%, 88.91%, 88.58%; `unidic-csj` 89.75%, 88.90%, 88.59% | The same |
| The same, end to end | `unidic` 85.06%, 84.88%, 85.93%; `unidic-cwj` 84.22%, 84.71%, 85.73%; `unidic-csj` 84.58%, 84.65%, 85.58% | The same |
| Size of the models for UniDic | `unidic` 6,257,148 bytes, `unidic-cwj` 6,258,600, `unidic-csj` 6,267,183 | Script: `build_model.sh --dictionary <name>` |
| `SYN_HEAD` of GSD against "last morpheme that is not a symbol" and "the morpheme after the head" | 87–88% and 81–82% | Prototype, lost |

### Parser

| Number | Value | From |
| --- | --- | --- |
| Sentences whose tree comes back when the answer decides | train 7,024 of 7,024, dev 506 of 506, test 542 of 542, PUD 994 of 994 | Script: `parser_instances.sh` |
| Training instances | train 66,000 (33,512 positive), dev 4,881 (2,474), test 5,377 (2,707), PUD 12,739 (6,077) | Script: `parser_instances.sh` |

### Training of the parser

Level 1 is the share of the bunsetsu, but the last of each sentence, with the right head. A cross-validation is over the 10 folds of the train file (49,372 bunsetsu), on the bunsetsu of the answer; "p" is McNemar's exact test on the arcs. Unless said, a setting has CaboCha's feature set, dual coordinate descent with the cost 0.0015, a weight for every pair, and weights of four bytes. The README of the crate has every setting.

| Number | Value | From |
| --- | --- | --- |
| Features of a training decision | 37.1 on average, 58 at most; 96,003 different ones in the train file, 60,050 in two decisions or more | The files of `parser_instances.sh` with CaboCha's feature set (`--features`), counted with `awk` and `sort`; not kept as a script |
| Dual coordinate descent, by the cost | 0.0005: 88.11%; 0.001: 88.79%; 0.0015: 88.96%; 0.002: 89.01%; 0.003: 88.91%; 0.005: 88.76%; 0.01: 88.72% | Script: `parser_experiments.sh algorithm` |
| Passive-aggressive, averaged, 30 passes, by the cost | 0.0005: 88.88%; 0.001: 88.81%; 0.005: 88.83%; without the average 88.76%, 88.70%, 88.33% | Script: `parser_experiments.sh algorithm` |
| Dual coordinate descent against passive-aggressive (0.0005, 30 passes, averaged) | 559 arcs only the first has right, 518 only the second, p = 0.22; 6.3 and 8.2 seconds a fold | Script: `parser_experiments.sh compare` |
| CaboCha's feature set without one group | `right`: 88.53% (p < 0.001); `gap`: 88.75% (p < 0.001); `children`: 88.87% (p = 0.07); `left`: 88.93%; `distance`: 88.94% | Script: `parser_experiments.sh features`, `compare` |
| With the morpheme after the head | 89.19% against 88.96%: 637 and 523 arcs, p = 0.0009 | Script: `parser_experiments.sh features`, `compare` |
| The head of CaboCha's rule against the head of the schema, CaboCha's features | 89.22% and 88.96%; with the morpheme after the head, 89.23% and 89.19% | A program outside the repository that trains with variants of the schema, as `head_variants/` makes them; not kept |
| Pairs in 3 decisions or more, every group of features | 89.19%, as with every pair (p = 0.93), with 1,310,821 pairs for 4,168,732 | Script: `parser_experiments.sh pairs`, `compare` |
| Pairs hashed into 2^16, 2^18, 2^20, 2^21, 2^22, 2^24 weights | 88.81%, 89.14%, 89.15%, 89.19%, 89.21%, 89.22%; against a weight for every pair (89.19%), 2^18: p = 0.43, 2^20: p = 0.36, 2^22: p = 0.60 | Script: `parser_experiments.sh storage`, `compare` |
| The same on dev, trained on the whole train file, weights of one byte | 2^18: 89.92%; 2^20: 89.81%; 2^21: 90.03%; 2^22: 90.11% | Script: `parser_experiments.sh storage` |
| Weights of two bytes and of one byte against four, pairs hashed into 2^22 | The same output; 18 and 16 arcs, p = 0.86 | Script: `parser_experiments.sh quantization`, `compare` |
| The defaults: level 1 with the bunsetsu of the answer | dev 90.11% (3,281 / 3,641), test 88.53% (3,512 / 3,967), PUD 88.81% (7,859 / 8,849) | Script: `parser_experiments.sh final` |
| The defaults: end to end | dev 85.39% (3,109 / 3,641), test 84.47% (3,351 / 3,967), PUD 86.19% (7,627 / 8,849) | Script: `parser_experiments.sh final` |
| The defaults with CaboCha's feature set | With the bunsetsu of the answer: dev 89.15%, test 88.30%, PUD 88.33%. End to end: dev 84.45%, test 84.30%, PUD 85.59% | Script: `parser_experiments.sh final` |
| The default model | 6,258,548 bytes with the chunker; 62,380 feature strings; 19 passes, 21,374 support vectors of 66,000 decisions; about 3 seconds; the same bytes when trained twice | Script: `parser_experiments.sh final` |
| The default model on the 542 sentences of test, one core of an AMD Ryzen AI Max+ 395 | Read in 2,732 microseconds; chunking 20,354, parsing 16,678, both 8,718 sentences per second | Script: `parser_experiments.sh speed` |
| The same with the other tables of the pairs | Hashed, four bytes: 19,028,600 bytes, parsing 13,375. A weight for every pair, one byte: 24,514,588 bytes, 2,741; four bytes: 38,162,161 bytes, 2,679 | Script: `parser_experiments.sh speed` |
| The command on the text of the 543 sentences of test, repeated 100 times, with the default model and the embedded IPADIC | About 6,600 sentences per second from the text to the lattice format; 5 ms to load the dictionary and the model | Script: `parse_speed.sh` |

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

### Token filter

`lindera tokenize` with the embedded IPADIC and the default model, on the text of the test file of UD Japanese-GSD repeated 20 times (10,860 sentences, 252,880 tokens), MeCab output, on one core of an AMD Ryzen AI Max+ 395, with Rust 1.100.0. The fastest of five runs, without the time of the same command on an empty input.

| Number | Value | From |
| --- | --- | --- |
| Without the filter | About 2,900,000 tokens per second | Script: `filter_speed.sh` |
| With the filter | About 180,000 tokens per second, 7,700 sentences per second | Script: `filter_speed.sh` |
| What the filter adds to a token | About 5.2 microseconds; about 78,000 instructions | Script: `filter_speed.sh`; the instructions with `perf stat`, not kept as a script |
| Making the filter, which reads the model | About 4 ms | Script: `filter_speed.sh` |
| The filter against `lindera-dependency parse` | The same bunsetsu, heads, roles and scores for the 12,644 tokens of the 543 sentences | A comparison of the two outputs, not kept as a script |
| A token without a dependency, against Lindera before the field | At most 0.35% more instructions on 15 workloads; one pointer more in a token | `perf stat`, in the pull request of the field (#1449) |

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
| [#1433](https://github.com/lindera/lindera/issues/1433) | The baselines of CaboCha and J.DepP, measured again | Done |
| [#1437](https://github.com/lindera/lindera/issues/1437) | The model file, and the training of the parser | Done |
| [#1438](https://github.com/lindera/lindera/issues/1438) | Token filter | Done |
| [#1439](https://github.com/lindera/lindera/issues/1439) | Command | Done |
| [#1440](https://github.com/lindera/lindera/issues/1440) | IPADIC model, and the publication of the crate | Open |
| [#1441](https://github.com/lindera/lindera/issues/1441) | UniDic schema and models | Done |
| [#1442](https://github.com/lindera/lindera/issues/1442) | Converter for the models of CaboCha | The model for IPA done; JUMAN and UniDic open |
| [#1466](https://github.com/lindera/lindera/issues/1466) | A JUMAN dictionary built from mecab-jumandic | Open |
| [#1464](https://github.com/lindera/lindera/issues/1464) | Where the chunker of J.DepP gains, and whether the chunker can gain the same | Open |

## References

- CaboCha: <https://github.com/taku910/cabocha> (`src/dep.cpp`, `src/chunker.cpp`, `src/selector.cpp`, `src/svm_learn.cpp`, `src/eval.cpp`, `tools/kc2cabocha.pl`); 3-clause BSD or LGPL
- J.DepP: <https://www.tkl.iis.u-tokyo.ac.jp/~ynaga/jdepp/>
- Sassano, "Linear-Time Dependency Analysis for Japanese", COLING 2004
- UD Japanese-GSD: <https://github.com/UniversalDependencies/UD_Japanese-GSD> (CC BY-SA 4.0); UD Japanese-PUD: <https://github.com/UniversalDependencies/UD_Japanese-PUD> (CC BY-SA 3.0)
