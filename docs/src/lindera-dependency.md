# Lindera Dependency

Lindera Dependency (`lindera-dependency`) is a crate for Japanese bunsetsu dependency analysis: it groups the morphemes of a sentence into bunsetsu (文節) and is to find, for each bunsetsu, the bunsetsu it depends on.

```text
彼は 本を 読んだ。
彼は ──────┐
本を ──────┤
読んだ。 ◀──┘   (root)
```

## Status

> The crate is **under development and not published** on crates.io. The **dependency parser, which finds the head of each bunsetsu, does not exist yet**. What exists is everything before it: the types, the training data, the evaluation, the segmentation into bunsetsu, and the head word and function word of a bunsetsu.

To use it today, build it from a checkout of the [repository](https://github.com/lindera/lindera). The interfaces may change until the first release. The work is tracked in [#1417](https://github.com/lindera/lindera/issues/1417).

## What it can do

- **Segment the morphemes of a sentence into bunsetsu** (chunking), with a conditional random field and a model that is a table of weights. With IPADIC, the F-score is 96.85 on the dev file and 96.92 on the test file of UD Japanese-GSD.
- **Give the head word and the function word of a bunsetsu**, and recognize punctuation and brackets, from a schema of the dictionary. IPADIC is the first schema.
- **Read and write the lattice format** of CaboCha (`cabocha -f1`), and check that a sentence is the tree the parsers assume.
- **Make training data** from a dependency corpus such as UD Japanese-GSD, on the morphemes of the dictionary the parser will run with.
- **Evaluate** chunking and dependency accuracy as CaboCha and J.DepP count them, and compare two outputs with a paired test.

The default build has no dependency and builds for `wasm32-unknown-unknown`: it holds the types, the lattice format, the schema and the chunker. The command, the training and the evaluation are behind the `train` feature.

## Contents

- [Architecture](./lindera-dependency/architecture.md) -- The modules, the types, the lattice format and the feature flags
- [Chunking](./lindera-dependency/chunking.md) -- Segmenting morphemes into bunsetsu, and training a model
- [Head and Function Word](./lindera-dependency/head_and_function_word.md) -- The schema of a dictionary and the two words of a bunsetsu
- [Training Data](./lindera-dependency/training_data.md) -- Making training data from UD Japanese-GSD
- [Evaluation](./lindera-dependency/evaluation.md) -- Counting accuracy, comparing two outputs, and cross-validation

There is no API reference page yet: the crate is not on docs.rs. Build the API documentation locally with `cargo doc -p lindera-dependency --features train --open`.

For the decisions behind the crate and the numbers they rest on, see the [design notes](./development/dependency_design.md).
