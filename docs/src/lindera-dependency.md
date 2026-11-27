# Lindera Dependency

Lindera Dependency (`lindera-dependency`) is a crate for Japanese bunsetsu dependency analysis: it groups the morphemes of a sentence into bunsetsu (文節) and finds, for each bunsetsu, the bunsetsu it depends on.

```text
彼は 本を 読んだ。
彼は ──────┐
本を ──────┤
読んだ。 ◀──┘   (root)
```

## Status

> The crate is on crates.io from Lindera 7.1.0 on, and each release of Lindera publishes the **models for IPADIC and for the three UniDic dictionaries** with it. The analysis is complete from morphemes to dependencies: the training data, the chunker, the parser, the model file, the evaluation and a command for text are here, and the token filter of Lindera is in the default builds of the `lindera` command and of the bindings.

Start with the [Installation](./lindera-dependency/installation.md) and the [Quick Start](./lindera-dependency/quickstart.md); [Reading and Using the Result](./lindera-dependency/reading_the_result.md) says what a bunsetsu and a dependency are and how to read what comes out. The work is tracked in [#1417](https://github.com/lindera/lindera/issues/1417).

## What it can do

- **Segment the morphemes of a sentence into bunsetsu** (chunking), with a conditional random field and a model that is a table of weights. With IPADIC, the F-score is 96.85 on the dev file and 96.92 on the test file of UD Japanese-GSD.
- **Keep a model in one file per dictionary and read it in place**: from bytes embedded in a program, read from a file or mapped into memory, without copying its tables. The file names its dictionary, and morphemes that cannot be of that dictionary are refused.
- **Parse the dependencies of the bunsetsu** with shift-reduce and a classifier over features and pairs of features, as CaboCha and J.DepP do, and train that classifier. With IPADIC, 90.11% of the bunsetsu of the dev file of UD Japanese-GSD get the right head with the bunsetsu of the answer (88.53% on the test file), and 85.39% from the morphemes (84.47%).
- **Parse plain text from the command line**, tokenized with a Lindera dictionary, and write what `cabocha -f1` writes.
- **Run as a token filter of Lindera**, `japanese_dependency`: from the `lindera` command, a configuration file, Rust and the five bindings, with the bunsetsu and the head of each token in the JSON output and on the `Token`.
- **Give the head word and the function word of a bunsetsu**, and recognize punctuation and brackets, from a schema of the dictionary. IPADIC is the first schema.
- **Read and write the lattice format** of CaboCha (`cabocha -f1`), and check that a sentence is the tree the parsers assume.
- **Make training data** from a dependency corpus such as UD Japanese-GSD, on the morphemes of the dictionary the parser will run with.
- **Convert the three models CaboCha distributes**, for IPA, for JUMAN and for UniDic, into a model file that analyzes as CaboCha does. Such a model is limited to research and personal use, as CaboCha's.
- **Evaluate** chunking and dependency accuracy as CaboCha and J.DepP count them, and compare two outputs with a paired test.

The default build has no dependency and builds for `wasm32-unknown-unknown`: it holds the types, the lattice format, the schema, the model file, the chunker and the parser. The command is behind the `cli` feature, and the training and the evaluation behind the `train` feature.

## Contents

- [Installation](./lindera-dependency/installation.md) -- The crate, the builds that have the token filter, and the three ways to get the model
- [Quick Start](./lindera-dependency/quickstart.md) -- One sentence parsed with the released model, from the command line and from Rust
- [Reading and Using the Result](./lindera-dependency/reading_the_result.md) -- What a bunsetsu and a dependency are, the result of several kinds of sentence, a program that uses it, and how far to trust it
- [Architecture](./lindera-dependency/architecture.md) -- The modules, the types, the lattice format and the feature flags
- [Chunking](./lindera-dependency/chunking.md) -- Segmenting morphemes into bunsetsu, and training a model
- [Parsing](./lindera-dependency/parsing.md) -- Finding the head of each bunsetsu: the loop, the features, training a model, and its accuracy
- [Model File](./lindera-dependency/model_file.md) -- The file of a dictionary: reading it in place, checking the dictionary, and its format
- [CaboCha's Models](./lindera-dependency/cabocha_models.md) -- Converting the models CaboCha distributes: their terms, the command, what is converted, and the agreement with CaboCha
- [Command Line](./lindera-dependency/command_line.md) -- Parsing plain text with the `lindera-dependency` command: the options, the input layers, the speed
- [Token Filter](./lindera-dependency/token_filter.md) -- The filter `japanese_dependency` of the Lindera tokenizer: the build, what is written on a token, the sentences, the order of the filters, the speed
- [Head and Function Word](./lindera-dependency/head_and_function_word.md) -- The schema of a dictionary and the two words of a bunsetsu
- [Training Data](./lindera-dependency/training_data.md) -- Making training data from UD Japanese-GSD
- [Evaluation](./lindera-dependency/evaluation.md) -- Counting accuracy, comparing two outputs, and cross-validation

- [API Reference](./lindera-dependency/api_reference.md) -- The documentation of the crate on docs.rs

For the decisions behind the crate and the numbers they rest on, see the [design notes](./development/dependency_design.md).
