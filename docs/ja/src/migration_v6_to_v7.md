# v6 から v7 への移行

Lindera v7.0.0 では `lindera` クレートがファサード（facade）になりました。
形態素セグメンター（`lindera-segmenter` として公開）を再エクスポートし、
新しくデフォルト有効になった `analysis` feature を通じて `lindera-analysis` の
分析チェーンを `lindera::analysis` として提供します。依存 1 行で
`Segmenter`・`Tokenizer`・すべてのフィルタが使えます。あわせて
`lindera-binding-core` を `lindera-binding` に改名し、v5.0.0 で予告していた
`LINDERA_DICTIONARIES_PATH` のフォールバックを削除し、IPADIC のスキーマで
`conjugation_type` と `conjugation_form` の名前が逆になっていた誤りを
修正し、表層形が空白だけの見出し語や先頭・末尾が空白の見出し語を保持し、
MeCab と同様に空白をラティス上で読み飛ばし、`char.def` の重複した行を MeCab と
同じく解決し、MeCab と同じく行の中では `、`・`。` をまたいで文脈を引き継ぎ、
N-best の各結果が入力全体を覆うようにし、MeCab と同じくどの位置からも未知語が
始まるようにし、`lindera-dictionary` のラティスのバックトレースが各トークンの
終了オフセットも返すようにしました。
このガイドでは、すべての破壊的変更とその対処方法を説明します。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| `lindera` がファサードになり、`lindera::analysis` が `lindera-analysis` を再エクスポート（`analysis` feature、デフォルト有効） | `lindera` と `lindera-analysis` の両方に依存する Rust ユーザー | 両方を同時に `"7"` に上げるか、`lindera-analysis` を外して `lindera::analysis::…` から import する |
| `default-features = false` は「セグメンターのみ」の意味になり、`analysis`（と従来どおり `mmap`）が無効になる | `default-features = false` で `lindera::analysis` を使いたい Rust ユーザー | `features` に `"analysis"` を追加する（メモリマップ読み込みを維持するなら `"mmap"` も列挙したままにする） |
| 従来の `lindera` クレートは `lindera-segmenter` として公開 | 直接依存する人はいない | 対応不要 — `lindera = "7"` が再エクスポートする |
| 生の `lindera-dictionary` モジュールを `lindera::dictionary` 配下から参照できる | `lindera-dictionary` を直接使うユーザー | 対応不要 — 任意で直接依存を外し、`lindera::dictionary::core::…`・`::builder`・`::viterbi` などを使う |
| **暗黙の feature `lindera-ipadic`・`lindera-ipadic-neologd`・`lindera-unidic`・`lindera-sudachidict`・`lindera-ko-dic`・`lindera-cc-cedict`・`lindera-jieba` の削除** | `features = [...]` に辞書クレート名を書いて有効化していたユーザー | 代わりに `embed-*` feature を使う |
| **`lindera-binding-core` を `lindera-binding` に改名** | バインディング用ヘルパークレートを使う Rust ユーザー | `lindera-binding` に依存し、`lindera_binding_core::` を `lindera_binding::` に置き換える |
| **`LINDERA_DICTIONARIES_PATH` の削除** | 非推奨のビルドキャッシュ変数をまだ設定しているユーザー | `LINDERA_BUILD_DICTIONARY_CACHE_DIR` を設定する。旧名は無視される |
| **IPADIC・IPADIC-NEologd の `conjugation_type` と `conjugation_form` が正しい列を指すように修正** | IPADIC・IPADIC-NEologd でこの 2 フィールドを名前で読むユーザー（`Token::get`・`Token::as_value`・`lindera tokenize -o json`・バインディングのスキーマ） | 2 つの値が入れ替わることを前提にする。v6.2.0 以前に作った辞書は再ビルドまたは再ダウンロードする |
| **空白を含む見出し語を保持** | U+3000 を含むテキスト（IPADIC・IPADIC-NEologd・UniDic）、空白を含むテキスト（SudachiDict）、末尾が空白の一部の語 | U+3000 が `記号,空白`・`空白` になり、SudachiDict の分割が Sudachi にずっと近くなることを前提にする。v6.2.0 以前に作った辞書は再ビルドまたは再ダウンロードする |
| **空白をラティス上で読み飛ばし、MeCab と同様に空白の前後の語を直接接続する** | `keep_whitespace` が false（デフォルト）で空白を含むテキストを分割するユーザー。ko-dic で特に顕著 | そのようなテキストは MeCab と同じ分割になることを前提にする。`skip_whitespace(false)`（`"skip_whitespace": false`、`--disable-skip-whitespace`）で空白の扱いを v6 に戻せる（このガイドの他の出力の変化は戻らない） |
| **`char.def` の重複した行は最後の行が決める** | `Ð`（U+00D0、ko-dic 以外のすべての辞書）・`々`（U+3005）・`〇`（U+3007）を含むテキスト、単一コードポイントの行や重複した行を持つ `char.def` で `lindera train` を使うユーザー | `Ð` が文字として出力に残ることを前提にする。v6.2.0 以前に作った辞書は再ビルドまたは再ダウンロードする |
| **MeCab と同じく、行の中では `、`・`。` をまたいで文脈を引き継ぐ** | 行の中に `、` や `。` を含む日本語のテキストを分割するユーザー | `、` や `。` の直後の語の解析が一部変わり（多くは MeCab と同じ解析になる）、そうした行の N-best のコストが変わることを前提にする。v6 の動作に戻す設定はない |
| **N-best: 各結果が入力全体を覆う** | 複数の文を含む入力で N-best（`segment_nbest`・`tokenize_nbest`・`lindera tokenize -N`・バインディングの N-best メソッド）を使うユーザー | 2 件目以降が入力全体のコストの小さい分割になることを前提にする。コストの閾値は入力全体に対して適用される |
| **MeCab と同じく、どの位置からも未知語が始まる** | カタカナ・記号・英字が続くテキストを通常モードで分割するユーザー。特に `・` でつないだカタカナ、`."` のような韓国語の文末の句読点、CC-CEDICT の英単語 | そうした連続が MeCab と同じ位置で分かれること、カタカナや英字の多いテキストで命令数が 13% 程度増えることを前提にする。v6 の動作に戻す設定はない |
| **`Lattice::tokens_offset`・`tokens_offset_into`・`nbest_tokens_offset`・`NBestGenerator::next` が `(start, end, WordId)` を返す** | これらの `lindera_dictionary` の関数を直接呼び出す Rust コード（`lindera::dictionary::viterbi::…`・`lindera::dictionary::Lattice`・`lindera::dictionary::nbest::…` 経由を含む） | 3 つの要素に分解し、次のトークンの開始位置の代わりに返された終了位置を使う |
| **`Lattice::tokens_offset_into` が最良パスの BOS の添字（`Option<usize>`）を返す** | `tokens_offset_into` の戻り値 `()` を値として使う Rust コード | 文として呼び出しているなら対応不要。それ以外は新しい戻り値を無視する |

言語バインディング（Python・Node.js・Ruby・PHP・WASM）と CLI の API・パッケージ名は
変わらず、バージョン番号だけが 7.0.0 になります。出力の違いは 7 つあり、いずれも
後述します。IPADIC・IPADIC-NEologd では、`conjugation_type` と
`conjugation_form` という名前で返る値が入れ替わります。表層形が空白だけの
見出し語や先頭・末尾が空白の見出し語が辞書に入るため、そうした空白を含む
テキストの分割が変わります。空白を含むテキストは MeCab と同じように分割されます。
`Ð`・`々`・`〇` が MeCab と同じ文字カテゴリになるため、これらを含むテキストの分割が
変わります。行の中では `、`・`。` の直後の語が前の語の文脈を引き継いで解析されるため、
そうしたテキストの一部で分割が変わり、`、`・`。` を含む行の N-best のコストは
すべて変わります。複数の文を含む入力では、N-best の 2 件目以降が入力全体で
コストの小さい分割になります。さらに通常モードでは、同じ文字種が続く範囲の内側からも
MeCab と同じく未知語が始まるため、カタカナ・記号・英字の連続の一部が分かれます。
それ以外は、同じ入力と辞書に対して、v7.0.0 は v6.2.0 と同じトークンを、同じ位置
ベースの詳細情報（details）とともに出力します。

## `lindera` クレートはファサードに

v6 までの Rust API は、`lindera`（セグメンターと辞書読み込み API）と
`lindera-analysis`（character filter・token filter・`Tokenizer`）の 2 クレートに
分かれており、それぞれに依存する必要がありました。v7 では `lindera` クレートが
両者を束ねる薄いファサードになります:

```text
lindera ──┬─▶ lindera-segmenter ──┬─▶ lindera-dictionary
          │                       ├─▶ 辞書クレート          [features: embed-*]
          │                       └─▶ lindera-trainer     [feature: train]
          └─▶ lindera-analysis    [feature: analysis、デフォルト有効] ──▶ lindera-segmenter
```

公開ツリーは v6 のすべての `lindera::…` パスを維持し、`lindera::analysis` を
追加します:

```text
lindera
├── LinderaResult, get_version()
├── dictionary, error, mode, segmenter, space_penalty, token, worker   （lindera-segmenter から。v6 と同じ）
└── analysis   [feature: analysis、デフォルト有効]   （lindera-analysis: character_filter, token_filter, tokenizer, worker）
```

### 依存は 1 つに

```toml
# v6
[dependencies]
lindera = "6"
lindera-analysis = "6"

# v7
[dependencies]
lindera = "7"
```

```rust
// v6
use lindera_analysis::tokenizer::Tokenizer;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;

// v7
use lindera::analysis::tokenizer::Tokenizer;
use lindera::analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
```

`lindera-analysis = "7"` への直接依存を残しても動作します。`lindera::analysis` は
同じクレートなので、`lindera_analysis::tokenizer::Tokenizer` と
`lindera::analysis::tokenizer::Tokenizer` は同一の型であり、既存の `use` パスを
変える必要はありません。

### `lindera` と `lindera-analysis` は同時に上げる

両方の依存を残す場合は、同じタイミングで両方を上げてください。
`lindera-analysis` 6.x は `lindera` 6.x（旧セグメンター）に依存するため、
`lindera` の行だけを `"7"` にして `lindera-analysis = "6"` を残すと、
セグメンターが 2 バージョン同時にビルドされます:

```toml
# Segmenter 型が 2 つになる: lindera 7（lindera-segmenter 経由）と lindera 6（lindera-analysis 6 経由）
[dependencies]
lindera = "7"
lindera-analysis = "6"
```

この状態では `lindera::segmenter::Segmenter` は v7 の型、
`lindera_analysis::tokenizer::Tokenizer::new` が受け取るのは v6 の型となり、
型不一致でコンパイルに失敗します。両方の行を同時に `"7"` にするか、
前述のとおり `lindera-analysis` を外してください。

### `default-features = false` は「セグメンターのみ」の意味に

ファサードのデフォルト feature は `mmap` と `analysis` です。
`default-features = false` にすると純粋なセグメンターになり、両方が無効に
なります。`lindera::analysis` は存在せず、ファイルシステム辞書のメモリマップ
読み込みもデフォルトではなくなります（v6 でも `default-features = false` は
`mmap` を無効にしていました）。必要なものを個別に有効にしてください:

```toml
# メモリマップ読み込み付きのセグメンター（v6 のデフォルトビルドと同じ）
lindera = { version = "7", default-features = false, features = ["mmap"] }

# セグメンターと分析チェーン。メモリマップ読み込みはデフォルトにしない
lindera = { version = "7", default-features = false, features = ["analysis"] }
```

`memmap2` 自体は `lindera-dictionary` のデフォルト feature を通じてどちらの場合も
リンクされます。ファサードの `mmap` feature は `load_dictionary` などの
`use_mmap` のデフォルト値を決めるだけで、
`load_dictionary_with_options(uri, true)` は feature なしでも動作します。

### セグメンターは `lindera-segmenter` として公開

v6 までの `lindera` クレートは、同じモジュール（`dictionary`・`error`・`mode`・
`segmenter`・`space_penalty`・`token`・`worker`）と同じ API を持つ
`lindera-segmenter` 7.0.0 として続いています。ファサードはそのすべてを
ルートで再エクスポートするため、v6 でコンパイルできた `lindera::…` パスは
v7 でもそのままコンパイルできます。このクレートは、`lindera-analysis` が
ファサードに依存せずにセグメンターを利用できるようにするために存在します。
`lindera-segmenter` に直接依存する必要はなく、アプリケーションコードは
引き続き `lindera` に依存してください。

## `lindera::dictionary` から生の辞書クレートを参照できる

`lindera-dictionary` の構成部品（辞書のデータ構造・辞書ビルダー・Viterbi
ラティスなど）を使うコードは、従来 `lindera-dictionary` への直接依存が
必要でした。v7 ではこれらのモジュールが、高レベルの読み込み API と並んで
`lindera::dictionary` の中に再エクスポートされます:

```text
lindera::dictionary
├── Dictionary, Metadata, Schema, load_dictionary, load_user_dictionary, DictionaryKind, …   （変更なし）
├── core          lindera_dictionary::dictionary（prefix_dictionary, character_definition, connection_cost_matrix, unknown_dictionary, …）
├── builder, error, loader, mode, nbest, space_penalty, util, viterbi
├── embedded_dictionary!, include_bytes_aligned!
└── trainer       lindera_trainer（`train` feature。変更なし）
```

既存の `lindera::dictionary::*` パスはすべて有効なままで、再エクスポートは同じ
アイテムを指します。`lindera::dictionary::Dictionary` と
`lindera::dictionary::core::Dictionary` は同一の型です。`lindera-dictionary` への
直接依存も引き続き動作しますが、必須ではなくなりました:

```rust
// v6: Cargo.toml に lindera-dictionary が必要
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::viterbi::Lattice;

// v7: ファサード経由で参照できる
use lindera::dictionary::core::prefix_dictionary::PrefixDictionary;
use lindera::dictionary::viterbi::Lattice;
```

> [!NOTE]
> `lindera::dictionary::core` は Rust の `core` クレートと同じ名前です。
> `use lindera::dictionary::*;` のような glob import はこのモジュールをスコープに
> 持ち込み、そのモジュール内で `core` クレートを隠してしまうため、
> `core::fmt::Debug` のようなパスが標準ライブラリを指さなくなります。
> 必要なアイテムを名前で個別に import してください。

## 暗黙の辞書 feature の削除

v6 では `lindera` の optional な辞書依存が Cargo の feature としても機能しており、
`features = ["lindera-ipadic"]` という指定が受け付けられていました（辞書を
埋め込まずにクレートだけを取り込む動作でした）。ファサードは辞書クレートに
依存しないため、次の 7 つの暗黙の feature は存在しなくなります:
`lindera-ipadic`・`lindera-ipadic-neologd`・`lindera-unidic`・
`lindera-sudachidict`・`lindera-ko-dic`・`lindera-cc-cedict`・`lindera-jieba`。
これらを指定したマニフェストは Cargo がエラーにします。辞書の埋め込みには、
従来から正式な方法である `embed-*` feature を使ってください:

```toml
# v6
[dependencies]
lindera = { version = "6", features = ["lindera-ipadic"] }

# v7
[dependencies]
lindera = { version = "7", features = ["embed-ipadic"] }
```

## `lindera-binding-core` は `lindera-binding` に改名

共通ヘルパークレートの上に独自の言語バインディングを構築している場合のみ
関係します。`lindera-binding` 7.0.0 は `lindera` と `lindera-analysis` の代わりに
`lindera` ファサード（`analysis` feature 付き）に依存します。クレート自身の
API（`CoreTokenizerBuilder`・`CoreTokenizer`・`TokenView`、および argument・
metadata・schema の各ヘルパー）は変わりません。公開済みの `lindera-binding-core`
（4.0.0〜6.2.0）は crates.io にそのまま残り、yank もされませんが、
以降のバージョンは公開されません。

```toml
# v6
[dependencies]
lindera-binding-core = "6"

# v7
[dependencies]
lindera-binding = "7"
```

```rust
// v6
use lindera_binding_core::tokenizer::CoreTokenizer;

// v7
use lindera_binding::tokenizer::CoreTokenizer;
```

## `LINDERA_DICTIONARIES_PATH` の削除

v5.0.0 でビルド時の辞書キャッシュ変数は `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に
改名され、旧名 `LINDERA_DICTIONARIES_PATH` はビルド警告付きの非推奨
フォールバックとして残されていました（[v4 から v5 への移行](./migration_v4_to_v5.md)
を参照）。v7.0.0 ではこのフォールバックを削除します。旧名だけを設定した
ビルドはキャッシュを使わず、警告も出しません。辞書クレートはキャッシュ
未設定の場合と同じように辞書をダウンロード・ビルドします。設定している
すべての場所（シェルのプロファイル・CI 設定・コンテナイメージ）で変数名を
変更してください:

```shell
# v6
export LINDERA_DICTIONARIES_PATH=/path/to/cache

# v7
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
```

## IPADIC の活用フィールド名の修正

IPADIC は 8 列目に活用型（例: `五段・カ行イ音便`）、9 列目に活用形（例:
`連用タ接続`）を格納しています。v1.0.0 から v6.2.0 までの `lindera-ipadic` と
`lindera-ipadic-neologd` のスキーマは、この 2 列の名前を逆に付けていたため、
名前で引くと常にもう一方の値が返っていました。v7.0.0 で名前を修正しました。
UniDic と SudachiDict は元から正しい順序です。

`書いた` の `書い` の場合:

| フィールド名 | v6 | v7 |
| --- | --- | --- |
| `conjugation_type` | `連用タ接続` | `五段・カ行イ音便` |
| `conjugation_form` | `五段・カ行イ音便` | `連用タ接続` |

変わるのは名前によるアクセスだけです。`Token::get("conjugation_type")` と
`Token::get("conjugation_form")`、`Token::as_value()` と
`lindera tokenize -o json` の JSON、バインディングが公開する辞書スキーマの
フィールド名が該当します。位置によるアクセス（`details`、MeCab 形式と wakati
形式の出力）は変わりません。値そのものは元から正しい列に入っていたためです。
これらのフィールドを名前で読んでいるコードや、回避策として 2 つを入れ替えて
読んでいたコードは修正してください。

フィールド名はライブラリではなく、ビルド済み辞書ごとに保存される
`metadata.json` から読み込まれます。また、辞書フォーマットのバージョンは
変わっていません。埋め込み辞書（`embed-ipadic`・`embed-ipadic-neologd`）と、
7.0.0 の CLI が `lindera download` で取得する辞書は修正後の名前になります。
v6.2.0 以前にビルドまたはダウンロードした辞書ディレクトリは v7.0.0 でも
そのまま読み込めますが、旧い名前のままです。このような辞書ディレクトリは、
次のいずれかで対応してください:

- v7.0.0 の `lindera-ipadic/metadata.json`（または
  `lindera-ipadic-neologd/metadata.json`）で再ビルドするか、7.0.0 の
  リリースアセットをダウンロードする。
- その辞書の `metadata.json` の `dictionary_schema.fields` で
  `"conjugation_form"` と `"conjugation_type"` を入れ替える。辞書データ自体は
  変わらないため、再ビルドは不要です。

IPADIC の `metadata.json` の独自のコピーで辞書をビルドしている場合は、
そのコピーも同じように入れ替えてください。

## 空白を含む見出し語を保持する

v6.2.0 までは、辞書ビルダーがすべての見出し語の表層形を trim していました。そのため、
表層形が空白だけの語は辞書から落ち、先頭や末尾が空白の語は空白を削った表層形で
登録され、コストの低い空白付きの語が空白のない語を乗っ取ることがありました。
v7.0.0 は、MeCab や Sudachi と同じく表層形を書いたとおりに読みます。

| 辞書 | 変化 |
| --- | --- |
| IPADIC・IPADIC-NEologd | U+3000（全角空白）は未知語の `名詞,サ変接続` ではなく辞書の `記号,空白` になり、後ろの語も MeCab と同じに解析されます（`東京　都` の `都` は `名詞,接尾` ではなく `名詞,一般`）。`ルーマニア` の原形は、末尾に U+3000 の付いた形ではなく `ルーマニア` になります |
| UniDic | U+3000 は未知語の `名詞,普通名詞,サ変可能` ではなく辞書の `空白` になります |
| SudachiDict | U+0020 には `SPACE` の未知語ではなく辞書の半角スペースの語（`空白`）が使われ、空白を含む文の分割が Sudachi にずっと近くなります（すべての語の間に空白を入れた 600 文で、Sudachi との一致が 201 文から 597 文に増加）。`keep_whitespace(true)` では空白が 1 つずつトークンになります。SudachiDict には U+3000 の語がないため、U+3000 は変わりません |
| ko-dic | 単独の `에듀` と `캘리` は、末尾に空白の付いた語の `NNG` ではなく `NNP`（人名）になります |
| CC-CEDICT・Jieba | 変化なし |

末尾が空白の語（IPADIC-NEologd 55 語、ko-dic 4 語、SudachiDict 4 語）は、MeCab と
同じく、テキストにその空白があるときだけ一致するようになります。IPADIC-NEologd では、
末尾に半角スペースの付いた `GeForce GTX Titan X` の語が、`GeForce GTX Titan X です` には
一致し、`GeForce GTX Titan Xです` には一致しなくなります。空白がラティスに残るとき
（`keep_whitespace(true)`、または SudachiDict のデフォルトである
`skip_whitespace(false)`）も、空白を読み飛ばすとき（ほかの辞書のデフォルト。次の節を
参照）も、MeCab と同じくトークンは末尾の空白を含みます。読み飛ばすのは語の後ろの
空白だけで、`GeForce GTX Titan X  です` のように空白が 2 つ続く場合、トークンは
1 つ目の空白で終わります。IPADIC-NEologd の 55 語のうち 50 語は、ko-dic と
SudachiDict の語と同じく半角スペースで終わります。残りの 5 語は U+3000 か U+00A0 で
終わり、これらは `SPACE` の文字ではないため読み飛ばされません。
Decompose の長さペナルティは、空白がラティスに残るときと同じく、語自身の空白も
数えます。`unique` を指定した N-best では、この空白だけが異なる結果が 1 つに
まとめられなくなります（`에듀 센터` に対する ko-dic の、半角スペースの付いた
`에듀`（`NNG`）と `에듀`（`NNP`）など）。

U+3000 は `SPACE` の文字カテゴリではないため、`keep_whitespace(false)` でも MeCab と
同じくトークンとして出力されます。取り除くには、`japanese_stop_tags` トークンフィルタで
`記号,空白`（IPADIC）や `空白`（UniDic）を除くか、`unicode_normalize` 文字フィルタ
（NFKC）で U+0020 に変換してください。

変更は辞書ビルダーにあり、辞書の形式バージョンは変わりません。埋め込み辞書と、
7.0.0 の CLI が `lindera download` で取得する辞書には、これらの語が含まれます。
v6.2.0 以前にビルドまたはダウンロードした辞書ディレクトリは v7.0.0 でも読み込めますが、
`lindera build` で再ビルドするか 7.0.0 のリリースアセットを再ダウンロードするまで、
従来の見出し語のままです。

## 空白をラティス上で読み飛ばす

`keep_whitespace` が false（デフォルト）のとき、v7.0.0 は MeCab と同様に空白を
Viterbi ラティス上で読み飛ばします。空白の後ろの語は空白の前の語に直接
接続します。v6 は空白を出力から除外していましたが、ラティスには `SPACE` の
未知語として残していました。MeCab で学習した辞書ではこの項目との連接は
学習時に一度も現れないため（ko-dic ではその連接コストがすべて 0）、空白の
たびに前後の語の文脈が途切れていました。

出力が変わるのは空白を含む文だけです。分割の区切りになった `\n` や `\t` で
終わる文（`。` で終わらない行など）もこれに含まれ、その文の最後の語は文末に
直接接続するようになります。

| 辞書 | 影響 | 例 |
| --- | --- | --- |
| ko-dic | 空白を含む韓国語が mecab-ko と同じ解析になる | `2년 전 대회` の `전` は `저/NP + ㄴ/JX` ではなく `NNG`、`하고 있다` の `있` は `VV` ではなく `VX` |
| IPADIC・IPADIC-NEologd・UniDic | MeCab と同様に、半角空白の後ろの語が空白の前の語に直接接続する | IPADIC の `Google が 新しい` の `が` は接続詞ではなく格助詞、`東京 都` の `都` は MeCab と同様に接尾 |
| SudachiDict | 変わらない。Sudachi は空白をラティスに残し、コストもそれを前提にしているため、`metadata.json` で `skip_whitespace` を `false` にしている | — |
| CC-CEDICT・Jieba | 最良パスは変わらない（連接コストを持たない辞書のため）。N-best のコストに空白ノードの分が含まれなくなる | — |

デフォルトは辞書の `metadata.json` で決まり、`skip_whitespace` を設定していない
辞書は読み飛ばします。Lindera 6.x でビルドした SudachiDict にはこの設定がない
ため、再ビルドするか `metadata.json` に設定を追加するまでは空白を読み飛ばします。

トークンの表層形とオフセットには読み飛ばした空白が含まれません。ただし、見出し語の
一部である空白は、語の途中にあっても末尾にあっても、MeCab と同じくその語のトークンに
残ります（前の節を参照）。読み飛ばしは空白を含まないテキストの分割を変えません。
`keep_whitespace(true)` では空白がラティスに残り、出力は変わりません。

空白の扱いを v6 に戻すには、読み飛ばしを無効にします。空白は引き続き出力から
除外されます。ただし、[MeCab と同じく、どの位置からも未知語が始まる](#mecab-と同じくどの位置からも未知語が始まる)
など、このガイドの他の出力の変化は戻りません。設定で `skip_whitespace` を省略するか
`null` にすると、辞書のデフォルトのままになります:

```rust
// Segmenter
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
// SegmentWorker
worker.set_skip_whitespace(false);
```

```yaml
# 設定ファイル / TokenizerBuilder::set_segmenter_skip_whitespace(false)
segmenter:
  skip_whitespace: false
```

```sh
lindera tokenize --disable-skip-whitespace
```

言語バインディングでは設定ファイルで指定します。

## `char.def` の重複した行を MeCab と同じく解決する

辞書の `char.def` では、同じコードポイントを複数の行に書くことができます。v6.2.0
までは、辞書ビルダーがそのコードポイントに、覆うすべての行のカテゴリを与えていました。
v7.0.0 は MeCab・vibrato・Kuromoji と同じく最後の行だけを使い、その行のカテゴリを
`char.def` で定義された順に並べます。同梱の辞書で変わるのは次の 3 文字です（漢字の
カテゴリ名は、ko-dic では `HANJA`・`HANJANUMERIC`、CC-CEDICT と Jieba では
`CHINESE`・`CHINESENUMERIC` です）。

| 文字 | v6 のカテゴリ | v7 のカテゴリ | 影響 |
| --- | --- | --- | --- |
| `Ð`（U+00D0） | `SPACE`・`ALPHA` | `ALPHA` | ko-dic 以外のすべての辞書は、`ALPHA` の範囲より前に U+00D0 を `SPACE` に割り当てています（mecab-ipadic から引き継がれた U+000D のタイポ）。このため `Ð` は空白として出力から消え、前後の語も分断されていました。`GUÐMUNDUR さん` は `GU`・`MUNDUR`・`さん` ではなく `GUÐMUNDUR`・`さん` になります |
| `々`（U+3005） | `KANJI`・`SYMBOL` | `SYMBOL` | `人々` や `佐々木` のような辞書の語は変わりません。辞書にない漢字の後ろでは、MeCab と同じく `々` が単独のトークンになります（`龘々` は 1 つの未知語ではなく `龘`・`々`）。ko-dic では `々` の品詞が `SH` ではなく `SY` になります |
| `〇`（U+3007） | `KANJI`・`SYMBOL`・`KANJINUMERIC` | `SYMBOL`・`KANJINUMERIC` | 数字の連続のまとまり方が変わることがあります。ko-dic では `二〇二六年` が `二`・`〇`・`二六`・`年` ではなく `二〇二六`・`年` になります |

漢数字 `一`〜`九`・`十`・`百`・`千`・`万`・`億`・`兆` も 2 つの行に書かれていますが、
カテゴリもその並びも変わりません。

`lindera train` は `char.def` を辞書ビルダーで読むようになりました。従来は各行の
先頭のカテゴリだけを使い、単一コードポイントの行（`0x0020 SPACE` など）を読み飛ばし、
重複は範囲の開始位置で解決していました。素性テンプレートの `%t` は、その文字を覆う
最後の行の先頭カテゴリ（MeCab の既定カテゴリ）になります。同梱の学習用ファイルには
どちらの種類の行もないため、それらから学習したモデルは変わりません。

カテゴリのない範囲行は、MeCab と同じく `lindera build` と `lindera train` でエラーに
なります。そうしないと、それより前の行を `DEFAULT` に戻してしまうためです。

変更は辞書ビルダーにあり、辞書の形式バージョンは変わりません。埋め込み辞書と、
7.0.0 の CLI が `lindera download` で取得する辞書は、新しいカテゴリを持ちます。
v6.2.0 以前にビルドまたはダウンロードした辞書ディレクトリは v7.0.0 でも読み込めますが、
`lindera build` で再ビルドするか 7.0.0 のリリースアセットを再ダウンロードするまで、
従来のカテゴリのままです。

## `、`・`。` をまたいで文脈を引き継ぐ

セグメンターは、各ラティスを小さく保つために、入力を `\n`・`\t`・`。`・`、` で
文に区切り、区切り文字のないまま 32 KiB を超えたところでも区切ります。v6 までは、
どの文も文頭（BOS）から始まり、文末（EOS）への連接で終わっていたため、`、` や `。` の
直後の語は、テキストの先頭にあるものとして解析されていました。MeCab は行の中では
区切りません。v7.0.0 は、`、`・`。` と強制的な区切りをまたいで文脈を引き継ぎます。
次の文は前の文の最後の語から始まり、EOS への連接は `\n`・`\t`・入力の終わりでだけ
払います。そこまでの文の並びが 1 つの区間（segment）になり、区間どうしは引き続き
独立しています。区間の最良のパスとそのコストは、区間全体を 1 つのラティスで解いた
場合と同じになり、MeCab が 1 行に対して返す結果と一致します。

| 入力（辞書） | v6 | v7（MeCab と同じ） |
| --- | --- | --- |
| `彼は、ああ言った`（IPADIC） | `ああ` が感動詞 | `ああ` が副詞 |
| `だから、こんなに答える`（IPADIC） | `こんな`（連体詞）と `に` | `こんなに`（副詞） |
| `選手が泳ぎ、さらにカヌーで進んだ`（UniDic） | `さらに` が接続詞 | `さらに` が副詞 |
| `言語としては、Javaのオブジェクト`（UniDic） | `J`・`a`・`v`・`a`（それぞれ `記号,文字`） | `Java`（`名詞,普通名詞`） |

UD Japanese GSD の `、` を含む文では、UniDic で 50 行の出力が変わり、そのうち
27 行は正解の分割と品詞に近づき、7 行は離れ（符号検定 p = 0.0008）、16 行は
同点でした。『坊っちゃん』の 505 段落のうち、MeCab と分割・品詞が完全に一致する
段落は、IPADIC で 352 から 438 に、UniDic で 428 から 503 に増えました。`、`・`。`・
強制的な区切りを含まないテキスト（`\n` と `\t` だけで区切られるテキストなど）は
影響を受けません。文脈の引き継ぎで、1-best の分割の命令数は 1% 程度増えます。

次の場合は、区間全体を 1 つのラティスで解いた結果と異なることがあります。

- 区切りをまたぐ語はできません。`、` や `。` を含む見出し語はマッチしません。
  UniDic の `一、二塁` を、MeCab は 1 つの名詞として、Lindera は `一`・`、`・
  `二塁` として解析します。未知語も区切りをまたいでまとめられず、強制的な区切りを
  またぐはずの語はそこで分かれます。
- 強制的な区切りの位置から始まる語には、区切りの直前が空白でも左側空白ペナルティ
  （ko-dic）がかかりません。

N-best の結果も同じモデルに従います。区間の中では、結果は区間全体を 1 つの
ラティスで解いたときの上位 N 本のパスと厳密に一致します。`unique` は区間全体の
単語境界で比較し、コストの閾値は 1 件目の結果から測ります。区間をまたぐ結果は、
区間ごとに 1 つのパスを選んだ組み合わせのうちコストの小さいものです（次の節を
参照）。1 件目の結果は、コストがまったく同じパスが複数ある場合を除き、1-best の
分割と同じです。`、` や `。` を含む行のコストは、区切りをまたぐ連接を含み、EOS への連接を 1 回だけ
含むようになるため、1 件目の結果も含めて変わります。

v6 の動作に戻す設定はありません。

## N-best の結果が入力全体を覆う

N-best のメソッド（`Segmenter::segment_nbest`・`Tokenizer::tokenize_nbest`、それらの
`_with_lattice` 版と worker 版、`lindera tokenize -N`、バインディングの N-best
メソッド）は、1-best の分割と同じく入力を文に区切ります。v6 までは文ごとに探索し、
k 件目の結果は各文の k 番目のパスをつないだものでした。2 件目以降はすべての文が
同時に変わるため、結果は入力全体でコストの小さい分割には
ならず、コストが下がることもありました。また、パスが k 個未満の文は k 件目の結果から
抜けていました。v7.0.0 は入力を区間ごとに探索し（前の節を参照）、区間ごとに 1 つの
パスを選んだ組み合わせのうちコストの小さい N 件を合計コストの昇順に返します。どの結果も
入力全体を覆います。

| 入力（IPADIC、`-N 3`） | v6 | v7 |
| --- | --- | --- |
| `東京、です` | 3079 `東京 、 です`、23850 `東 京 、 で す`、24249 `東 京 、 で す` | 3357 `東京 、 です`、12059 `東京 、 です`（`、` が `名詞,数`）、13633 `東京 、 で す` |
| `。東京` | 13 `。 東京`、30433 `。 東 京`、13520 `東 京`（`。` がない） | 852 `。 東京`、13569 `。 東 京`、13636 `。 東 京` |

どちらの行も 1 つの区間なので、v7 の結果は、行全体を 1 つのラティスで解いたときの
上位 3 本のパスで、MeCab が返すものと同じです。1 件目の結果は 1-best の分割と同じで、
文が 1 つだけの入力の出力は変わりません。コストの閾値
（`cost_threshold`・`--nbest-cost-threshold`）は入力全体で測るようになりました。
合計コストが 1 件目の結果のコストから閾値以内の結果を残します。v6 は閾値を文ごとに
適用していました。`unique` を指定した場合も、結果の単語境界は互いに異なります。

## MeCab と同じく、どの位置からも未知語が始まる

辞書にない語は未知語として扱われ、辞書の `char.def` の文字種（カタカナ・英字・
数字・記号など）をもとに作られます。多くの文字種はグルーピングする設定で、
カタカナの連続のように同じ文字種が続く範囲が 1 つの候補になります。これを
「グルーピングした未知語」と呼びます。v6 までの通常モードは、直前にグルーピング
した未知語の内側の位置では未知語を作りませんでした。Kuromoji から引き継いだ
近道で、MeCab にはないものです。そのため、こうした範囲の途中で終わる辞書語の
後ろに未知語を続けられず、辞書語を飲み込んだグルーピングした未知語しか通り道が
ないことがよくありました。v7.0.0 は、経路が到達するすべての位置で、どのモードでも、
MeCab と同じ条件で未知語候補を作ります。条件は、その文字種が常に未知語を作る設定
（`char.def` の `INVOKE`）であるか、その位置から始まる辞書語がないことです。
Decompose モードはもともとこの動作で、出力は変わりません。

| 入力（辞書） | v6 | v7（MeCab と同じ） |
| --- | --- | --- |
| `それが「⁂⁂第一だ`（IPADIC） | `「⁂⁂`（辞書の記号 `「` を飲み込んだ 1 つの未知語の名詞） | `「`（`記号,括弧開`）と `⁂⁂` |
| `ジョン・レノンが歌う`（IPADIC） | `ジョン・レノン`（1 つの未知語の名詞） | `ジョン`、`・`、`レノン` |
| `ホテル・コルテシアに泊まる`（UniDic） | `ホテル・コルテシア`（1 つの未知語の名詞） | `ホテル`、`・`、`コルテシア` |
| `"좋아."`（ko-dic） | `아`（`EC`）と `."`（`SY`） | `아`（`EF`）、`.`（`SF`）、`"`（`SY`） |
| `The`（CC-CEDICT） | `The` | `T` と `he` |

- **日本語**: IPADIC・IPADIC-NEologd・UniDic の `char.def` は `・` をカタカナの
  範囲に含めています。辞書語と `・` の後ろからはカタカナの未知語を始められなかった
  ため、`ジョン・レノン` は 1 つの未知語になっていました。v7 では MeCab と同じく、
  その後ろから未知語が始まります。`レノン・ジョン` のように未知語で始まる範囲は、
  MeCab でも 1 つの未知語のままです。同じように、辞書の記号がその後ろの未知の記号と
  まとめられることもなくなります。
- **韓国語**: 文末の `."`・`?"`・`!"`・`.'` は 1 つの記号（`SY`）になっていました。
  v7 では MeCab と同じく `.`・`?`・`!` が文末の句読点（`SF`）になり、`"좋아."` の
  ように、その前の語尾の解析も変わることがあります。
- **中国語**: CC-CEDICT では、`A`・`B`・`P`・`Q`・`T` で始まる英単語がその文字の
  後ろで分かれ（`The` は `T` と `he`、`TBS` は `T`、`B`、`S`）、日本語の仮名の
  連続は 1 文字ずつに分かれます。同じ辞書を使う MeCab と同じ分け方です。原因は
  辞書にあります。未知語のコストは長さによらず −3,200、連接表は 1 マスだけで
  コストは 0、これらの英字 1 文字はコスト −400 の辞書語なので、語の数が多い経路ほど
  安くなります。たとえば `T` と `he`（−3,600）は `The`（−3,200）より安くなります。
  これまでは近道がこの分割を隠していました。Jieba は影響を受けません。
- **SudachiDict** でも、同じような範囲で出力が変わることがあります。
- **N-best**: 1 件目の結果は 1-best の分割と同じように変わります。2 件目以降には、
  `Google` を `G` と `oogle` に分ける（UniDic）ような結果も現れるようになります。

同じソースからビルドした辞書で、Lindera の分割と品詞が MeCab と完全に一致する
行は、このガイドの他の変更に加えてさらに増えます。IPADIC では日本語 600 文のうち
589 から 596 に、『坊っちゃん』の 505 段落のうち 438 から 459 に、IPADIC-NEologd
では同じく 590 から 596 に、441 から 459 に、ko-dic では韓国語 1,500 文のうち 877 から
1,498 に増えました（MeCab には左側空白ペナルティがないため、ko-dic はペナルティを
無効にして比較）。一致していた行が一致しなくなった例はなく、UniDic はこれらの
テキストでは変わりません。UD Japanese GSD を UniDic で解析すると 64 文が変わり、
53 文は正解の分割と品詞に近づき、10 文は離れ（符号検定 p = 3.4 × 10⁻⁸）、1 文は
同点でした。離れた 10 文はいずれも MeCab と同じ出力で、GSD が `・` でつないだ名前や
数字の一部を 1 語としているためです。

同じ文字種が続く範囲の内側のすべての位置で、MeCab と同じく候補を作るため、
カタカナや英字の連続が多いテキストでは処理が増えます。この変更を除いた同じ
リリースと比べると、あわせて入れた高速化を含めて、命令数は UD Japanese GSD
（Wikipedia の文章）で 13%（N-best では 14〜17%）増え、『坊っちゃん』と韓国語で
0.8%、Decompose モードで 14〜16% 減ります。区切り文字のないカタカナ 10,240 文字の
行では、命令数が 1-best で 3〜5 倍、N-best で 3〜15 倍になり、その行に対する
`lindera tokenize -N 3` の最大メモリは IPADIC で 12 MiB から 56 MiB に、UniDic で
18 MiB から 127 MiB に増えます。

v6 の動作に戻す設定はありません。`unknown_word_ladder(false)`
（`--disable-unknown-word-ladder`）は引き続き短い未知語候補を無効にしますが、
v6 より前の Lindera の出力は再現しなくなりました。

## `lindera_dictionary` の Rust API 変更

以下は、ラティスのバックトレースを直接呼び出すコードにのみ影響します。対象は
`lindera_dictionary::viterbi::Lattice` と `lindera_dictionary::nbest::NBestGenerator`
で、`lindera::dictionary::viterbi::…`・`lindera::dictionary::Lattice`・
`lindera::dictionary::nbest::…` からも参照できます。`Segmenter`・`Tokenizer`・
`SegmentWorker`・`AnalysisWorker` の API、言語バインディング、CLI は影響を
受けません。

| v6 | v7 |
| --- | --- |
| `Lattice::tokens_offset() -> Vec<(usize, WordId)>` | `Lattice::tokens_offset() -> Vec<TokenOffset>` |
| `Lattice::tokens_offset_into(&mut Vec<(usize, WordId)>)` | `Lattice::tokens_offset_into(&mut Vec<TokenOffset>) -> Option<usize>` |
| `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<(Vec<(usize, WordId)>, i64)>` | `Lattice::nbest_tokens_offset(n, unique, cost_threshold) -> Vec<NBestPath>` |
| `NBestGenerator::next() -> Option<(Vec<(usize, WordId)>, i64)>` | `NBestGenerator::next() -> Option<NBestPath>` |
| — | `viterbi` に新しい型エイリアス `TokenOffset = (usize, usize, WordId)` と `NBestPath = (Vec<TokenOffset>, i64)` を追加 |
| — | `viterbi` に `BosContext`・`LatticeOptions::bos`・`LatticeExit`・`Lattice::exits_into`・`Lattice::exit_tokens_offset_into` を追加 |
| — | `nbest` に `NBestGenerator::from_exit`・`NBestGenerator::next_with_bos` を追加 |

各トークンは `(start, word_id)` ではなく `(start, end, word_id)` になりました。
オフセットは文内のバイト位置です。空白を読み飛ばすとき（v7 で追加された
`LatticeOptions::skip_whitespace`）は、トークンの終わりが次のトークンの開始位置と
一致するとは限りません。間に読み飛ばした空白があると、そのうちどこまでが語に
属するかはラティスにしかわからないためです。返される終了位置は、読み飛ばした空白を含まず、
語自身の末尾の空白を含みます（[空白を含む見出し語を保持する](#空白を含む見出し語を保持する)
を参照）。`set_text` や `set_text_nbest` のように空白をラティスに残す場合、終了位置は
これまでどおり次のトークンの開始位置（最後のトークンでは文の終わり）です。`unique` を
指定した `nbest_tokens_offset` は、パスの開始位置ではなく `(start, end)` の組を
比較するようになりました。

```rust
use lindera::dictionary::viterbi::TokenOffset;

// v6: トークンは次のトークンの開始位置で終わる
let offsets = lattice.tokens_offset();
for (i, &(start, word_id)) in offsets.iter().enumerate() {
    let end = offsets.get(i + 1).map_or(sentence.len(), |&(next, _)| next);
    handle(&sentence[start..end], word_id);
}

// v7: 終了位置が返される
let offsets: Vec<TokenOffset> = lattice.tokens_offset();
for &(start, end, word_id) in &offsets {
    handle(&sentence[start..end], word_id);
}
```

`nbest_tokens_offset` と `NBestGenerator::next` が返すパスも、同じ 3 つ組を持ちます。

`tokens_offset_into` は、最良パスの BOS の添字も返すようになりました。パスが始まる
BOS の辺の、`LatticeOptions::bos` での添字です。デフォルトの単一の BOS では
`Some(0)`、ラティスに完全なパスがないときは `None` です。文として呼び出している
コードはそのままコンパイルできます。`for_each` に渡すクロージャの本体のように `()` を
期待される場所で使っている場合は、`;` を付けてください。

新しい項目は、セグメンターが `、`・`。` をまたいで文脈を引き継ぐために使うものです
（[`、`・`。` をまたいで文脈を引き継ぐ](#-をまたいで文脈を引き継ぐ)を参照）。
既存のコードには影響しません。

- `BosContext` と `LatticeOptions::bos`: 文を複数の BOS の辺から始められます。
  各辺は右文脈 ID とコストを持ちます。デフォルトの空のスライスは、v6 と同じ単一の
  BOS です。
- `LatticeExit`・`Lattice::exits_into`・`Lattice::exit_tokens_offset_into`: 文の最後の
  語の右文脈 ID ごとに、その語までの最良のパス（EOS への連接を含まない）と、その
  トークンを返します。
- `NBestGenerator::from_exit` と `NBestGenerator::next_with_bos`: ある出口の右文脈 ID
  で終わるパスを、EOS なしでコストの昇順に返し、各パスとともにそのパスが始まる BOS の
  添字を返します。`next` は、`next_with_bos` のパスを BOS の添字なしで返します。

`nbest_tokens_offset` はデフォルトの単一の BOS を前提にしています。BOS が複数ある
場合は、パスがどの BOS から始まるかがわからないため、`NBestGenerator::next_with_bos`
を使ってください。

## 対応が不要なケース

- **言語バインディングと CLI のユーザー**: Python・Node.js・Ruby・PHP・WASM の
  各パッケージと `lindera-cli` の API・パッケージ名は変わりません。今回の
  構成変更は Rust クレート内部のものです。出力の変化は、この 2 フィールドを
  名前で読む場合にのみ影響する IPADIC の活用フィールドの修正、U+3000 を含む
  テキスト（IPADIC・IPADIC-NEologd・UniDic）や空白を含むテキスト（SudachiDict）に
  のみ影響する空白の見出し語、空白を含むテキストの分割、`Ð`・`々`・`〇` を含む
  テキストにのみ影響する `char.def` の修正、行の中に `、` や `。` を含むテキストに
  のみ影響する `、`・`。` をまたぐ文脈の引き継ぎ、複数の文を含む入力の N-best に
  のみ影響する N-best の修正、そして通常モードでのカタカナ・記号・英字の連続に
  影響する、どの位置からも始まる未知語です。
- **`lindera = "6"` のまま使い続けるプロジェクト**: `lindera` と `lindera-analysis`
  の 6.x は crates.io に残り、組み合わせて動作し続けます。メジャーバージョンを
  上げるまで何も変わりません。
- **セグメンター API だけを使うユーザー**: `Segmenter`・`load_dictionary`・`Mode`
  など v6 の `lindera::…` アイテムを使うコードは、`lindera = "7"` でそのまま
  コンパイルできます（`lindera::dictionary::Lattice` のバックトレースを呼び出す
  場合を除く。[`lindera_dictionary` の Rust API 変更](#lindera_dictionary-の-rust-api-変更)
  を参照）。ただしデフォルトビルドでは `lindera-analysis` とその依存
  （kanaria・regex・serde_yaml_ng・unicode-blocks・unicode-normalization・
  unicode-segmentation）もコンパイルされるため、v6 の依存ツリーを維持したい
  場合は `default-features = false, features = ["mmap"]` を指定してください。

## アップグレードチェックリスト

Rust クレートのユーザー:

- `lindera` を `"7"` に上げ、同じコミットで `lindera-analysis` も `"7"` に上げるか、
  取り除いて `lindera::analysis::…` から import する。
- 任意で `use` パスの `lindera_analysis::` を `lindera::analysis::` に置き換える
  （どちらも同じ型を指す）。
- `default-features = false` を指定している場合は、必要に応じて `features` に
  `"analysis"`（`Tokenizer` とフィルタ）と `"mmap"`（メモリマップ読み込み）を
  追加する。
- `features = ["lindera-<辞書>"]` を `features = ["embed-<辞書>"]` に置き換える。
- 任意で `lindera-dictionary` への直接依存を外し、`lindera::dictionary::core`・
  `::builder`・`::viterbi` などの再エクスポートに切り替える。
- `Lattice::tokens_offset`・`tokens_offset_into`・`nbest_tokens_offset`・
  `NBestGenerator::next` を呼び出している場合は、`(start, end, word_id)` に分解し、
  次のトークンの開始位置の代わりに返された終了位置を使う。
- `Lattice::tokens_offset_into` の戻り値 `()` をクロージャの本体などで値として
  使っている場合は、`;` を付ける。最良パスの BOS の添字を返すようになったため。

バインディングの作者:

- `lindera-binding-core` の代わりに `lindera-binding` に依存し、
  `lindera_binding_core::` を `lindera_binding::` に置き換える。

ビルド環境:

- シェルのプロファイル・CI 設定・コンテナイメージで `LINDERA_DICTIONARIES_PATH`
  を `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に改名する。

IPADIC・IPADIC-NEologd のユーザー:

- `conjugation_type` や `conjugation_form` を名前で読んでいる場合（CLI の JSON
  出力を含む）は、2 つの値が入れ替わることを前提にする。
- v6.2.0 以前に作った辞書ディレクトリは、再ビルド・再ダウンロードするか、
  `metadata.json` の 2 つの名前を入れ替える。

U+3000 や空白を含むテキストを IPADIC・IPADIC-NEologd・UniDic・SudachiDict で
解析するユーザー:

- U+3000 が `記号,空白`（IPADIC）や `空白`（UniDic）のトークンになり、SudachiDict の
  空白を含む文の分割が Sudachi にずっと近くなることを前提にする。U+3000 が不要なら
  `japanese_stop_tags` か NFKC 正規化で取り除く。
- 空白の見出し語を使うには、v6.2.0 以前に作った辞書ディレクトリを再ビルド・
  再ダウンロードする。

空白を含むテキストを分割するすべてのユーザー:

- そのようなテキストは MeCab と同じ分割になることを前提にする（ko-dic で特に
  顕著）。空白の扱いを v6 に戻したい場合は `skip_whitespace(false)`、
  `"skip_whitespace": false`、`--disable-skip-whitespace` を指定する。このガイドの
  他の出力の変化は戻らない。
- 末尾が半角スペースの見出し語（IPADIC-NEologd・ko-dic・SudachiDict）のトークンは、
  MeCab と同じくその空白を含むことを前提にする。

`Ð`・`々`・`〇` を含むテキストを同梱の辞書で解析するユーザー:

- `Ð` が出力に残り、未知語の漢字に続く `々` が単独のトークンになることを前提にする。
- 新しい文字カテゴリを使うには、v6.2.0 以前に作った辞書ディレクトリを再ビルド・
  再ダウンロードする。

`lindera train` のユーザー:

- `char.def` のすべての範囲行に 1 つ以上のカテゴリを書く。
- `char.def` に単一コードポイントの行や重複した行がある場合は、再学習する。
  `%t` 素性が MeCab と同じくそれらの行に従うようになったため。

行の中に `、` や `。` を含む日本語のテキストを解析するユーザー:

- `、` や `。` の直後の語の一部が、前の語の文脈を引き継いで解析されることを前提に
  する（多くは MeCab と同じ解析になる）。v6 の出力に戻す設定はない。

N-best を使うユーザー:

- どの結果も入力全体を覆い、複数の文を含む入力では 2 件目以降の結果が変わることを
  前提にする。
- `、` や `。` を含む行では、1 件目の結果も含めてすべてのコストが変わることを前提に
  する。コストは行全体を 1 つのラティスで解いたときの値になる。
- コストの閾値を指定している場合、閾値は文ごとではなく入力全体の合計コストに
  適用される。

カタカナ・記号・英字が続くテキストを通常モードで解析するユーザー:

- そうした連続が MeCab と同じ位置で分かれることを前提にする。`・` でつないだ
  カタカナ（IPADIC・IPADIC-NEologd・UniDic）、辞書の記号の直後の未知の記号、
  `."` のような韓国語の文末の句読点、CC-CEDICT で `A`・`B`・`P`・`Q`・`T` で始まる
  英単語と日本語の仮名の連続が該当する。v6 の出力に戻す設定はなく、
  `unknown_word_ladder(false)` でも v6 より前の Lindera の出力にはならない。
- カタカナや英字の多いテキストで命令数が 13% 程度増え、カタカナの非常に長い連続の
  N-best では時間とメモリが大きく増えることを前提にする。

言語バインディングと CLI:

- 7.0.0 リリースを取り込む以外に対応は不要（上記の辞書・空白・`、` と `。`・N-best・
  未知語の項目を除く）。
