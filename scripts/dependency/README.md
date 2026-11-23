# Measurement scripts of lindera-dependency

The scripts that give the numbers of the design notes of `lindera-dependency`
([English](../../docs/src/development/dependency_design.md),
[Japanese](../../docs/ja/src/development/dependency_design.md)) and of the
README of the crate. They are not part of the build and CI does not run
them: they need corpora this repository does not hold.

## What they need

- **The corpora**, which `fetch_corpora.sh` gets at the commits the numbers
  were taken at: UD Japanese-GSD c662dc9 (CC BY-SA 4.0) and UD Japanese-PUD
  cf69c5c (CC BY-SA 3.0). They are not committed, and neither is anything
  made from them: everything goes to `scripts/dependency/data/`, which git
  ignores. See the README of the crate for what the licenses ask.
- **The dictionary**: IPADIC, embedded by the `embed-ipadic` feature. No
  dictionary has to be downloaded.
- **Python 3.10 or later**, with the standard library only.
- A Rust toolchain, as for the rest of the repository.

Run everything from the root of the repository.

## The scripts

| Script | What it reproduces |
| --- | --- |
| `fetch_corpora.sh` | Gets the two corpora |
| `convert_corpora.sh` | The reports of `lindera-dependency convert` for GSD train, dev, test and PUD: the sentences kept and dropped, the bunsetsu merged, the agreement of the head word with the content head of the corpus |
| `conversion_stats.py` | The conversion of word dependencies to bunsetsu dependencies, written independently in Python: the bunsetsu, the sentences kept, the arcs of type `P`, the causes of the sentences dropped |
| `projection_stats.py` | The projection onto the morphemes of IPADIC, written independently in Python: the boundaries that fit, the bunsetsu merged. It writes lattice files |
| `compare_outputs.py` | Compares the lattice files of `projection_stats.py` with those of `convert`, line by line |
| `baseline_next.py` | The baseline "every bunsetsu depends on the next one", counted independently of `lindera-dependency evaluate` |
| `chunker_experiments.sh` | The experiments behind the defaults of the chunker: the cross-validation, the pairs of settings, the result |
| `chunker_experiment.py`, `make_templates.py` | The driver and the feature templates of `chunker_experiments.sh` |
| `head_variants/` | The agreement of the head word for CaboCha's rule and for each addition of `Schema::ipadic()`, measured with `corpus::convert` |
| `head_rules.py` | The same in Python, with examples by surface and the variants that were not taken |

`head_variants/` is a Rust program with a workspace of its own: it is not a
member of the workspace of the repository, CI does not build it and it is
not published. Its `Cargo.lock` is not committed.

## The steps

```shell
# The corpora, then the training data and the reports of convert.
scripts/dependency/fetch_corpora.sh
scripts/dependency/convert_corpora.sh

data=scripts/dependency/data
corpora="$data/gsd/ja_gsd-ud-train.conllu $data/gsd/ja_gsd-ud-dev.conllu \
    $data/gsd/ja_gsd-ud-test.conllu $data/pud/ja_pud-ud-test.conllu"

# The same conversion and projection in Python, compared with convert.
python3 -I -B scripts/dependency/conversion_stats.py $corpora
cargo build --release -p lindera-cli --features embed-ipadic
python3 -I -B scripts/dependency/projection_stats.py \
    target/release/lindera embedded://ipadic $data/python $corpora
python3 -I -B scripts/dependency/compare_outputs.py $data/python $data/lattice \
    ja_gsd-ud-train ja_gsd-ud-dev ja_gsd-ud-test ja_pud-ud-test

# The baseline, by the script and by evaluate.
python3 -I -B scripts/dependency/baseline_next.py --write $data/next \
    $data/lattice/ja_gsd-ud-dev.lattice $data/lattice/ja_gsd-ud-test.lattice \
    $data/lattice/ja_pud-ud-test.lattice
target/release/lindera-dependency evaluate \
    --gold $data/lattice/ja_gsd-ud-dev.lattice --system $data/next/ja_gsd-ud-dev.lattice

# The chunker: about an hour for all the stages.
scripts/dependency/chunker_experiments.sh all

# The head word.
cargo run --release --manifest-path scripts/dependency/head_variants/Cargo.toml -- $corpora
python3 -I -B scripts/dependency/head_rules.py $data table
python3 -I -B scripts/dependency/head_rules.py $data disagreements
```

`-I -B` keeps Python from reading the environment and from writing
`__pycache__` into the repository.

## What to expect

With the corpora at the commits above and the IPADIC of this repository
(mecab-ipadic 2.7.0-20250920), the numbers do not depend on the machine.
The models of the chunker are the same bytes each time.

| Step | Number |
| --- | --- |
| `conversion_stats.py` | GSD: 65,966 bunsetsu, each with exactly one word whose head is outside it; 8,072 of 8,100 sentences kept (train 7,024, dev 506, test 542). PUD: 994 of 1,000 |
| `convert_corpora.sh` | Bunsetsu boundaries on a morpheme boundary: 99.21% (train), 99.16% (dev), 99.10% (test), 99.45% (PUD). Bunsetsu written: 56,396, 4,147, 4,509, 9,843 |
| `compare_outputs.py` | `COMPARE_OK`, every line identical |
| `baseline_next.py` | Level 1: 64.10% (2,334 / 3,641) on dev, 62.77% (2,490 / 3,967) on test, 60.87% (5,386 / 8,849) on PUD; the same from `evaluate` |
| `chunker_experiments.sh default` | F 96.85 on dev, 96.92 on test, 98.04 on PUD; 4,325 feature strings, 8,656 weights |
| `chunker_experiments.sh cv` | F over the 10 folds of train, such as 96.83 for CaboCha's columns with L1 and 0.25, and 96.93 for the default setting; the README of the crate has the table |
| `head_variants/` | The head word on the content head of the corpus: from 85.43% with CaboCha's rule to 96.55% with `Schema::ipadic()` on GSD train; 96.96% on dev, 96.92% on test, 97.46% on PUD |

## What these scripts do not reproduce

Some numbers of the design notes came from prototypes that are lost, and
nothing here gives them: the accuracy of CaboCha and of J.DepP on the same
data, the learning curve, the bunsetsu boundaries per set of break
characters, the decoding of a CRF++ text model against CaboCha, and the
agreement of the head word for UniDic. The design notes mark each of them.

## Keeping them current

A pull request that changes a number these scripts give runs the script
again and updates the design notes and the README of the crate with it.
`head_variants/` checks that its last variant is `Schema::ipadic()`, so a
change of the schema that it does not follow stops it.
