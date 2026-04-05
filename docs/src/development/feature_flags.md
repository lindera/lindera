# Feature Flags

Lindera uses Cargo feature flags to control optional functionality and dictionary embedding.

## Core Features

| Feature | Description | Default |
| --- | --- | --- |
| `mmap` | Memory-mapped file support | Yes |
| `train` | CRF-based dictionary training (depends on `lindera-crf`) | CLI only |

- `mmap` is enabled by default in the main `lindera` crate.
- `train` is enabled by default only in `lindera-cli`. For library usage, enable it explicitly with `--features train`.

## Using External Dictionaries (Recommended)

The recommended approach is to use pre-built dictionaries as external files. Download a dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify its path at runtime:

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

No additional feature flags are required for this usage.

## Dictionary Embedding Features (Advanced)

These features embed pre-built dictionaries directly into the binary, eliminating the need for external dictionary files at runtime. This is intended for advanced users who need self-contained binaries.

| Feature | Dictionary | Language |
| --- | --- | --- |
| `embed-ipadic` | IPADIC | Japanese |
| `embed-ipadic-neologd` | IPADIC NEologd | Japanese |
| `embed-unidic` | UniDic | Japanese |
| `embed-ko-dic` | ko-dic | Korean |
| `embed-cc-cedict` | CC-CEDICT | Chinese |
| `embed-jieba` | Jieba | Chinese |

None of these are enabled by default. Enable them as needed:

```toml
[dependencies]
lindera = { version = "2.3.2", features = ["embed-ipadic"] }
```

When embedding is enabled, you can load the dictionary with:

```rust
let dictionary = load_dictionary("embedded://ipadic")?;
```

### Combination Features

These meta-features enable multiple dictionaries at once for multilingual applications.

| Feature | Included Dictionaries |
| --- | --- |
| `embed-cjk` | IPADIC + ko-dic + Jieba |
| `embed-cjk2` | UniDic + ko-dic + Jieba |
| `embed-cjk3` | IPADIC NEologd + ko-dic + Jieba |

### Combining Feature Flags

Multiple feature flags can be combined. For example, to embed both Japanese and Korean dictionaries:

```toml
[dependencies]
lindera = { version = "2.3.2", features = ["embed-ipadic", "embed-ko-dic"] }
```

Or from the command line:

```bash
cargo build --features embed-ipadic,embed-ko-dic
```

### Notes

- Embedding dictionaries increases binary size significantly. Only embed dictionaries you actually need.
- The `train` feature adds a dependency on `lindera-crf` and increases compile time. It is not needed for tokenization-only use cases.
- The `mmap` feature enables memory-mapped dictionary loading, which reduces memory usage for large dictionaries loaded from disk. It has no effect on embedded dictionaries.
