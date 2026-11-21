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
│   ├── compound.rs                 # Shared compound-word merge helper (private)
│   ├── japanese_base_form.rs
│   ├── japanese_compound_word.rs
│   ├── japanese_kana.rs
│   ├── japanese_katakana_stem.rs
│   ├── japanese_keep_tags.rs
│   ├── japanese_number.rs
│   ├── japanese_reading_form.rs
│   ├── japanese_stop_tags.rs
│   ├── keep_words.rs
│   ├── korean_compound_word.rs
│   ├── korean_decompound.rs
│   ├── korean_keep_tags.rs
│   ├── korean_number.rs
│   ├── korean_reading_form.rs
│   ├── korean_stop_tags.rs
│   ├── length.rs
│   ├── lowercase.rs
│   ├── mapping.rs
│   ├── numeral.rs                  # Shared numeral conversion engine (private)
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

`TokenizerBuilder` assembles a `Tokenizer` from a `TokenizerConfig` (a `serde_json::Value`), which can be constructed programmatically, loaded from a YAML file (via `TokenizerBuilder::from_file`, or automatically from the `LINDERA_CONFIG_FILE` environment variable via `TokenizerBuilder::new`), or built up incrementally with `set_segmenter_mode`, `set_segmenter_dictionary`, `append_character_filter`, and `append_token_filter`. See [Configuration](./configuration.md) for the YAML file format and [Filters](./filters.md) for the full filter reference.

`TokenizerBuilder::build` loads the dictionaries the configuration names. `TokenizerBuilder::build_with_dictionaries` (and `Tokenizer::from_config_with_dictionaries`) takes already-loaded dictionaries instead, such as ones loaded from bytes, and applies every other setting of the configuration to them, filters included; see [Building from Config](../lindera/segmenter.md#building-from-config) for how the given dictionaries replace the configured ones. The WASM binding's `TokenizerBuilder` builds its tokenizers this way, with the dictionaries set by `setDictionaryInstance()` and `setUserDictionaryInstance()`.

### AnalysisWorker

`AnalysisWorker` (created with `Tokenizer::new_worker` or `Tokenizer::into_worker`) is a reusable session over the full analysis chain. It owns every per-call buffer — the Viterbi lattice and backtrace scratch (via `SegmentWorker`), the normalized-text buffer the character filters operate on, and the offset-mapping scratch — so repeated `tokenize` calls avoid the per-call allocations `Tokenizer::tokenize` pays. When character filters are configured, token surfaces borrow the worker's buffer instead of being copied into per-token `String`s. Returned tokens borrow the worker and must be consumed before the next call; for multi-threaded use, create one worker per thread (or guard one with a `Mutex`, as `lindera-binding` does), or tokenize a list of texts with an [`AnalysisWorkerPool`](#analysisworkerpool). See the [Segmenter](../lindera/segmenter.md) page for the underlying `SegmentWorker` and its automatic memory-shrink policy.

`AnalysisWorker`'s public methods:

- `tokenize(&mut self, text: &str)` — tokenizes `text` through the full analysis chain, reusing the worker's internal buffers; produces exactly the same tokens as `Tokenizer::tokenize`.
- `tokenize_nbest(&mut self, text: &str, n, unique, cost_threshold)` — tokenizes `text` and returns the top-N results with costs, reusing the worker's internal buffers; produces exactly the same results as `Tokenizer::tokenize_nbest`.
- `set_mode(&mut self, mode: Mode)` — sets the segmentation mode for subsequent calls.
- `set_keep_whitespace(&mut self, keep: bool)` — sets whether whitespace tokens are kept in the output for subsequent calls; kept whitespace also stays in the lattice, so the other tokens can differ.
- `set_skip_whitespace(&mut self, skip: bool)` — sets whether whitespace is skipped in the lattice for subsequent calls when whitespace tokens are not kept (by default the dictionary's setting).
- `set_max_grouping_len(&mut self, max_grouping_len: Option<usize>)`, `set_unknown_word_ladder(&mut self, unknown_word_ladder: bool)` — set the unknown-word grouping cap and the length ladder for subsequent calls.
- `set_break_characters(&mut self, chars: &str)` — sets the break characters for subsequent calls (`・` by default; an empty string gives MeCab's segmentation).
- `set_space_penalty(&mut self, space_penalty: Option<SpacePenaltyConfig>)`, `set_space_penalty_from_dictionary(&mut self)` — set the left-space penalty rules for subsequent calls, or restore the dictionary's.
- `shrink_to(&mut self, text_len_hint: usize)` — immediately shrinks the worker's internal buffers to what an input of `text_len_hint` bytes needs.
- `reset(&mut self)` — discards all internal buffers, replacing them with fresh ones; intended for recovery paths (e.g. after a panic poisoned a mutex holding the worker), with configuration (dictionary, filters, mode) preserved.
- `to_tokenizer(&self)` — returns a `Tokenizer` with the worker's configuration, the settings changed by the setters above included, from which more workers that tokenize as this one does can be made. The user dictionary, if any, is copied.

### AnalysisWorkerPool

`AnalysisWorkerPool` (created with `Tokenizer::new_worker_pool` or `Tokenizer::into_worker_pool`) tokenizes a list of texts on several threads. Every text is tokenized as a call of its own, as `AnalysisWorker::tokenize` tokenizes it, and the results come back in the order of the texts, so they are the same for every number of threads. A list of the lines of a document gives the tokens that `lindera tokenize` gives, since it also tokenizes every line on its own; it does not give those of the whole document as one text, whose offsets, positions and N-best results span its lines.

```rust
use std::num::NonZeroUsize;

let mut pool = tokenizer.into_worker_pool();
let threads = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
let surfaces: Vec<Vec<String>> = pool.tokenize_batch(&lines, threads, |tokens| {
    tokens.into_iter().map(|token| token.surface.into_owned()).collect()
})?;
```

The closure runs on the thread that tokenized the text, right after it: the tokens borrow that thread's worker, so it turns them into what the caller keeps (`String`s, counts, a formatted output), and that work runs in parallel too.

- `tokenize_batch(&mut self, texts, max_threads, f)` — tokenizes every text on up to `max_threads` threads, the calling thread included, and returns the results of `f` in the order of `texts`.
- `tokenize_nbest_batch(&mut self, texts, n, unique, cost_threshold, max_threads, f)` — the same with `AnalysisWorker::tokenize_nbest`; `f` gets the `(tokens, cost)` pairs of one text.
- `worker_count(&self)`, `truncate_workers(&mut self, max_workers)` — the number of workers the pool holds, and dropping the ones beyond `max_workers`.
- `shrink_to(&mut self, text_len_hint)`, `reset(&mut self)` — as `AnalysisWorker`'s, for every worker.

How a batch runs:

- The pool starts with one worker and makes more from it (`to_tokenizer`) when a batch runs on more threads than it has workers. Every worker keeps its buffers for the next batches and holds its own copy of the user dictionary, if any; the system dictionary is shared.
- A batch runs on the calling thread alone when it is given one thread, one text or a short input (the input of a thread is at least 16 KiB), and always on wasm, which has no threads. Otherwise it starts threads with `std::thread::scope`, which hand out the texts in chunks.
- If texts fail, the error of the first one in input order is returned, as a loop over the texts would return it, and the texts after it may not be tokenized. A panic, in the tokenization or in `f`, is resumed on the calling thread in the same way, after the worker that panicked is reset, so the pool stays usable.
- The workers are clones of one tokenizer, so a filter of your own whose instances keep state of their own gives each worker its own state.

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `embed-ipadic` | Embed the IPADIC dictionary in the binary (forwards to `lindera/embed-ipadic`) | No |
| `embed-ipadic-neologd` | Embed the IPADIC-NEologd dictionary in the binary (forwards to `lindera/embed-ipadic-neologd`) | No |
| `embed-unidic` | Embed the UniDic dictionary in the binary (forwards to `lindera/embed-unidic`) | No |
| `embed-unidic-cwj` | Embed the UniDic CWJ dictionary in the binary (forwards to `lindera/embed-unidic-cwj`) | No |
| `embed-unidic-csj` | Embed the UniDic CSJ dictionary in the binary (forwards to `lindera/embed-unidic-csj`) | No |
| `embed-sudachidict` | Embed the SudachiDict dictionary in the binary (forwards to `lindera/embed-sudachidict`) | No |
| `embed-ko-dic` | Embed the ko-dic dictionary in the binary (forwards to `lindera/embed-ko-dic`) | No |
| `embed-cc-cedict` | Embed the CC-CEDICT dictionary in the binary (forwards to `lindera/embed-cc-cedict`) | No |
| `embed-jieba` | Embed the Jieba dictionary in the binary (forwards to `lindera/embed-jieba`) | No |
