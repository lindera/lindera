# Architecture

## Module Structure

```text
lindera-analysis/src/
├── lib.rs                          # Public API re-exports, CLI flag parsing helper
├── character_filter.rs             # CharacterFilter trait, OffsetMapping, CharacterFilterLoader
├── character_filter/
│   ├── unicode_normalize.rs        # Unicode normalization (NFC/NFD/NFKC/NFKD)
│   ├── japanese_iteration_mark.rs  # Japanese iteration mark normalization
│   ├── mapping.rs                  # Mapping-based text replacement
│   └── regex.rs                    # Regex-based text replacement
├── token_filter.rs                 # TokenFilter trait, TokenFilterLoader
├── token_filter/
│   ├── japanese_base_form.rs
│   ├── japanese_compound_word.rs
│   ├── japanese_kana.rs
│   ├── japanese_katakana_stem.rs
│   ├── japanese_keep_tags.rs
│   ├── japanese_number.rs
│   ├── japanese_reading_form.rs
│   ├── japanese_stop_tags.rs
│   ├── keep_words.rs
│   ├── korean_keep_tags.rs
│   ├── korean_reading_form.rs
│   ├── korean_stop_tags.rs
│   ├── length.rs
│   ├── lowercase.rs
│   ├── mapping.rs
│   ├── remove_diacritical_mark.rs
│   ├── stop_words.rs
│   ├── tags.rs                     # Shared keep/stop-tag filtering helpers (private)
│   └── uppercase.rs
└── tokenizer.rs                     # Tokenizer, TokenizerBuilder
```

## Key Components

### CharacterFilter

A trait for filters that preprocess text before segmentation. Each implementation provides a `name()` and an `apply(&self, text: &mut String) -> LinderaResult<OffsetMapping>` method that rewrites `text` in place and returns an `OffsetMapping` describing every transformation it performed.

The `OffsetMapping` (built from a list of `Transformation` records) lets the `Tokenizer` translate token byte offsets computed against the filtered text back to byte offsets in the original input, even after multiple filters have run in sequence. `BoxCharacterFilter` wraps any `CharacterFilter` implementation as a boxed, cloneable trait object, and `CharacterFilterLoader` builds one from a `kind` string plus a `serde_json::Value` of arguments (used both by YAML configuration loading and by CLI flag parsing).

### TokenFilter

A trait for filters that post-process the tokens produced by the segmenter. Each implementation provides a `name()` and an `apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()>` method that modifies, merges, reorders, or removes tokens in place. `BoxTokenFilter` wraps any `TokenFilter` implementation as a boxed, cloneable trait object, and `TokenFilterLoader` builds one from a `kind` string plus a `serde_json::Value` of arguments, mirroring `CharacterFilterLoader`.

### Tokenizer / TokenizerBuilder

`Tokenizer` composes character filters, a [`lindera::segmenter::Segmenter`](../lindera/segmenter.md), and token filters into a single analysis pipeline. Calling `tokenize` runs the character filters over the input text, segments the filtered text, applies the token filters to the resulting tokens, and finally corrects each token's byte offsets back to the original text via the recorded `OffsetMapping`s.

`TokenizerBuilder` assembles a `Tokenizer` from a `TokenizerConfig` (a `serde_json::Value`), which can be constructed programmatically, loaded from a YAML file (via `TokenizerBuilder::from_file`, or automatically from the `LINDERA_CONFIG_PATH` environment variable via `TokenizerBuilder::new`), or built up incrementally with `set_segmenter_mode`, `set_segmenter_dictionary`, `append_character_filter`, and `append_token_filter`. See [Configuration](./configuration.md) for the YAML file format and [Filters](./filters.md) for the full filter reference.

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `embed-ipadic` | Embed the IPADIC dictionary in the binary (forwards to `lindera/embed-ipadic`) | No |
| `embed-ipadic-neologd` | Embed the IPADIC-NEologd dictionary in the binary (forwards to `lindera/embed-ipadic-neologd`) | No |
| `embed-unidic` | Embed the UniDic dictionary in the binary (forwards to `lindera/embed-unidic`) | No |
| `embed-ko-dic` | Embed the ko-dic dictionary in the binary (forwards to `lindera/embed-ko-dic`) | No |
| `embed-cc-cedict` | Embed the CC-CEDICT dictionary in the binary (forwards to `lindera/embed-cc-cedict`) | No |
| `embed-jieba` | Embed the Jieba dictionary in the binary (forwards to `lindera/embed-jieba`) | No |
