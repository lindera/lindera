# Dictionary Release

The nine official dictionaries (IPADIC, IPADIC NEologd, UniDic, UniDic CWJ, UniDic CSJ, SudachiDict, ko-dic, CC-CEDICT and Jieba) are built once for each change of their inputs and published, and an `embed-*` build embeds the published dictionary instead of building it from source ([#1336](https://github.com/lindera/lindera/issues/1336)). This page describes how they are published, and the steps of a Lindera release that depend on them. How a build chooses between the published dictionary and the build from source is in [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](../getting_started/environment_variables.md#lindera_build_dictionary_from_source).

Only these nine dictionaries are published and embedded. A dictionary that a user builds is not embedded: it is built with `lindera build` and loaded from its directory at runtime.

## What is published

Each published dictionary is an archive, `<key>.tar.gz`, attached to its own GitHub release under the tag `dictionary-<key>`. The archive holds one directory named `<key>`, with the nine files that the dictionary crate embeds (`char_def.bin`, `matrix.mtx`, `dict.trie`, `dict.valsidx`, `dict.vals`, `dict.wordsidx`, `dict.words`, `unk.bin` and `metadata.json`) and the crate's `NOTICE.txt`. A release is never replaced or deleted, because the dictionary crates published on crates.io pin its archive.

The key names the inputs of the dictionary:

```text
<name>-<version of the source archive>-fmt<dictionary format version>-r<revision>
```

For example, `ipadic-2.7.0-20250920-fmt3-r1` is the first IPADIC dictionary built from `mecab-ipadic-2.7.0-20250920.tar.gz` in the dictionary format version 3.

## The pin

Each dictionary crate pins its published dictionary in `prebuilt.json`, next to its `metadata.json`:

```json
{
  "key": "ipadic-2.7.0-20250920-fmt3-r1",
  "format_version": 3,
  "base_url": "https://github.com/lindera/lindera/releases/download",
  "path": "dictionary-ipadic-2.7.0-20250920-fmt3-r1/ipadic-2.7.0-20250920-fmt3-r1.tar.gz",
  "sha256": null
}
```

`sha256` is the SHA-256 of the archive. While it is `null`, the dictionary is built from source, which is how a new key waits for its dictionary to be published. The URL is `base_url` and `path` joined by `/`.

## When the key changes

A dictionary needs a new key, and so a new release, whenever the dictionary built from source would differ from the published one:

| Change | New key |
| --- | --- |
| A new source archive (a new `file_name` and `sha256` in `build.rs`) | Its version, and revision 1 |
| A new dictionary format version (`DICTIONARY_FORMAT_VERSION`) | `fmt<new version>` and `format_version`, and revision 1, for all nine dictionaries |
| Anything else that changes the built files: the builder, a dependency whose output it writes, the crate's `metadata.json` or `context_id_freq.txt` | The next revision |

The `Dictionaries` workflow (`.github/workflows/dictionaries.yml`) finds such a change. On a pull request or a push to `main` that changes `lindera-dictionary`, a dictionary crate or `Cargo.lock`, it builds the nine dictionaries from source on Linux, packs them as published archives, prints the SHA-256 of each archive and of each file in the summary of its jobs, and, when `prebuilt.json` pins a SHA-256, compares each built file with the published one: byte for byte, and `metadata.json` without `lindera_version`. A difference fails the job and asks for a new revision.

The source archives of seven dictionaries are hosted on lindera.dev. UniDic CWJ's and UniDic CSJ's are NINJAL's own packages, `unidic-cwj-202512_full.zip` (2.77 GiB) and `unidic-csj-202512_full.zip` (2.81 GiB), downloaded from NINJAL's official URL ([#216](https://github.com/lindera/lindera/issues/216), [#1358](https://github.com/lindera/lindera/issues/1358)). That server is not run by Lindera, so the workflow, and the `build-dictionary` job of `release.yml`, do not download these packages each time: they keep each archive between runs with `actions/cache` (`.github/actions/source-archive-cache`), keyed on its file name and SHA-256, and download it only for a new source archive, or after the cache entry has been evicted.

A build reading a pinned dictionary of another format version builds from source, so a pull request that changes the format builds and tests as before; the warning says so.

## Publishing a dictionary

1. In a pull request with the change, give each dictionary that changes its new key in `prebuilt.json` (with `format_version` when the format changes), and set its `sha256` to `null`. Until the dictionary is published, builds build it from source.
2. Once the pull request is merged, run the `Dictionaries` workflow on `main` with `publish`:

   ```shell
   gh workflow run dictionaries.yml --repo lindera/lindera --ref main -f publish=true
   ```

   The workflow builds the dictionaries again, creates a release for each key that has none yet, attaches the archive, and checks that GitHub reports the SHA-256 it uploaded. It never replaces a release: for a key that is published already, it reports the release, with a warning when the archive it built differs.
3. Take the SHA-256 of each new archive from the summary of the run, or from the release:

   ```shell
   gh release view dictionary-ipadic-2.7.0-20250920-fmt3-r1 --repo lindera/lindera --json assets --jq '.assets[].digest'
   ```

   Set it, without the `sha256:` that the release prefixes it with, as `sha256` in `prebuilt.json` in a pull request. Its `Dictionaries` run downloads the published dictionary and compares it with the build from source.

## Releasing Lindera

The dictionary crates on crates.io pin their published dictionaries, so the dictionaries must be published, and pinned, before the crates are:

1. Do not change the dictionary format until the release is out.
2. Make sure that each `prebuilt.json` pins a SHA-256, and that the latest `Dictionaries` run on `main` passed. Publish and pin any dictionary that is missing, as above.
3. Tag the release. `release.yml` publishes the crates, and builds the `lindera-<name>-<version>.zip` archives for `lindera download` from source, so that their `metadata.json` names the release's version as `lindera_version`: `lindera download` takes an installed dictionary for current only when that equals the version of the CLI.

For v7.0.0, the first release with published dictionaries ([#1085](https://github.com/lindera/lindera/issues/1085)): freeze the dictionary format, publish the nine dictionaries, merge the pull request that pins their SHA-256, then tag `v7.0.0`.
