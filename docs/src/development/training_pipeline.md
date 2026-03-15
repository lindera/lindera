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

Training quality depends heavily on the quantity and quality of this corpus.

### 3. Character Definition (char.def)

Defines character type categories and Unicode code point ranges.

```text
# Category definition: category_name compatibility_flag continuity_flag length
DEFAULT 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# Character range mapping
0x3041..0x3096 HIRAGANA  # Hiragana
0x30A1..0x30F6 KATAKANA  # Katakana
0x4E00..0x9FAF KANJI     # Kanji
0x0030..0x0039 NUMERIC   # Numbers
0x0041..0x005A ALPHA     # Uppercase letters
0x0061..0x007A ALPHA     # Lowercase letters
```

Parameters control how unknown words of each character type are segmented: compatibility with adjacent characters, whether runs of the same type continue as a single token, and default token length.

### 4. Unknown Word Definition (unk.def)

Defines how out-of-vocabulary words are handled by character type.

```csv
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
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

## Training Parameters

| Parameter | Description | Default |
| --- | --- | --- |
| `lambda` | L1 regularization coefficient (controls overfitting) | 0.01 |
| `max-iterations` | Maximum number of training iterations | 100 |
| `max-threads` | Number of parallel processing threads | 1 |

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
    --lambda 0.01 \
    --max-iter 100 \
    --max-threads 4 \
    --output model.dat
```

### Export

Convert the trained model into dictionary source files:

```bash
lindera export --model model.dat --output-dir ./dict-source
```

This produces the following files:

| File | Description |
| --- | --- |
| `lex.csv` | Lexicon with trained costs |
| `matrix.def` | Connection cost matrix |
| `unk.def` | Unknown word definition |
| `char.def` | Character definition |
| `feature.def` | Feature template |
| `rewrite.def` | Feature rewrite rules |
| `left-id.def` | Left context ID mapping |
| `right-id.def` | Right context ID mapping |
| `metadata.json` | Dictionary metadata |

### Build

Compile the exported source files into a binary dictionary:

```bash
lindera build --input-dir ./dict-source --output-dir ./dict-compiled
```

## Output Model Format

The trained model is serialized in `rkyv` binary format for fast loading. It contains:

- Feature weights learned by the CRF
- Label set (vocabulary entries)
- Part-of-speech information
- Feature templates
- Training metadata (regularization, iterations, feature/label counts)

## API Usage

```rust,no_run
use std::fs::File;
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

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
    .regularization_cost(0.01)
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

// Output in Lindera dictionary format
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
