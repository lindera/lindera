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

The source archives of five dictionaries are hosted on lindera.dev. UniDic's, UniDic CWJ's and UniDic CSJ's are NINJAL's own packages, `unidic-mecab-2.1.2_src.zip` (134 MiB), `unidic-cwj-202512_full.zip` (2.77 GiB) and `unidic-csj-202512_full.zip` (2.81 GiB), downloaded from NINJAL's official URL ([#216](https://github.com/lindera/lindera/issues/216), [#1358](https://github.com/lindera/lindera/issues/1358), [#1384](https://github.com/lindera/lindera/issues/1384)). The source of ko-dic is mecab-ko-dic's official release, `mecab-ko-dic-2.1.1-20180720.tar.gz` (47 MiB), downloaded from the Downloads of its upstream repository on Bitbucket ([#1386](https://github.com/lindera/lindera/issues/1386)). Those servers are not run by Lindera, so the `build-dictionary` job of `release.yml` does not download these archives each time: it keeps each archive between runs with `actions/cache` (`.github/actions/source-archive-cache`), keyed on its file name and SHA-256, and downloads it only for a new source archive, or after the cache entry has been evicted.

## The model of lindera-dependency

A release publishes one more archive the same way: the model of [`lindera-dependency`](../lindera-dependency/model_file.md#the-model-for-ipadic) for IPADIC, `lindera-dependency-ipadic-<version>.zip` ([#1440](https://github.com/lindera/lindera/issues/1440)). It holds one directory named `lindera-dependency-ipadic`, with `dependency.model`, `model.json`, which names the release's version as `lindera_version`, and `NOTICE.txt`.

The `build-dependency-model` job of `release.yml` makes it, with `scripts/dependency/build_model.sh`: it gets UD Japanese-GSD and UD Japanese-PUD at their fixed commits from GitHub, trains the model on the train file, and measures it on the files it was not trained on (`scripts/dependency/model_accuracy.py`). The counts have to be those of `scripts/dependency/model_accuracy_ipadic.json`, which are the numbers of the documentation: a release whose model measures differently stops there, before anything is uploaded. The bytes of the model are those of the runner; they are not promised to be those of another machine.

`lindera-dependency` pins the archive as a dictionary crate does: the release writes its `prebuilt.json` (`scripts/write_dependency_model_pin.sh`), and a build of the published crate with `embed-model-ipadic` downloads the archive and checks it. There is one difference. A dictionary crate without a pin builds its dictionary from source; the model cannot be trained by a build script, so a build without a pin, which is every build from the repository, embeds the file that [`LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`](../getting_started/environment_variables.md#lindera_build_dependency_model_ipadic_file) names.

## Releasing Lindera

1. Raise the version (`scripts/bump-up-version.sh <version>`) in a pull request, and merge it.
2. Optionally, run `release.yml` by hand on `main` (`workflow_dispatch`). This dry run runs every test and every build of a release, gathers the assets and checks that none is missing (`scripts/check_release_assets.sh`). It uploads and publishes nothing.
3. Tag the release (`v<version>`). `release.yml` then goes through these steps, each only after the one before has passed ([#1413](https://github.com/lindera/lindera/issues/1413)):
   1. runs the tests and builds every asset: the dictionaries from source, as above, the model of `lindera-dependency`, the CLI binaries, the wheels and the sdist, the gem, the PIE binaries, the WASM package and the Node.js binaries;
   2. creates the release as a draft, and uploads the assets to it;
   3. publishes the release, so that the dictionary archives have their public URLs;
   4. downloads the archives again, and writes the `prebuilt.json` of each dictionary crate with its archive's SHA-256 (`scripts/write_dictionary_pins.sh`), and the one of `lindera-dependency` with the SHA-256 of the archive of the model (`scripts/write_dependency_model_pin.sh`);
   5. builds `lindera-ipadic` with `embed-ipadic` and `lindera-dependency` with `embed-model-ipadic`, which download and check their pinned archives, so that no crate is published with a pin that does not work;
   6. publishes the crates; the dictionary crates and `lindera-dependency` with `cargo publish --allow-dirty`, as their `prebuilt.json` is not committed;
   7. installs the gem, which compiles against the crates just published, and tokenizes with it;
   8. publishes to PyPI, npm and RubyGems.

A job that failed can be run again: a version that a registry already has is skipped, and the assets of a release that is already public are left as they are.

RubyGems is published with an API key. The account's multi-factor authentication level has to be "UI and gem signin" for a release: with "UI and API", `gem push` asks for a one-time password, and the job fails.

Nothing is published or pinned before the tag: a change to a dictionary, to its source archive or to the dictionary format needs no step of its own, and reaches the published crates with the next release.
