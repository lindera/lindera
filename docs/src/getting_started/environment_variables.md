# Environment Variables

Lindera reads the environment variables below; none of them is required.

| Variable | Read by | Value | Without it |
| --- | --- | --- | --- |
| [`LINDERA_BUILD_DICTIONARY_CACHE_DIR`](#lindera_build_dictionary_cache_dir) | The build scripts of the dictionary crates | A directory | Each build downloads and builds the dictionary in its own output directory |
| [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source) | The build scripts of the dictionary crates | `1` or `true` | A dictionary crate downloads the published dictionary it pins, and builds from source only when it cannot use it |
| [`LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`](#lindera_build_context_id_freq_file) | The build scripts of the dictionary crates | A file | The histogram bundled with the dictionary crate, if any |
| [`DOCS_RS`](#docs_rs) | The build scripts of the dictionary crates | Set by docs.rs | The real dictionaries are built |
| [`LINDERA_CONFIG_FILE`](#lindera_config_file) | `TokenizerBuilder::new()`, and so `lindera tokenize` and the bindings | A file | An empty configuration |
| [`LINDERA_DICTIONARIES_DIR`](#lindera_dictionaries_dir) | The CLI | A directory | `lindera/dictionaries` in the OS's local data directory |
| [`RUST_LOG`](#rust_log) | The CLI | A log filter | Warnings and errors |
| [`RUST_LOG_STYLE`](#rust_log) | The CLI | A color setting, such as `never` | Colors when standard error is a terminal |
| [`LINDERA_FEATURES`](#lindera_features) | The build of the Ruby gem | A comma-separated list of features | The default features |

Three of them have a former name that still works until v8.0.0, with a deprecation warning: `LINDERA_CTX_FREQ_FILE` for `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`, `LINDERA_CONFIG_PATH` for `LINDERA_CONFIG_FILE`, and `LINDERA_DATA_DIR`, read as `LINDERA_DICTIONARIES_DIR=<dir>/dictionaries`. See [Migration v6 to v7](../migration_v6_to_v7.md).

## Build-time variables

The build scripts of the dictionary crates, such as `lindera-ipadic`, read these variables when they build a dictionary, for example for `lindera` with an `embed-*` feature. They have no effect at runtime.

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

The `LINDERA_BUILD_DICTIONARY_CACHE_DIR` environment variable designates a build-time cache directory for the embedded-dictionary build pipeline. It is read only by the dictionary crates' build scripts and has no effect at runtime.

When set, each build stores two kinds of files under `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>-fmt<format>/` (where `<version>` is the dictionary crate version and `<format>` is the dictionary format version):

- the downloaded distribution archive (validated against the SHA-256 that the crate's `build.rs` pins; invalid files are re-downloaded)
- the built binary dictionary, with a `build_inputs.json` that records what it was built from

The format version is part of the path so that a cache written by a build with a different on-disk dictionary layout is a miss rather than a stale hit. A cached dictionary is reused only when all of the following match the current build, and is rebuilt otherwise:

- the dictionary format version and the contents of the crate's `metadata.json`
- the `lindera-dictionary` crate that builds it: its version and a digest of its source, so that any change to the dictionary builder, an uncommitted one included, rebuilds the dictionary
- the source archive (file name and SHA-256) and the directory the build reads in it
- the context-id frequency histogram the build reads (`context_id_freq.txt`, or the file named by `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`) when `connection_id_mapping` is enabled

A published dictionary that a build downloads instead of building it (see [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source)) is cached in the same place, and `build_inputs.json` records it by its key and SHA-256 besides the inputs above. The next build that would download the same published dictionary uses the cached one; a build that switches between the published dictionary and the build from source replaces it.

A dictionary left behind by an interrupted build, or cached by a release from before `build_inputs.json`, is rebuilt. The versions of the builder's dependencies are not compared: after upgrading a dependency of `lindera-dictionary` without a format version change, clear the cache if the built dictionaries may differ. Directories from older format versions are not deleted automatically; remove them by hand if the cache grows.

A build does not embed the cached files themselves: it links the nine files of the dictionary into the build script's output directory (`OUT_DIR`), copies those it cannot link (when the cache is on another file system than the target directory, for example), and embeds them from there. A crate thus keeps the dictionary it embedded when another build, with other inputs, replaces the cached one, or when the cache is removed, and builds in any checkouts and target directories can share one directory.

`cargo build -vv` shows, for each dictionary crate, whether the cached dictionary was used and why or why not, and whether it was linked or copied, on lines that start with `lindera dictionary cache:`.

This enables:

- **Offline builds**: once cached, subsequent builds need no network access
- **Faster builds**: download and dictionary build are skipped when valid cached files exist
- **Reproducible builds**: consistent dictionary versions across builds

Usage:

```shell
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
cargo build --features=embed-ipadic
```

To share one directory among every target directory and worktree, set an absolute path once: in the shell profile, as above, or in `~/.cargo/config.toml`, which cargo passes to the build scripts:

```toml
[env]
LINDERA_BUILD_DICTIONARY_CACHE_DIR = "/path/to/cache"
```

Changing the value under `[env]` reruns the build scripts, as changing the variable does, and a variable set in the shell takes precedence over `[env]`.

Notes:

- The directory is managed automatically and is safe to delete: a crate already built keeps the dictionary it linked or copied, and the next build that needs a dictionary downloads or builds it again
- Version subdirectories accumulate across upgrades and are not garbage-collected; old ones can be removed freely
- Setting this variable causes dictionary crates to download and build their dictionaries even when no `embed-*` feature is enabled (useful for pre-populating the cache). Such a build embeds nothing, so it keeps a dictionary already in the cache without checking it; a build with the `embed-*` feature checks it and rebuilds it if needed
- A relative path resolves under the workspace root, which, in this repository, gives each checkout its own cache; set an absolute path to share one
- Checkouts whose builders differ, or whose builds differ in their inputs (`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`, for example), replace each other's cached dictionaries when their build scripts run, which costs a download or a build each time, but does not change what an already built crate embeds
- A cache kept between CI runs, for example with `actions/cache`, needs a key that changes with `Cargo.lock` as well as with `lindera-dictionary` and the dictionary crate, since the versions of the builder's dependencies are not compared. This repository's CI keeps one entry per dictionary under such a key (`.github/actions/dictionary-cache`)
- Builds that share the directory, such as `cargo build` alongside rust-analyzer with its own target directory, write each dictionary one at a time: a build that finds another build writing the same dictionary waits for it, and then uses the dictionary it left (`cargo build -vv` shows `lindera dictionary cache: waiting for another build`). The lock is an empty `<dictionary>.lock` file next to the dictionary, held while a build writes it. It works between builds on one machine, and on a network file system only as far as the file system supports file locks. Where the file cannot be locked, the build prints a warning and goes on without the lock, as builds before 7.0.0 did, so two such builds can still remove each other's files
- Builds with `DOCS_RS` set ignore this variable (see `DOCS_RS` below)

### LINDERA_BUILD_DICTIONARY_FROM_SOURCE

The seven official dictionaries are built once for each change of their inputs and published as GitHub releases, one release per dictionary and version, which are never replaced. Each dictionary crate pins its published dictionary, by URL and SHA-256, in the `prebuilt.json` next to its `metadata.json`, and its build script downloads that archive, checks its SHA-256, and embeds it, instead of downloading the source archive and building the dictionary. The archive holds the same files as a build from source; only the `lindera_version` in its `metadata.json` names the Lindera version that built it. How the dictionaries are published is in [Dictionary Release](../development/dictionary_release.md).

A failed download, an archive with another SHA-256, or one that lacks a file or does not fit the crate's `metadata.json`, stops the build with an error that says what went wrong; the build does not embed anything it could not check, and does not build from source instead.

The build script builds the dictionary from source, as up to v6, in these cases:

- `LINDERA_BUILD_DICTIONARY_FROM_SOURCE` is `1` or `true`. `0`, `false` and the empty value leave the choice to the build script; any other value counts as `1`, with a warning.
- `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` names a histogram, with which the dictionary differs from the published one.
- The published dictionary has another dictionary format version than this build reads, as on the development branch between a change of the format and the publication of the new dictionaries.
- `prebuilt.json` pins no SHA-256 yet, while a new dictionary waits to be published.

In the first three cases, the build script says why in a warning, such as:

```text
warning: lindera-ipadic@7.0.0: building the ipadic dictionary from source, because LINDERA_BUILD_DICTIONARY_FROM_SOURCE is set
```

Cargo shows these warnings for a dictionary crate built from a path, such as a checkout of this repository; for one from crates.io, it shows them only when the build fails or runs with `cargo build -vv`. Changing the variable, or `prebuilt.json`, reruns the build script. Builds with `DOCS_RS` set build neither: they use dummy dictionaries (see `DOCS_RS` below).

```shell
LINDERA_BUILD_DICTIONARY_FROM_SOURCE=1 cargo build --features=embed-ipadic
```

### LINDERA_BUILD_CONTEXT_ID_FREQ_FILE

The `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` environment variable names a context-ID frequency histogram that the dictionary crates' build scripts read in place of the `context_id_freq.txt` bundled with the crate. It applies only to a dictionary whose `metadata.json` enables `connection_id_mapping`, as those of UniDic and ko-dic do among the bundled dictionaries, and has no effect at runtime. The histogram orders the context IDs of the connection cost matrix so that frequent ones sit close together, which changes the speed of tokenization but never its result; the `ctxfreq_dump` example of the `lindera` crate writes one from a corpus.

```shell
export LINDERA_BUILD_CONTEXT_ID_FREQ_FILE=/path/to/context_id_freq.txt
cargo build --features=embed-unidic
```

Only the build scripts read it: `lindera build` takes the histogram from its `--context-id-freq` option, and the functions of the bindings that build a dictionary take it as an optional argument, `context_id_freq` (`contextIdFreq` in Node.js). Changing the variable, or the file it names, reruns the build script, and the cache of `LINDERA_BUILD_DICTIONARY_CACHE_DIR` compares the histogram's contents. With the variable set, the build script builds the dictionary from source rather than downloading the published one (see [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source)).

`LINDERA_CTX_FREQ_FILE`, the name of this variable up to v6, still works until v8.0.0, with a deprecation warning:

```text
warning: lindera-ko-dic@7.0.0: LINDERA_CTX_FREQ_FILE is deprecated and will be removed in v8.0.0; set LINDERA_BUILD_CONTEXT_ID_FREQ_FILE instead
```

When both are set, `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` is used, and the warning says that `LINDERA_CTX_FREQ_FILE` is ignored. Cargo shows these warnings for a dictionary crate built from a path, such as a checkout of this repository; for one from crates.io, it shows them only when the build fails or runs with `cargo build -vv`.

### DOCS_RS

The `DOCS_RS` environment variable is automatically set by docs.rs when building documentation. When this variable is detected, Lindera creates dummy dictionary files instead of downloading actual dictionary data, allowing documentation to be built without network access or large file downloads.

Such a build ignores `LINDERA_BUILD_DICTIONARY_CACHE_DIR`: the dummy dictionaries are built in the build script's output directory, never in the cache, so a later build without `DOCS_RS` does not embed them.

This is primarily used internally by docs.rs and typically doesn't need to be set by users.

## Runtime variables

### LINDERA_CONFIG_FILE

The `LINDERA_CONFIG_FILE` environment variable names a YAML configuration file for the tokenizer, which `TokenizerBuilder::new()` reads. This allows you to configure tokenizer behavior without modifying Rust code.

```shell
export LINDERA_CONFIG_FILE=./resources/config/lindera.yml
```

`LINDERA_CONFIG_PATH`, the name of this variable up to v6, still works until v8.0.0, with a deprecation warning on standard error. When both are set, `LINDERA_CONFIG_FILE` is used.

See the [Configuration](../lindera-analysis/configuration.md) section for details on the configuration format.

## CLI variables

The `lindera` command reads these variables; see [Commands](../lindera-cli/commands.md) of the Lindera CLI for the commands.

### LINDERA_DICTIONARIES_DIR

The `LINDERA_DICTIONARIES_DIR` environment variable names the dictionaries directory: `lindera download <name>` installs the dictionary in `<dictionaries directory>/<name>/`, `lindera list` lists the dictionaries there, and the error for a dictionary name given to `--dict` or `--system-dict` names the path there. Without it, the directory is `lindera/dictionaries` in the OS's local data directory:

| OS | Default location |
| --- | --- |
| Linux | `~/.local/share/lindera/dictionaries` |
| macOS | `~/Library/Application Support/lindera/dictionaries` |
| Windows | `%LOCALAPPDATA%\lindera\dictionaries` |

For example, with `LINDERA_DICTIONARIES_DIR=/var/lindera/dict`, `lindera download ipadic` installs to `/var/lindera/dict/ipadic/`. `lindera list` lists the dictionaries in the same directory.

`LINDERA_DATA_DIR`, the variable of the CLI before v7.0.0, keeps working until v8.0.0, with a warning on stderr: a value `<dir>` is read as `LINDERA_DICTIONARIES_DIR=<dir>/dictionaries`. For example, with `LINDERA_DATA_DIR=/home/user/.local/share/lindera`:

```text
Warning: LINDERA_DATA_DIR is deprecated and will be removed in v8.0.0; set LINDERA_DICTIONARIES_DIR=/home/user/.local/share/lindera/dictionaries instead
```

When both are set, `LINDERA_DICTIONARIES_DIR` is used, and the warning says that `LINDERA_DATA_DIR` is ignored. Neither variable has to do with the build-time source cache variable `LINDERA_BUILD_DICTIONARY_CACHE_DIR`, or with `LINDERA_DICTIONARIES_PATH`, its former name, which v7.0.0 removed.

### RUST_LOG

The `RUST_LOG` environment variable selects the messages that the CLI writes to standard error (see [Warnings and log messages](../lindera-cli/commands.md#warnings-and-log-messages)), in the syntax of [env_logger](https://docs.rs/env_logger/0.11.11/env_logger/#enabling-logging):

- Not set: warnings and errors.
- `RUST_LOG=error`: errors only, which hides the warnings. `RUST_LOG=off` writes no log messages at all.
- `RUST_LOG=debug`: also debugging messages, such as the files `lindera build` reads and writes.

The setting does not change standard output, and the error of a command that fails is written whatever the setting. The messages are colored when standard error is a terminal; `RUST_LOG_STYLE=never` turns the colors off.

## Ruby gem build

### LINDERA_FEATURES

The build of the Ruby gem, by `gem install lindera` or `bundle exec rake compile`, reads `LINDERA_FEATURES`, a comma-separated list of the Cargo features to enable. Without it, the gem builds with the default features (`train`) and no embedded dictionary.

```bash
LINDERA_FEATURES="embed-ipadic" gem install lindera
```

See [Feature Flags](../lindera-ruby/installation.md#feature-flags) of the Ruby binding for the features.
