# Filters

Lindera's analysis pipeline (the `Tokenizer` provided by the [`lindera-analysis`](../lindera-analysis.md) crate) has two extension points around the segmenter: **character filters**, which transform the raw input text before tokenization, and **token filters**, which transform the list of tokens after tokenization.

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

## Character filters

Character filters preprocess the input text before it reaches the segmenter. They're typically used to normalize text so that tokenization is more consistent -- for example, converting full-width characters to half-width, canonicalizing Unicode representations, or expanding Japanese iteration marks into their repeated form. Because a character filter can change the length of the text, Lindera tracks every transformation it makes and corrects each token's byte offsets back to positions in the original, unfiltered text.

## Token filters

Token filters postprocess the list of tokens produced by the segmenter. They're typically used to normalize or reduce the token list for search and analysis -- for example, replacing a token with its base (dictionary) form, converting a token's surface text between hiragana and katakana, removing tokens by part-of-speech tag, or removing stop words.

## Configuring filters

Every filter -- character or token -- is identified by a `kind` string and configured with a JSON `args` object of filter-specific parameters. Filters run in the order they're added, and there are two ways to add them:

- **YAML configuration file**: list filters under the `character_filters` and `token_filters` keys.

  ```yaml
  character_filters:
    - kind: unicode_normalize
      args:
        kind: nfkc

  token_filters:
    - kind: japanese_base_form
  ```

- **Rust API**: append filters to a `Tokenizer`, in order, with `append_character_filter` and `append_token_filter`.

  ```rust
  // Character filters run first, transforming the raw input text;
  // token filters run last, transforming the resulting token list.
  tokenizer
      .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
      .append_token_filter(BoxTokenFilter::from(japanese_base_form_filter));
  ```

## Available filters

Lindera ships 4 character filters and 18 token filters, covering Japanese, Korean, and general-purpose text normalization:

| Category | Filters |
| --- | --- |
| Character filters | `unicode_normalize`, `japanese_iteration_mark`, `mapping`, `regex` |
| Token filters -- normalization | `japanese_base_form`, `japanese_reading_form`, `korean_reading_form`, `japanese_kana`, `japanese_katakana_stem`, `japanese_number`, `mapping`, `remove_diacritical_mark`, `lowercase`, `uppercase` |
| Token filters -- tag-based filtering | `japanese_keep_tags`, `japanese_stop_tags`, `korean_keep_tags`, `korean_stop_tags` |
| Token filters -- word-based filtering | `keep_words`, `stop_words` |
| Token filters -- structural transformation | `japanese_compound_word`, `length` |

See the [Filters reference](../lindera-analysis/filters.md) for what each filter does, its parameters, and runnable YAML and Rust API examples.
