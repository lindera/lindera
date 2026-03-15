# Error Handling

Lindera uses a structured error system based on `anyhow` and `thiserror` for ergonomic error handling throughout the library.

## LinderaResult

The `LinderaResult<T>` type alias is the standard return type for fallible operations in Lindera:

```rust
pub type LinderaResult<T> = Result<T, LinderaError>;
```

## LinderaError

`LinderaError` is the main error type, containing an error kind and a source error with full context:

```rust
pub struct LinderaError {
    pub kind: LinderaErrorKind,
    source: anyhow::Error,
}
```

The `add_context` method allows attaching additional context to an error:

```rust
let error = error.add_context("failed to load dictionary from /path/to/dict");
```

## LinderaErrorKind

`LinderaErrorKind` is an enum that categorizes errors:

| Kind | Description |
| ------ | ------------- |
| `Io` | I/O errors (file read/write, network) |
| `Parse` | Parsing errors (invalid input format) |
| `Serialize` | Serialization errors |
| `Deserialize` | Deserialization errors |
| `Content` | Invalid content or data errors |
| `Args` | Invalid argument errors |
| `Decode` | Decoding errors |
| `Compression` | Compression/decompression errors |
| `NotFound` | Resource not found (e.g., dictionary file missing) |
| `Build` | Dictionary build errors |
| `Dictionary` | Dictionary-related errors |
| `Mode` | Invalid tokenization mode errors |
| `Algorithm` | Algorithm errors (e.g., Viterbi failure) |
| `FeatureDisabled` | Attempted to use a feature that is not enabled |

## Creating Errors

Use `LinderaErrorKind::with_error` to create an error from a kind and a source:

```rust
use lindera::error::LinderaErrorKind;

let error = LinderaErrorKind::Io.with_error(anyhow::anyhow!("file not found: config.yml"));
```

## Using the ? Operator

Since Lindera functions return `LinderaResult`, the `?` operator can propagate errors naturally:

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

## Error Handling Patterns

### Matching on Error Kind

```rust
use lindera::dictionary::load_dictionary;
use lindera::error::LinderaErrorKind;

match load_dictionary("/path/to/dictionary") {
    Ok(dict) => { /* use dictionary */ }
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

### Converting from External Errors

```rust
use lindera::error::LinderaErrorKind;

let content = std::fs::read_to_string("config.yml")
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
```
