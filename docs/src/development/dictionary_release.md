# Dictionary Release

A Lindera release builds the nine official dictionaries (IPADIC, IPADIC NEologd, UniDic, UniDic CWJ, UniDic CSJ, SudachiDict, ko-dic, CC-CEDICT and Jieba) from source and puts them on the page of its version, next to the binaries ([#1362](https://github.com/lindera/lindera/issues/1362)). `lindera download` installs these archives, and an `embed-*` build of a dictionary crate from crates.io embeds its archive instead of building the dictionary from source ([#1336](https://github.com/lindera/lindera/issues/1336)). This page describes what is published, how a build gets its dictionary, and the steps of a release.

Only these nine dictionaries are published and embedded. A dictionary that a user builds is not embedded: it is built with `lindera build` and loaded from its directory at runtime.

## What is published

The release `v<version>` holds one archive per dictionary, `lindera-<name>-<version>.zip`, for example `lindera-ipadic-7.0.0.zip` in `v7.0.0`. The archive holds one directory named `lindera-<name>`, with the nine files that the dictionary crate embeds (`char_def.bin`, `matrix.mtx`, `dict.trie`, `dict.valsidx`, `dict.vals`, `dict.wordsidx`, `dict.words`, `unk.bin` and `metadata.json`) and the crate's `NOTICE.txt`. Its `metadata.json` names the release's version as `lindera_version`: `lindera download` takes an installed dictionary for current only when that equals the version of the CLI.

The `build-dictionary` job of `release.yml` builds the archives on the tag. The dictionary crates published on crates.io pin them, so a release's archives are never replaced or deleted.

## How a build gets its dictionary

A dictionary crate published on crates.io has a `prebuilt.json` next to its `metadata.json`, which the release writes just before it publishes the crate:

```json
{
  "key": "lindera-ipadic",
  "format_version": 3,
  "base_url": "https://github.com/lindera/lindera/releases/download",
  "path": "v7.0.0/lindera-ipadic-7.0.0.zip",
  "sha256": "<SHA-256 of the archive>"
}
```

An `embed-*` build of that crate downloads the archive (`base_url` and `path` joined by `/`), checks it against `sha256`, checks that it holds the directory `key` with the nine files and a `metadata.json` that fits the crate, and embeds it. A build that cannot download or check it stops with an error, rather than embedding anything unchecked or falling back to the source.

The repository has no `prebuilt.json`. A build from the repository (`main`, a pull request, a git dependency) builds every dictionary from source; `LINDERA_BUILD_DICTIONARY_CACHE_DIR` keeps the built dictionaries between builds, and CI keeps that directory in its cache ([#1337](https://github.com/lindera/lindera/issues/1337)). [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](../getting_started/environment_variables.md#lindera_build_dictionary_from_source) makes a published crate build from source too.

## Source archives

The source archives of seven dictionaries are hosted on lindera.dev. UniDic CWJ's and UniDic CSJ's are NINJAL's own packages, `unidic-cwj-202512_full.zip` (2.77 GiB) and `unidic-csj-202512_full.zip` (2.81 GiB), downloaded from NINJAL's official URL ([#216](https://github.com/lindera/lindera/issues/216), [#1358](https://github.com/lindera/lindera/issues/1358)). That server is not run by Lindera, so the `build-dictionary` job of `release.yml` does not download these packages each time: it keeps each archive between runs with `actions/cache` (`.github/actions/source-archive-cache`), keyed on its file name and SHA-256, and downloads it only for a new source archive, or after the cache entry has been evicted.

## Releasing Lindera

1. Raise the version (`scripts/bump-up-version.sh <version>`) in a pull request, and merge it.
2. Tag the release (`v<version>`). `release.yml` then:
   1. builds the dictionaries from source and uploads their archives to the release, as above;
   2. downloads them again, and writes the `prebuilt.json` of each dictionary crate with its archive's SHA-256 (`scripts/write_dictionary_pins.sh`);
   3. builds `lindera-ipadic` with `embed-ipadic`, which downloads and checks its pinned archive, so that no crate is published with a pin that does not work;
   4. publishes the crates; the dictionary crates with `cargo publish --allow-dirty`, as their `prebuilt.json` is not committed.

Nothing is published or pinned before the tag: a change to a dictionary, to its source archive or to the dictionary format needs no step of its own, and reaches the published crates with the next release.
