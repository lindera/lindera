# Installation

`lindera-dependency` is on crates.io from Lindera 7.1.0 on. Most programs do not depend on it directly: they use the token filter `japanese_dependency` of the Lindera tokenizer, which the `lindera` crate, the `lindera` command and the bindings carry.

## In a tokenizer

The filter is behind the `dependency` feature of `lindera`:

```toml
[dependencies]
lindera = { version = "7.1", features = ["dependency"] }
```

The `lindera` command and the bindings (Python, Node.js, Ruby, PHP, WebAssembly) have the feature in their default builds, from 7.1.0 on: the packages you install have the filter. See [Token Filter](./token_filter.md), and the installation page of each: [command](../lindera-cli/installation.md), [Python](../lindera-python/installation.md), [Node.js](../lindera-nodejs/installation.md), [Ruby](../lindera-ruby/installation.md), [PHP](../lindera-php/installation.md), [WebAssembly](../lindera-wasm/installation.md).

A build of the command without the default features knows the name of the filter and says what is missing:

```text
Error: LinderaError { kind: FeatureDisabled, source: the token filter japanese_dependency needs a build with the feature `dependency` }
```

## The crate

```toml
[dependencies]
lindera-dependency = "7.1"
```

The default build has no dependency and builds for `wasm32-unknown-unknown`: it takes any sequence of morphemes, from Lindera or not. The [feature flags](./architecture.md#feature-flags) add the training and the embedded model.

The crate is a library. Its command, which parses plain text and writes the lattice format, is `lindera dependency`, a part of the `lindera` command, in its default build:

```shell
cargo install lindera-cli
```

## The model

The analysis needs a [model file](./model_file.md) for the dictionary of the tokenizer. Lindera publishes four with each release: one for IPADIC ([The model for IPADIC](./model_file.md#the-model-for-ipadic)) and one for each of `unidic`, `unidic-cwj` and `unidic-csj` ([The models for UniDic](./model_file.md#the-models-for-unidic)). A model is not part of a package or of a crate, as the dictionaries are not. There are three ways to get one, shown here for IPADIC; for a UniDic dictionary, its name takes the place of `ipadic`:

- `lindera download dependency-ipadic` installs the model of the version of the command, and prints the path of the model file ([`download`](../lindera-cli/commands.md#downloading-the-model-of-the-dependency-parser)).
- `lindera-dependency-ipadic-<version>.zip`, on the page of a [release](https://github.com/lindera/lindera/releases), holds the model file `dependency.model` and its `NOTICE.txt`.
- The feature `embed-dependency-ipadic` of `lindera`, of the command and of the bindings (`embed-model-ipadic` of `lindera-dependency`) embeds it in the build ([Embedding it](./model_file.md#embedding-it)).

The model is under the MIT license, and comes with a notice ([License of the model](./model_file.md#license-of-the-model)).
