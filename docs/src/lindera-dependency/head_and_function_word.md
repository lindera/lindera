# Head and Function Word

Each bunsetsu has a head word (主辞), its last content word, and a function word (機能語), its last particle or auxiliary. In 読ん / で / いる / 。 the head word is 読ん and the function word いる.

- The head word is the word to take for a bunsetsu when indexing pairs of bunsetsu or extracting triples.
- The parser will build most of its features from the two.
- The lattice format has their positions on the bunsetsu line (`0/2`).

Which morphemes they are depends on the dictionary, and so does the column that holds the part of speech or the base form. A `Schema` holds all of it as data, and one piece of code applies it.

## The positions

```rust
use lindera_dependency::Schema;
use lindera_dependency::lattice::LatticeMorpheme;

let schema = Schema::ipadic();
// 利用 / し / た / 。
let bunsetsu = [
    LatticeMorpheme::new("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
    LatticeMorpheme::new("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
    LatticeMorpheme::new("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];
// The head word is 利用 and the function word た: (0, 2).
assert_eq!(schema.positions(&bunsetsu), Some((0, 2)));
```

`Schema::positions` takes the morphemes of one bunsetsu and returns the positions of the head word and of the function word, counted from the first morpheme of the bunsetsu; `None` when there is no morpheme.

## The rule

The rule is the one of CaboCha (`findHead`):

1. Both positions start at the first morpheme of the bunsetsu.
2. From the left, a morpheme that matches a function word pattern becomes the function word. A morpheme that matches none of them, is not a symbol, and is not in the list of what is not a head becomes the head word.
3. A head word after the function word becomes the function word too.

So a bunsetsu without a function word has both at its head word (本 alone: `0/0`), and a bunsetsu of symbols only has both at its first morpheme.

A pattern is a prefix of the part-of-speech levels (動詞, 非自立), optionally with the base forms the morpheme may have and with a pattern for the morpheme before it in the bunsetsu. A morpheme with fewer feature columns than the schema names matches no pattern on the missing ones; nothing panics.

## Punctuation and brackets

A schema also recognizes symbols, for the features of the parser:

```rust
assert!(schema.is_period(&bunsetsu[3]));
assert!(schema.is_symbol(&bunsetsu[3]));
// A half-width comma is an unknown word of IPADIC, and still a comma.
let comma = LatticeMorpheme::new(",", "名詞,サ変接続,*,*,*,*,*,*,*");
assert!(schema.is_comma(&comma));
```

| Method | Recognizes |
| --- | --- |
| `is_comma` | 、 and `,` `，` |
| `is_period` | 。 and `.` `．` |
| `is_open_bracket`, `is_close_bracket` | Brackets and quotation marks, in full width and half width |
| `is_symbol` | Any of those, and the other symbols |

A symbol is recognized by its part of speech or by its surface. The surface is needed because a dictionary does not always give a symbol the part of speech of one: IPADIC has no half-width symbols, so `,` is an unknown word and comes out as 名詞,サ変接続.

## The schema of IPADIC

`Schema::ipadic()` names the columns of IPADIC (four levels of part of speech, then the conjugation type, the conjugation form and the base form), and starts from the patterns of CaboCha for IPA:

| | Patterns |
| --- | --- |
| Function words | 助詞, 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立, 形容詞,接尾 (CaboCha); する, できる and 出来る (動詞,自立) |
| Not heads | 記号 (CaboCha) and symbols by their surface; 名詞,非自立 (こと, の, よう); the suffixes 的 (名詞,接尾,形容動詞語幹) and さ (名詞,接尾,特殊); a 動詞,自立 right after a 助詞 (こと / に / なっ / た, かも / しれ / ない) |

What is added to CaboCha's patterns comes from a measurement. UD Japanese-GSD marks the content head of each bunsetsu, and [`convert`](./training_data.md) counts how often the head word of the schema is that word:

| Rule | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| The rule of CaboCha for IPA | 85.43% | 86.30% | 84.61% | 86.70% |
| `Schema::ipadic()` | 96.55% (54,449 / 56,396) | 96.96% (4,021 / 4,147) | 96.92% (4,370 / 4,509) | 97.46% (9,593 / 9,843) |

The largest difference is the verbal noun with する (利用 / し / た): CaboCha's rule makes し the head word, the corpus the noun. The steps between the two lines, and the variants that were not taken, are in the [README of the crate](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#agreement-with-ud-japanese-gsd).

The function word is defined by the schema alone: the corpus has no label that means the same.

## Another dictionary

A schema is a value whose fields are public: the columns, `function_words`, `not_heads` and `symbols`. A schema for another dictionary is built from data, without new code. IPADIC is the only one the crate provides so far; UniDic is to follow.

```rust
use lindera_dependency::{Marks, Pattern, Schema, Symbols};

// A dictionary with two levels of part of speech and the base form in the
// third column.
let schema = Schema {
    part_of_speech: 0..2,
    base_form: Some(2),
    function_words: vec![Pattern::new(&["助詞"]), Pattern::new(&["助動詞"])],
    not_heads: vec![Pattern::new(&["動詞"]).after(Pattern::new(&["助詞"]))],
    symbols: Symbols {
        others: Marks::new(&[&["記号"]], "!?"),
        ..Symbols::default()
    },
    ..Schema::default()
};
```
