# Build

This page describes how to build the SudachiDict dictionary from source files.

## Build system dictionary

Download the SudachiDict source archive and build the dictionary:

```shell
% curl -L -o /tmp/sudachidict-20260723.tar.gz "https://lindera.dev/sudachidict-20260723.tar.gz"
% tar zxvf /tmp/sudachidict-20260723.tar.gz -C /tmp

% lindera build \
  --src /tmp/sudachidict-20260723 \
  --dest /tmp/lindera-sudachidict \
  --metadata ./lindera-sudachidict/metadata.json
```

The archive bundles the raw lexicons (small + core + notcore), the connection matrix, and pre-aligned `char.def` / `unk.def`, so no manual preprocessing is needed. The built dictionary is about 570MB.

> [!TIP]
> To build from a newer upstream SudachiDict release than the bundled one, see
> [SudachiDict (Custom Build)](../sudachidict.md), which builds directly from
> the upstream raw distribution.

## Build user dictionary

Build a user dictionary from a CSV file:

```shell
% lindera build \
  --src ./resources/user_dict/sudachidict_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-sudachidict/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the SudachiDict dictionary directly into the binary:

```shell
cargo build --features=embed-sudachidict
```

This allows using `embedded://sudachidict` as the dictionary path without external dictionary files.

> [!NOTE]
> The embedded dictionary adds roughly 570MB to the binary. If binary size
> matters, prefer building the dictionary to a directory as shown above and
> loading it by path.
