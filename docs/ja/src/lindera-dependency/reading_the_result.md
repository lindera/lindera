# 結果の読み方と使い方

このページは、[クイックスタート](./quickstart.md)を動かしたあとで、結果が何を表しているのかを知りたい人のためのものです。文節と係り受けとは何か、いろいろな種類の文の結果をどう読むか、プログラムからどう使うか、どこまで信じてよいかを説明します。

このページの出力は、どれも、Lindera のリリースが作る IPADIC 用のモデルと、IPADIC の辞書で作りました。`lindera download dependency-ipadic` と `lindera download ipadic` が取得する 2 つです。コマンドの中の `$MODEL` と `$DICT` は、クイックスタートと同じく、この 2 つのコマンドが出力するパスです。

## 文節と係り受け

日本語の文は、**文節**の並びです。文節は、話すときに区切れる最小のまとまりで、1 つの内容語と、それに続く助詞・助動詞からなります。

```text
太郎は | 本を | 読んだ。
```

トークナイザーは、この文を 7 つの形態素（太郎, は, 本, を, 読ん, だ, 。）に切ります。これを 3 つの文節にまとめるのが、解析の前半の**文節の区切り**です。

文の最後の文節を除いて、どの文節も、ほかの 1 つの文節を修飾します。これを、その文節に**係る**と言います。太郎は は誰が読んだかを、本を は何を読んだかを表すので、どちらも 読んだ。 に係ります。

```text
太郎は ───┐
本を ─────┤
読んだ。 ◀─┘
```

この**係り受け**を求めるのが、解析の後半の**係り受けの解析**です。ある文節が係る相手の文節を、その**係り先**と呼びます。

文節の中の 2 つの語には、名前があります。

- **主辞**は、文節の内容語です。太郎, 本, 読ん が主辞です。
- **機能語**は、最後の助詞か助動詞です。は, を, だ が機能語です。機能語は、その文節が係り先とどういう関係にあるかを表します。は は主題を、を は対象を示します。

赤い のように、助詞も助動詞も無い文節では、1 つの語が両方を兼ねます。規則は[主辞と機能語](./head_and_function_word.md)にあります。

### 結果から分かること、分からないこと

結果は、すべての文節について、その係り先の文節を示します。全体は木になり、次の 3 つの性質があります。プログラムは、この性質を前提にできます。

- 最後の文節を除いて、どの文節も、自分より**後ろ**の文節に係ります。前の文節に係ることはありません。
- **最後**の文節は、どこにも係りません。これが木の根で、ふつうは文の主な述語です。
- 2 つの係り受けが交差することはありません。

結果は、係り受けの種類を示しません。「主語」「目的語」のようなラベルはありません。文節が係り先にとって何であるかは、機能語から読み取ります。が と は は誰が・何が、を は対象、に と で は場所や手段、といった具合です。[結果の使い方](#結果の使い方)に例があります。

## 結果の読み方

結果には 2 つの形があり、中身は同じです。

- `lindera dependency parse` の**ラティス形式**。`cabocha -f1` が書く形式です。
- Lindera のトークナイザーの**トークンの欄**。トークンフィルター `japanese_dependency` が書き込みます。

### ラティス形式

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

`*` で始まる行が文節の始まりで、その下の行が、その文節の形態素です。`* 2 4D 0/1 1.393004` を例に取ります。

| 部分 | この例 | 意味 |
| --- | --- | --- |
| 文節の番号 | `2` | 文の中で 0 から数えます。買った です |
| 係り先の番号 | `4` | 買った は、文節 4 の 本を に係ります。係り先の無い最後の文節は `-1` です |
| 文字 | `D` | 係り受けを表します。解析器は、どの係り受けにもこの文字を付けます。形式にあるほかの文字（`P`, `A`, `I`）は係り受けの種類で、正解を付けたデータにだけ現れます |
| 2 つの位置 | `0/1` | 主辞は文節の 0 番目の形態素（買っ）、機能語は 1 番目の形態素（た）です |
| 点数 | `1.393004` | 判定の点数です。[点数](#点数)を参照してください |

文節ごとに読むと、この文は次のようになります。

| | 文節 | 係り先 | |
| --- | --- | --- | --- |
| 0 | 太郎は | 6 読んだ。 | 誰が読んだか |
| 1 | 昨日 | 2 買った | いつ買ったか |
| 2 | 買った | 4 本を | どの本か（買った本） |
| 3 | 新しい | 4 本を | どの本か（新しい本） |
| 4 | 本を | 6 読んだ。 | 何を読んだか |
| 5 | 図書館で | 6 読んだ。 | どこで |
| 6 | 読んだ。 | | 根 |

ここで見ておくことが 2 つあります。1 つは、名詞に修飾語がいくつも付くとき、それぞれが別の係り受けになることです。買った と 新しい は、どちらも 本を に係ります。もう 1 つは、係り受けが遠くまで届くことです。太郎は は、関係の無い節をまたいで、6 つ先の最後の文節に係ります。昨日 は 読んだ。 ではなく 買った に係ります。解析は「昨日読んだ」ではなく「昨日買った本」と読んでいます。

### トークンの欄

トークンフィルターを使うと、同じ情報が、各トークンの `dependency` に入ります（フィルターの使い方は[トークンフィルター](./token_filter.md)にあります）。

```shell
% echo "太郎は花子が書いた手紙を読んだ。" | lindera tokenize --dict "$DICT" --output json \
    --token-filter "japanese_dependency:{\"model\":\"$MODEL\"}"
```

JSON の出力の各トークンの表層と、その `dependency` の欄は、次のとおりです。

| トークン | `sentence` | `bunsetsu` | `head` | `role` | `score` |
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

`bunsetsu` が同じトークンが 1 つの文節で、そのどれにも、その文節の `head` が入ります。`role` は、どのトークンが主辞で、どれが機能語かを表します。文節として読むと、次のようになります。

| | 文節 | 係り先 | |
| --- | --- | --- | --- |
| 0 | 太郎は | 4 読んだ。 | 誰が読んだか |
| 1 | 花子が | 2 書いた | 誰が書いたか |
| 2 | 書いた | 3 手紙を | どの手紙か（書いた手紙） |
| 3 | 手紙を | 4 読んだ。 | 何を読んだか |
| 4 | 読んだ。 | | 根 |

この文は、節の中に節があります。係り受けは、2 つの節を区別しています。花子が は内側の節の動詞 書いた に係り、太郎は は外側の節の動詞 読んだ。 に係ります。

### ほかの種類の文

以下の表は、上の表と同じく、`lindera dependency parse` の出力から読み取ったものです。

**引用**: 彼は明日来ると言った。

| | 文節 | 係り先 |
| --- | --- | --- |
| 0 | 彼は | 3 言った。 |
| 1 | 明日 | 2 来ると |
| 2 | 来ると | 3 言った。 |
| 3 | 言った。 | |

言った内容は と で終わる節で、その最後の文節 来ると が、言う動詞に係ります。明日 は引用の中に属し、彼は は 言った。 に属します。

**並んだ 2 つの節**: 猫が魚を食べ、犬が骨をかじった。

| | 文節 | 係り先 |
| --- | --- | --- |
| 0 | 猫が | 2 食べ、 |
| 1 | 魚を | 2 食べ、 |
| 2 | 食べ、 | 5 かじった。 |
| 3 | 犬が | 5 かじった。 |
| 4 | 骨を | 5 かじった。 |
| 5 | かじった。 | |

それぞれの動詞に、主語と目的語が付いています。前の節は、その動詞で後ろの節につながります。食べ、 が かじった。 に係ります。並んだ 2 つのものは、前が後ろに係る、ふつうの係り受けになります。結果の中に、並列を表す印はありません。

**理由**: 雨が降ったので、試合は中止になった。

| | 文節 | 係り先 |
| --- | --- | --- |
| 0 | 雨が | 1 降ったので、 |
| 1 | 降ったので、 | 4 なった。 |
| 2 | 試合は | 4 なった。 |
| 3 | 中止に | 4 なった。 |
| 4 | なった。 | |

理由・時・条件を表す節は、その最後の文節（ここでは 降ったので、）が、主節の述語に係ります。

## 結果の使い方

結果を使うときは、向きを逆にするのがふつうです。ある文節について、そこに係る文節を集めます。動詞なら、誰が・何を・どこで・いつが分かります。名詞なら、それを説明する語句が分かります。

次のプログラムは、トークンから文の文節を組み立て、主辞が動詞か形容詞の文節と、主辞が名詞の文節のそれぞれについて、そこに係る文節を、機能語と一緒に出力します。

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

`lindera = { version = "7.1", features = ["dependency"] }` を依存に入れ、2 つのパスを引数に渡すと、次のように出力します。

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

- **誰が何をしたか**。読んだ。 の行が、この文の出来事です。太郎 に は、本 に を、図書館 に で が付いています。主語や目的語が欲しいプログラムは、動詞に係る文節を取り、その機能語を見ます。が と は は動作の主、を は動作の対象です。2 つ目の文では、2 つの動詞がそれぞれの行を持ち、花子が は 書いた だけに付いています。
- **名詞を説明するもの**。本を の行には、節 買った と形容詞 新しい があります。名詞を修飾する節は、その最後の文節で名詞につながるので、買った は 昨日買った の代表です。修飾語の全体が欲しいときは、買った に係る文節を、さらにたどります。
- **1 文ずつ扱う**。文節の番号は、文ごとに 0 から始まります。複数の文からなるテキストでは、トークンの `sentence` が文ごとに変わるので、文節は文ごとに組み立てます。[文](./token_filter.md#文)を参照してください。

同じ欄が、[バインディング](./token_filter.md#バインディングから)のトークンと、`lindera tokenize` の JSON の出力にもあります。ほかの言語でも、プログラムは同じ形になります。

## 結果をどこまで信じてよいか

解析は、UD Japanese-GSD で学習した統計的なモデルによるもので、誤る文があります。

**どのくらい正しいか**。評価用のファイルで、テキストから解析すると、文節の係り先が正しいのは、100 文節のうち約 85 文節です（UD Japanese-GSD の dev で 85.39%、test で 84.47%、UD Japanese-PUD で 86.19%）。係り先の無い、文の最後の文節は数えていません。係り先の誤りは、文節の区切りの誤りからも、解析器からも生じます。正しい文節を与えると、解析器だけでは 90.11%、88.53%、88.81% が正しくなります。数字と、UniDic の数字は、[係り受けの解析](./parsing.md#精度)にあります。

**誤る例**: 私は友人と京都の古い寺を訪ねた。

| | 文節 | 係り先 | 点数 |
| --- | --- | --- | --- |
| 0 | 私は | 5 訪ねた。 | 0.000000 |
| 1 | 友人と | 2 京都の | 0.205908 |
| 2 | 京都の | 4 寺を | 1.087528 |
| 3 | 古い | 4 寺を | 1.380217 |
| 4 | 寺を | 5 訪ねた。 | 0.000000 |
| 5 | 訪ねた。 | | 0.000000 |

友人と は「友人と一緒に」の意味で、訪ねた。 に係るはずです。モデルは、「友人と京都」であるかのように、京都の に係らせています。と は 2 つの名詞を並べる助詞でもあり、すぐ後ろの名詞が、いちばん近い候補になるためです。

**2 通りに読める文**: 美しい水車小屋の娘が歌った。

| | 文節 | 係り先 | 点数 |
| --- | --- | --- | --- |
| 0 | 美しい | 1 水車小屋の | 0.566797 |
| 1 | 水車小屋の | 2 娘が | 1.707006 |
| 2 | 娘が | 3 歌った。 | 0.000000 |
| 3 | 歌った。 | | 0.000000 |

美しいのは水車小屋でしょうか、娘でしょうか。どちらも日本語として成り立ち、結果はその片方（水車小屋）になっています。係り受けの解析は、1 つの文に 1 つの木を返します。文そのものが 2 通りに読めるときは、学習データから見てありそうなほうを選びます。

### 点数

文節の点数は、その文節を係り先につないだ判定の点数です。解析器は、ある文節と、その後ろの候補について、前が後ろに係るかを判定し、点数が 0 より大きいときにつなぎます（[文を解析する手順](./parsing.md#文を解析する手順)）。

- 0 に近い点数は、きわどい判定です。上の 友人と は 0.205908 で、このページの 0 より大きい点数の中でいちばん低く、これが誤っている係り受けです。
- 点数は確率ではありません。違うモデルの点数どうしは、比べられません。
- **判定をしていないところの点数は 0 です**。文の最後の文節に係る文節と、最後の文節がそうです。それより前のどの候補にもつながらなかった文節で、残った係り先が最後の文節です。0 は、その係り受けが疑わしいという意味でも、確かだという意味でもありません。
