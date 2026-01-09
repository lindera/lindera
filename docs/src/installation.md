# Installation

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.2.0", features = ["embed-ipadic"] }
```

## Environment Variables

### LINDERA_DICTIONARIES_PATH

The `LINDERA_DICTIONARIES_PATH` environment variable specifies a directory for caching dictionary source files. This enables:

- **Offline builds**: Once downloaded, dictionary source files are preserved for future builds
- **Faster builds**: Subsequent builds skip downloading if valid cached files exist
- **Reproducible builds**: Ensures consistent dictionary versions across builds

Usage:

```shell
export LINDERA_DICTIONARIES_PATH=/path/to/dicts
cargo build --features=ipadic
```

When set, dictionary source files are stored in `$LINDERA_DICTIONARIES_PATH/<version>/` where `<version>` is the lindera-dictionary crate version. The cache validates files using MD5 checksums - invalid files are automatically re-downloaded.

> [!NOTE]
> `LINDERA_CACHE` is deprecated but still supported for backward compatibility. It will be used if `LINDERA_DICTIONARIES_PATH` is not set.

### LINDERA_CONFIG_PATH

The `LINDERA_CONFIG_PATH` environment variable specifies the path to a YAML configuration file for the tokenizer. This allows you to configure tokenizer behavior without modifying Rust code.

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

See the [Configuration](./configuration.md) section for details on the configuration format.

### DOCS_RS

The `DOCS_RS` environment variable is automatically set by docs.rs when building documentation. When this variable is detected, Lindera creates dummy dictionary files instead of downloading actual dictionary data, allowing documentation to be built without network access or large file downloads.

This is primarily used internally by docs.rs and typically doesn't need to be set by users.

### LINDERA_WORKDIR

The `LINDERA_WORKDIR` environment variable is automatically set during the build process by the lindera-dictionary crate. It points to the directory containing the built dictionary data files and is used internally by dictionary crates to locate their data files.

This variable is set automatically and should not be modified by users.
