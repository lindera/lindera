# Evaluation

`lindera dependency evaluate`, behind the `train` feature, compares an output with the answer, both in the [lattice format](./architecture.md#the-lattice-format) with the same sentences in the same order.

```shell
cargo run --release -p lindera-cli -- dependency evaluate \
    --gold dev.lattice \
    --system output.lattice
```

It counts what CaboCha's `cabocha-system-eval` and J.DepP count, so that the numbers can be set next to theirs. Every ratio comes with its numerator and its denominator; `--json` writes the same as JSON.

## What is counted

- **Chunking**: precision, recall and F over bunsetsu. A bunsetsu of the output is right when the answer has a bunsetsu that starts at the same morpheme and has the same length.
- **Dependency**, on the sentences whose segmentation is the one of the answer (all of them when the parser was given the bunsetsu of the answer): the bunsetsu whose head is right. Level 0 counts every bunsetsu, level 1 every one but the last of its sentence, level 2 every one but the last two. Level 1 is the number J.DepP reports, and the headline number here.
- **End to end**: over the bunsetsu of the answer but the last of each sentence, the ones for which the output has a bunsetsu with the same span, whose head has the span of the head in the answer.

The arc type (`D`, `P`) is not compared. A sentence whose morphemes differ between the two files is left out of every count and reported with its number, and a sentence without bunsetsu in the answer is not counted.

## An example

For the output of the [chunker](./chunking.md) with its defaults on the dev file of UD Japanese-GSD. The chunker writes every bunsetsu as depending on the next one, so the dependency lines are those of that baseline on the sentences it segmented right:

```text
sentences: 506
left out, different morphemes: 0
not counted, no bunsetsu: 0
chunking precision: 97.02% (4009/4132)
chunking recall: 96.67% (4009/4147)
chunking F: 96.85%
chunking sentences: 85.38% (432/506)
dependency level 0: 68.55% (2308/3367)
dependency level 1: 63.92% (1876/2935)
dependency level 2: 57.84% (1453/2512)
dependency sentences: 16.90% (73/432)
end to end: 60.12% (2189/3641)
end to end sentences: 14.43% (73/506)
```

"Every bunsetsu depends on the next one" with the bunsetsu of the answer is the baseline any parser has to beat. At level 1 it is 64.10% (2,334 / 3,641) on the dev file, 62.77% (2,490 / 3,967) on the test file and 60.87% (5,386 / 8,849) on UD Japanese-PUD.

## Comparing two outputs

With `--compare`, a second output is evaluated too, and the two are paired: each segmentation of a sentence, each end-to-end arc and each sentence is right in both, in the first only, in the second only or in neither. The p-value is the one of McNemar's exact test, two-sided, on the two discordant counts. The segmentations compare two chunkers, whatever the heads.

```shell
cargo run --release -p lindera-cli -- dependency evaluate \
    --gold dev.lattice --system first.lattice --compare second.lattice
```

For two chunkers on the 10 folds of the train file (the template of CaboCha, then the default one):

```text
paired segmentations: both 5908, first only 74, second only 107, neither 935, McNemar p = 0.0171
paired arcs: both 29235, first only 117, second only 164, neither 19856, McNemar p = 0.0060
paired sentences: both 887, first only 4, second only 12, neither 6121, McNemar p = 0.0768
```

## Which data for which question

The dev and the test file are small: 3,641 and 3,967 arcs. For an accuracy near 90%, the 95% interval of one such number is about ±1 point, so a difference under a point cannot be told on them alone.

- **Features and settings are compared by cross-validation on the train file**, which gives about 49,000 arcs (about ±0.3 point), and with the paired test above.
- **The dev file chooses between settings.**
- **The test file and UD Japanese-PUD are for the reported result only.**

## Cross-validation

`lindera dependency split` writes the folds. Sentence `i` of the input, from 0, is in the test file of fold `i mod k` and in the training file of every other fold, so the folds are the same wherever the command runs:

```shell
cargo run --release -p lindera-cli -- dependency split \
    --input train.lattice --folds 10 --output-dir folds
```

This writes `fold0.test.lattice` and `fold0.train.lattice` to `fold9.test.lattice` and `fold9.train.lattice`. For the train file of UD Japanese-GSD, each test file has 702 or 703 sentences.

[`scripts/dependency/chunker_experiments.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) is an example of a cross-validation with these commands: it trains on each training file, segments the test file, joins the outputs and evaluates them as one.
