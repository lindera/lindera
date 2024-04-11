# Lindera Filter

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

Character and token filters for [Lindera](https://github.com/lindera-morphology/lindera).

## Character filters

### Japanese iteration mark filter

Normalizes Japanese horizontal [iteration marks](https://en.wikipedia.org/wiki/Iteration_mark) (odoriji) to their expanded form.
Sequences of iteration marks are supported. In case an illegal sequence of iteration marks is encountered, the implementation emits the illegal source character as-is without considering its script. For example, with input "?ゝ", we get "??" even though the question mark isn't hiragana.

### Mapping filter

Replace characters with the specified character mappings, and correcting the resulting changes to the offsets.
Matching is greedy (longest pattern matching at a given point wins). Replacement is allowed to be the empty string.

### Regex filter

Character filter that uses a regular expression for the target of replace string.

### Unicode normalize filter

Unicode normalization to normalize the input text, that using the specified normalization form, one of NFC, NFD, NFKC, or NFKD.


## Token filters

### Japanese base form filter

Replace the term text with the base form registered in the morphological dictionary.
This acts as a lemmatizer for verbs and adjectives.

### Japanese compound word filter

Compound consecutive tokens that have specified part-of-speech tags into a single token.
This is useful for handling compound words that are not registered in the morphological dictionary.

### Japanese katakana stem filter

Normalizes common katakana spelling variations ending with a long sound (U+30FC) by removing that character.
Only katakana words longer than the minimum length are stemmed.

### Japanese keep tags filter

Keep only tokens with the specified part-of-speech tag.

### Japanese number filter

Convert tokens representing Japanese numerals, including Kanji numerals, to Arabic numerals.

### Japanese reading form filter

Replace the text of a token with the reading of the text as registered in the morphological dictionary.
The reading is in katakana.

### Japanese stop tags filter

Remove tokens with the specified part-of-speech tag.

### Keep words filter

Keep only the tokens of the specified text.

### Korean keep tags filter

Keep only tokens with the specified part-of-speech tag.

### Korean reading form filter

Replace the text of a token with the reading of the text as registered in the morphological dictionary.

### Korean stop tags filter

Remove tokens with the specified part-of-speech tag.

### Length filter

Keep only tokens with the specified number of characters of text.

### Lowercase filter

Normalizes token text to lowercase.

### Mapping filter

Replace characters with the specified character mappings.

### Stop words filter

Remove the tokens of the specified text.

### Uppercase filter

Normalizes token text to uppercase.


## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-filter" target="_blank">lindera-filter</a>
