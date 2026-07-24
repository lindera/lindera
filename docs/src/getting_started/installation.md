# Installation

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = "5.0"
```

> [!NOTE]
> v5.0.0 is the next planned release and has not been published to crates.io yet; the current
> published version is `4.0.1`. This guide describes the v5.0.0 API, which already exists on the
> `main` branch. See [Migration v4 to v5](../migration_v4_to_v5.md) for details.

## Dictionary Setup

Lindera requires a pre-built dictionary at runtime. Download a dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify its path when loading:

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```rust
> // Cargo.toml: lindera = { version = "5.0", features = ["embed-ipadic"] }
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Environment Variables

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

The `LINDERA_BUILD_DICTIONARY_CACHE_DIR` environment variable designates a build-time cache directory for the embedded-dictionary build pipeline. It is read only by the dictionary crates' build scripts and has no effect at runtime.

When set, each build stores two kinds of files under `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>/` (where `<version>` is the dictionary crate version):

- the downloaded distribution archive (validated with MD5; invalid files are re-downloaded)
- the built binary dictionary that gets embedded into the crate

This enables:

- **Offline builds**: once cached, subsequent builds need no network access
- **Faster builds**: download and dictionary build are skipped when valid cached files exist
- **Reproducible builds**: consistent dictionary versions across builds

Usage:

```shell
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
cargo build --features=embed-ipadic
```

Notes:

- The directory is managed automatically and is safe to delete; contents are re-downloaded and rebuilt as needed
- Version subdirectories accumulate across upgrades and are not garbage-collected; old ones can be removed freely
- Setting this variable causes dictionary crates to download and build their dictionaries even when no `embed-*` feature is enabled (useful for pre-populating the cache)

> **Deprecated:** the previous name `LINDERA_DICTIONARIES_PATH` still works as a fallback (the new name wins when both are set) and will be removed in v6.0.0.

### LINDERA_CONFIG_PATH

The `LINDERA_CONFIG_PATH` environment variable specifies the path to a YAML configuration file for the tokenizer. This allows you to configure tokenizer behavior without modifying Rust code.

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

See the [Configuration](../lindera-analysis/configuration.md) section for details on the configuration format.

### DOCS_RS

The `DOCS_RS` environment variable is automatically set by docs.rs when building documentation. When this variable is detected, Lindera creates dummy dictionary files instead of downloading actual dictionary data, allowing documentation to be built without network access or large file downloads.

This is primarily used internally by docs.rs and typically doesn't need to be set by users.

### LINDERA_WORKDIR

The `LINDERA_WORKDIR` environment variable is automatically set during the build process by the lindera-dictionary crate. It points to the directory containing the built dictionary data files and is used internally by dictionary crates to locate their data files.

This variable is set automatically and should not be modified by users.
