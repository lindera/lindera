# Measurement scripts of lindera-dependency

The scripts that give the numbers of the design notes of `lindera-dependency`
([English](../../docs/src/development/dependency_design.md),
[Japanese](../../docs/ja/src/development/dependency_design.md)) and of the
README of the crate, and the script that builds the model that is
released. They are not part of the build and CI does not run them: they
need corpora this repository does not hold.

## What they need

- **The corpora**, which `fetch_corpora.sh` gets at the commits the numbers
  were taken at: UD Japanese-GSD c662dc9 (CC BY-SA 4.0) and UD Japanese-PUD
  cf69c5c (CC BY-SA 3.0). They are not committed, and neither is anything
  made from them: everything goes to `scripts/dependency/data/`, which git
  ignores. See the README of the crate for what the licenses ask.
- **The dictionary**: IPADIC, embedded by the `embed-ipadic` feature: no
  dictionary has to be downloaded for it. The scripts that take
  `--dictionary unidic`, `unidic-cwj` or `unidic-csj` get that dictionary
  with `lindera download`, which installs the one of the release of the
  version of the workspace into `data/dictionaries`, or take the one of
  `--dict`. UniDic is not embedded: embedding UniDic CWJ or CSJ takes 8 to
  11 GiB of memory to compile.
- **Python 3.10 or later**, with the standard library only.
- **For the baselines**: CaboCha and J.DepP, with CRF++ and MeCab, which
  `build_baselines.sh` builds from their sources into
  `scripts/dependency/data/tools`. It needs a C++ compiler, make, curl and
  the helper scripts of automake. None of their sources is committed here.
- A Rust toolchain, as for the rest of the repository.

Run everything from the root of the repository.

## The scripts

| Script | What it reproduces |
| --- | --- |
| `fetch_corpora.sh` | Gets the two corpora |
| `convert_corpora.sh` | The reports of `lindera-dependency convert` for GSD train, dev, test and PUD: the sentences kept and dropped, the bunsetsu merged, the agreement of the head word with the content head of the corpus. `--dictionary unidic`, `unidic-cwj` or `unidic-csj` converts with that dictionary, into `data/<name>/` |
| `dictionary.sh` | The arguments `--dictionary` and `--dict` of the scripts that take them; it is sourced, not run |
| `compare_dictionaries.py` | How much the output of two dictionaries differs on the same sentences: the sentences with the same morphemes, with the same part of speech too, and with the same bunsetsu and heads |
| `conversion_stats.py` | The conversion of word dependencies to bunsetsu dependencies, written independently in Python: the bunsetsu, the sentences kept, the arcs of type `P`, the causes of the sentences dropped |
| `projection_stats.py` | The projection onto the morphemes of IPADIC, written independently in Python: the boundaries that fit, the bunsetsu merged. It writes lattice files |
| `compare_outputs.py` | Compares the lattice files of `projection_stats.py` with those of `convert`, line by line |
| `baseline_next.py` | The baseline "every bunsetsu depends on the next one", counted independently of `lindera-dependency evaluate` |
| `build_baselines.sh` | Builds the tools of the baselines from source, at fixed versions: CRF++ c603ea6, MeCab 61b90ba with its IPA dictionary, CaboCha 96e100a, J.DepP 2022-03-18. Its header says what each needs to build with a current compiler |
| `baselines.sh` | The baselines: CaboCha and J.DepP trained on the train file with the morphemes of IPADIC, and run on dev, test and PUD. For each, the dependency accuracy with the bunsetsu of the answer, the one from the morphemes and the F of the chunking, counted by `lindera-dependency evaluate`, next to what `cabocha-system-eval` and J.DepP report; and the arcs on which the two differ |
| `cabocha_chunker_cost.sh` | The cost of CaboCha's chunker, which the baseline takes at 4: the F of the chunking for each candidate, on dev and test and over a 10-fold cross-validation of train |
| `jdepp_format.py` | The input of J.DepP from a lattice file, and its outputs back in the lattice format. J.DepP takes a line by its first character, so a morpheme that starts with `*` or `#` is given to it with the full-width character |
| `baseline_numbers.py` | The lines `baselines.sh` prints, from the JSON of `evaluate` |
| `chunker_experiments.sh` | The experiments behind the defaults of the chunker: the cross-validation, the pairs of settings, the result. `--dictionary <name>` runs them with a UniDic dictionary; the stage `base` has the pairs around the base form, which the settings for UniDic rest on, and the stage `check` the settings for UniDic and their neighbours only |
| `chunker_experiment.py`, `make_templates.py` | The driver and the feature templates of `chunker_experiments.sh` |
| `parser_instances.sh` | The training instances of the parser for the four files, with `lindera-dependency instances`: the sentences, the instances, and that the answer of every sentence gives its tree back. `--dictionary` as for `convert_corpora.sh` |
| `parser_experiments.sh` | The experiments behind the defaults of the parser: the cross-validations of the algorithm, the feature set, the pairs, their table and the quantization, the pairs of settings, the result on dev, test and PUD, and the speed. `--dictionary <name>` runs them with a UniDic dictionary |
| `parser_experiment.py` | The driver of `parser_experiments.sh` |
| `parse_speed.sh` | The sentences per second of `lindera-dependency parse` on plain text, in the build with `cli,embed-ipadic`: the text of the test file of UD Japanese-GSD, with and without the time to load the dictionary and the model. `--dict` gives another dictionary than the embedded IPADIC |
| `filter_speed.sh` | What the token filter `japanese_dependency` costs in `lindera tokenize`, in the build with `embed-ipadic,dependency`: the tokens per second without and with the filter on the text of the test file of UD Japanese-GSD, and the time the filter adds to a token. `--dict` as for `parse_speed.sh` |
| `build_model.sh` | The model for IPADIC as a release of Lindera publishes it: the directory `lindera-dependency-ipadic` with the model file, `model.json` and `NOTICE.txt`, and its archive, from the train file of UD Japanese-GSD and the sources of the repository, with the dependencies of `Cargo.lock`. `--check <sha256>` compares the model with a known digest. `release.yml` runs it for a release; a build from the repository embeds its model through `LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`. `--dictionary unidic`, `unidic-cwj` or `unidic-csj` builds the model for that dictionary, `lindera-dependency-<name>`, with the dictionary of `--dict` or the one `lindera download` installs |
| `model_notice.txt` | The template of the notice `build_model.sh` writes, for any dictionary |
| `model_accuracy.py` | The counts of a model file on GSD dev, GSD test and PUD: the dependency accuracy with the bunsetsu of the answer and from the morphemes, and the F of the chunking. `--dict` gives the dictionary of a model for UniDic. With `--expect model_accuracy_<dictionary>.json` it fails when a count is not the one of the documentation, which is how `release.yml` checks the model it is about to publish |
| `model_speed/` | The size of a model file, the time to read it, and the sentences per second of chunking, of parsing and of both; with a training file, also of a chunker in memory |
| `head_variants/` | The agreement of the head word for CaboCha's rule and for each addition of `Schema::ipadic()`, measured with `corpus::convert`; with `--dict` and a UniDic dictionary, the same for `Schema::unidic()`, and for the changes of it that were not taken |
| `head_rules.py` | The same in Python, with examples by surface and the variants that were not taken |

`head_variants/` and `model_speed/` are Rust programs, each with a workspace
of its own: they are not members of the workspace of the repository, CI does
not build them and they are not published. Their `Cargo.lock` is not
committed.

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

# The baselines of CaboCha and J.DepP: the tools (about a minute and a
# half, with the downloads), then the trainings and the counts (half a
# minute), and the cost of CaboCha's chunker (four minutes).
scripts/dependency/build_baselines.sh
scripts/dependency/baselines.sh
scripts/dependency/cabocha_chunker_cost.sh

# The chunker: about an hour for all the stages.
scripts/dependency/chunker_experiments.sh all

# The parser: every experiment, about ten minutes. The model of the
# defaults is $data/parser/full/default/model.
scripts/dependency/parser_experiments.sh all

# The model file: its size, the time to read it, the speed; and the chunker
# in memory against the chunker of a file.
cargo run --release --manifest-path scripts/dependency/model_speed/Cargo.toml -- \
    $data/parser/full/default/model $data/lattice/ja_gsd-ud-train.lattice 7 \
    $data/lattice/ja_gsd-ud-train.lattice

# The model as a release publishes it: its directory and its archive go
# to $data/model. It needs a tree without uncommitted changes.
scripts/dependency/build_model.sh
python3 -I -B scripts/dependency/model_accuracy.py target/release/lindera-dependency \
    $data/model/lindera-dependency-ipadic/dependency.model $data/model/work/corpora \
    $data/model/work/accuracy --expect scripts/dependency/model_accuracy_ipadic.json

# The same for a UniDic dictionary: the experiments, the model, its counts.
# The chunker: about an hour and a quarter for all the stages; `check`
# alone, about a quarter of an hour, is what was run for unidic and for
# unidic-csj.
scripts/dependency/chunker_experiments.sh --dictionary unidic-cwj all
scripts/dependency/parser_experiments.sh --dictionary unidic-cwj all
scripts/dependency/build_model.sh --dictionary unidic-cwj \
    --dict $data/dictionaries/unidic-cwj $data/model-unidic-cwj
python3 -I -B scripts/dependency/model_accuracy.py target/release/lindera-dependency \
    $data/model-unidic-cwj/lindera-dependency-unidic-cwj/dependency.model \
    $data/model-unidic-cwj/work/corpora $data/model-unidic-cwj/work/accuracy \
    --dict $data/dictionaries/unidic-cwj \
    --expect scripts/dependency/model_accuracy_unidic-cwj.json
scripts/dependency/parse_speed.sh --dict $data/dictionaries/unidic-cwj \
    $data/model-unidic-cwj/lindera-dependency-unidic-cwj/dependency.model

# The training instances of the parser.
scripts/dependency/parser_instances.sh

# The head word.
cargo run --release --manifest-path scripts/dependency/head_variants/Cargo.toml -- $corpora
python3 -I -B scripts/dependency/head_rules.py $data table
python3 -I -B scripts/dependency/head_rules.py $data disagreements

# The same with a UniDic dictionary (unidic, unidic-cwj or unidic-csj): the
# dictionary goes to $data/dictionaries, the files to $data/unidic-cwj.
scripts/dependency/convert_corpora.sh --dictionary unidic-cwj
scripts/dependency/parser_instances.sh --dictionary unidic-cwj
cargo run --release --manifest-path scripts/dependency/head_variants/Cargo.toml -- \
    --dict $data/dictionaries/unidic-cwj $corpora

# How much the three UniDic dictionaries differ, once each is converted.
python3 -I -B scripts/dependency/compare_dictionaries.py \
    unidic=$data/unidic/lattice unidic-cwj=$data/unidic-cwj/lattice \
    unidic-csj=$data/unidic-csj/lattice
```

`-I -B` keeps Python from reading the environment and from writing
`__pycache__` into the repository.

## What to expect

With the corpora at the commits above and the IPADIC of this repository
(mecab-ipadic 2.7.0-20250920), the numbers do not depend on the machine.
The baselines were measured with the tools built by GCC 15.2 on Linux
(x86_64); another compiler or system was not tried.
The models of the chunker are the same bytes each time. The numbers for
UniDic are with the dictionaries of the release 7.0.0 of Lindera, which
`lindera download` installs.

| Step | Number |
| --- | --- |
| `conversion_stats.py` | GSD: 65,966 bunsetsu, each with exactly one word whose head is outside it; 8,072 of 8,100 sentences kept (train 7,024, dev 506, test 542). PUD: 994 of 1,000 |
| `convert_corpora.sh` | Bunsetsu boundaries on a morpheme boundary: 99.21% (train), 99.16% (dev), 99.10% (test), 99.45% (PUD). Bunsetsu written: 56,396, 4,147, 4,509, 9,843 |
| `compare_outputs.py` | `COMPARE_OK`, every line identical |
| `baseline_next.py` | Level 1: 64.10% (2,334 / 3,641) on dev, 62.77% (2,490 / 3,967) on test, 60.87% (5,386 / 8,849) on PUD; the same from `evaluate` |
| `build_baselines.sh` | `the tools are in ...`; a second run says that the tools of these versions are there |
| `baselines.sh` | CaboCha: level 1 89.84% (3,271 / 3,641) on dev, 88.51% (3,511 / 3,967) on test, 88.65% (7,845 / 8,849) on PUD; from the morphemes 85.00%, 84.25%, 85.69%; chunking F 96.79, 96.80, 97.81; the same counts from `cabocha-system-eval`. J.DepP: level 1 90.11% (3,281 / 3,641), 88.71% (3,519 / 3,967), 88.98% (7,874 / 8,849); from the morphemes 85.99%, 84.65%, 86.45%; chunking F 97.17, 97.02, 98.04; the same from J.DepP itself. The two differ on 127, 196 and 424 arcs. The same lines twice |
| `cabocha_chunker_cost.sh` | For the costs 0.5, 1, 2, 4, 8 and 16: chunking F 96.66, 96.64, 96.68, 96.79, 96.82, 96.86 on dev, and 96.45, 96.57, 96.63, 96.66, 96.66, 96.67 over the cross-validation |
| `chunker_experiments.sh default` | F 96.85 on dev, 96.92 on test, 98.04 on PUD; 4,325 feature strings, 8,656 weights, 261,851 bytes |
| `chunker_experiments.sh cv` | F over the 10 folds of train, such as 96.83 for CaboCha's columns with L1 and 0.25, and 96.93 for the default setting; the README of the crate has the table |
| `model_speed/` | 261,851 bytes; about 228 microseconds to read the file; chunking the train file at about 18,223 sentences per second with the model in memory and 16,896 on the bytes of the file. The times depend on the machine: these are of one core of an AMD Ryzen AI Max+ 395 |
| `parser_experiments.sh final` | With the bunsetsu of the answer 90.11% on dev, 88.53% on test, 88.81% on PUD; from the morphemes 85.39%, 84.47%, 86.19%; 6,258,548 bytes, the same bytes twice |
| `build_model.sh` | The model is 6,258,548 bytes, SHA-256 `793b22e6ef2d92abf0fd91f282942acc6ebd568c22b930770ff2ff57573fbe46`, on x86_64 Linux with glibc 2.43 and Rust 1.100.0; the same bytes as the model of `parser_experiments.sh final`. The same sources, corpus, toolchain and platform give the same bytes, and the same archive for the same version. Another platform is not known to: the training of the chunker calls `exp` and `ln_1p` of its mathematics library |
| `model_accuracy.py` | The counts of `parser_experiments.sh final`, and `the counts are those of model_accuracy_ipadic.json` |
| `parser_experiments.sh algorithm` and the other stages | Level 1 over the 10 folds of train, such as 88.96% for CaboCha's feature set and 89.21% for the default setting; the README of the crate has the tables |
| `parser_experiments.sh speed` | About 16,700 sentences per second for parsing and 8,700 for chunking and parsing with the default model; the times depend on the machine |
| `parse_speed.sh <model>` | 543 sentences; about 6,600 sentences per second from the text to the lattice format, about 5 ms to load; the times depend on the machine |
| `filter_speed.sh <model>` | 12,644 tokens in 543 sentences, 20 times; about 2,900,000 tokens per second without the filter and 180,000 with it (7,700 sentences); the filter adds about 5.2 microseconds per token, and making it takes about 4 ms; the times depend on the machine |
| `chunker_experiments.sh --dictionary <name> check` | F over the 10 folds with the settings for IPADIC and with those for UniDic: 97.02 and 97.15 with `unidic`, 97.02 and 97.22 with `unidic-cwj`, 96.94 and 97.08 with `unidic-csj`; the README of the crate has the tables of UniDic CWJ |
| `chunker_experiments.sh --dictionary <name> default` | F on dev, test and PUD: 97.03, 97.24, 98.00 with `unidic`; 96.63, 97.02, 97.89 with `unidic-cwj`; 96.60, 96.95, 97.78 with `unidic-csj`. The chunker has 360,869, 360,019 and 365,215 bytes |
| `parser_experiments.sh --dictionary <name> final` | With the bunsetsu of the answer on dev, test and PUD: 89.67%, 88.69%, 88.67% with `unidic`; 89.35%, 88.91%, 88.58% with `unidic-cwj`; 89.75%, 88.90%, 88.59% with `unidic-csj`. From the morphemes: 85.06%, 84.88%, 85.93%; 84.22%, 84.71%, 85.73%; 84.58%, 84.65%, 85.58%. The same bytes twice |
| `build_model.sh --dictionary <name>` | The models are 6,257,148, 6,258,600 and 6,267,183 bytes, with the dictionaries of the release 7.0.0; each the same bytes as the model of `parser_experiments.sh --dictionary <name> final` |
| `model_accuracy.py --dict <dictionary>` | The counts of `parser_experiments.sh --dictionary <name> final`, and `the counts are those of model_accuracy_<name>.json` |
| `parse_speed.sh --dict <dictionary> <model>` | About 5,300 sentences per second with `unidic`, 4,500 with `unidic-cwj` and 4,100 with `unidic-csj`; about 60 ms, 390 ms and 390 ms to load; the times depend on the machine |
| `filter_speed.sh --dict <dictionary> <model>` | The filter adds about 6.4, 6.9 and 7.2 microseconds per token; the times depend on the machine |
| `parser_instances.sh` | Sentences 7,024, 506, 542, 994, as `convert` kept; instances 66,000, 4,881, 5,377, 12,739. The command stops at a tree that does not come back, so four reports mean that every tree did |
| `head_variants/` | The head word on the content head of the corpus: from 85.43% with CaboCha's rule to 96.55% with `Schema::ipadic()` on GSD train; 96.96% on dev, 96.92% on test, 97.46% on PUD |
| `convert_corpora.sh --dictionary <name>` | Bunsetsu boundaries of GSD train on a morpheme boundary: 99.87% with `unidic`, 99.95% with `unidic-cwj`, 99.94% with `unidic-csj`. Bunsetsu written for train: 56,724, 56,763, 56,759 |
| `parser_instances.sh --dictionary <name>` | Sentences as with IPADIC; instances of train 66,528 with `unidic`, 66,618 with `unidic-cwj`, 66,609 with `unidic-csj` |
| `head_variants/ --dict <UniDic CWJ>` | From 82.50% with CaboCha's rule to 97.70% with `Schema::unidic()` on GSD train; 97.61% on dev, 97.95% on test, 98.16% on PUD. The schema on train with `unidic` 97.52%, with `unidic-csj` 97.55%. Not taken: 91.14%, 96.75% and 97.50% on train |
| `compare_dictionaries.py` | Sentences of GSD train with the same morphemes, and with the same part of speech too: `unidic` and `unidic-cwj` 85.88% and 66.06%, `unidic` and `unidic-csj` 86.33% and 62.64%, `unidic-cwj` and `unidic-csj` 88.10% and 73.22% |

## What these scripts do not reproduce

Some numbers of the design notes came from prototypes that are lost, and
nothing here gives them: the learning curve, the bunsetsu boundaries per set of break
characters, the decoding of a CRF++ text model against CaboCha, and the
agreement of the head word for UniDic on the tags of the corpus itself
(`head_variants/` gives it through the output of Lindera). The design
notes mark each of them.

## Keeping them current

A pull request that changes a number these scripts give runs the script
again and updates the design notes and the README of the crate with it.
`head_variants/` checks that its last variant is `Schema::ipadic()`, or
`Schema::unidic()` for a UniDic dictionary, so a change of the schema that
it does not follow stops it.
