# エラーハンドリング

Lindera はライブラリ全体で使いやすいエラーハンドリングを実現するため、`anyhow` と `thiserror` に基づく構造化されたエラーシステムを使用しています。

## LinderaResult

`LinderaResult<T>` 型エイリアスは、Lindera における失敗する可能性のある操作の標準的な戻り値型です：

```rust
pub type LinderaResult<T> = Result<T, LinderaError>;
```

## LinderaError

`LinderaError` はメインのエラー型で、エラー種別と完全なコンテキストを持つソースエラーを含みます：

```rust
pub struct LinderaError {
    pub kind: LinderaErrorKind,
    source: anyhow::Error,
}
```

`add_context` メソッドを使用して、エラーに追加のコンテキストを付与できます：

```rust
let error = error.add_context("failed to load dictionary from /path/to/dict");
```

## LinderaErrorKind

`LinderaErrorKind` はエラーを分類する列挙型です：

| Kind | 説明 |
| ------ | ------ |
| `Io` | I/Oエラー（ファイルの読み書き、ネットワーク） |
| `Parse` | パースエラー（無効な入力形式） |
| `Serialize` | シリアライズエラー |
| `Deserialize` | デシリアライズエラー |
| `Content` | 無効なコンテンツまたはデータのエラー |
| `Args` | 無効な引数のエラー |
| `Decode` | デコードエラー |
| `Compression` | 圧縮・解凍エラー |
| `NotFound` | リソースが見つからない（例: 辞書ファイルの欠落） |
| `Build` | 辞書ビルドエラー |
| `Dictionary` | 辞書関連のエラー |
| `Mode` | 無効なトークナイズモードのエラー |
| `Algorithm` | アルゴリズムエラー（例: Viterbi の失敗） |
| `FeatureDisabled` | 有効化されていない機能を使用しようとした |

## エラーの作成

`LinderaErrorKind::with_error` を使用して、種別とソースからエラーを作成します：

```rust
use lindera::error::LinderaErrorKind;

let error = LinderaErrorKind::Io.with_error(anyhow::anyhow!("file not found: config.yml"));
```

## ? 演算子の使用

Lindera の関数は `LinderaResult` を返すため、`?` 演算子で自然にエラーを伝播できます：

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn analyze(text: &str) -> LinderaResult<Vec<String>> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let tokens = tokenizer.tokenize(text)?;
    Ok(tokens.iter().map(|t| t.surface.as_ref().to_string()).collect())
}
```

## エラーハンドリングパターン

### エラー種別によるマッチング

```rust
use lindera::dictionary::load_dictionary;
use lindera::error::LinderaErrorKind;

match load_dictionary("/path/to/dictionary") {
    Ok(dict) => { /* 辞書を使用 */ }
    Err(e) if e.kind() == LinderaErrorKind::NotFound => {
        eprintln!("Dictionary not found: {}", e);
    }
    Err(e) if e.kind() == LinderaErrorKind::Io => {
        eprintln!("I/O error loading dictionary: {}", e);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

### 外部エラーからの変換

```rust
use lindera::error::LinderaErrorKind;

let content = std::fs::read_to_string("config.yml")
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
```
