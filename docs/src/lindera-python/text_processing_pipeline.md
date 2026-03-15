# Text Processing Pipeline

Lindera Python supports a composable text processing pipeline that applies character filters before tokenization and token filters after tokenization. Filters are added to the `TokenizerBuilder` and executed in the order they are appended.

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

## Character Filters

Character filters transform the input text before tokenization.

### unicode_normalize

Applies Unicode normalization to the input text.

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("unicode_normalize", {"kind": "nfkc"})
    .build()
)
```

Supported normalization forms: `"nfc"`, `"nfkc"`, `"nfd"`, `"nfkd"`.

### mapping

Replaces characters or strings according to a mapping table.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("mapping", {
        "mapping": {
            "\u30fc": "-",
            "\uff5e": "~",
        }
    })
    .build()
)
```

### japanese_iteration_mark

Resolves Japanese iteration marks (odoriji) into their full forms.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("japanese_iteration_mark", {
        "normalize_kanji": True,
        "normalize_kana": True,
    })
    .build()
)
```

## Token Filters

Token filters transform or remove tokens after tokenization.

### lowercase

Converts token surface forms to lowercase.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("lowercase", {})
    .build()
)
```

### japanese_base_form

Replaces inflected forms with their base (dictionary) form using the morphological details from the dictionary.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_base_form", {})
    .build()
)
```

### japanese_stop_tags

Removes tokens whose part-of-speech matches any of the specified tags.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_stop_tags", {
        "tags": ["助詞", "助動詞"],
    })
    .build()
)
```

### japanese_keep_tags

Keeps only tokens whose part-of-speech matches one of the specified tags. All other tokens are removed.

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_keep_tags", {
        "tags": ["名詞"],
    })
    .build()
)
```

## Complete Pipeline Example

The following example combines multiple character filters and token filters into a single pipeline:

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_mode("normal")
    .set_dictionary("embedded://ipadic")
    # Preprocessing
    .append_character_filter("unicode_normalize", {"kind": "nfkc"})
    .append_character_filter("japanese_iteration_mark", {
        "normalize_kanji": True,
        "normalize_kana": True,
    })
    # Postprocessing
    .append_token_filter("japanese_base_form", {})
    .append_token_filter("japanese_stop_tags", {
        "tags": ["助詞", "助動詞", "記号"],
    })
    .append_token_filter("lowercase", {})
    .build()
)

tokens = tokenizer.tokenize("Ｌｉｎｄｅｒａは形態素解析を行うライブラリです。")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```

In this pipeline:

1. `unicode_normalize` converts full-width characters to half-width (NFKC normalization)
2. `japanese_iteration_mark` resolves iteration marks
3. `japanese_base_form` converts inflected tokens to base form
4. `japanese_stop_tags` removes particles, auxiliary verbs, and symbols
5. `lowercase` normalizes alphabetic characters to lowercase
