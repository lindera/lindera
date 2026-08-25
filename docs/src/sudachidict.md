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
> This recipe produces a dictionary whose **vocabulary and lattice behavior**
> match Sudachi, but it does not reproduce Sudachi's engine plugins.
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

Character and unknown-word definitions from the UniDic 2.1.2 source
(the same archive `lindera-unidic` builds from):

```shell
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp
% cp /tmp/unidic-mecab-2.1.2/char.def /tmp/unidic-mecab-2.1.2/unk.def .
```

## Align unk.def with the SudachiDict column layout

SudachiDict lexicon rows have 19 columns; column 4 (0-based) is the display
surface, so morphological details start at column 5. UniDic's `unk.def` has
its part-of-speech at column 4. Insert a placeholder display column so
unknown words expose their details at the same indices as dictionary words:

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
  "normalize_details": false,
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
  "user_dictionary_schema": { "fields": ["surface", "part_of_speech", "reading"] }
}
```

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
> or UniDic. Token filters that read the leading details positionally as
> part-of-speech tags (`japanese_stop_tags`, `japanese_keep_tags`,
> `japanese_compound_word`) will not match as expected with this dictionary.
> Schema-aware access such as `token.get("part_of_speech")` works correctly.

## Behavioral differences from Sudachi

The lattice layer is verified equivalent: the matrix file is byte-identical
(sha256) to the one SudachiDict's official build uses, and with Sudachi's
path-rewrite and input-normalization plugins disabled, the Sudachi engine
produces the same minimum-cost paths as this dictionary under Lindera.
The remaining differences are Sudachi *engine plugins*, not dictionary data:

| Sudachi feature | Effect | Status under Lindera |
| --- | --- | --- |
| `JoinKatakanaOovPlugin` | Rewrites paths to join katakana runs (e.g. `サブスク` instead of the cheaper `サブ`+`スク` path) | Not replicated |
| `JoinNumericPlugin` | Joins numeric sequences | Not replicated |
| `DefaultInputTextPlugin` | NFKC + lowercase normalization before lookup (e.g. half-width `AI` hits a normalized entry) | Partially available via Lindera character filters (`unicode_normalize`) |
| A/B/C split modes | Splits lexicon entries via `split_a` / `split_b` references | Not applied; entries are used whole. The split references are preserved in details, so a post-processing step can apply them |

## License

SudachiDict is Apache-2.0. `matrix.def`, `char.def`, and `unk.def` are part
of UniDic (BSD/GPL/LGPL triple license — see the [SudachiDict LEGAL
notice](https://github.com/WorksApplications/SudachiDict/blob/develop/LEGAL)).
A dictionary built with this recipe combines both; review the licenses
before redistribution.
