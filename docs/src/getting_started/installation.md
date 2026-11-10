# Installation

## Requirements

Rust **1.89** or newer. The workspace declares this as `rust-version`, so an
older toolchain reports it as a cargo error rather than failing to compile.

## Adding the Dependency

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = "7"
```

> [!NOTE]
> This single dependency also provides the analysis chain (character filters,
> token filters, and the `Tokenizer`) as `lindera::analysis`; use
> `default-features = false` if you only need the segmenter.
> Upgrading from v6? See [Migration v6 to v7](../migration_v6_to_v7.md) for details.

## Dictionary Setup

Lindera requires a pre-built dictionary at runtime. Download a dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify its path when loading:

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```rust
> // Cargo.toml: lindera = { version = "7", features = ["embed-ipadic"] }
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> An `embed-*` build downloads the dictionary that Lindera publishes on GitHub for the dictionary crate, checks its SHA-256, and embeds it; it no longer builds the dictionary from source, so it needs access to `github.com` (which serves the file from `release-assets.githubusercontent.com`) rather than `lindera.dev`. With `LINDERA_BUILD_DICTIONARY_CACHE_DIR`, a later build uses the downloaded dictionary without network access. See [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](./environment_variables.md#lindera_build_dictionary_from_source) for when the build script builds from source instead, and [`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`](./environment_variables.md#lindera_build_dictionary_published_base_url) for downloading from a mirror.
>
> Only the bundled dictionaries can be embedded. Build your own dictionary with `lindera build` and load it from its directory at runtime.
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Environment Variables

The build scripts of the dictionary crates read `LINDERA_BUILD_DICTIONARY_CACHE_DIR`, `LINDERA_BUILD_DICTIONARY_FROM_SOURCE`, `LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`, `LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL`, `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` and `DOCS_RS`, and `TokenizerBuilder::new()` reads `LINDERA_CONFIG_FILE`. See [Environment Variables](./environment_variables.md) for these and the other variables that Lindera reads.
