# Lindera Filter

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) [![Crates.io](https://img.shields.io/crates/v/lindera-filter.svg)](https://crates.io/crates/lindera-filter)

Character and token filters for [Lindera](https://github.com/lindera-morphology/lindera).

## Character filters

### Japanese iteration mark character filter

Normalizes Japanese horizontal [iteration marks](https://en.wikipedia.org/wiki/Iteration_mark) (odoriji) to their expanded form.
Sequences of iteration marks are supported. In case an illegal sequence of iteration marks is encountered, the implementation emits the illegal source character as-is without considering its script. For example, with input "?ゝ", we get "??" even though the question mark isn't hiragana.

### Mapping character filter

Replace characters with the specified character mappings, and correcting the resulting changes to the offsets.
Matching is greedy (longest pattern matching at a given point wins). Replacement is allowed to be the empty string.

### Regex character filter

Character filter that uses a regular expression for the target of replace string.

### Unicode normalize character filter

Unicode normalization to normalize the input text, that using the specified normalization form, one of NFC, NFD, NFKC, or NFKD.

## Token filters

### Japanese base form token filter

Replace the term text with the base form registered in the morphological dictionary.
This acts as a lemmatizer for verbs and adjectives.

### Japanese compound word token filter

Compound consecutive tokens that have specified part-of-speech tags into a single token.
This is useful for handling compound words that are not registered in the morphological dictionary.

### Japanese katakana stem token filter

Normalizes common katakana spelling variations ending with a long sound (U+30FC) by removing that character.
Only katakana words longer than the minimum length are stemmed.

### Japanese keep tags token filter

Keep only tokens with the specified part-of-speech tag.

### Japanese number token filter

Convert tokens representing Japanese numerals, including Kanji numerals, to Arabic numerals.

### Japanese reading form token filter

Replace the text of a token with the reading of the text as registered in the morphological dictionary.
The reading is in katakana.

### Japanese stop tags token filter

Remove tokens with the specified part-of-speech tag.

### Keep words token filter

Keep only the tokens of the specified text.

### Korean keep tags token filter

Keep only tokens with the specified part-of-speech tag.

### Korean reading form token filter

Replace the text of a token with the reading of the text as registered in the morphological dictionary.

### Korean stop tags token filter

Remove tokens with the specified part-of-speech tag.

### Length token filter

Keep only tokens with the specified number of characters of text.

### Lowercase token filter

Normalizes token text to lowercase.

### Mapping token filter

Replace characters with the specified character mappings.

### Stop words token filter

Remove the tokens of the specified text.

### Uppercase token filter

Normalizes token text to uppercase.

## API reference

The API reference is available. Please see following URL:

- [lindera-filter](https://docs.rs/lindera-filter)
