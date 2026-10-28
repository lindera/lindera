# Training Pipeline

Lindera provides CRF-based dictionary training functionality for creating custom morphological analysis models. This feature requires the `train` feature flag.

## Overview

The training pipeline follows three stages:

```text
lindera train --> model.dat --> lindera export --> dictionary files --> lindera build --> compiled dictionary
```

1. **Train**: Learn CRF weights from an annotated corpus and seed dictionary, producing a binary model file.
2. **Export**: Convert the trained model into Lindera dictionary source files.
3. **Build**: Compile the source files into a binary dictionary that Lindera can load at runtime.

### How training works

`lindera train` builds one lattice for each sentence of the corpus, as MeCab's learner does. At every position, the lattice has the seed words that start there and the unknown words that the runtime creates there from `char.def` and `unk.def` (see [Character Definition](#3-character-definition-chardef) and [Unknown Word Definition](#4-unknown-word-definition-unkdef)). The CRF learns the weights that make the annotated path, the gold tokens, the cheapest. Each gold token is labelled with the first of these that fits:

1. The first seed row, in file order, with the same span and the same whole feature string. Unlike MeCab's learner, which compares only the first `eval-size` columns, every column must match, so a token whose reading or other columns differ from the seed row falls through to the next steps.
2. The first `unk.def` entry, in file order, of an unknown-word candidate with the same span whose feature columns other than `*` all equal the gold token's columns.
3. A virtual label built from the gold feature string, as MeCab's learner adds a virtual node. Gold tokens with the same feature string share one virtual label. Virtual labels are trained but not exported.

Every other candidate is a negative example. Once the lattices are built, `lindera train` prints how the gold tokens were labelled; with the files in `resources/training/`:

```text
Lattice statistics: gold_lexicon=14 gold_unknown=0 gold_virtual=0 virtual_labels=0 negative_lexicon=0 negative_unknown=17
```

`gold_lexicon`, `gold_unknown` and `gold_virtual` count the gold tokens labelled by each of the three steps, `virtual_labels` counts the virtual labels, and `negative_lexicon` and `negative_unknown` count the negative edges from the seed lexicon and from the unknown-word candidates.

A seed row that no gold token is labelled with is only ever a negative example. So `lindera train` warns on standard error when no gold token is a seed word, or when fewer than half of the gold tokens whose surface has seed rows are seed words, as when the corpus does not give its gold tokens the feature strings of the seed rows. The warning shows the first gold token whose surface has seed rows but none with its feature string, next to the feature strings of the first five of those rows, and training goes on. For example, with every feature of `resources/training/corpus.txt` cut to its first six columns, without the base form and the readings:

```text
Warning: 0 of the 14 gold tokens are seed words, and 14 are not although seed rows have their surface. A gold token is a seed word only when a seed row has its surface and its whole feature string, as the seed lexicon writes it; otherwise the seed rows with its surface are only wrong candidates at its position. The first such gold token, and the seed rows with its surface:
  gold token: 外国	名詞,一般,*,*,*,*
  seed row:   外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
```

Gold tokens whose surface no seed row has do not count toward the warning, so a seed lexicon that covers little of the corpus gives none.

The CRF is then trained with L-BFGS (OWL-QN when the regularization has an L1 part). Training stops at the first of these:

- `converged`: the relative change of the objective, `|previous - current| / |previous|`, stayed below `eta` for 3 iterations in a row. The default `eta` is that of MeCab's learner, which applies the same test to every evaluation, line-search trials included, where Lindera counts iterations.
- `no progress`: an iteration left the objective unchanged. When this happens after a few iterations, the model is barely trained. `lindera train` writes a warning on standard error whenever training stops this way, after any number of iterations.
- The line search of an iteration failed; the model keeps the best weights of the run, and `lindera train` writes a warning on standard error.
- `max-iterations` was reached before the objective converged.

`lindera train` prints which one stopped it, with the number of iterations and of evaluations, each evaluation being one forward-backward pass over every lattice; with the files in `resources/training/`:

```text
L-BFGS stopped after 9 iterations and 11 evaluations of the loss and its gradient: converged
```

## Required Input Files

### 1. Seed Lexicon (seed.csv)

Base vocabulary dictionary in MeCab CSV format.

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

Each line contains: `surface,left_id,right_id,cost,pos,pos_detail1,pos_detail2,pos_detail3,inflection_type,inflection_form,base_form,reading,pronunciation`

The `left_id`, `right_id`, and `cost` fields are set to 0 in the seed dictionary -- the trainer will compute appropriate values from the CRF model.

The seed lexicon is read as CSV, as `lindera build` reads `lex.csv`:

- A field may be quoted, as mecab-ko-dic's comma surface is: `",",1792,3558,788,SC,*,*,*,*,*,*,*`.
- A line that starts with `#` is a row, not a comment, as UniDic's `#` surface is. Blank lines and rows with fewer than five fields are skipped.
- The feature of a row is the text after its fourth field, as written, quotes included. `lindera export` writes it back as it is, and quotes a surface that contains `,`, `"` or a line break.
- A quoted field that spans lines is an error that names the line. It is most likely a quote that is never closed, which would otherwise swallow the rows after it.
- The rows keep their order, which gives their labels their ids.

To read the columns of a feature, for the rewrite rules, the feature templates and unknown-word matching, `lindera train` splits it as MeCab's learner does (`tokenizeCSV`), not as the CSV reader above does. The features of the corpus are split the same way.

- A quoted column is one column, with `""` read as `"`. The definition of mecab-jieba's `4S店`, `"authorized full-service car dealership (offering sales, spare parts, after-sales service and customer support)"`, is one column.
- Spaces and tabs before a column are skipped: CC-CEDICT's `COVID-19, the coronavirus disease identified in 2019/` is the two columns `COVID-19` and `the coronavirus disease identified in 2019/`.
- The text from a closing quote to the next `,` is dropped: `"bamboo" radical in Chinese characters (Kangxi radical 118)/` is the column `bamboo`.
- A `,` at the end of a feature does not start an empty last column.

### 2. Training Corpus (corpus.txt)

Annotated text data in tab-separated format. Each line is `surface<TAB>pos_info`, and sentences are separated by `EOS`.

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
EOS
```

A gold token is a seed word only when a seed row has its surface and its whole feature string, as the seed lexicon writes it, quotes included. In the example above, each feature is that of the seed row with the same surface. A gold token whose feature differs in any column, the reading for instance, is learned as an unknown word or a virtual label, and the seed rows with its surface become negative examples at its position; `lindera train` warns when most gold tokens are like this (see [How training works](#how-training-works)).

Training quality depends heavily on the quantity and quality of this corpus.

### 3. Character Definition (char.def)

Defines character type categories and Unicode code point ranges.

```text
# Category definition: category_name compatibility_flag continuity_flag length
DEFAULT 0 1 0
SPACE 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# Character range mapping
0x0020 SPACE             # Space (reserved for SPACE)
0x0009 SPACE             # Tab
0x000A SPACE             # Line feed
0x000B SPACE             # Vertical tab
0x000D SPACE             # Carriage return
0x3041..0x3096 HIRAGANA  # Hiragana
0x30A1..0x30F6 KATAKANA  # Katakana
0x4E00..0x9FAF KANJI     # Kanji
0x0030..0x0039 NUMERIC   # Numbers
0x0041..0x005A ALPHA     # Uppercase letters
0x0061..0x007A ALPHA     # Lowercase letters
```

Parameters control the unknown words that a character's default category creates: whether they are created even where a dictionary word starts (`INVOKE`), whether a run in which each character shares a category with the one before continues as a single token (`GROUP`), and up to which length shorter candidates are created (`LENGTH`).

A range line can list more than one category (for example `0x4E00 KANJINUMERIC KANJI`), and every range line needs at least one. When several lines cover the same code point, the last line decides its categories, as in MeCab. The first category on that line is the character's default category: the only one that creates unknown-word candidates, and the one the `%t` feature template uses.

As in MeCab, a range line can name only categories defined on an earlier line, and each category is defined once. `lindera train` and `lindera build` reject a `char.def` that breaks either, with an error that names the line and the category. For example, `lindera train` with `0x30FC KATAKANA` inserted as the first line of `char.def`, before the line that defines `KATAKANA`:

```text
Error: LinderaError { kind: Args, source: failed to parse char.def: LinderaError(kind=Content, source=char.def: line 1: category KATAKANA is not defined; a range line can only name a category defined on an earlier line) }
```

Keep the `SPACE` category, which MeCab requires in every `char.def` along with `DEFAULT`, and its line in `unk.def`: every category needs at least one `unk.def` line (see the next section). The segmenter treats its characters as whitespace: by default it skips them, as MeCab does, and does not output them. Without `--keep-whitespace`, a dictionary whose `char.def` has no `SPACE` category cannot be used: `lindera tokenize` fails with `SPACE category is not defined in the dictionary (char.def)`.

Training creates the unknown-word candidates with the runtime's own code (`Lattice::unknown_word_candidates`), so it learns weights for the candidates that the built dictionary produces:

- At each position, the character's default category creates the candidates. It creates none when its `INVOKE` is 0 and a seed word starts there.
- With `GROUP` 1, the run in which each character shares a category with the one before becomes one candidate.
- With `LENGTH` n, the candidates of 1 to n characters are created too, while each character shares a category with the first one (the length ladder). In the example above, `KANJI 0 0 2` gives a kanji where no seed word starts the candidates of one and two kanji.
- Each `unk.def` entry of the category gets every one of these spans.

Grouping is unbounded, as at runtime by default. The Rust API's `Trainer::max_grouping_len` caps it with the runtime's meaning (characters beyond the first); the CLI has no option for it.

### 4. Unknown Word Definition (unk.def)

Defines the unknown words that each character category creates. Each line is `category,left_id,right_id,cost,feature...`, where `category` is a category name from `char.def`. As in the seed lexicon, the ids and the cost are set to 0; export writes the trained values.

```csv
# Unknown word definitions
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
SPACE,0,0,0,記号,空白,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

- Every line is an entry of its own, and a category can have several: one for each part of speech its unknown words can take. mecab-ipadic's `unk.def`, for example, has 40 lines for 11 categories, six of them for `KANJI` (`名詞,一般`, `名詞,サ変接続` and four kinds of `名詞,固有名詞`).
- Each entry becomes a CRF label of its own, whose `%t` is the id of its category. A candidate of the category is created once for each entry, so training learns which entry fits where.
- The order of the lines is kept: `lindera export` writes one line for each entry, in the same order.
- Lines that start with `#` and empty lines are skipped. A line with fewer than five fields (the category, the two ids, the cost and at least one feature column) is an error:

```text
Error: LinderaError { kind: Args, source: unk.def line 1: expected at least 5 fields (category, left id, right id, cost, feature), got 4 }
```

- As in MeCab, every line names a category that `char.def` defines, and every category of `char.def` has at least one line. `lindera train` and `lindera build` reject an `unk.def` that breaks either. The error names the category and, for a line whose category `char.def` does not define, the line, counted with the comment and empty lines. For example, `lindera train` with `SYMBOL,0,0,0,記号,一般,*,*,*,*,*,*,*` appended to the `unk.def` above as its ninth line, while `char.def` has no `SYMBOL` category:

```text
Error: LinderaError { kind: Args, source: failed to check unk.def: LinderaError(kind=Content, source=unk.def: line 9: category SYMBOL is not defined in char.def) }
```

### 5. Feature Template (feature.def)

MeCab-compatible feature extraction patterns that define what information the CRF model uses for learning.

```text
# Unigram features (word-level)
UNIGRAM U00:%F[0]           # POS
UNIGRAM U01:%F[0],%F?[1]    # POS + POS detail (%F?[n] = optional, skipped if *)
UNIGRAM U02:%F[6]           # Base form
UNIGRAM U03:%w              # Surface form

# Bigram features (context combination)
BIGRAM B00:%L[0]/%R[0]      # Left POS / Right POS
BIGRAM B01:%L[0],%L[1]/%R[0],%R[1]  # Left POS detail / Right POS detail
```

Template variables:

| Variable | Description |
| --- | --- |
| `%F[n]` / `%F?[n]` | Feature field at index n (`?` = optional, skipped if value is `*`) |
| `%L[n]` | Left context feature field (from rewrite.def left section) |
| `%R[n]` | Right context feature field (from rewrite.def right section) |
| `%w` | Surface form of the word |
| `%u` | Unigram rewritten feature string |
| `%l` | Left rewritten feature string |
| `%r` | Right rewritten feature string |

The fields of `%F[n]`, `%L[n]` and `%R[n]` are the columns of the rewritten features, split as described in [Seed Lexicon](#1-seed-lexicon-seedcsv). `%u`, `%l` and `%r` are the rewritten features as `rewrite.def` writes them, with a column that contains `,` or `"` quoted.

A template that reads past the last column of a feature gives no feature for it, with or without `?`, as in MeCab's learner. For example, `UNIGRAM U06:%F[6]` gives nothing for the `unk.def` feature `n,*,*,*,*`, which has five columns. A bigram template gives nothing when either `%L[n]` or `%R[n]` reads past the last column.

### 6. Feature Rewrite Rules (rewrite.def)

Feature normalization rules in MeCab-compatible 3-section format. Sections are separated by blank lines.

```text
# Section 1: Unigram rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助動詞,*,*,*,特殊・デス  助動詞
*  *

# Section 2: Left context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *

# Section 3: Right context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *
```

Each line is `pattern<TAB>replacement`. Patterns use `*` as a wildcard and are matched by prefix. The first matching rule in each section is applied. Different rules can be applied to unigram, left context, and right context independently, enabling fine-grained feature normalization to reduce sparsity.

The pattern, the replacement and the feature are split into columns as described in [Seed Lexicon](#1-seed-lexicon-seedcsv), so a quoted column with `,` in it is one column. As MeCab does, the rewritten columns are joined with `,`, and a column that contains `,` or `"` is quoted, with its quotes doubled: with the left rule `* $5`, the feature `n,dian4,店,店,"shop, store",1` is rewritten to `"shop, store"`.

## Training Parameters

| Parameter | Description | Default |
| --- | --- | --- |
| `lambda` | Regularization coefficient (controls overfitting), finite and >= 0; with `l2`, MeCab's `-c C` is `1 / C` | 1.0 |
| `regularization` | Regularization type: `l1`, `l2`, or `elasticnet`, in any case (`elastic_net` and `elastic-net` also select Elastic Net) | `l2` |
| `elastic-net-l1-ratio` | L1 ratio for Elastic Net regularization (0.0-1.0, only used with `--regularization elasticnet`) | 0.5 |
| `max-iterations` | Maximum number of training iterations, 1 or more | 100 |
| `eta` | Tolerance of the convergence test (see [How training works](#how-training-works)); `0` disables the test | 0.00005 |
| `max-threads` | Number of parallel processing threads, 1 or more | CPU core count |

A `lambda` of 0 disables regularization for every type. For Elastic Net, a ratio of 0 uses only L2, and a ratio of 1 uses only L1.

The defaults are those of MeCab's learner: L2 regularization with `lambda` 1.0 is its objective with `-c 1.0`. On the mecab-ipadic cross-validation data of [#1110](https://github.com/lindera/lindera/issues/1110), they score as MeCab's learner does and converge in about as many iterations, 81 to 87 ([#1261](https://github.com/lindera/lindera/issues/1261)): boundary F1 0.9613 and span + POS (4 columns) F1 0.9398 on average over 5 folds, where v6's default, L1 with `lambda` 0.01, reached `max-iterations` on every fold with 0.9572 and 0.9344. L1 sets many weights to 0, and `lindera export` gives labels with equal weights one context ID, so the L1 `matrix.def` is smaller: 8.7 MB there against 18.9 MB with L2, which makes the built dictionary 3.8 % larger. Training there takes about twice as long as MeCab's learner, and about two thirds of the time is spent outside the optimizer: before it starts ([#1263](https://github.com/lindera/lindera/issues/1263)) and after it stops.

## CLI Usage

### Train

```bash
lindera train \
    --seed seed.csv \
    --corpus corpus.txt \
    --char-def char.def \
    --unk-def unk.def \
    --feature-def feature.def \
    --rewrite-def rewrite.def \
    --max-iterations 100 \
    --max-threads 4 \
    --output model.dat
```

### Export

Convert the trained model into dictionary source files:

```bash
lindera export \
    --model model.dat \
    --metadata metadata.json \
    --output ./dict-source
```

This produces the following files:

| File | Description |
| --- | --- |
| `lex.csv` | Lexicon with trained costs |
| `matrix.def` | Connection cost matrix over every context ID, plus 0 for BOS/EOS, even when regularization leaves no bigram weight. A connection without a trained weight costs 0, as in training and in MeCab |
| `unk.def` | Unknown word definition: one line for each input `unk.def` entry, in input order, with its category, trained IDs and cost, and feature |
| `char.def` | Character definition |
| `feature.def` | Feature template |
| `rewrite.def` | Feature rewrite rules |
| `left-id.def` | Left context ID mapping |
| `right-id.def` | Right context ID mapping |
| `metadata.json` | Dictionary metadata; its `model_info.connection_matrix_size` is the size of `matrix.def`, rows (right IDs) by columns (left IDs) |

For example, if the `unk.def` above has a second `KANJI` line, `KANJI,0,0,0,名詞,サ変接続,*,*,*,*,*,*,*`, after the first one, the exported `unk.def` keeps both, in input order:

```csv
DEFAULT,1,1,31698,名詞,一般,*,*,*,*,*,*,*
SPACE,10,7,0,記号,空白,*,*,*,*,*,*,*
HIRAGANA,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KATAKANA,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KANJI,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KANJI,3,3,4179,名詞,サ変接続,*,*,*,*,*,*,*
ALPHA,11,10,10957,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,11,10,10957,名詞,数,*,*,*,*,*,*,*
```

### Build

Compile the exported source files into a binary dictionary:

```bash
lindera build \
    --src ./dict-source \
    --dest ./dict-compiled \
    --metadata ./dict-source/metadata.json
```

`lindera build` checks that the context IDs of every `lex.csv` row and `unk.def` line lie inside `matrix.def`, as MeCab's `mecab-dict-index` does (see [Context ID check](../lindera-cli/commands.md#context-id-check)). The files that `lindera export` writes pass this check.

### Tokenize

Tokenize text with the built dictionary:

```bash
echo "これは外国人参政権です。" | lindera tokenize -d ./dict-compiled
```

With the files in `resources/training/` as the inputs above, the output is:

```text
これ	名詞,代名詞,一般,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

A space in the input, as in `これは 外国人参政権です。`, is skipped and gives the same output. With `--keep-whitespace`, it is output as the `SPACE` unknown word:

```text
これ	名詞,代名詞,一般,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
 	記号,空白,*,*,*,*,*,*,*
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

## Output Model Format

The trained model is serialized in `rkyv` binary format for fast loading. It contains:

- Feature weights learned by the CRF
- Label set (the seed rows, then the `unk.def` entries)
- Part-of-speech information
- The `unk.def` entries, in file order, and the content of `char.def`, `feature.def` and `rewrite.def`
- Training metadata (format version, the regularization coefficient, type and Elastic Net ratio, iterations, feature/label counts)

The format version is `3.0.0` (`lindera_trainer::model::MODEL_FORMAT_VERSION`), and it changes whenever the layout or the meaning of the model changes. `lindera export` rejects a `model.dat` written by another version, including every `model.dat` written before 7.0.0, with an error that ends in "re-run `lindera train` to regenerate it". Train such a model again with the current version.

The output is byte-reproducible: training the same inputs twice with the same
flags produces identical `model.dat` bytes, so the artifact can be checksummed
and diffed. This holds for any `--max-threads` value: the gradient and the
loss are summed over a fixed partition of the training data in a fixed order,
so the thread count changes only how long training takes, never its result.

## API Usage

For the full `lindera-trainer` API surface, see [Lindera Trainer Architecture](../lindera-trainer/architecture.md) and [API Reference](../lindera-trainer/api_reference.md).

```rust,no_run
use std::fs::File;
use lindera_trainer::{Corpus, Trainer, TrainerConfig};

// Load configuration from files
let seed_file = File::open("resources/training/seed.csv")?;
let char_file = File::open("resources/training/char.def")?;
let unk_file = File::open("resources/training/unk.def")?;
let feature_file = File::open("resources/training/feature.def")?;
let rewrite_file = File::open("resources/training/rewrite.def")?;

let config = TrainerConfig::from_readers(
    seed_file,
    char_file,
    unk_file,
    feature_file,
    rewrite_file
)?;

// Initialize and configure trainer
let trainer = Trainer::new(config)?
    .max_iter(100)
    .num_threads(4);

// Load corpus
let corpus_file = File::open("resources/training/corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// Execute training
let model = trainer.train(corpus)?;

// Save model (binary format)
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Output in Lindera dictionary format. output_user.csv holds the entries
// loaded by read_user_lexicon, in input order; without that call it is empty.
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

## Recommended Corpus Specifications

For generating effective dictionaries for real applications:

### Corpus Size

| Level | Sentences | Use Case |
| --- | --- | --- |
| Minimum | 100+ | Basic operation verification |
| Recommended | 1,000+ | Practical applications |
| Ideal | 10,000+ | Commercial quality |

### Quality Guidelines

- **Vocabulary diversity**: Balanced distribution of different parts of speech, coverage of inflections and suffixes, appropriate inclusion of technical terms and proper nouns.
- **Consistency**: Apply analysis criteria consistently across the corpus.
- **Verification**: Manually verify morphological analysis results. Maintain an error rate below 5%.
