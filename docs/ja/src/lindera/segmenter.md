# Segmenter

`Segmenter` は形態素解析を実行するコアコンポーネントです。辞書とコストモデルに基づいて、入力テキストの最適な分割を Viterbi アルゴリズムで探索します。

## Segmenter の作成

`Segmenter` には以下の3つのコンポーネントが必要です：

- **Mode** - トークナイズ戦略（`Normal` または `Decompose`）
- **Dictionary** - 形態素解析用のシステム辞書
- **UserDictionary**（オプション） - カスタム単語用の補助辞書

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("embedded://ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
```

## トークナイズモード

### Mode::Normal

辞書に登録されたエントリに基づく標準的なトークナイズです。辞書に登録された単語に忠実に分割します。

```rust
use lindera::mode::Mode;

let mode = Mode::Normal;
```

### Mode::Decompose

複合名詞を構成要素に分解します。このモードでは、長い複合語にペナルティを適用し、Segmenter がより短い構成要素に分割するよう促します。

例えば、「関西国際空港限定トートバッグ」という文中の複合語「関西国際空港」は、`Mode::Normal` では1つのトークンの一部のままですが、`Mode::Decompose` では「関西」「国際」「空港」に分割されます。

```rust
use lindera::mode::Mode;

let mode = Mode::Decompose(Default::default());
```

## 辞書の読み込み

Lindera は様々なソースから辞書を読み込むための `load_dictionary` 関数を提供しています。

### 埋め込み辞書

適切な Feature フラグ（例: `embed-ipadic`）を指定してビルドすると、バイナリから直接辞書を読み込むことができます：

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("embedded://ipadic")?;
```

利用可能な埋め込み辞書URI：

- `embedded://ipadic` - IPADIC（日本語）
- `embedded://ipadic-neologd` - IPADIC NEologd（日本語）
- `embedded://unidic` - UniDic（日本語）
- `embedded://ko-dic` - ko-dic（韓国語）
- `embedded://cc-cedict` - CC-CEDICT（中国語）
- `embedded://jieba` - Jieba（中国語）

### 外部辞書

ビルド済みの辞書ディレクトリをファイルシステムから読み込むことができます：

```rust
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("/path/to/dictionary")?;
```

## Tokenizer との連携

`Segmenter` は通常、Character Filter と Token Filter のサポートを追加する `Tokenizer` を通じて使用されます：

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

`token` を `mut` で束縛している点に注意してください。`Token::details` は `&mut self` を取るため、単純な `for token in tokens` ではコンパイルエラー（`E0596`: 可変として借用できません）になります。

## Config からの構築

`Segmenter::from_config` は、`Tokenizer`/`TokenizerBuilder` と同じ設定フォーマット（[設定](../lindera-analysis/configuration.md)を参照）のうち `segmenter:` セクション相当を受け取り、`SegmenterConfig`（`serde_json::Value`）から `Segmenter` を構築します：

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

注: ここでは `use_mmap` を明示的に示すためにあえて `false` を指定していますが、省略した場合も後述のデフォルト（`true`）と同じ挙動になります。

## メモリマップド読み込み

ファイルシステム辞書（`embedded://` ではない辞書）に対しては、`mmap` cargo
feature がコンパイルに含まれている場合（デフォルトで含まれます）、
`use_mmap` はデフォルトで `true` になり、メモリマップド読み込みが自動的に
使用されます。単純なファイル読み込み（非メモリマップド）を強制したい場合は
`false` を指定してください。
辞書フォーマットバージョン 2 以降、トライ・単語リストファイル・接続コスト
行列はいずれもマップしたバイト列上で直接参照されるため、大きなコンポーネント
はすべて遅延読み込みされ、ロード時に所有メモリへ全体展開されるものは
ありません。
`use_mmap` は `embedded://` 辞書に対しては無視されます（埋め込みデータは
既に静的なゼロコピーバイトスライスであるため）。`mmap` cargo feature
（デフォルトで有効）が必要です。

## 空白文字の扱い

デフォルトでは、MeCab と同様に空白をラティス上で読み飛ばします。空白文字から始まる語はなく、空白の後ろの語は空白の前の語に直接接続し、辞書が学習した連接コストが使われます。読み飛ばした空白は出力に現れず、トークンの表層形とオフセットにも含まれません。ただし、見出し語の一部である空白は、語の途中（IPADIC-NEologd の `GeForce GTX`、ユーザー辞書の `해운대 해수욕장`）でも末尾（ko-dic の、半角スペースの付いた `에듀`）でも、MeCab と同じくその語のトークンに残ります。「空白」とは辞書の `SPACE` 文字カテゴリ（`char.def`）のことで、ko-dic では U+0020、U+0009、U+000A、U+000B、U+000D、IPADIC・IPADIC-NEologd・UniDic・SudachiDict・CC-CEDICT・Jieba では U+0020、U+0009、U+000A、U+000B です。これらの `char.def` は U+00D0（`Ð`）も `SPACE` に割り当てています（U+000D のタイポ）が、後続の `0x00C0..0x00FF ALPHA` 行が優先されるため、MeCab と同じく `Ð` は英字として扱われます。

デフォルトは辞書の `metadata.json`（`skip_whitespace`）で決まります。SudachiDict はこれを `false` にしています。Sudachi は空白をラティスに残して `空白` トークンとして出力し、SudachiDict のコストもそれに合わせて調整されているため、読み飛ばすと、たとえば `caramel man` という項目が 2 つの普通名詞に分割されてしまいます。そのため SudachiDict では、デフォルトで空白が `SPACE` ノードとしてラティスに残ります（出力からは除外されます）。他の同梱辞書はこの設定を持たず、その場合は読み飛ばします。設定が導入される前にビルドされた辞書（再ビルドするまでの Lindera 6.x 製の SudachiDict を含む）も同様です。

`Segmenter` に対して `keep_whitespace(true)` を呼び出すと、空白のトークンを出力します。このとき空白は `SPACE` の未知語ノードとしてラティスに残ります：

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
```

v7 より前の Lindera は、空白を出力から除外する場合でもラティスにはノードとして残していました。MeCab で学習した辞書では `SPACE` の未知語との連接は学習時に一度も現れません。ko-dic では連接表のその行と列がすべて 0 なので、空白のたびに文脈が途切れ、空白に挟まれた語は単語コストだけで選ばれていました。ko-dic は `2년 전 대회` の `전` を名詞 `NNG` ではなく `저/NP + ㄴ/JX` と解析し、`SPACE` の項目が全角空白と文脈 ID を共有する IPADIC は `Google が 新しい` の `が` を接続詞と解析していました。CC-CEDICT と Jieba には連接コスト自体がないため、出力はこの違いに左右されません。`skip_whitespace` は辞書のデフォルトを上書きします。`skip_whitespace(false)` は従来どおり空白ノードをラティスに残し（出力からは除外）、`skip_whitespace(true)` は SudachiDict でも読み飛ばしを有効にします。`SegmenterConfig` の `skip_whitespace` キーには `true` または `false` を指定し、省略するか `null` にすると辞書のデフォルトのままになります。CLI では `--disable-skip-whitespace` で読み飛ばしを無効にします：

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
```

`\n` と `\t` は引き続き区間の終わりになるため（[文分割](#文分割) を参照）、改行やタブをまたいで文脈は引き継がれません。mecab-ko は 1 行ずつ解析し、行内のタブは他の空白と同様に読み飛ばします。

## 未知語のグルーピング

未知語のグルーピングはデフォルトでは無制限です。`max_grouping_len(Some(n))` で MeCab の `max-grouping-size` と同じ意味論（先頭を除いた文字数で数え、MeCab のデフォルトは 24）の上限を設定できます:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).max_grouping_len(Some(24));
```

上限はラン全体ではなく、ラティスの各位置で判定されます。ある位置から始まる同じ文字種のランが上限を超える場合、その位置ではグルーピング候補を出さず、1 文字候補（および後述の候補ラダーと辞書語）が残ります。残りの末尾は上限に収まった時点で再びグルーピングされます。そのため未知語トークンは最長で `n + 1` 文字になりますが、上限を超えるランが全体として 1 文字ずつになるわけではありません。IPADIC で辞書外のカタカナ列 `ヷヸヹヺヷ` を `max_grouping_len(Some(2))` で分割すると、候補ラダーを無効にした場合は `ヷ / ヸ / ヹヺヷ` になります。先頭 2 つの位置ではランの長さが 5 文字・4 文字で上限を超えるため 1 文字候補だけが残り、末尾 3 文字は上限に収まるのでグルーピングされます。候補ラダーが有効（デフォルト）の場合は先頭位置で 2 文字のラダー候補が選ばれ、`ヷヸ / ヹヺヷ` になります。

`Some(0)` は 2 文字以上のランを一切グルーピングしません。設定キー `max_grouping_len` と CLI の `--max-grouping-len` では `0` は無制限を意味します。

グルーピングとは独立に、Lindera はデフォルトで MeCab/Vibrato 由来の
「候補ラダー（length ladder）」も生成します。各カテゴリの `char.def` の
`LENGTH` フィールドまでの短い未知語候補を段階的に生成し、Viterbi 探索が
最もコストの低い長さを選べるようにします。v6 以前と同一の出力にするには
`unknown_word_ladder(false)` で無効化してください:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).unknown_word_ladder(false);
```

## 左側空白ペナルティ（韓国語）

MeCab ベースの韓国語解析器（mecab-ko + mecab-ko-dic、Lucene の nori）は、直前に空白がある候補の品詞が、本来は前の語に空白なしで付く品詞（助詞 `J*`、語尾 `E*`、指定詞 `VCP`、派生接尾辞 `XS*`）である場合にコストを加算します。これがないと `검색 이 잘 된다` の `이` は感動詞 `IC` ではなく主格助詞 `JKS` と解析されます。Lindera は、ルールを `metadata.json` に同梱する辞書（ko-dic）ではこのペナルティをデフォルトで適用します。他の辞書には影響しません。`Segmenter::space_penalty(None)`、設定の `"space_penalty": false`、CLI の `--disable-space-penalty` でオフにできます。`skip_whitespace(false)` と併用すると v6.0 の出力に戻ります。

`SpacePenaltyConfig` は「先頭品詞タグの一覧とコスト」の組（ルール）のリストです。候補は品詞タグの最初の `+` より前の部分（ko-dic の `Inflect` 行では `first_part_of_speech` 列）で照合され、最初に一致したルールが適用されます。一覧にないタグのコストは 0 です。mecab-ko-dic の `dicrc` のルールは次のように書けます:

```rust
use lindera::space_penalty::{SpacePenaltyConfig, SpacePenaltyRule};

let rules = SpacePenaltyConfig::new(vec![
    SpacePenaltyRule::new(["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], 3000),
    SpacePenaltyRule::new(["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], 6000),
]);
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty(Some(rules))?;
```

「空白」とは辞書の `SPACE` カテゴリ（`char.def`）に属する文字のことで、`keep_whitespace` が除外する文字集合と同じです。`SPACE` カテゴリを持たない辞書では Unicode の `White_Space` にフォールバックします。ペナルティは両モードと N-best 探索で適用され、システム辞書・ユーザー辞書・未知語のいずれのエントリにも効きます。`space_penalty` は単語 ID ごとの参照表を一度だけ構築し（ko-dic で数十ミリ秒）、辞書スキーマに `part_of_speech_tag` も `part_of_speech` もない場合はエラーを返します。

辞書は `metadata.json` の `space_penalty` にデフォルトのルールを同梱できます。ko-dic は上記とまったく同じルールを同梱しており、`Segmenter::new` がそれを自動的に適用します。`space_penalty_from_dictionary()` はオフにした後に再び有効化するためのもので、ルールを同梱しない辞書ではエラーを返します。同梱ルールを適用できない辞書（スキーマに品詞列が無い辞書）では、`Segmenter::new` は警告を出してオフのままにします:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).space_penalty_from_dictionary()?;
```

このルールを同梱しているのは、Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースからダウンロードした ko-dic にはルールがないため、その辞書ではペナルティは黙ってオフのままになり、`space_penalty_from_dictionary()`（および `"space_penalty": true`）は辞書を再ビルドするまでエラーを返します。明示的なルール（`space_penalty`、`--space-penalty-rules`）はどの ko-dic でも使えます。

`SegmenterConfig` の `space_penalty` キーには、明示的なルールのオブジェクト、オフにする `false`、辞書のルールを要求する `true`（同梱しない辞書ではエラー）を指定できます。キーを省略するか `null` にするとデフォルト、つまり辞書がルールを同梱していればそのルールが使われます:

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

ペナルティは候補の直前の文字で判定し、これは mecab-ko の判定と同じです。そのため、空白をラティス上で読み飛ばす場合（デフォルト。[空白文字の扱い](#空白文字の扱い) を参照）でも、ノードとして残す場合でも同じように適用されます。空白を読み飛ばすと、ペナルティの対象となる読みの一部は連接コストだけで排除されます（`서울 시 에서` の `시` はペナルティなしでも `NNG`）が、すべてではありません（`검색 이 잘 된다` の `이`）。

## N-Best セグメンテーション

`segment_nbest` は、コストの合計で並べた上位 `n` 件の分割結果を、それぞれのコストと共に返します。`unique` を指定すると、単語境界は同じで品詞タグのみ異なる結果を重複排除できます。`cost_threshold` を指定すると、`best_cost + threshold` を超えるコストのパスを除外できます：

```rust
let results = segmenter.segment_nbest(Cow::Borrowed("すもももももももものうち"), 3, false, None)?;
for (tokens, cost) in results {
    println!("cost={cost}");
    for token in tokens {
        println!("  {}", token.surface.as_ref());
    }
}
```

どの結果も入力全体の分割です。入力は[文分割](#文分割)のとおりに区間に分けられ、区間ごとに探索されます。区間の中では `、`・`。`・強制的な区切りをまたいで文脈を引き継ぐため、区間のパスは、区間全体を 1 つのラティスで解いたときのコストの小さい `n` 本のパスです（例外は[文分割](#文分割)を参照）。`unique` は区間全体の単語境界で比較します。各結果は区間ごとに 1 つのパスを選んだ組み合わせで、そのコストは選んだパスのコストの合計です。結果は、これらの組み合わせのうちコストの小さい `n` 件です。そのため、1 つの区間だけが最良の結果と異なる結果もあります。`best_cost` は 1 件目の結果のコストなので、`cost_threshold` は入力全体に対して適用されます。1 件目の結果は、コストがまったく同じパスが複数ある場合を除き、`segment` の出力と同じです。

`segment_nbest_with_lattice` は同じ処理を行いますが、呼び出しごとの `Lattice` バッファの再確保を避けるために、再利用可能な `Lattice` を自分で渡すことができます。

## 文分割

ラティスを構築する前に、Linderaは入力テキストを区切り文字（`\n`、`\t`、`。`、`、`）で文単位に分割し、文ごとに Viterbi ラティスを構築します。文の先頭からおよそ 32 KiB 以内に区切り文字が見つからない場合、Linderaはその位置で強制的に文の境界を区切り、警告をログに出力します。こうして区切ることで、各ラティスのサイズ（ひいてはメモリ・CPU コスト）を制限しています。

`\n`・`\t`・入力の終わりは、*区間*（segment）の終わりになります。区間とは、そこまでの文の並びのことで、区間どうしは互いに独立しています。区間の中では、`、`・`。` と強制的な区切りをまたいで文脈を引き継ぎます。次の文は文頭（BOS）からではなく前の文の最後の語から始まり（その右文脈 ID ごとに、そこまでの最良のパスのコストを引き継ぎます）、文末（EOS）への連接コストは区間の終わりでだけ払います。そのため、区間の最良のパスとそのコストは、区間全体を 1 つのラティスで解いた場合と同じになり、行の中で区切らない MeCab の結果と一致します。v7 より前は、どの文も BOS から始まり EOS で終わっていたため、`、` の直後の語は文脈を失っていました。たとえば IPADIC では、`彼は、ああ言った` の `ああ` が感動詞になっていましたが、現在は MeCab と同じく副詞になります。

次の場合は、区間全体を 1 つのラティスで解いた結果と異なることがあります。

- 区切りをまたぐ語はできません。UniDic の `一、二塁` のように `、` や `。` を含む見出し語はマッチせず、未知語も区切りをまたいでまとめられず、強制的な区切りをまたぐはずの語はそこで分かれます。
- 強制的な区切りの位置から始まる語には、区切りの直前が空白でも左側空白ペナルティ（[左側空白ペナルティ（韓国語）](#左側空白ペナルティ韓国語)を参照）がかかりません。

通常のテキストが強制的な区切りに達することはありませんが、区切り文字を含まない病的な入力（例えば minify されたテキストや base64 エンコードされたデータなど）に対しては、この人為的な分割位置でトークナイズ結果に影響が出ることがあります。

N-best の探索も同じ区間を使います。[N-Best セグメンテーション](#n-best-セグメンテーション)を参照してください。

## 再利用可能な Worker

`SegmentWorker` は、Viterbi ラティスと分割処理用のスクラッチバッファを所有する再利用可能なセグメンテーションセッションです。呼び出しのたびに `segment` が支払うアロケーションを回避できます。`new_worker`（セグメンターを clone）または `into_worker`（セグメンターを消費し、ユーザー辞書のコピーを回避）で作成し、`segment`/`segment_nbest` を呼び出します：

```rust
let mut worker = segmenter.new_worker();
for line in lines {
    let tokens = worker.segment(line)?;
    for token in &tokens {
        println!("{}", token.surface.as_ref());
    }
}
```

返されるトークンは worker を借用するため、次の呼び出しの前に消費する必要があります（上記のような行単位のループはそのままコンパイルできます）。`set_mode` と `set_keep_whitespace` で呼び出しごとに設定を切り替えられます。`segmenter()` は内部の Segmenter への共有参照を返します。`&mut` でのアクセサは意図的に提供されていません。再利用中のラティスの下で辞書を差し替えてしまうと、この worker が保証する辞書とラティスの対応関係が壊れてしまうためです。

Worker は保持メモリも制限します。区切り文字のない最大長の文を 1 回処理するとラティスは数 MB まで成長し（32 KiB の ASCII 文で約 18 MB、32 KiB の CJK 文では文字索引ラティスのスロット数が 1/3 のため約 6 MB）、素の `Lattice` はそれを保持し続けます。Worker は一定回数の呼び出し窓で容量が過大と判明した場合に、自動でラティスを縮小してスクラッチバッファを解放し、`shrink_to(text_len_hint)` で即時に縮小することもできます。`reset()` は内部バッファを破棄して新しいものに置き換えます。これは、例えばパニックによって worker を保持する Mutex がポイズニングされた場合など、バッファが不整合な中間状態を保持している可能性がある回復経路のために用意されています。Segmenter の設定（mode や空白の扱いなど）自体は保持されます。

Worker は作成元セグメンターの辞書に恒久的に紐付けられ、稼働中の worker の辞書を差し替える手段はありません。これにより、ラティス再利用に起因するバグの一群を構造的に排除しています。マルチスレッドで使う場合は、共有の `Segmenter` からスレッドごとに worker を 1 つ作成してください。
