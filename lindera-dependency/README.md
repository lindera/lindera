# lindera-dependency

Japanese bunsetsu dependency analysis for [Lindera](https://github.com/lindera/lindera).

A dependency analysis groups the morphemes of a sentence into bunsetsu (文節)
and finds, for each bunsetsu, the bunsetsu it depends on.

This crate is under development and is not published yet. So far it holds
what the analysis is built on:

- the `Morpheme` trait: a surface and the feature columns of its dictionary;
  the crate depends on no tokenizer
- the `Sentence` and `Bunsetsu` types
- the reading and writing of the lattice format of CaboCha (`cabocha -f1`)
- a check that a sentence is the tree the parsers assume

In its default build it has no dependency, and builds for
`wasm32-unknown-unknown`.

## Training data

The parser is trained on bunsetsu dependencies, on the morphemes of the
dictionary it runs with. The `lindera-dependency` command, behind the `train`
feature, makes such data from a corpus that has word dependencies and
bunsetsu boundaries: a CoNLL-U file with `BunsetuBILabel` in its last column,
as UD Japanese-GSD and UD Japanese-PUD have it.

```shell
cargo run --release -p lindera-dependency --features embed-ipadic -- convert \
    --dict embedded://ipadic \
    --input ja_gsd-ud-train.conllu \
    --output train.lattice
```

`--dict` also takes the directory of a dictionary built for Lindera; the
`train` feature alone is then enough. For each sentence, the command:

1. turns the word dependencies into bunsetsu dependencies. Coordination is
   put back in the form of the Kyoto corpus and CaboCha: each conjunct
   depends on the next one, with an arc of type `P`;
2. tokenizes the text of the sentence with the dictionary, with the default
   settings of Lindera, and lays the bunsetsu over the morphemes. Where a
   bunsetsu boundary falls inside a morpheme, the bunsetsu are merged up to
   the next boundary that fits, and the merged bunsetsu takes the head of
   the last one, as `tools/kc2cabocha.pl` of CaboCha does;
3. leaves the sentence out unless it has one root that is its last bunsetsu,
   every head to the right, and no crossing arcs.

The output is in the lattice format, with bunsetsu lines that hold the head
and the arc type only. A report goes to the standard error. For the train
file of UD Japanese-GSD with IPADIC:

```text
ja_gsd-ud-train.conllu
sentences kept: 7024 of 7050 (99.63%)
sentences dropped: 26
  bunsetsu that cannot be made: 0
  root: 0
  arc to the left: 3
  crossing arcs: 24
  text that does not match: 0
bunsetsu with more than one word that leaves it: 0
arcs of type P: 517 of 49764 (1.04%)
bunsetsu boundaries on a morpheme boundary: 49372 of 49764 (99.21%)
sentences with merged bunsetsu: 372 of 7024 (5.30%)
bunsetsu merged: 392 of 56788 (0.69%)
bunsetsu written: 56396
arcs of type P written: 489 of 49372 (0.99%)
morphemes written: 162456
```

A sentence dropped for two causes counts for both. The arcs of type `P` are
counted before the bunsetsu are laid over the morphemes, and again in what
is written: a merged bunsetsu takes the arc of the last one merged.

### Corpora

The corpora are not part of this repository. The numbers here were taken at
these commits, with the IPADIC that Lindera embeds (`embed-ipadic`):

| Corpus | Commit | File | Sentences kept | Bunsetsu | Morphemes |
| --- | --- | --- | --- | --- | --- |
| [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) | c662dc9 | train | 7,024 of 7,050 | 56,396 | 162,456 |
| | | dev | 506 of 507 | 4,147 | 11,934 |
| | | test | 542 of 543 | 4,509 | 12,591 |
| [UD Japanese-PUD](https://github.com/UniversalDependencies/UD_Japanese-PUD) | cf69c5c | test | 994 of 1,000 | 9,843 | 27,728 |

### Licenses of the corpora

UD Japanese-GSD is licensed under CC BY-SA 4.0 and UD Japanese-PUD under
CC BY-SA 3.0 (`LICENSE.txt` in each repository). This is how this project
handles them; it is not legal advice.

- The corpora, and data converted from them, are not committed to this
  repository.
- Converted data is adapted from the annotations of the corpus. If you share
  it, share it under the license of the corpus, name the corpus and say that
  the data was converted.
- A model trained on converted data is released with the corpus, its version
  and its license stated. Whether the share-alike condition extends to a
  trained model is not settled, and this project makes no claim either way.

## License

MIT
