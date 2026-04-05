# Dictionaries

Lindera supports various dictionaries for Japanese, Korean, and Chinese morphological analysis. Each dictionary is provided as a separate crate.

| Dictionary | Language | Crate | Description |
| --- | --- | --- | --- |
| [IPADIC](../lindera-ipadic.md) | Japanese | `lindera-ipadic` | The most common dictionary for Japanese |
| [IPADIC NEologd](../lindera-ipadic-neologd.md) | Japanese | `lindera-ipadic-neologd` | IPADIC with neologisms (new words) |
| [UniDic](../lindera-unidic.md) | Japanese | `lindera-unidic` | Uniform word unit definitions |
| [ko-dic](../lindera-ko-dic.md) | Korean | `lindera-ko-dic` | Korean morphological analysis |
| [CC-CEDICT](../lindera-cc-cedict.md) | Chinese | `lindera-cc-cedict` | Chinese-English dictionary |
| [Jieba](../lindera-jieba.md) | Chinese | `lindera-jieba` | Jieba-based Chinese dictionary |

## Obtaining Dictionaries

Pre-built dictionaries are available for download from [GitHub Releases](https://github.com/lindera/lindera/releases). Download the dictionary archive for your target language and extract it to a local directory.

```rust
// Load an external dictionary from a local path
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> If you need a self-contained binary without external dictionary files, you can embed dictionaries using the `embed-*` feature flags and load them using the `embedded://` scheme:
>
> ```rust
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

See each dictionary crate's documentation for format details, build instructions, and usage examples.
