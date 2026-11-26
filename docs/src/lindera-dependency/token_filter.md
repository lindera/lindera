# Token Filter

The token filter `japanese_dependency` runs the analysis inside a Lindera tokenizer: it groups the tokens of a text into bunsetsu, finds the bunsetsu each one depends on, and writes the result on every token. It is what the `lindera` command, a configuration file and a program that uses the `Tokenizer` of Lindera call. To parse text into the lattice format of CaboCha, see [Command Line](./command_line.md).

The filter adds, removes and rewrites no token. The MeCab and wakati outputs are the ones without it, and the JSON output has one key more for each token.

## A build with the filter

The filter is in `lindera-analysis`, behind the `dependency` feature, which `lindera`, `lindera-cli` and the bindings forward. The `lindera` command and the five bindings have the feature in their default builds, from 7.1.0 on, so the packages you install have the filter. The libraries `lindera` and `lindera-analysis` do not have it by default:

```toml
[dependencies]
lindera = { version = "7.1", features = ["dependency"] }
```

A build without the feature knows the name of the filter and says what is missing:

```text
the token filter japanese_dependency needs a build with the feature `dependency`
```

The filter needs a model, which is not in a package: see [The model](#the-model).

## From the bindings

Python, Node.js, Ruby and PHP append the filter as the others, by its `kind` and the path of the model, and a token has a `dependency`: an object with the five fields below, or nothing (`None`, `nil`, `null`, no property) without the filter. In WebAssembly a path cannot be read, and `appendDependencyFilter` takes the bytes of the model. The distributed packages have the filter, from 7.1.0 on. The API page of each binding has an example: [Python](../lindera-python/tokenizer_api.md#tokendependency), [Node.js](../lindera-nodejs/tokenizer_api.md#tokendependency), [Ruby](../lindera-ruby/tokenizer_api.md#tokendependency), [PHP](../lindera-php/tokenizer_api.md#tokendependency), [WebAssembly](../lindera-wasm/tokenizer_api.md#tokendependency).

## The model

The filter takes one argument, `model`: the path of a model file with a chunker and a parser, for the dictionary of the tokenizer; or `embedded://ipadic`, the model that a build with the feature `embed-dependency-ipadic` holds. A release of Lindera publishes the [model for IPADIC](./model_file.md#the-model-for-ipadic), from 7.1.0 on, and [`lindera download dependency-ipadic`](../lindera-cli/commands.md#downloading-the-model-of-the-dependency-parser) installs it and prints its path. The same goes for the three UniDic dictionaries, with `unidic`, `unidic-cwj` or `unidic-csj` in place of `ipadic` ([The models for UniDic](./model_file.md#the-models-for-unidic)). [Parsing](./parsing.md) shows how to train one, and [Model File](./model_file.md) what the file holds.

The model is read once, when the filter is made, and shared by the copies of the tokenizer. A file that cannot be read, that is not a model file, or that lacks the chunker or the parser is an error then, before any text is read.

## From the command line

```shell
echo '彼は赤い花を見た。' | lindera tokenize --dict embedded://ipadic --output json \
    --token-filter 'japanese_dependency:{"model":"ipadic.model"}'
```

Each token of the output has a `dependency`. The first one:

```json
{
  "base_form": "彼",
  "byte_end": 3,
  "byte_start": 0,
  "conjugation_form": "*",
  "conjugation_type": "*",
  "dependency": {
    "bunsetsu": 0,
    "head": 3,
    "role": "head",
    "score": 0.0,
    "sentence": 0
  },
  "part_of_speech": "名詞",
  "part_of_speech_subcategory_1": "代名詞",
  "part_of_speech_subcategory_2": "一般",
  "part_of_speech_subcategory_3": "*",
  "pronunciation": "カレ",
  "reading": "カレ",
  "surface": "彼",
  "word_id": 211398
}
```

And all eight, as the surface and the `dependency` of each:

```text
彼    {"bunsetsu": 0, "head": 3, "role": "head", "score": 0.0, "sentence": 0}
は    {"bunsetsu": 0, "head": 3, "role": "function", "score": 0.0, "sentence": 0}
赤い  {"bunsetsu": 1, "head": 2, "role": "head_function", "score": 0.7046164205315681, "sentence": 0}
花    {"bunsetsu": 2, "head": 3, "role": "head", "score": 0.0, "sentence": 0}
を    {"bunsetsu": 2, "head": 3, "role": "function", "score": 0.0, "sentence": 0}
見    {"bunsetsu": 3, "head": null, "role": "head", "score": 0.0, "sentence": 0}
た    {"bunsetsu": 3, "head": null, "role": "function", "score": 0.0, "sentence": 0}
。    {"bunsetsu": 3, "head": null, "role": "other", "score": 0.0, "sentence": 0}
```

That is 彼は → 見た。, 赤い → 花を and 花を → 見た。, the analysis `lindera-dependency parse` writes for the same sentence.

## From a configuration file

```yaml
segmenter:
  mode: "normal"
  dictionary: "embedded://ipadic"

token_filters:
  - kind: "japanese_dependency"
    args:
      model: "ipadic.model"
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "助詞"
        - "助動詞"
        - "記号"
```

A path that is not absolute is read from the directory the program runs in, not from the directory of the configuration file.

## From Rust

```rust
use std::path::Path;

use lindera::analysis::token_filter::BoxTokenFilter;
use lindera::analysis::token_filter::japanese_dependency::JapaneseDependencyTokenFilter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

fn main() -> lindera::LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let mut tokenizer = Tokenizer::new(Segmenter::new(Mode::Normal, dictionary, None));
    let filter = JapaneseDependencyTokenFilter::from_path(Path::new("ipadic.model"))?;
    tokenizer.append_token_filter(BoxTokenFilter::from(filter));

    for token in tokenizer.tokenize("彼は赤い花を見た。")? {
        if let Some(dependency) = token.dependency() {
            println!(
                "{}\t{}\t{:?}\t{}",
                token.surface,
                dependency.bunsetsu,
                dependency.head,
                dependency.role.as_str()
            );
        }
    }
    Ok(())
}
```

```text
彼      0       Some(3) head
は      0       Some(3) function
赤い    1       Some(2) head_function
花      2       Some(3) head
を      2       Some(3) function
見      3       None    head
た      3       None    function
。      3       None    other
```

`JapaneseDependencyTokenFilter::from_bytes` makes the filter from the bytes of a model, for a model that is embedded in the program or that comes from somewhere else than a file. `Token::dependency` is `None` for a token no filter wrote on.

## What is written on a token

| Field | What it is |
| --- | --- |
| `sentence` | The number of the sentence in the text, from 0 |
| `bunsetsu` | The number of the bunsetsu of the token in its sentence, from 0 |
| `head` | The number of the bunsetsu that one depends on, in the same sentence; `null` (`None`) for the last bunsetsu of a sentence |
| `role` | What the token is in its bunsetsu: `head`, `function`, `head_function` or `other` |
| `score` | The score of the decision that attached the bunsetsu to its head; 0 where nothing was decided, as for a bunsetsu that depends on the last one |

The tokens with the same `sentence` and `bunsetsu` are one bunsetsu. `head` is the head word of the bunsetsu and `function` its function word; `head_function` is a token that is both, as 赤い above, and `other` is what is neither, as a symbol. [Head and Function Word](./head_and_function_word.md) has the rule.

## Sentences

A model is trained on sentences, so the filter cuts the text into sentences and analyzes each on its own. The bunsetsu of each sentence are numbered from 0.

- A sentence ends after a token of `。`, `！` or `？` (and `!`, `?`), when it is not inside brackets. More of them, and closing brackets, that follow go to the same sentence.
- The full-width period `．` ends a sentence unless a digit follows it, as in `３．１４`.
- The half-width period `.` ends none. It stands in numbers (`4.20`), in `...` and in names.
- What is left after the last end is a sentence.
- A line break is not a token and ends no sentence. `lindera tokenize` reads its input line by line, so there a line break does separate.

```text
彼は「本を読んだ。面白かった！」と言った。本当？
```

```text
彼        0  0  4     head
は        0  0  4     function
「        0  1  2     other
本        0  1  2     head
を        0  1  2     function
読ん      0  2  3     head
だ        0  2  3     function
。        0  2  3     other
面白かっ  0  3  4     head
た        0  3  4     other
！        0  3  4     other
」        0  3  4     other
と        0  3  4     function
言っ      0  4  None  head
た        0  4  None  function
。        0  4  None  other
本当      1  0  None  head_function
？        1  0  None  other
```

The columns are the surface, the sentence, the bunsetsu, the head and the role. The period and the `！` inside the brackets end nothing; the second sentence starts at 本当, and its bunsetsu is number 0 again.

A bracket that is never closed keeps the rest of the text in one sentence.

## Where the filter goes

Put the filter first among the token filters. The model reads the tokens as the dictionary gave them, and the filters after it see the tokens it wrote on: in the configuration above, `japanese_stop_tags` removes the particles, and the tokens that are left keep their `dependency`.

Tokens that an earlier filter removed or joined are refused:

```text
japanese_dependency: token 1 ("赤い") is at position 2 with a length of 1: an earlier token filter removed or joined tokens; put this filter before it
```

This is seen from the positions of the tokens. What is not seen, and gives a wrong analysis without an error: a surface or a feature that an earlier filter rewrote (`japanese_base_form`, `lowercase`), and tokens removed from the end of the text. A character filter changes the text before the dictionary reads it, which the model was not trained on either.

## The dictionary

A model gives wrong answers on the tokens of another dictionary without failing. The filter compares the name of the dictionary of the tokens with the name in the model file, and stops when they differ:

```text
the model is for the dictionary "unidic", and the morphemes are of "ipadic"
```

A token with fewer features than the model reads is refused as well.

## Speed

On the text of the test file of UD Japanese-GSD (543 sentences, 12,644 tokens), with the default model and the embedded IPADIC, the filter adds about 5.2 microseconds per token to `lindera tokenize`: about 180,000 tokens per second with it, which is about 7,700 sentences, against about 2,900,000 tokens per second without. Making the filter, which reads the model, takes about 4 ms. The numbers depend on the machine.
