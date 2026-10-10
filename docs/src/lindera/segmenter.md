# Segmenter

The `Segmenter` is the core component that performs morphological analysis. It uses the Viterbi algorithm to find the optimal segmentation of input text based on a dictionary and cost model. Among dictionary entries that share the surface, the context IDs and the cost, and so tie, it chooses the first CSV row, as MeCab does, and a user-dictionary entry over a tied system entry (see [Tied entries](../concepts/user_dictionary.md#tied-entries)). Of two segmentations of equal cost that end at the same position, it chooses the one whose last word starts later, as MeCab does.

## Creating a Segmenter

A `Segmenter` requires three components:

- **Mode** - the tokenization strategy (`Normal` or `Decompose`)
- **Dictionary** - a system dictionary for morphological analysis
- **UserDictionary** (optional) - a supplementary dictionary for custom words

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("embedded://ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
```

## Tokenization Modes

### Mode::Normal

Standard tokenization based on the dictionary entries. Words are segmented faithfully according to what is registered in the dictionary.

```rust
use lindera::mode::Mode;

let mode = Mode::Normal;
```

### Mode::Decompose

Decomposes compound nouns into their constituent parts. This mode applies a configurable penalty to long compound words, encouraging the segmenter to split them into shorter components.

For example, with `Mode::Normal`, the compound word "関西国際空港" in the sentence "関西国際空港限定トートバッグ" remains part of a single token, while with `Mode::Decompose`, it is split into "関西", "国際", and "空港".

```rust
use lindera::mode::Mode;

let mode = Mode::Decompose(Default::default());
```

## Dictionary Loading

Lindera provides the `load_dictionary` function to load dictionaries from various sources.

### Embedded Dictionaries

When built with the appropriate feature flag (e.g., `embed-ipadic`), dictionaries can be loaded directly from the binary:

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("embedded://ipadic")?;
```

Available embedded dictionary URIs:

- `embedded://ipadic` - IPADIC (Japanese)
- `embedded://ipadic-neologd` - IPADIC NEologd (Japanese)
- `embedded://unidic` - UniDic (Japanese)
- `embedded://ko-dic` - ko-dic (Korean)
- `embedded://cc-cedict` - CC-CEDICT (Chinese)
- `embedded://jieba` - Jieba (Chinese)

### External Dictionaries

Pre-built dictionary directories can be loaded from the filesystem:

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("/path/to/dictionary")?;
```

## Using with Tokenizer

The `Segmenter` is typically used through the `Tokenizer`, which adds support for character filters and token filters:

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "日本語の形態素解析を行うことができます。";
    let tokens = tokenizer.tokenize(text)?;

    for mut token in tokens {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

Note the `mut` binding on `token`: `Token::details` takes `&mut self`, so iterating with a plain `for token in tokens` fails to compile (`E0596: cannot borrow as mutable`).

## Building from Config

`Segmenter::from_config` builds a `Segmenter` from a `SegmenterConfig` (a `serde_json::Value`), the same configuration format used by `Tokenizer`/`TokenizerBuilder` (see [Configuration](../lindera-analysis/configuration.md)) but scoped to just the `segmenter:` section:

```rust
use serde_json::json;
use lindera::segmenter::{Segmenter, SegmenterConfig};

let config: SegmenterConfig = json!({
    "mode": "normal",
    "dictionary": "embedded://ipadic",
    "keep_whitespace": false,
    "use_mmap": false
});
let segmenter = Segmenter::from_config(&config)?;
```

Note: `use_mmap` here is set to `false` only to illustrate the option explicitly; omitting it entirely gives the same default (`true`, see below).

`Segmenter::from_config_with_dictionaries` applies a configuration to dictionaries that are already loaded, such as ones loaded from bytes. A dictionary passed as `Some` is used instead of the one the `dictionary` key names, and the key may then be absent (`use_mmap` does not apply); a user dictionary passed as `Some` is used instead of the one the `user_dictionary` key names. For `None`, the dictionary is loaded from the configuration as `from_config` loads it, and a configured user dictionary is built with the metadata of the system dictionary in use. Every other key applies as with `from_config`:

```rust
use serde_json::json;
use lindera::dictionary::load_dictionary;
use lindera::segmenter::{Segmenter, SegmenterConfig};

let dictionary = load_dictionary("embedded://ipadic")?;
let config: SegmenterConfig = json!({
    "mode": "normal",
    "keep_whitespace": true
});
let segmenter = Segmenter::from_config_with_dictionaries(&config, Some(dictionary), None)?;
```

`Tokenizer::from_config_with_dictionaries` and `TokenizerBuilder::build_with_dictionaries` do the same for a tokenizer configuration, filters included (see [Architecture](../lindera-analysis/architecture.md)).

## Memory-Mapped Loading

For a filesystem-based (not `embedded://`) dictionary, `use_mmap` defaults
to `true` whenever the `mmap` Cargo feature is compiled in (the default) —
memory-mapped reads are used automatically. Set it to `false` to force
eager, non-memory-mapped file reads instead. Every large component stays
lazily paged this way: the word-list files, the connection-cost matrix,
and the prefix-dictionary trie, which is walked in place over its
serialized bytes. `use_mmap` is silently ignored for `embedded://`
dictionaries, since their data is already a static, zero-copy byte slice.
Requires the `mmap` cargo feature (enabled by default).

## Whitespace Handling

By default, whitespace is skipped in the lattice, as MeCab does: no word starts on a whitespace character, and the word after a run of whitespace connects directly to the word before it, with the connection cost the dictionary was trained with. Skipped whitespace is not output and is left out of token surfaces and offsets; whitespace that belongs to a dictionary entry, inside it (IPADIC-NEologd `GeForce GTX`, a user entry `해운대 해수욕장`) or at its end (ko-dic `에듀` followed by a space), stays in that entry's token, as in MeCab. "Whitespace" is the dictionary's `SPACE` character category (`char.def`): U+0020, U+0009, U+000A, U+000B and U+000D for ko-dic, and U+0020, U+0009, U+000A and U+000B for IPADIC, IPADIC-NEologd, UniDic, SudachiDict, CC-CEDICT and Jieba. Those `char.def` files also map U+00D0 (`Ð`) to `SPACE`, a typo for U+000D, but a later `0x00C0..0x00FF ALPHA` line takes precedence, so `Ð` is a letter, as in MeCab.

The default comes from the dictionary's `metadata.json` (`skip_whitespace`). SudachiDict sets it to `false`: Sudachi keeps whitespace in the lattice and outputs it as `空白` tokens, and SudachiDict's costs are tuned for that, so skipping would, for example, split the entry `caramel man` into two common nouns. With SudachiDict, whitespace therefore stays in the lattice by default, as a node of its own (a `空白` dictionary entry, such as the one for U+0020, or the `SPACE` unknown word), though it is still dropped from the output. The other bundled dictionaries leave the setting out, which means skipping, and so does a dictionary built before the setting existed (a SudachiDict built by Lindera 6.x included, until it is rebuilt).

Call `keep_whitespace(true)` on a `Segmenter` to output whitespace tokens. Whitespace then also stays in the lattice, as a node of its own (a whitespace dictionary entry, such as IPADIC's or UniDic's for U+3000, or the `SPACE` unknown word), so the other tokens can differ from those of the default output, which skips whitespace:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
```

Before v7, Lindera kept whitespace in the lattice even when it dropped it from the output, so the words on either side of a space connected to the whitespace node instead of to each other. How much that mattered depends on the dictionary's connection costs for the `SPACE` unknown word. In ko-dic its row and column of the connection matrix are all zero, so every space reset the context and a word between spaces was chosen by its word cost alone: ko-dic read `전` in `2년 전 대회` as `저/NP + ㄴ/JX` instead of the noun `NNG`. In IPADIC and UniDic the costs are not zero (IPADIC's `SPACE` unknown word shares its context with the full-width space entry `記号,空白`), and IPADIC read `が` in `Google が 新しい` as a conjunction. CC-CEDICT and Jieba have no connection costs at all, so their segmentation does not depend on this, although the path costs that N-best search reports do. `skip_whitespace` overrides the dictionary's default: `skip_whitespace(false)` keeps whitespace nodes in the lattice as before, while still dropping whitespace from the output, and `skip_whitespace(true)` turns skipping on for SudachiDict. In a `SegmenterConfig` the `skip_whitespace` key takes `true` or `false`, and leaving it out, or `null`, keeps the dictionary's default; on the CLI, `--disable-skip-whitespace` turns skipping off:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
```

A `\n` or a `\t` still ends a segment (see [Sentence Splitting](#sentence-splitting)), so the context is not carried across a newline or a tab. mecab-ko analyzes a line at a time and skips a tab inside it like any other whitespace.

## Unknown-Word Grouping

A word that is not in the dictionary is read as an *unknown word*, built from the categories that the dictionary's `char.def` gives its characters (katakana, Latin letters, digits, symbols and so on). A character can have more than one category, such as a kanji numeral (`KANJINUMERIC KANJI` in IPADIC); the first category of the `char.def` line that decides its categories is its *default category*. At every position that a path through the lattice reaches, Lindera creates unknown-word candidates from the default category of the character there, if that category is set to always create them (`INVOKE` in `char.def`) or if no dictionary word starts at that position, as MeCab does. If the category is set to group (`GROUP`), one candidate is a *grouped unknown word*: the run of characters from that position on in which each character shares a category with the one before it, such as a run of katakana.

With ko-dic, the numeral `三` is `HANJANUMERIC HANJA` (default category `HANJANUMERIC`, which groups) and the other hanja are `HANJA`, so the grouped unknown word that starts at `三` runs through the hanja after it: `三國史記` is one unknown word (`SH`), as in MeCab. Before v7, Lindera created candidates for every category of a character and went on with a run only while the next character had the same category at the same position of its category list, so it returned `三 / 國史 / 記`.

An unknown word can start at any such position, also inside a run that a grouped unknown word from an earlier position covers. IPADIC's `char.def` puts `・` in the katakana range, so in `ジョン・レノンが歌う` the first position has the grouped unknown word `ジョン・レノン`. The dictionary words `ジョン` and `・` reach the position of `レ`, where the unknown word `レノン` starts, and the result is `ジョン`, `・`, `レノン`, `が` and `歌う`, as in MeCab. Before v7, normal mode created no unknown words at the positions inside the last grouped unknown word, a shortcut inherited from Kuromoji, so it returned `ジョン・レノン` as one unknown word; Decompose mode never took this shortcut.

Unknown-word grouping is unbounded by default. `max_grouping_len(Some(n))` caps it with MeCab's `max-grouping-size` semantics, counting characters beyond the first (MeCab defaults to 24):

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).max_grouping_len(Some(24));
```

The cap is applied at each lattice position rather than to a run as a whole. When the run starting at a position is longer than the cap, the grouped candidate is not emitted there; the single-character candidate (plus the length-ladder candidates described below and any dictionary words) remains, and the remaining tail is grouped again once it fits. No unknown token is therefore longer than `n + 1` characters, but an over-long run does not turn into single characters throughout. With IPADIC, the out-of-vocabulary katakana run `ヷヸヹヺヷ` and `max_grouping_len(Some(2))` segment as `ヷ / ヸ / ヹヺヷ` when the length ladder is off: the first two positions see runs of 5 and 4 characters (over the cap) and keep only the single-character candidate, while the 3-character tail fits the cap and groups. With the ladder on (the default), the 2-character ladder candidate wins at the first position and the output is `ヷヸ / ヹヺヷ`.

`Some(0)` never groups a run of two or more characters. The `max_grouping_len` config key and the CLI flag `--max-grouping-len` treat `0` as unbounded instead.

Independently of grouping, Lindera also generates a MeCab/Vibrato-inspired "length ladder" of shorter unknown-word candidates by default, so the Viterbi search can pick whichever length scores lowest. The candidates are one character long and longer, up to the `char.def` `LENGTH` field of the default category, over the characters that share a category with the first one; as in MeCab, each character is compared with the first one, not with the one before it. Call `unknown_word_ladder(false)` to disable it: a position then gets only the grouped or the single-character candidate, as before v6. The output still differs from that of Lindera before v6 wherever later changes apply, such as unknown words that start inside a grouped run (see [Migrating from v6 to v7](../migration_v6_to_v7.md#unknown-words-start-at-every-position-as-in-mecab)):

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).unknown_word_ladder(false);
```

## Left-Space Penalty (Korean)

MeCab-based Korean analyzers (mecab-ko with mecab-ko-dic, Lucene's nori) add a cost to a candidate that starts right after whitespace when its part-of-speech tag is one that attaches to the preceding word without a space: particles (`J*`), endings (`E*`), the copula (`VCP`) and derivational suffixes (`XS*`). Without it, `검색 이 잘 된다` reads `이` as the subject particle `JKS` rather than the interjection `IC`. Lindera applies the same penalty by default for dictionaries that ship the rules in their metadata (ko-dic does); other dictionaries are unaffected. Opt out with `Segmenter::space_penalty(None)`, `"space_penalty": false` in the config, or `--disable-space-penalty` on the CLI. Together with `skip_whitespace(false)`, this turns off both changes since v6.0 in how Korean is read around spaces (this penalty and whitespace skipping), but not the other output changes since then, such as the split of punctuation runs like `."` (see [Migrating from v6 to v7](../migration_v6_to_v7.md#unknown-words-start-at-every-position-as-in-mecab)).

`SpacePenaltyConfig` is a list of rules, each pairing first part-of-speech tags with a cost. A candidate is matched by the part of its tag before the first `+` (for ko-dic `Inflect` rows, the `first_part_of_speech` column); the first matching rule wins and unlisted tags cost nothing. mecab-ko-dic's `dicrc` rules translate to:

```rust
use lindera::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};

let rules = SpacePenaltyConfig::new(vec![
    SpacePenaltyRule::new(["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], 3000),
    SpacePenaltyRule::new(["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], 6000),
]);
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty(Some(rules))?;
```

"Whitespace" means a character of the dictionary's `SPACE` category (`char.def`), the same set that is skipped in the lattice and that `keep_whitespace` filters on; a dictionary without that category falls back to Unicode `White_Space`. The penalty applies in both modes and in N-best search, to system, user and unknown-word entries alike. `space_penalty` builds a per-word-id lookup once (a few tens of milliseconds for ko-dic), and returns an error when the dictionary schema has neither a `part_of_speech_tag` nor a `part_of_speech` field.

A dictionary can ship its default rules in `metadata.json` under `space_penalty`; ko-dic does, with exactly the rules above, and `Segmenter::new` applies them automatically. `space_penalty_from_dictionary()` re-enables them after an opt-out and returns an error for a dictionary that ships none. Shipped rules that cannot be applied, because the schema has no part-of-speech field, only log a warning in `Segmenter::new`:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty_from_dictionary()?;
```

Only a ko-dic built by Lindera 6.1.0 or later carries these rules. A ko-dic downloaded from the v6.0.0 release ships none, so with it the penalty silently stays off, and `space_penalty_from_dictionary()` (or `"space_penalty": true`) returns an error until the dictionary is rebuilt. Explicit rules (`space_penalty`, `--space-penalty-rules`) work with any ko-dic.

In a `SegmenterConfig`, the `space_penalty` key takes an object for explicit rules, `false` for off, and `true` for the dictionary's rules (an error when it ships none). Leaving the key out, or `null`, keeps the default: the dictionary's rules when it ships any.

```json
{
  "dictionary": "embedded://ko-dic",
  "space_penalty": {
    "rules": [
      { "pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000 },
      { "pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000 }
    ]
  }
}
```

The penalty keys on the character before a candidate, which is mecab-ko's test as well, so it applies the same way whether whitespace is skipped in the lattice (the default, see [Whitespace Handling](#whitespace-handling)) or kept as nodes. With whitespace skipped, the connection costs on their own already rule out some readings the penalty targets (`시` in `서울 시 에서` reads as `NNG` without it), but not all of them (`이` in `검색 이 잘 된다`).

## N-Best Segmentation

`segment_nbest` returns the top-`n` segmentations ordered by total path cost, each paired with its cost. Set `unique` to deduplicate results that share the same word boundaries but differ only in POS tags (each result is the cheapest path of its word boundaries, and the search does not go through the others, so it stays fast when the words of a line have many entries), and `cost_threshold` to discard paths whose cost exceeds `best_cost + threshold`:

```rust
let results = segmenter.segment_nbest(Cow::Borrowed("すもももももももものうち"), 3, false, None)?;
for (tokens, cost) in results {
    println!("cost={cost}");
    for token in tokens {
        println!("  {}", token.surface.as_ref());
    }
}
```

Every result segments the whole input. The input is split into segments as described in [Sentence Splitting](#sentence-splitting), and each segment is searched on its own. Within a segment, the context is carried across `、`, `。` and forced cuts, so the paths of a segment are the `n` best paths of one lattice over the whole segment, with the exceptions listed there; `unique` compares the word boundaries of the whole segment. A result takes one path in each segment, its cost is the sum of those paths' costs, and the results are the `n` cheapest of these combinations. So a result can differ from the best one in a single segment. `best_cost` is the cost of the first result, so `cost_threshold` applies to the whole input. The first result is always the output of `segment`. An input without words gives one result without tokens, with the connection cost from BOS to EOS, as in MeCab: an input of skipped whitespace and, since v7, an empty input (which gave no result before).

`segment_nbest_with_lattice` is the same operation but lets you pass in a reusable `Lattice` buffer to avoid reallocating one per call.

## Sentence Splitting

Before building the lattice, Lindera splits the input into sentences at delimiter characters (`\n`, `\t`, `。`, `、`) and builds one Viterbi lattice per sentence. If no delimiter appears within roughly 32 KiB of a sentence's start, Lindera forces a sentence boundary there anyway and logs a warning. The cuts bound the size — and therefore the memory and CPU cost — of each lattice.

A `\n`, a `\t` or the end of the input ends a *segment*, the run of sentences up to it. Segments are independent of each other. Within a segment, the cuts at `、` and `。` and the forced cuts carry the context: the next sentence starts from the words that end the previous one (from their right context IDs, each with the cost of the best path to it) rather than from the beginning of a sentence (BOS), and the connection to the end of the sentence (EOS) is paid only at the end of the segment. So the best path of a segment, and its cost, are those of one lattice over the whole segment, as MeCab, which does not cut inside a line, finds them. Before v7, every sentence started from BOS and ended with EOS, so the word after `、` lost its context: with IPADIC, `ああ` in `彼は、ああ言った` was an interjection, and it is now an adverb, as in MeCab.

The result can still differ from one lattice over the segment:

- No word spans a cut: a dictionary entry that contains `、` or `。`, such as UniDic's `一、二塁`, never matches, an unknown word is not grouped across a cut, and a word that would span a forced cut is split there.
- The left-space penalty (see [Left-Space Penalty (Korean)](#left-space-penalty-korean)) does not apply to a word that starts right at a forced cut, even when whitespace precedes the cut.
- A sentence without a path, which only a dictionary that leaves some character without an unknown-word candidate gives, ends the segment before it: the best path up to the cut is kept as a path that ends there, the EOS connection included, and the sentence is segmented on its own. The first N-best result keeps the same path.

Ordinary text never reaches the forced cut, but for pathological delimiter-free input (e.g. minified text or base64-encoded blobs), it can affect tokenization at the artificial cut point.

N-best search uses the same segments; see [N-Best Segmentation](#n-best-segmentation).

## Reusable Worker

`SegmentWorker` is a reusable segmentation session that owns the Viterbi lattice and the scratch buffers of the segmentation, so repeated calls avoid the per-call allocations `segment` pays. Create one with `new_worker` (clones the segmenter) or `into_worker` (consumes it, avoiding a user-dictionary copy), then call `segment`/`segment_nbest` on it:

```rust
let mut worker = segmenter.new_worker();
for line in lines {
    let tokens = worker.segment(line)?;
    for token in &tokens {
        println!("{}", token.surface.as_ref());
    }
}
```

The returned tokens borrow the worker, so they must be consumed before the next call (the usual per-line loop above compiles as-is). `set_mode`, `set_keep_whitespace`, `set_skip_whitespace`, `set_max_grouping_len`, `set_unknown_word_ladder`, `set_space_penalty` and `set_space_penalty_from_dictionary` switch the configuration between calls. `segmenter()` returns a shared reference to the underlying segmenter; no `&mut` accessor is provided, since swapping the dictionary out from under the reused lattice would break the dictionary-lattice pairing the worker exists to guarantee.

A worker also bounds retained memory: one delimiter-free maximum-length sentence grows the lattice to several MB (roughly 18 MB for a 32 KiB ASCII sentence, 6 MB for a 32 KiB CJK one, whose character-indexed lattice has a third of the slots), and a plain `Lattice` keeps that forever. The worker automatically shrinks its lattice, and releases its scratch buffers, once a window of calls shows the capacity is oversized, and `shrink_to(text_len_hint)` forces a shrink immediately. `reset()` discards the internal buffers outright and replaces them with fresh ones; it is intended for recovery paths (e.g. after a panic poisoned a mutex holding the worker) where the buffers may hold an inconsistent intermediate state — the segmenter configuration itself is preserved.

The worker is permanently bound to the dictionary of the segmenter that created it; there is no way to swap dictionaries under a live worker, which rules out a class of lattice-reuse bugs by construction. For multi-threaded use, create one worker per thread from a shared `Segmenter`.
