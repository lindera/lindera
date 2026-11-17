# SudachiDict (Custom Build)

This page describes how to build [SudachiDict](https://github.com/WorksApplications/SudachiDict)
— the actively maintained Japanese dictionary used by Sudachi — as a Lindera
dictionary directly from the upstream raw distribution, using only the
generic dictionary builder and a metadata file. No engine changes are
required.

> [!TIP]
> SudachiDict is also available as an official dictionary crate:
> [`lindera-sudachidict`](./lindera-sudachidict.md). Prefer the crate for
> normal use (`--features embed-sudachidict`, `lindera download sudachidict`,
> or the pre-built release archive). Use this page when you want to track a
> newer upstream release than the bundled one, or to build a custom variant
> (for example, small + core only).

Related issue: [lindera/lindera#487](https://github.com/lindera/lindera/issues/487)

> [!NOTE]
> This recipe produces a dictionary whose **vocabulary, character
> definitions and unknown words** match Sudachi, but it does not reproduce
> Sudachi's engine plugins and a few engine rules.
> See [Behavioral differences](#behavioral-differences-from-sudachi) below
> before relying on it.

## Why

The classic MeCab-format dictionaries (IPADIC, UniDic 2.1.2) stopped
receiving vocabulary updates years ago. SudachiDict is updated several times a
year and is distributed in a MeCab-compatible raw CSV format, which the
generic builder can compile directly. The official `lindera-sudachidict`
crate bundles the `20260723` release; building from the upstream raw
distribution with this recipe lets you pick up newer releases yourself.
With the `20260723` release:

- `令和` is a single proper noun (IPADIC and UniDic 2.1.2 split it into `令|和`)
- `スマホ`, `テレワーク`, `推し活`, `コロナ禍` are in-vocabulary
- On a 321K-character novel from 1905, the unknown-token rate drops from
  2.00% (IPADIC) to 0.51%

## Sources

SudachiDict raw lexicons (check
[the listing](http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/)
for the latest date):

```shell
% mkdir -p /tmp/sudachidict-src
% cd /tmp/sudachidict-src
% for f in small_lex core_lex notcore_lex; do
    curl -LO "http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/20260723/${f}.zip"
    unzip -o "${f}.zip" && rm "${f}.zip"
  done
```

The connection matrix. SudachiDict's own build downloads this file — it is
the UniDic 2.1.2 matrix (SudachiDict entries use UniDic context IDs):

```shell
% curl -LO "https://d2ej7fkh96fzlu.cloudfront.net/sudachidict-raw/matrix.def.zip"
% unzip -o matrix.def.zip && rm matrix.def.zip
```

Sudachi's own character and unknown-word definitions, from the `v0.7.0` tag
of sudachi.rs (Sudachi `v0.8.2` ships the same two files). Its `unk.def` is
the same file as UniDic 2.1.2's; its `char.def` is UniDic's with Sudachi's
fixes and with syntax that only Sudachi reads, which the
[metadata](#metadata) below selects:

```shell
% curl -LO "https://raw.githubusercontent.com/WorksApplications/sudachi.rs/v0.7.0/resources/char.def"
% curl -LO "https://raw.githubusercontent.com/WorksApplications/sudachi.rs/v0.7.0/resources/unk.def"
```

## Align unk.def with the SudachiDict column layout

SudachiDict lexicon rows have 19 columns; column 4 (0-based) is the display
surface, so morphological details start at column 5. Sudachi's `unk.def`, like
UniDic's, has its part-of-speech at column 4. Insert a placeholder display
column so unknown words expose their details at the same indices as
dictionary words:

```shell
% awk -F, 'BEGIN{OFS=","} {out=$1 OFS $2 OFS $3 OFS $4 OFS "*"; for(i=5;i<=NF;i++) out=out OFS $i; print out}' unk.def > unk.def.tmp && mv unk.def.tmp unk.def
```

## Metadata

Save as `sudachidict-metadata.json`. The first four fields are the builder's
reserved columns; the rest name SudachiDict's columns in order:

```json
{
  "name": "sudachidict",
  "encoding": "UTF-8",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": true,
  "skip_invalid_cost_or_id": true,
  "dictionary_schema": {
    "fields": [
      "surface",
      "left_context_id",
      "right_context_id",
      "cost",
      "display_surface",
      "part_of_speech",
      "part_of_speech_subcategory_1",
      "part_of_speech_subcategory_2",
      "part_of_speech_subcategory_3",
      "conjugation_type",
      "conjugation_form",
      "reading",
      "normalized_form",
      "dictionary_form_id",
      "split_mode",
      "split_a",
      "split_b",
      "word_structure",
      "synonym_group_ids"
    ]
  },
  "user_dictionary_schema": { "fields": ["surface", "part_of_speech", "reading"] },
  "skip_whitespace": false,
  "char_def_format": "sudachi"
}
```

`skip_whitespace: false` keeps whitespace in the lattice, as Sudachi does and
as SudachiDict's costs assume; without it, Lindera skips whitespace as MeCab
does (see [Segmenter](./lindera/segmenter.md#whitespace-handling)).

`char_def_format: "sudachi"` reads `char.def` as Sudachi does: a code point
gets the categories of every line that covers it, `ALL` stands for every
category, and the flags `NOOOVBOW` and `NOOOVEOW` keep unknown words from
starting at a character and right after it. Unknown words then come from
every category of a character, over runs cut as Sudachi cuts them, and a
position where nothing else starts gets a one-character unknown word of
`DEFAULT`, as in Sudachi. Without it, the builder reads `char.def` in
MeCab's format, which stops at the first `NOOOVBOW` of Sudachi's file (see
[`lindera build`](./lindera-cli/commands.md#build)).

## Build and verify

```shell
% lindera build \
  --src /tmp/sudachidict-src \
  --dest /tmp/lindera-sudachidict \
  --metadata ./sudachidict-metadata.json
```

Building small + core + notcore (2026-07-23) with Lindera v6 takes about
10 seconds and produces a ~570MB dictionary. Note that built dictionaries
must be rebuilt when the on-disk dictionary format version changes across
Lindera releases (see [Migration v5 to v6](./migration_v5_to_v6.md)).

```shell
% echo "令和五年に始まった。推し活が楽しい。" | lindera tokenize --dict /tmp/lindera-sudachidict
令和	令和,名詞,固有名詞,一般,*,*,*,レイワ,令和,*,A,*,*,*,*
五	五,名詞,数詞,*,*,*,*,ゴ,五,*,A,*,*,*,017040
年	年,名詞,普通名詞,助数詞可能,*,*,*,ネン,年,*,A,*,*,*,*
...
推し活	推し活,名詞,普通名詞,一般,*,*,*,オシカツ,推し活,*,A,*,*,*,*
```

Details expose SudachiDict's extra columns (normalized form, split
references, synonym group IDs), so downstream code can use them.

> [!NOTE]
> Because `display_surface` sits before the part-of-speech columns,
> `details[0]` is the display surface — not the part-of-speech as in IPADIC
> or UniDic. Code that reads the details positionally must account for
> this; schema-aware access such as `token.get("part_of_speech")` returns
> the part-of-speech directly. The part-of-speech token filters
> (`japanese_stop_tags`, `japanese_keep_tags`, `japanese_compound_word`,
> and `japanese_number` with `tags`) resolve the part-of-speech columns
> through the dictionary schema, so they match this dictionary the same way
> they match IPADIC or UniDic.

## Behavioral differences from Sudachi

The dictionary data is Sudachi's: the matrix file is byte-identical (sha256)
to the one SudachiDict's official build uses, `char.def` and `unk.def` are
Sudachi's own, and `char_def_format: "sudachi"` makes Lindera read
`char.def` and create unknown words as Sudachi does
([#1388](https://github.com/lindera/lindera/issues/1388)). Compared with
sudachi.rs `v0.7.0` on the same lexicons, with Sudachi's input-text and
path-rewrite plugins disabled (its unknown-word plugins only, mode C,
sentences not split), Lindera gives the same tokens on 502 of the 505 lines
of *Botchan* (`resources/bocchan.txt`): two lines have two paths of equal
cost, and Sudachi chooses the other one (`大学 / 卒業生` where Lindera gives
`大学卒業 / 生`), and one has a URL, where Sudachi uses the dictionary word
`http://`, which the lexicon writes as `http://` and the builder
does not decode yet ([#1389](https://github.com/lindera/lindera/issues/1389)).
The remaining differences are in the engine:

| Sudachi feature | Effect | Status under Lindera |
| --- | --- | --- |
| No word starts at a character that is Latin, Greek or Cyrillic (`ALPHA`, `GREEK`, `CYRILLIC`, or `ALL` as combining marks, variation selectors and ZWJ are) and shares a category with the character before it (`can_bow`) | A dictionary word cannot end before such a character either: `cafe` followed by a combining acute accent is one unknown word `café`, and `♥` with a variation selector is one word | Not replicated |
| Ties between paths of equal cost | Sudachi chooses another path than MeCab's rule | Lindera breaks ties as MeCab does |
| The fallback where nothing else starts (`SimpleOovPlugin`) | The word runs to the next character where a word can start (`can_bow`) | One character |
| `JoinKatakanaOovPlugin` | Rewrites paths to join katakana runs (e.g. `サブスク` instead of the cheaper `サブ`+`スク` path) | Not replicated |
| `JoinNumericPlugin` | Joins numeric sequences | Not replicated |
| `DefaultInputTextPlugin` | NFKC + lowercase normalization before lookup (e.g. half-width `AI` hits a normalized entry) | Partially available via Lindera character filters (`unicode_normalize`) |
| A/B/C split modes | Splits lexicon entries via `split_a` / `split_b` references | Not applied; entries are used whole. The split references are preserved in details, so a post-processing step can apply them |

## License

SudachiDict is Apache-2.0. `matrix.def` is part of UniDic (BSD/GPL/LGPL
triple license — see the [SudachiDict LEGAL
notice](https://github.com/WorksApplications/SudachiDict/blob/develop/LEGAL)).
`char.def` and `unk.def` come from sudachi.rs (Apache-2.0); `unk.def` is the
same file as UniDic's, and `char.def` is UniDic's with Sudachi's changes. A
dictionary built with this recipe combines them; review the licenses before
redistribution.
