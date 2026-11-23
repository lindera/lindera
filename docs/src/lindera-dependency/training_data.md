# Training Data

The chunker is trained on bunsetsu, and the parser will be trained on bunsetsu dependencies, both on the morphemes of the dictionary they run with. `lindera-dependency convert`, behind the `train` feature, makes such data from a corpus that has word dependencies and bunsetsu boundaries: a CoNLL-U file with `BunsetuBILabel` in its last column, as UD Japanese-GSD and UD Japanese-PUD have it.

## Converting a corpus

```shell
cargo run --release -p lindera-dependency --features embed-ipadic -- convert \
    --dict embedded://ipadic \
    --input ja_gsd-ud-train.conllu \
    --output train.lattice
```

`--dict` also takes the directory of a dictionary built for Lindera; the `train` feature alone is then enough.

For each sentence, the command:

1. **turns the word dependencies into bunsetsu dependencies.** A word with `BunsetuBILabel=B` starts a bunsetsu, and the word whose head is outside the bunsetsu gives the head of the bunsetsu. Coordination is put back in the form of the Kyoto corpus and CaboCha: each conjunct depends on the next one, with an arc of type `P`;
2. **lays the bunsetsu over the morphemes of the dictionary.** The text of the sentence is tokenized with the dictionary and the default settings of Lindera, as the parser will be used. Where a bunsetsu boundary falls inside a morpheme, the bunsetsu are merged up to the next boundary that fits, and the merged bunsetsu takes the head of the last one, as `tools/kc2cabocha.pl` of CaboCha does;
3. **leaves the sentence out** unless it has one root that is its last bunsetsu, every head to the right, and no crossing arcs.

## The output

The output is in the [lattice format](./architecture.md#the-lattice-format), with bunsetsu lines that hold the head and the arc type only:

```text
* 0 6D 
ただし	接続詞,*,*,*,*,*,ただし,タダシ,タダシ
、	記号,読点,*,*,*,*,、,、,、
* 1 2D 
50	名詞,数,*,*,*,*,*,*,*
周年	名詞,接尾,助数詞,*,*,*,周年,シュウネン,シューネン
ソング	名詞,一般,*,*,*,*,ソング,ソング,ソング
に	助詞,格助詞,一般,*,*,*,に,ニ,ニ
* 2 6D 
変更	名詞,サ変接続,*,*,*,*,変更,ヘンコウ,ヘンコー
後	名詞,接尾,副詞可能,*,*,*,後,ゴ,ゴ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
```

The positions of the head word and of the function word are not written: what reads the data computes them from its [schema](./head_and_function_word.md), so a change of the rule cannot leave stale positions in a file.

## The report

A report goes to the standard error. For the train file of UD Japanese-GSD with IPADIC:

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
head words on the content head of the corpus: 54449 of 56396 (96.55%)
  275: 名詞-一般 is the head word, to the right of 名詞-一般
  240: 名詞-接尾-一般 is the head word, to the right of 名詞-一般
```

- A sentence dropped for two causes counts for both.
- The arcs of type `P` are counted before the bunsetsu are laid over the morphemes, and again in what is written: a merged bunsetsu takes the arc of the last one merged.
- The last lines compare the head word of the schema of IPADIC with the content head the corpus marks (`BunsetuPositionType`), and list the ten most frequent disagreements by part of speech; two are shown here. See [Head and Function Word](./head_and_function_word.md).

## Corpora

The corpora are not part of the repository. [`scripts/dependency/fetch_corpora.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) gets them at the commits the numbers of this chapter were taken at, and `convert_corpora.sh` converts them.

| Corpus | Commit | File | Sentences kept | Bunsetsu | Morphemes |
| --- | --- | --- | --- | --- | --- |
| [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) | c662dc9 | train | 7,024 of 7,050 | 56,396 | 162,456 |
| | | dev | 506 of 507 | 4,147 | 11,934 |
| | | test | 542 of 543 | 4,509 | 12,591 |
| [UD Japanese-PUD](https://github.com/UniversalDependencies/UD_Japanese-PUD) | cf69c5c | test | 994 of 1,000 | 9,843 | 27,728 |

## Licenses of the corpora

UD Japanese-GSD is licensed under CC BY-SA 4.0 and UD Japanese-PUD under CC BY-SA 3.0 (`LICENSE.txt` in each repository). This is how this project handles them; it is not legal advice.

- The corpora, and data converted from them, are not committed to the repository.
- Converted data is adapted from the annotations of the corpus. If you share it, share it under the license of the corpus, name the corpus and say that the data was converted.
- A model trained on converted data is released with the corpus, its version and its license stated. Whether the share-alike condition extends to a trained model is not settled, and this project makes no claim either way.
