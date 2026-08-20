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

例えば、「関西国際空港限定トートバッグ」という文中の複合語「関西国際空港」は、`Mode::Normal` では1つのトークンの一部のままですが、`Mode::Decompose` では「関西」「国際」「空港」に分割されます（分割されるかどうかは前後の文脈にも依存し、同じ文字列単独では同じ結果にならない場合があります）。

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
use lindera_analysis::tokenizer::Tokenizer;
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

デフォルトでは、MeCab互換のため空白のみのトークンは出力から除外されます。`Segmenter` に対して `keep_whitespace(true)` を呼び出すと、これらを保持できます：

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
```

## 未知語のグルーピング

未知語のグルーピングはデフォルトでは無制限です。`max_grouping_len(Some(n))` で MeCab の `max-grouping-size` と同じ意味論（先頭を除いた文字数で数え、MeCab のデフォルトは 24）の上限を設定できます。上限を超えるランは 1 文字ずつの未知語になります:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).max_grouping_len(Some(24));
```

グルーピングとは独立に、Lindera はデフォルトで MeCab/Vibrato 由来の
「候補ラダー（length ladder）」も生成します。各カテゴリの `char.def` の
`LENGTH` フィールドまでの短い未知語候補を段階的に生成し、Viterbi 探索が
最もコストの低い長さを選べるようにします。v6 以前と同一の出力にするには
`unknown_word_ladder(false)` で無効化してください:

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).unknown_word_ladder(false);
```

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

`segment_nbest_with_lattice` は同じ処理を行いますが、呼び出しごとの `Lattice` バッファの再確保を避けるために、再利用可能な `Lattice` を自分で渡すことができます。

## 文分割

ラティスを構築する前に、Linderaは入力テキストを区切り文字（`\n`、`\t`、`。`、`、`）で文単位に分割し、1文ずつ処理します。文の先頭からおよそ 32 KiB 以内に区切り文字が見つからない場合、Linderaはその位置で強制的に文の境界を区切り、警告をログに出力します。これは、その文に対して構築される Viterbi ラティスのサイズ（ひいてはメモリ・CPU コスト）を制限するためです。通常のテキストに対してはこの挙動は意識する必要がありませんが、区切り文字を含まない病的な入力（例えば minify されたテキストや base64 エンコードされたデータなど）に対しては、この人為的な分割位置でトークナイズ結果に影響が出ることがあります。

## 再利用可能な Worker

`SegmentWorker` は、Viterbi ラティスとバックトレース用スクラッチバッファを所有する再利用可能なセグメンテーションセッションです。呼び出しのたびに `segment` が支払うアロケーションを回避できます。`new_worker`（セグメンターを clone）または `into_worker`（セグメンターを消費し、ユーザー辞書のコピーを回避）で作成し、`segment`/`segment_nbest` を呼び出します：

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

Worker は保持メモリも制限します。区切り文字のない最大長の文を 1 回処理するとラティスは数 MB まで成長し（32 KiB の ASCII 文で約 18 MB、32 KiB の CJK 文では文字索引ラティスのスロット数が 1/3 のため約 6 MB）、素の `Lattice` はそれを保持し続けます。Worker は一定回数の呼び出し窓で容量が過大と判明した場合に自動でラティスを縮小し、`shrink_to(text_len_hint)` で即時に縮小することもできます。`reset()` は内部バッファを破棄して新しいものに置き換えます。これは、例えばパニックによって worker を保持する Mutex がポイズニングされた場合など、バッファが不整合な中間状態を保持している可能性がある回復経路のために用意されています。Segmenter の設定（mode や空白の扱いなど）自体は保持されます。

Worker は作成元セグメンターの辞書に恒久的に紐付けられ、稼働中の worker の辞書を差し替える手段はありません。これにより、ラティス再利用に起因するバグの一群を構造的に排除しています。マルチスレッドで使う場合は、共有の `Segmenter` からスレッドごとに worker を 1 つ作成してください。
