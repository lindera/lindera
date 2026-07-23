# Filters

Character filters and token filters are the two pre/post-processing stages of the `lindera-analysis` `Tokenizer` pipeline.

- **Character filters** transform the input text *before* segmentation. Byte offsets are corrected automatically so that the resulting tokens still report positions relative to the original, unfiltered text.
- **Token filters** transform the list of tokens produced by the segmenter *after* segmentation.

Both kinds of filters are configured the same way: a `kind` string (also used with the CLI's `--character-filter` / `--token-filter` flags in the form `kind:{"json": "args"}`) and a JSON `args` object with filter-specific parameters.

## Character Filters

Character filters are configured under the `character_filters` key of the YAML configuration file. Each entry is applied in order, and each filter's output feeds into the next.

### unicode_normalize

Normalizes the input text using one of the four standard Unicode normalization forms.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `kind` | string | Yes | One of `nfc`, `nfd`, `nfkc`, or `nfkd` |

**Example:**

```json
{
  "kind": "unicode_normalize",
  "args": {
    "kind": "nfkc"
  }
}
```

### japanese_iteration_mark

Normalizes Japanese iteration marks (`々`, `ゝ`, `ゞ`, `ヽ`, `ヾ`) by replacing each mark with the character it repeats, adding or removing the voiced sound mark (dakuten) as needed for the hiragana/katakana variants.

**Parameters:**

| Argument | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `normalize_kanji` | bool | No | `false` | Normalize the kanji iteration mark `々` |
| `normalize_kana` | bool | No | `false` | Normalize the hiragana/katakana iteration marks `ゝ`, `ゞ`, `ヽ`, `ヾ` |

**Example:**

```json
{
  "kind": "japanese_iteration_mark",
  "args": {
    "normalize_kanji": true,
    "normalize_kana": true
  }
}
```

### mapping (character filter)

Replaces occurrences of the keys in `mapping` with their corresponding values, using longest-match search over the input text (built with an Aho-Corasick automaton).

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `mapping` | object (string to string) | Yes | Substrings to replace, mapped to their replacements |

**Example:**

```json
{
  "kind": "mapping",
  "args": {
    "mapping": {
      "リンデラ": "Lindera"
    }
  }
}
```

### regex

Replaces every match of a regular expression in the input text with a literal replacement string. Capture groups are not interpolated into the replacement.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `pattern` | string | Yes | A regular expression (using the [`regex`](https://docs.rs/regex) crate's syntax) |
| `replacement` | string | Yes | The literal string that replaces every match of `pattern` |

**Example:**

```json
{
  "kind": "regex",
  "args": {
    "pattern": "\\s{2,}",
    "replacement": " "
  }
}
```

## Token Filters

Token filters are configured under the `token_filters` key of the YAML configuration file. Each filter is applied in order to the token list produced by the segmenter.

### japanese_base_form

Replaces the token's surface text with its base (dictionary) form, as registered in the `base_form` or `orthographic_base_form` field of the dictionary. Acts as a lemmatizer for verbs and adjectives. Tokens whose first detail is `UNK` (unknown words) are left unchanged.

This filter takes no configuration parameters.

**Example:**

```json
{
  "kind": "japanese_base_form"
}
```

### japanese_compound_word

Merges consecutive tokens whose part-of-speech tag matches one of `tags` into a single compound token.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> | Yes | Part-of-speech tags (up to 4 comma-separated levels) that mark tokens eligible for merging |
| `new_tag` | string | No | Part-of-speech tag assigned to the merged token. When omitted, the merged token is tagged `複合語` |

**Example:**

```json
{
  "kind": "japanese_compound_word",
  "args": {
    "tags": [
      "名詞,数",
      "名詞,接尾,助数詞"
    ],
    "new_tag": "名詞,数"
  }
}
```

### japanese_kana

Converts token text between hiragana and katakana.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `kind` | string | Yes | `"hiragana"` converts katakana to hiragana; `"katakana"` converts hiragana to katakana |

**Example:**

```json
{
  "kind": "japanese_kana",
  "args": {
    "kind": "hiragana"
  }
}
```

### japanese_katakana_stem

Removes a trailing prolonged sound mark (`ー`, U+30FC) from katakana tokens, but only when the token is longer than `min` characters.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `min` | positive integer | Yes | Minimum katakana token length (in characters) required before the trailing prolonged sound mark is stemmed |

**Example:**

```json
{
  "kind": "japanese_katakana_stem",
  "args": {
    "min": 3
  }
}
```

### japanese_keep_tags

Keeps only tokens whose part-of-speech tag matches one of `tags`, removing all others.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> | Yes | Part-of-speech tags (up to 4 comma-separated levels) to keep |

**Example:**

```json
{
  "kind": "japanese_keep_tags",
  "args": {
    "tags": [
      "名詞",
      "名詞,一般",
      "名詞,固有名詞"
    ]
  }
}
```

### japanese_number

Converts Japanese numeral representations (kanji numerals, formal/legal kanji numerals, and fullwidth digits) in the token's surface text to Arabic numerals.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> or `null` | No | Part-of-speech tags (up to 4 comma-separated levels) to restrict the conversion to. When omitted or `null`, every token is converted |

**Example:**

```json
{
  "kind": "japanese_number",
  "args": {
    "tags": [
      "名詞,数"
    ]
  }
}
```

### japanese_reading_form

Replaces the token's surface text with its reading, in katakana, as registered in the dictionary's `reading` field. Tokens whose first detail is `UNK` (unknown words) are left unchanged.

This filter takes no configuration parameters.

**Example:**

```json
{
  "kind": "japanese_reading_form"
}
```

### japanese_stop_tags

Removes tokens whose part-of-speech tag matches one of `tags`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> | Yes | Part-of-speech tags (up to 4 comma-separated levels) to remove |

**Example:**

```json
{
  "kind": "japanese_stop_tags",
  "args": {
    "tags": [
      "助詞",
      "助動詞",
      "記号"
    ]
  }
}
```

### keep_words

Keeps only tokens whose surface text exactly matches one of `words`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `words` | array\<string\> | Yes | Surface forms to keep |

**Example:**

```json
{
  "kind": "keep_words",
  "args": {
    "words": [
      "すもも",
      "もも"
    ]
  }
}
```

### korean_keep_tags

Keeps only Korean tokens whose first part-of-speech tag matches one of `tags`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> | Yes | Part-of-speech tags to keep |

**Example:**

```json
{
  "kind": "korean_keep_tags",
  "args": {
    "tags": [
      "NNG"
    ]
  }
}
```

### korean_reading_form

Replaces the token's surface text with its reading, as registered in the dictionary's `reading` field. Tokens whose first detail is `UNK` (unknown words) are left unchanged.

This filter takes no configuration parameters.

**Example:**

```json
{
  "kind": "korean_reading_form"
}
```

### korean_stop_tags

Removes Korean tokens whose first part-of-speech tag matches one of `tags`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | array\<string\> | Yes | Part-of-speech tags to remove |

**Example:**

```json
{
  "kind": "korean_stop_tags",
  "args": {
    "tags": [
      "EP",
      "EF",
      "JKG"
    ]
  }
}
```

### length

Keeps only tokens whose surface text length (in characters) falls within `[min, max]`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `min` | unsigned integer | No | Minimum character length (inclusive) |
| `max` | unsigned integer | No | Maximum character length (inclusive) |

**Example:**

```json
{
  "kind": "length",
  "args": {
    "min": 2,
    "max": 3
  }
}
```

### lowercase

Converts token surface text to lowercase.

This filter takes no configuration parameters.

**Example:**

```json
{
  "kind": "lowercase"
}
```

### mapping (token filter)

Replaces occurrences of the keys in `mapping` with their corresponding values in each token's surface text, using longest-match search (built with an Aho-Corasick automaton). This is the token-level counterpart of the `mapping` character filter.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `mapping` | object (string to string) | Yes | Substrings to replace, mapped to their replacements |

**Example:**

```json
{
  "kind": "mapping",
  "args": {
    "mapping": {
      "籠": "篭"
    }
  }
}
```

### remove_diacritical_mark

Removes diacritical (combining) marks from token surface text, re-applying the text's original Unicode normalization form afterward.

**Parameters:**

| Argument | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `japanese` | bool | No | `false` | Also remove Japanese (han-)dakuten combining marks (e.g. from decomposed voiced/semi-voiced kana) |

**Example:**

```json
{
  "kind": "remove_diacritical_mark",
  "args": {
    "japanese": false
  }
}
```

### stop_words

Removes tokens whose surface text exactly matches one of `words`.

**Parameters:**

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `words` | array\<string\> | Yes | Surface forms to remove |

**Example:**

```json
{
  "kind": "stop_words",
  "args": {
    "words": [
      "も",
      "の"
    ]
  }
}
```

### uppercase

Converts token surface text to uppercase.

This filter takes no configuration parameters.

**Example:**

```json
{
  "kind": "uppercase"
}
```

## YAML Configuration

Character filters and token filters are configured together with the segmenter in a single YAML file. See [Configuration](./configuration.md) for the full file format; the relevant excerpt looks like this:

```yaml
character_filters:
  - kind: "unicode_normalize"
    args:
      kind: "nfkc"
  - kind: "japanese_iteration_mark"
    args:
      normalize_kanji: true
      normalize_kana: true

token_filters:
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "助詞"
        - "助動詞"
        - "記号"
  - kind: "japanese_katakana_stem"
    args:
      min: 3
  - kind: "lowercase"
  - kind: "length"
    args:
      min: 2
```

## Rust API

Character filters and token filters can also be created and applied programmatically:

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::character_filter::BoxCharacterFilter;
use lindera_analysis::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera_analysis::token_filter::BoxTokenFilter;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera_analysis::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
use lindera_analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let mut tokenizer = Tokenizer::new(segmenter);

    // Add a character filter
    let normalize_filter = UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);
    tokenizer.append_character_filter(BoxCharacterFilter::from(normalize_filter));

    // Add token filters
    let stop_tags_filter = JapaneseStopTagsTokenFilter::new(
        vec![
            "助詞".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
        ]
        .into_iter()
        .collect(),
    );
    tokenizer.append_token_filter(BoxTokenFilter::from(stop_tags_filter));

    let katakana_stem_filter =
        JapaneseKatakanaStemTokenFilter::new(std::num::NonZeroUsize::new(3).unwrap());
    tokenizer.append_token_filter(BoxTokenFilter::from(katakana_stem_filter));

    // Tokenize with filters applied
    let tokens = tokenizer.tokenize("Linderaは形態素解析エンジンです。")?;

    for token in tokens {
        println!(
            "token: {:?}, details: {:?}",
            token.surface, token.details
        );
    }

    Ok(())
}
```

The `append_character_filter` and `append_token_filter` methods add filters in order. Character filters are applied sequentially to the text before segmentation; token filters are applied sequentially to the token list after segmentation.
