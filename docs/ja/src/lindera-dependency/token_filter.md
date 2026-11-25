# トークンフィルター

トークンフィルター `japanese_dependency` は、Lindera のトークナイザーの中で係り受けを解析します。テキストのトークンを文節にまとめ、各文節の係り先の文節を求めて、結果をすべてのトークンに書き込みます。`lindera` コマンド、設定ファイル、Lindera の `Tokenizer` を使うプログラムから呼ぶのは、このフィルターです。テキストを CaboCha のラティス形式に解析するには、[コマンドライン](./command_line.md)を見てください。

フィルターは、トークンを足しも消しも書き換えもしません。MeCab 形式と分かち書きの出力はフィルターなしのときと同じで、JSON の出力には、トークンごとにキーが 1 つ増えます。

## フィルターの入ったビルド

フィルターは `lindera-analysis` の `dependency` feature の裏にあり、`lindera` と `lindera-cli` がその feature を引き継ぎます。既定のビルドには、どれにも入っていません。

```shell
cargo build --release -p lindera-cli --features embed-ipadic,dependency
```

feature を付けていないビルドも、フィルターの名前は知っていて、何が足りないかを伝えます。

```text
the token filter japanese_dependency needs a build with the feature `dependency`
```

`lindera-dependency` は crates.io にまだ公開していないので、この feature は、リポジトリのチェックアウトでだけ使えます。バインディングには、まだフィルターが入っていません（[#1438](https://github.com/lindera/lindera/issues/1438)）。

## バインディングから

Python、Node.js、Ruby、PHP では、ほかのフィルターと同じく、`kind` とモデルのパスでフィルターを足します。トークンには `dependency` が入ります。下の 5 つのフィールドを持つオブジェクトで、フィルターを使わなければ何も入りません（`None`、`nil`、`null`、プロパティなし）。WebAssembly ではパスを読めないので、`appendDependencyFilter` にモデルのバイト列を渡します。どれも、バインディングを `dependency` feature を付けてビルドする必要があります。配布しているパッケージには、まだ入っていません。各バインディングの API のページに例があります: [Python](../lindera-python/tokenizer_api.md#tokendependency)、[Node.js](../lindera-nodejs/tokenizer_api.md#tokendependency)、[Ruby](../lindera-ruby/tokenizer_api.md#tokendependency)、[PHP](../lindera-php/tokenizer_api.md#tokendependency)、[WebAssembly](../lindera-wasm/tokenizer_api.md#tokendependency)。

## モデル

フィルターの引数は `model` の 1 つです。文節の区切りと解析器の両方を持ち、トークナイザーの辞書に合ったモデルファイルのパスを渡します。`embedded://ipadic` を渡すと、feature `embed-dependency-ipadic` を付けたビルドが持つモデルを使います。[IPADIC のモデル](./model_file.md#ipadic-のモデル)は、7.1.0 から、Lindera のリリースが公開します。学習の方法は[係り受けの解析](./parsing.md)に、ファイルの中身は[モデルファイル](./model_file.md)にあります。

モデルは、フィルターを作るときに 1 回だけ読み、トークナイザーの複製の間で共有します。読めないファイル、モデルファイルでないファイル、区切りか解析器の無いファイルは、テキストを読む前の、この時点でエラーになります。

## コマンドラインから

```shell
echo '彼は赤い花を見た。' | lindera tokenize --dict embedded://ipadic --output json \
    --token-filter 'japanese_dependency:{"model":"ipadic.model"}'
```

出力の各トークンに `dependency` が付きます。最初のトークンは、次のとおりです。

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

8 つのトークンすべてを、表層と `dependency` だけで並べると、次のようになります。

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

彼は → 見た。、赤い → 花を、花を → 見た。 という解析で、同じ文に `lindera-dependency parse` が書き出すものと同じです。

## 設定ファイルから

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

絶対パスでないパスは、設定ファイルのあるディレクトリからではなく、プログラムを動かしているディレクトリから読みます。

## Rust から

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

`JapaneseDependencyTokenFilter::from_bytes` は、モデルのバイト列からフィルターを作ります。プログラムに埋め込んだモデルや、ファイル以外から得たモデルに使います。どのフィルターも書き込んでいないトークンでは、`Token::dependency` は `None` です。

## トークンに書かれるもの

| フィールド | 内容 |
| --- | --- |
| `sentence` | テキストの中での文の番号。0 から |
| `bunsetsu` | 文の中での、そのトークンの文節の番号。0 から |
| `head` | その文節の係り先の文節の番号。同じ文の中の番号で、文の最後の文節では `null`（`None`） |
| `role` | 文節の中でのトークンの役割。`head`、`function`、`head_function`、`other` のどれか |
| `score` | その文節を係り先に付けた判定のスコア。判定の無いところは 0（最後の文節に係る文節など） |

`sentence` と `bunsetsu` が同じトークンが、1 つの文節です。`head` は文節の主辞、`function` は機能語です。`head_function` は両方を兼ねるトークンで、上の例の 赤い がそうです。`other` は、記号のように、どちらでもないものです。規則は[主辞と機能語](./head_and_function_word.md)にあります。

## 文

モデルは文を単位に学習しているので、フィルターはテキストを文に分け、文ごとに解析します。文節の番号は、文ごとに 0 から振ります。

- 文は、括弧の外にある `。`、`！`、`？`（と `!`、`?`）のトークンの後で終わります。その後に続く同じ種類のトークンと閉じ括弧は、同じ文に入ります。
- 全角のピリオド `．` は、`３．１４` のように直後が数字のときを除いて、文を終えます。
- 半角のピリオド `.` は、文を終えません。数（`4.20`）や `...`、名前の中に現れるためです。
- 最後の終わりの後に残ったものも、1 つの文です。
- 改行はトークンではないので、文を終えません。`lindera tokenize` は入力を 1 行ずつ読むので、そこでは改行で分かれます。

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

列は、表層、文、文節、係り先、役割です。括弧の中の句点と `！` では文が終わりません。2 つ目の文は 本当 から始まり、その文節の番号はまた 0 です。

閉じられていない括弧があると、テキストの残りは 1 つの文になります。

## フィルターを置く位置

このフィルターは、トークンフィルターの最初に置きます。モデルは、辞書が返したままのトークンを読みます。後ろのフィルターは、このフィルターが書き込んだトークンを受け取ります。上の設定では、`japanese_stop_tags` が助詞を消し、残ったトークンは `dependency` を持ったままです。

前のフィルターが消したりつないだりしたトークンは、断ります。

```text
japanese_dependency: token 1 ("赤い") is at position 2 with a length of 1: an earlier token filter removed or joined tokens; put this filter before it
```

これは、トークンの位置から見分けています。見分けられず、エラーなしに誤った解析になるものもあります。前のフィルターが書き換えた表層や素性（`japanese_base_form`、`lowercase`）と、テキストの末尾から消されたトークンです。文字フィルターは、辞書が読む前のテキストを変えます。これも、モデルが学習したテキストではなくなります。

## 辞書

モデルは、別の辞書のトークンに対して、失敗せずに誤った答えを返します。フィルターは、トークンの辞書の名前をモデルファイルの中の名前と比べ、違えば止まります。

```text
the model is for the dictionary "unidic", and the morphemes are of "ipadic"
```

モデルが読む数より素性の少ないトークンも、断ります。

## 速度

UD Japanese-GSD の test のテキスト（543 文、12,644 トークン）で、既定のモデルと埋め込んだ IPADIC を使うと、フィルターは `lindera tokenize` に 1 トークンあたり約 5.2 マイクロ秒を足します。フィルター付きで毎秒約 180,000 トークン（約 7,700 文）、フィルターなしでは毎秒約 2,900,000 トークンです。モデルを読むフィルターの作成には、約 4 ミリ秒かかります。数字はマシンによって変わります。
