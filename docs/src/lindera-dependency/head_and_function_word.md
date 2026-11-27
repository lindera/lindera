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

A schema can have the other form `findHead` has, the one CaboCha uses for JUMAN (`HeadRule::LastWords` in `Schema::head_rule`): the function word is the last morpheme that is not a symbol, and the head word the last one that is neither a symbol nor in the list of what is not a head. The two are found apart, the patterns of the function words are not read, and step 3 is left out. The schemas of this crate do not use it; a model converted from [CaboCha's for JUMAN](./cabocha_models.md) does, with 特殊 as its symbols and 助詞 and 接尾辞 as what is not a head: 読んで / いる has `0/1`, as いる is a 接尾辞 there.

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

## The schema of UniDic

`Schema::unidic()` is the schema of the three UniDic dictionaries of Lindera: `unidic` (unidic-mecab 2.1.2), `unidic-cwj` and `unidic-csj`. The columns it reads are at the same places in the three: four levels of part of speech, the conjugation type, the conjugation form, and the 語彙素 in the eighth column, which is the base form. The base form of こと is therefore 事. It starts from the pattern of CaboCha for UniDic:

| | Patterns |
| --- | --- |
| Function words | 助詞, 助動詞, 接尾辞,形容詞的, 接尾辞,形状詞的, 接尾辞,動詞的 (CaboCha); 動詞,非自立可能 and 形容詞,非自立可能 |
| Not heads | 空白, 補助記号 and 記号; a 動詞 right after に, と or を (事故 / に / よっ / て); 助動詞語幹 (よう, そう, みたい); a formal noun (こと, ため, もの, 際, うえ, ところ) right after a 動詞, a 助動詞 or a 形容詞; a nominal suffix (接尾辞,名詞的) right after a 形容詞 (大き / さ) |

```rust
use lindera_dependency::Schema;
use lindera_dependency::lattice::LatticeMorpheme;

let schema = Schema::unidic();
// 利用 / する / こと / が
let bunsetsu = [
    LatticeMorpheme::new("利用", "名詞,普通名詞,サ変可能,*,*,*,リヨウ,利用"),
    LatticeMorpheme::new("する", "動詞,非自立可能,*,*,サ行変格,連体形-一般,スル,為る"),
    LatticeMorpheme::new("こと", "名詞,普通名詞,一般,*,*,*,コト,事"),
    LatticeMorpheme::new("が", "助詞,格助詞,*,*,*,*,ガ,が"),
];
// The head word is 利用 and the function word が: する is 非自立可能,
// and こと is a formal noun after a verb.
assert_eq!(schema.positions(&bunsetsu), Some((0, 3)));
```

The additions were measured as those of IPADIC were. With UniDic CWJ, each line is the one before with one addition, and the last line is `Schema::unidic()`:

| Rule | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| The rule of CaboCha for UniDic | 82.50% | 82.81% | 83.96% | 81.75% |
| And 非自立可能 is a function word | 94.73% | 94.64% | 94.70% | 94.47% |
| And a verb after に, と or を is not a head | 96.71% | 96.53% | 96.43% | 96.73% |
| And 助動詞語幹 is not a head | 97.19% | 97.05% | 97.10% | 97.15% |
| And a formal noun after a predicate is not a head | 97.63% | 97.49% | 97.69% | 98.04% |
| And a nominal suffix after an adjective is not a head | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |

The same schema with each of the three dictionaries:

| Dictionary | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| `unidic` | 97.52% (55,320 / 56,724) | 97.60% (4,074 / 4,174) | 97.86% (4,440 / 4,537) | 97.84% (9,676 / 9,890) |
| `unidic-cwj` | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |
| `unidic-csj` | 97.55% (55,366 / 56,759) | 97.49% (4,071 / 4,176) | 97.91% (4,448 / 4,543) | 97.91% (9,682 / 9,889) |

- **非自立可能 is the largest step.** UniDic gives it to する, いる, ある and なる, which the pattern of CaboCha takes as head words: in 利用 / する the corpus has the noun.
- **Not taken.** A nominal suffix that is never a head lowers GSD train to 91.14% (51,736 / 56,763): the corpus has a suffix such as 長 or 内 as the head. So does one that is not a head after a number, 96.75% (54,917 / 56,763). UniDic tags a letter of the Latin alphabet 記号,文字; taking it out of the symbols gives 97.50% (55,346 / 56,763).
- **What a schema does not express.** A percent sign after a number is the content head in the corpus (3 / %, 65 bunsetsu of GSD train). UniDic tags it 補助記号,一般, as it does ・, and a schema takes a symbol by its part of speech or by its characters, not by both; so it stays a symbol, and the number is the head word. With IPADIC `%` is a noun and the head word.
- **What is left is mostly not a matter of the rule.** The content head of the corpus is taken at the morpheme that holds its first character. A word of the corpus that the dictionary splits (a number in digits, a name in katakana, k / m) therefore disagrees with a head word that is the last morpheme.

## Another dictionary

A schema is a value whose fields are public: the columns, `function_words`, `not_heads`, `symbols`, and the two lists the [parser](./parsing.md) reads, `particles` and `lexicalized`. A schema for another dictionary is built from data, without new code. The crate provides two, `Schema::ipadic()` and `Schema::unidic()`. `Schema::for_dictionary` gives the one of a dictionary of Lindera by its name, and `None` for a dictionary without one:

```rust
use lindera_dependency::Schema;

assert_eq!(Schema::for_dictionary("ipadic"), Some(Schema::ipadic()));
// The three UniDic dictionaries have one schema.
assert_eq!(Schema::for_dictionary("unidic-cwj"), Some(Schema::unidic()));
assert_eq!(Schema::for_dictionary("ko-dic"), None);
assert_eq!(
    Schema::DICTIONARIES,
    ["ipadic", "unidic", "unidic-cwj", "unidic-csj"]
);
```

A schema of your own is a value like these:

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
