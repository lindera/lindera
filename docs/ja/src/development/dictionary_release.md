# 辞書のリリース

Lindera のリリースは、公式の 9 つの辞書（IPADIC、IPADIC NEologd、UniDic、UniDic CWJ、UniDic CSJ、SudachiDict、ko-dic、CC-CEDICT、Jieba）をソースからビルドし、そのバージョンのリリースのページに、バイナリと一緒に置きます（[#1362](https://github.com/lindera/lindera/issues/1362)）。`lindera download` はこのアーカイブをインストールし、crates.io の辞書クレートの `embed-*` のビルドは、辞書をソースからビルドする代わりにこのアーカイブを埋め込みます（[#1336](https://github.com/lindera/lindera/issues/1336)）。このページでは、公開するもの、ビルドが辞書を得る方法、リリースの手順を説明します。

公開して埋め込むのは、この 9 つの辞書だけです。利用者がビルドする辞書は埋め込みません。`lindera build` でビルドし、実行時にそのディレクトリから読み込みます。

## 公開するもの

リリース `v<バージョン>` には、辞書ごとに 1 つのアーカイブ `lindera-<名前>-<バージョン>.zip` があります。たとえば `v7.0.0` の `lindera-ipadic-7.0.0.zip` です。アーカイブには `lindera-<名前>` という名前のディレクトリが 1 つあり、辞書クレートが埋め込む 9 つのファイル（`char_def.bin`、`matrix.mtx`、`dict.trie`、`dict.valsidx`、`dict.vals`、`dict.wordsidx`、`dict.words`、`unk.bin`、`metadata.json`）と、クレートの `NOTICE.txt` が入っています。`metadata.json` の `lindera_version` は、そのリリースのバージョンです。`lindera download` は、入っている辞書の `lindera_version` が CLI のバージョンと同じときだけ最新とみなします。

アーカイブは、タグで `release.yml` の `build-dictionary` のジョブがビルドします。crates.io に公開した辞書クレートがアーカイブを固定しているので、リリースのアーカイブは置き換えも削除もしません。

## ビルドが辞書を得る方法

crates.io に公開した辞書クレートには、`metadata.json` の隣に `prebuilt.json` があります。リリースが、クレートを公開する直前に書いたものです。

```json
{
  "key": "lindera-ipadic",
  "format_version": 3,
  "base_url": "https://github.com/lindera/lindera/releases/download",
  "path": "v7.0.0/lindera-ipadic-7.0.0.zip",
  "sha256": "<アーカイブの SHA-256>"
}
```

そのクレートの `embed-*` のビルドは、アーカイブ（`base_url` と `path` を `/` でつないだ URL）をダウンロードし、`sha256` と照合します。さらに、`key` のディレクトリに 9 つのファイルがあり、`metadata.json` がクレートに合うことを確かめてから埋め込みます。ダウンロードや確かめに失敗したビルドは、確かめていないものを埋め込んだり、ソースからのビルドに切り替えたりせずに、エラーで止まります。

リポジトリには `prebuilt.json` がありません。リポジトリからのビルド（`main`、pull request、git の依存）は、すべての辞書をソースからビルドします。ビルドした辞書は `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に残せて、CI はそのディレクトリをキャッシュに保存します（[#1337](https://github.com/lindera/lindera/issues/1337)）。[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](../getting_started/environment_variables.md#lindera_build_dictionary_from_source) を設定すると、公開したクレートでもソースからビルドします。

## ソースのアーカイブ

5 つの辞書のソースのアーカイブは lindera.dev に置いています。UniDic、UniDic CWJ、UniDic CSJ のソースは、国立国語研究所のパッケージ `unidic-mecab-2.1.2_src.zip`（134 MiB）、`unidic-cwj-202512_full.zip`（2.77 GiB）、`unidic-csj-202512_full.zip`（2.81 GiB）そのもので、国立国語研究所の公式の URL から取得します（[#216](https://github.com/lindera/lindera/issues/216)、[#1358](https://github.com/lindera/lindera/issues/1358)、[#1384](https://github.com/lindera/lindera/issues/1384)）。ko-dic のソースは、mecab-ko-dic の公式のリリース `mecab-ko-dic-2.1.1-20180720.tar.gz`（47 MiB）そのもので、上流のリポジトリの Bitbucket の Downloads から取得します（[#1386](https://github.com/lindera/lindera/issues/1386)）。Lindera が運用していないサーバーから実行のたびにこれらのアーカイブを取得しないように、`release.yml` の `build-dictionary` のジョブは、それぞれのアーカイブを `actions/cache`（`.github/actions/source-archive-cache`）で実行をまたいで保存します。キーはファイル名と SHA-256 で、ダウンロードするのは、ソースのアーカイブが新しくなったときと、キャッシュが消されたときだけです。

## Lindera のリリース

1. pull request でバージョンを上げ（`scripts/bump-up-version.sh <バージョン>`）、マージします。
2. 必要なら、`main` で `release.yml` を手動で実行します（`workflow_dispatch`）。この dry run は、リリースの全テストと全ビルドを実行し、成果物を集めて、欠けがないことを確かめます（`scripts/check_release_assets.sh`）。アップロードも公開もしません。
3. リリースのタグ（`v<バージョン>`）を打ちます。`release.yml` は次の順に進めます。どの手順も、前の手順が通ってから始まります（[#1413](https://github.com/lindera/lindera/issues/1413)）。
   1. テストを実行し、すべての成果物をビルドします。辞書（上のとおりソースから）、CLI のバイナリ、wheel と sdist、gem、PIE のバイナリ、WASM のパッケージ、Node.js のバイナリです。
   2. リリースを下書きで作り、成果物を上げます。
   3. リリースを公開します。辞書のアーカイブに公開の URL が付きます。
   4. アーカイブをもう一度ダウンロードし、各辞書クレートの `prebuilt.json` に、そのアーカイブの SHA-256 を書きます（`scripts/write_dictionary_pins.sh`）。
   5. `lindera-ipadic` を `embed-ipadic` 付きでビルドし、固定したアーカイブのダウンロードと照合が通ることを確かめます。固定が働かないクレートを公開しないためです。
   6. クレートを公開します。辞書クレートの `prebuilt.json` は commit しないので、辞書クレートは `cargo publish --allow-dirty` で公開します。
   7. gem をインストールして、トークナイズを試します。gem は、公開したばかりのクレートでコンパイルされます。
   8. PyPI、npm、RubyGems に公開します。

失敗したジョブは、実行し直せます。レジストリにすでにあるバージョンは飛ばし、公開済みのリリースの成果物は置き換えません。

RubyGems には API キーで公開します。リリースのときは、アカウントの多要素認証のレベルを「UI and gem signin」にしておきます。「UI and API」では `gem push` がワンタイムパスワードを求め、ジョブが失敗します。

タグの前に公開や固定をする手順はありません。辞書、そのソースのアーカイブ、辞書フォーマットを変えても、それだけのための手順は要らず、次のリリースで公開したクレートに入ります。
