# Morphological Analysis

## What is morphological analysis?

Morphological analysis is the process of breaking down text into its smallest meaningful units (morphemes) and identifying their grammatical properties. For languages like Japanese, Chinese, and Korean -- where words are not separated by spaces -- morphological analysis is an essential first step for natural language processing tasks such as search indexing, text classification, and machine translation.

## How Lindera works

Lindera is a dictionary-based morphological analyzer. It uses a pre-compiled system dictionary containing known words along with their costs, and applies the **Viterbi algorithm** to find the optimal segmentation of input text.

The analysis process works as follows:

1. **Lattice construction**: Lindera scans the input text and looks up all possible words in the dictionary at every position, building a directed acyclic graph (lattice) of candidate segmentations.
2. **Cost assignment**: Each candidate word has an associated word cost (from the dictionary), and each pair of adjacent words has a connection cost (from the connection cost matrix).
3. **Optimal path search**: The Viterbi algorithm finds the path through the lattice with the minimum total cost, producing the best segmentation.

## Key terminology

| Term | Description |
| --- | --- |
| **Surface form** | The actual text as it appears in the input (e.g., "食べ"). |
| **Part-of-speech (POS)** | The grammatical category of a word (e.g., noun, verb, particle). Lindera dictionaries provide hierarchical POS tags with up to four levels of subcategories. |
| **Reading** | The pronunciation of a word, typically in Katakana for Japanese dictionaries. |
| **Base form** | The uninflected (dictionary) form of a word (e.g., "食べる" for the surface "食べ"). |
| **Conjugation** | Inflection information for words that conjugate, consisting of a conjugation type and a conjugation form. |

## Cost-based segmentation

The Viterbi algorithm selects the segmentation path with the minimum total cost. The total cost of a path is the sum of:

- **Word costs**: Each word in the dictionary has an associated cost. Lower cost means the word is more likely to appear. Common words tend to have lower costs, while rare words have higher costs.
- **Connection costs**: The cost of connecting two adjacent words, determined by the right context ID of the left word and the left context ID of the right word.

The algorithm computes:

```text
Total cost = sum of word costs + sum of connection costs
```

By minimizing this total cost, Lindera finds the most natural segmentation of the input text.

## Connection cost matrix

The connection cost matrix stores the cost of transitioning from one word to another. It is a two-dimensional matrix indexed by:

- The **right context ID** of the preceding word
- The **left context ID** of the following word

These context IDs encode grammatical information about word boundaries. For example, the connection cost between a noun and a particle is typically low (natural sequence), while the connection cost between two verbs in base form might be high (unnatural sequence).

The connection cost matrix is compiled into binary format as part of the dictionary build process and is loaded at runtime for efficient lookup.
