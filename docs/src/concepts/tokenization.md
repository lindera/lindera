# Tokenization

Lindera provides multiple tokenization modes and supports N-Best analysis for enumerating alternative segmentation candidates.

## Tokenization modes

### Normal mode

Normal mode performs standard tokenization based on dictionary entries. Compound words that exist as single entries in the dictionary are kept as-is.

**Example** -- tokenizing "関西国際空港限定トートバッグ" in Normal mode:

```text
関西国際空港 | 限定 | トートバッグ
```

The compound noun "関西国際空港" (Kansai International Airport) is preserved as a single token because it exists as one entry in the dictionary.

### Decompose mode

Decompose mode further breaks down compound nouns into their constituent parts, even when the compound exists as a dictionary entry.

**Example** -- tokenizing "関西国際空港限定トートバッグ" in Decompose mode:

```text
関西 | 国際 | 空港 | 限定 | トートバッグ
```

The compound "関西国際空港" is decomposed into "関西", "国際", and "空港".

### Selecting a mode

In Rust, specify the mode when creating a `Segmenter`:

```rust
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("embedded://ipadic")?;

// Normal mode
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

// Decompose mode
let segmenter = Segmenter::new(Mode::Decompose, dictionary, None);
```

With the CLI, use the `--mode` flag:

```shell
echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic --mode normal
echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic --mode decompose
```

## N-Best tokenization

N-Best tokenization enumerates the top N tokenization candidates ordered by total path cost (lower cost = better segmentation). This is useful when the best result is ambiguous, or when you want to explore alternative interpretations of the input text.

### Algorithm

N-Best tokenization is based on the **Forward-DP Backward-A\*** algorithm, which is compatible with MeCab's N-Best implementation. The forward pass computes optimal costs using dynamic programming, and the backward pass uses A\* search to enumerate paths in order of increasing total cost.

### Parameters

The `tokenize_nbest` method accepts the following parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `text` | `&str` | The text to tokenize. |
| `n` | `usize` | Number of N-best results to return. |
| `unique` | `bool` | When `true`, deduplicates results that produce the same word boundary positions. |
| `cost_threshold` | `Option<i64>` | When `Some(threshold)`, only returns paths with cost within `best_cost + threshold`. |

### Rust API example

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "すもももももももものうち";

    // Get top 3 tokenization results
    let results = tokenizer.tokenize_nbest(text, 3, false, None)?;

    for (rank, (tokens, cost)) in results.iter().enumerate() {
        println!("--- NBEST {} (cost={}) ---", rank + 1, cost);
        for token in tokens {
            let details = token.details().join(",");
            println!("{}\t{}", token.surface.as_ref(), details);
        }
    }

    Ok(())
}
```

Output:

```text
--- NBEST 1 (cost=7546) ---
すもも  名詞,一般,*,*,*,*,すもも,スモモ,スモモ
も      助詞,係助詞,*,*,*,*,も,モ,モ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
も      助詞,係助詞,*,*,*,*,も,モ,モ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
うち    名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ
--- NBEST 2 (cost=7914) ---
...
```

### CLI example

```shell
echo "すもももももももものうち" | lindera tokenize --dict embedded://ipadic -N 3
```

### Lattice reuse

For repeated tokenization, you can reuse a `Lattice` to reduce memory allocations:

```rust
use lindera_dictionary::viterbi::Lattice;

let mut lattice = Lattice::default();
let results = tokenizer.tokenize_nbest_with_lattice(text, &mut lattice, 3, false, None)?;
```
