# Reading and Using the Result

This page is for a reader who has run the [Quick Start](./quickstart.md) and wants to know what the result says: what a bunsetsu and a dependency are, how to read the result of several kinds of sentence, how to use it in a program, and how far to trust it.

Every output on this page was made with the model a release of Lindera builds for IPADIC and the IPADIC dictionary, the two that `lindera download dependency-ipadic` and `lindera download ipadic` install. In the commands, `$MODEL` and `$DICT` are the two paths those commands print, as in the Quick Start.

## Bunsetsu and dependencies

A Japanese sentence is a sequence of **bunsetsu** (文節): the smallest pieces a speaker can pause between. A bunsetsu is one content word, with the particles and auxiliaries that follow it:

```text
太郎は | 本を | 読んだ。
```

A tokenizer cuts this sentence into seven morphemes (太郎, は, 本, を, 読ん, だ, 。). Grouping them into three bunsetsu is the first half of the analysis, the **chunking**.

Each bunsetsu, except the last of the sentence, modifies one other bunsetsu: it **depends on** it. 太郎は says who read, and 本を what was read, so both depend on 読んだ。:

```text
太郎は ───┐
本を ─────┤
読んだ。 ◀─┘
```

Finding these **dependencies** (係り受け) is the second half, the **parsing**. The bunsetsu that another depends on is its **head**.

Two words of a bunsetsu have a name:

- the **head word** (主辞) is its content word: 太郎, 本, 読ん;
- the **function word** (機能語) is the last particle or auxiliary: は, を, だ. It says how the bunsetsu relates to its head: は marks a topic, を an object.

A bunsetsu without a particle or an auxiliary, such as 赤い, has one word that is both. [Head and Function Word](./head_and_function_word.md) has the rule.

### What the result gives, and what it does not

For every bunsetsu, the result gives the bunsetsu it depends on. It is a tree with three properties, which a program can rely on:

- every bunsetsu but the last depends on a **later** one, never on an earlier one;
- the **last** bunsetsu depends on nothing: it is the root, as a rule the main predicate of the sentence;
- no two dependencies cross.

The result does not say what kind of dependency one is. There is no label such as "subject" or "object". What a bunsetsu is to its head is read from its function word: が and は for who or what, を for an object, に and で for a place or a means, and so on. [Using the result](#using-the-result) shows it.

## Reading the result

The result comes in two forms, with the same content:

- the **lattice format** of `lindera dependency parse`, which is what `cabocha -f1` writes;
- **fields on the tokens** of the Lindera tokenizer, with the token filter `japanese_dependency`.

### The lattice format

```shell
% echo "太郎は昨日買った新しい本を図書館で読んだ。" \
    | lindera dependency parse --model "$MODEL" --dict "$DICT"
```

```text
* 0 6D 0/1 0.000000
太郎	名詞,固有名詞,人名,名,*,*,太郎,タロウ,タロー
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/0 0.865880
昨日	名詞,副詞可能,*,*,*,*,昨日,キノウ,キノー
* 2 4D 0/1 1.393004
買っ	動詞,自立,*,*,五段・ワ行促音便,連用タ接続,買う,カッ,カッ
た	助動詞,*,*,*,特殊・タ,基本形,た,タ,タ
* 3 4D 0/0 1.364588
新しい	形容詞,自立,*,*,形容詞・イ段,基本形,新しい,アタラシイ,アタラシイ
* 4 6D 0/1 0.000000
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 5 6D 0/1 0.000000
図書館	名詞,一般,*,*,*,*,図書館,トショカン,トショカン
で	助詞,格助詞,一般,*,*,*,で,デ,デ
* 6 -1D 0/1 0.000000
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ	助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

A line that starts with `*` opens a bunsetsu, and the lines under it are its morphemes. Take `* 2 4D 0/1 1.393004`:

| Part | Here | Meaning |
| --- | --- | --- |
| The number of the bunsetsu | `2` | Counted from 0 in the sentence: 買った |
| The number of its head | `4` | 買った depends on bunsetsu 4, 本を. `-1` for the last bunsetsu, which has no head |
| The letter | `D` | A dependency. The parser gives every dependency this letter; the other letters of the format (`P`, `A`, `I`) are kinds of dependency that only annotated data has |
| The two positions | `0/1` | The head word is morpheme 0 of the bunsetsu (買っ) and the function word morpheme 1 (た) |
| The score | `1.393004` | The score of the decision: see [The score](#the-score) |

Read bunsetsu by bunsetsu, the sentence is:

| | Bunsetsu | Depends on | |
| --- | --- | --- | --- |
| 0 | 太郎は | 6 読んだ。 | Who read |
| 1 | 昨日 | 2 買った | When it was bought |
| 2 | 買った | 4 本を | Which book: the one that was bought |
| 3 | 新しい | 4 本を | Which book: the new one |
| 4 | 本を | 6 読んだ。 | What was read |
| 5 | 図書館で | 6 読んだ。 | Where |
| 6 | 読んだ。 | | The root |

Two things to see here. A noun can have several modifiers, each with its own dependency: 買った and 新しい both depend on 本を. And a dependency can be long: 太郎は depends on the last bunsetsu, six away, over a clause that has nothing to do with it. 昨日 depends on 買った, not on 読んだ。: the analysis reads "the book bought yesterday", not "read yesterday".

### The fields on a token

With the token filter, each token carries the same information in its `dependency` ([Token Filter](./token_filter.md) has every way to use the filter):

```shell
% echo "太郎は花子が書いた手紙を読んだ。" | lindera tokenize --dict "$DICT" --output json \
    --token-filter "japanese_dependency:{\"model\":\"$MODEL\"}"
```

The surface of each token of the JSON output with the fields of its `dependency`:

| Token | `sentence` | `bunsetsu` | `head` | `role` | `score` |
| --- | --- | --- | --- | --- | --- |
| 太郎 | 0 | 0 | 4 | `head` | 0.0 |
| は | 0 | 0 | 4 | `function` | 0.0 |
| 花子 | 0 | 1 | 2 | `head` | 0.9120563246694273 |
| が | 0 | 1 | 2 | `function` | 0.9120563246694273 |
| 書い | 0 | 2 | 3 | `head` | 1.0015680641261748 |
| た | 0 | 2 | 3 | `function` | 1.0015680641261748 |
| 手紙 | 0 | 3 | 4 | `head` | 0.0 |
| を | 0 | 3 | 4 | `function` | 0.0 |
| 読ん | 0 | 4 | `null` | `head` | 0.0 |
| だ | 0 | 4 | `null` | `function` | 0.0 |
| 。 | 0 | 4 | `null` | `other` | 0.0 |

The tokens with the same `bunsetsu` are one bunsetsu, and all of them have the `head` of that bunsetsu. `role` says which token is the head word and which the function word. Read as bunsetsu:

| | Bunsetsu | Depends on | |
| --- | --- | --- | --- |
| 0 | 太郎は | 4 読んだ。 | Who read |
| 1 | 花子が | 2 書いた | Who wrote |
| 2 | 書いた | 3 手紙を | Which letter: the one that was written |
| 3 | 手紙を | 4 読んだ。 | What was read |
| 4 | 読んだ。 | | The root |

The sentence has a clause inside a clause, and the dependencies keep the two apart: 花子が depends on 書いた, the verb of the inner clause, and 太郎は on 読んだ。, the verb of the outer one.

### More kinds of sentence

The tables below are read from the output of `lindera dependency parse`, as the one above.

**A quotation**: 彼は明日来ると言った。

| | Bunsetsu | Depends on |
| --- | --- | --- |
| 0 | 彼は | 3 言った。 |
| 1 | 明日 | 2 来ると |
| 2 | 来ると | 3 言った。 |
| 3 | 言った。 | |

What was said is a clause that ends in と, and its last bunsetsu, 来ると, depends on the verb of saying. 明日 belongs to the quotation, and 彼は to 言った。.

**Two clauses in a row**: 猫が魚を食べ、犬が骨をかじった。

| | Bunsetsu | Depends on |
| --- | --- | --- |
| 0 | 猫が | 2 食べ、 |
| 1 | 魚を | 2 食べ、 |
| 2 | 食べ、 | 5 かじった。 |
| 3 | 犬が | 5 かじった。 |
| 4 | 骨を | 5 かじった。 |
| 5 | かじった。 | |

Each verb has its own subject and object. The first clause is attached to the second by its verb: 食べ、 depends on かじった。. Two things side by side are a dependency like any other, of the first on the second; nothing in the result marks them as parallel.

**A reason**: 雨が降ったので、試合は中止になった。

| | Bunsetsu | Depends on |
| --- | --- | --- |
| 0 | 雨が | 1 降ったので、 |
| 1 | 降ったので、 | 4 なった。 |
| 2 | 試合は | 4 なった。 |
| 3 | 中止に | 4 なった。 |
| 4 | なった。 | |

A clause that gives a reason, a time or a condition depends on the predicate of the main clause by its last bunsetsu, here 降ったので、.

## Using the result

The usual way to use the result is to turn it around: for a bunsetsu, collect the bunsetsu that depend on it. For a verb, that gives who did what, where and when; for a noun, what describes it.

This program puts the bunsetsu of a sentence together from its tokens, and prints, for the bunsetsu whose head word is a verb or an adjective and then for those whose head word is a noun, the bunsetsu that depend on each, with their function words:

```rust
use std::path::Path;

use lindera::analysis::token_filter::BoxTokenFilter;
use lindera::analysis::token_filter::japanese_dependency::JapaneseDependencyTokenFilter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::token::{BunsetsuRole, Token};

/// A bunsetsu, put together from the tokens that have its number.
#[derive(Default)]
struct Bunsetsu {
    /// The surfaces of its tokens, joined.
    text: String,
    /// The bunsetsu it depends on.
    head: Option<usize>,
    /// The part of speech of its head word.
    part_of_speech: String,
    /// Its function word, when it has one that is not its head word.
    function_word: Option<String>,
}

/// The bunsetsu of one sentence.
fn bunsetsu_of(tokens: &mut [Token]) -> Vec<Bunsetsu> {
    let mut bunsetsu: Vec<Bunsetsu> = Vec::new();
    for token in tokens {
        let Some(dependency) = token.dependency().cloned() else {
            continue;
        };
        let number = dependency.bunsetsu as usize;
        if bunsetsu.len() <= number {
            bunsetsu.resize_with(number + 1, Bunsetsu::default);
        }
        let surface = token.surface.to_string();
        let part_of_speech = token.get_detail(0).unwrap_or("*").to_string();
        let one = &mut bunsetsu[number];
        one.text.push_str(&surface);
        one.head = dependency.head.map(|head| head as usize);
        match dependency.role {
            BunsetsuRole::Head | BunsetsuRole::HeadAndFunction => {
                one.part_of_speech = part_of_speech;
            }
            BunsetsuRole::Function => one.function_word = Some(surface),
            BunsetsuRole::Other => {}
        }
    }
    bunsetsu
}

/// Prints each bunsetsu whose head word has one of the parts of speech,
/// with the bunsetsu that depend on it and their function words.
fn print_dependents(bunsetsu: &[Bunsetsu], parts_of_speech: &[&str]) {
    for (number, one) in bunsetsu.iter().enumerate() {
        if !parts_of_speech.contains(&one.part_of_speech.as_str()) {
            continue;
        }
        let dependents: Vec<String> = bunsetsu
            .iter()
            .filter(|other| other.head == Some(number))
            .map(|other| match &other.function_word {
                Some(word) => format!("{} ({word})", other.text),
                None => other.text.clone(),
            })
            .collect();
        if !dependents.is_empty() {
            println!("  {} <- {}", one.text, dependents.join(", "));
        }
    }
}

fn main() -> lindera::LinderaResult<()> {
    // The paths that `lindera download ipadic` and
    // `lindera download dependency-ipadic` print.
    let mut args = std::env::args().skip(1);
    let dictionary = args.next().expect("the directory of the dictionary");
    let model = args.next().expect("the model file");

    let dictionary = load_dictionary(&dictionary)?;
    let mut tokenizer = Tokenizer::new(Segmenter::new(Mode::Normal, dictionary, None));
    let filter = JapaneseDependencyTokenFilter::from_path(Path::new(&model))?;
    tokenizer.append_token_filter(BoxTokenFilter::from(filter));

    for sentence in [
        "太郎は昨日買った新しい本を図書館で読んだ。",
        "太郎は花子が書いた手紙を読んだ。",
    ] {
        let mut tokens = tokenizer.tokenize(sentence)?;
        let bunsetsu = bunsetsu_of(&mut tokens);
        println!("{sentence}");
        println!(" what depends on a verb or an adjective:");
        print_dependents(&bunsetsu, &["動詞", "形容詞"]);
        println!(" what depends on a noun:");
        print_dependents(&bunsetsu, &["名詞"]);
    }
    Ok(())
}
```

With `lindera = { version = "7.1", features = ["dependency"] }` and the two paths as its arguments, it prints:

```text
太郎は昨日買った新しい本を図書館で読んだ。
 what depends on a verb or an adjective:
  買った <- 昨日
  読んだ。 <- 太郎は (は), 本を (を), 図書館で (で)
 what depends on a noun:
  本を <- 買った (た), 新しい
太郎は花子が書いた手紙を読んだ。
 what depends on a verb or an adjective:
  書いた <- 花子が (が)
  読んだ。 <- 太郎は (は), 手紙を (を)
 what depends on a noun:
  手紙を <- 書いた (た)
```

- **Who did what.** The line of 読んだ。 is the event of the sentence: 太郎 with は, 本 with を, 図書館 with で. A program that wants subjects and objects takes the dependents of a verb and looks at their function words: が and は for the one who acts, を for what is acted on. In the second sentence the two verbs have their own lines, and 花子が is with 書いた alone.
- **What describes a noun.** The line of 本を has the clause 買った and the adjective 新しい. A clause that modifies a noun is attached by its last bunsetsu, so 買った stands for 昨日買った: to get the whole modifier, follow the dependents of 買った in turn.
- **One sentence at a time.** The numbers of the bunsetsu start again in each sentence. A text of several sentences gives tokens with another `sentence`, and the bunsetsu are put together for each: see [Sentences](./token_filter.md#sentences).

The same fields are on the tokens of the [bindings](./token_filter.md#from-the-bindings) and of the JSON output of `lindera tokenize`, and the program is the same in another language.

## How far to trust the result

The analysis is a statistical model, trained on UD Japanese-GSD, and it is wrong in some sentences.

**How often it is right.** On the test files, from plain text, the head of a bunsetsu is right for about 85 bunsetsu in 100 (85.39% on the dev file of UD Japanese-GSD, 84.47% on its test file, 86.19% on UD Japanese-PUD), not counting the last bunsetsu of a sentence, which has no head. A wrong head can come from a bunsetsu cut at the wrong place or from the parser: with the bunsetsu given, the parser alone is right for 90.11%, 88.53% and 88.81%. [Parsing](./parsing.md#accuracy) has the numbers, and those for UniDic.

**A wrong answer**: 私は友人と京都の古い寺を訪ねた。

| | Bunsetsu | Depends on | Score |
| --- | --- | --- | --- |
| 0 | 私は | 5 訪ねた。 | 0.000000 |
| 1 | 友人と | 2 京都の | 0.205908 |
| 2 | 京都の | 4 寺を | 1.087528 |
| 3 | 古い | 4 寺を | 1.380217 |
| 4 | 寺を | 5 訪ねた。 | 0.000000 |
| 5 | 訪ねた。 | | 0.000000 |

友人と, "with a friend", belongs to 訪ねた。. The model attaches it to 京都の, as if it were "a friend and Kyoto": と is also the particle that joins two nouns, and the noun right after it is the nearest candidate.

**A sentence with two readings**: 美しい水車小屋の娘が歌った。

| | Bunsetsu | Depends on | Score |
| --- | --- | --- | --- |
| 0 | 美しい | 1 水車小屋の | 0.566797 |
| 1 | 水車小屋の | 2 娘が | 1.707006 |
| 2 | 娘が | 3 歌った。 | 0.000000 |
| 3 | 歌った。 | | 0.000000 |

Is the mill beautiful, or the girl? Both are Japanese, and the result has one of them: the mill. A dependency analysis gives one tree for a sentence, and where the sentence itself allows two, it takes the one its training data makes more likely.

### The score

The score of a bunsetsu is the score of the decision that attached it to its head. The parser asks, for a bunsetsu and a candidate after it, whether the first depends on the second, and attaches it when the score is above 0 ([How a sentence is parsed](./parsing.md#how-a-sentence-is-parsed)).

- A score near 0 is a decision that was close. 友人と above has 0.205908, the lowest score above 0 on this page, and is the wrong one.
- A larger score is not a probability, and scores are not comparable between models.
- **The score is 0 where nothing was decided**: for a bunsetsu that depends on the last bunsetsu of its sentence, and for the last one. Those are the bunsetsu no earlier candidate took, so the last bunsetsu is what is left for them. A 0 does not say that the dependency is in doubt, and does not say that it is sure.
