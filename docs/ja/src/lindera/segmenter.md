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
    "dictionary": "embedded://ipadic"
});
let segmenter = Segmenter::from_config(&config)?;
```

## 空白文字の扱い

デフォルトでは、MeCab互換のため空白のみのトークンは出力から除外されます。`Segmenter` に対して `keep_whitespace(true)` を呼び出すと、これらを保持できます：

```rust
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
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
