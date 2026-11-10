# 辞書のリリース

公式の 7 つの辞書（IPADIC、IPADIC NEologd、UniDic、SudachiDict、ko-dic、CC-CEDICT、Jieba）は、入力が変わるたびに一度だけビルドして公開し、`embed-*` のビルドは、辞書をソースからビルドする代わりに公開済みの辞書を埋め込みます（[#1336](https://github.com/lindera/lindera/issues/1336)）。このページでは、辞書の公開のしかたと、それに関わる Lindera のリリースの手順を説明します。ビルドが公開済みの辞書とソースからのビルドのどちらを使うかは、[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](../getting_started/environment_variables.md#lindera_build_dictionary_from_source) にあります。

公開して埋め込むのは、この 7 つの辞書だけです。利用者がビルドする辞書は埋め込みません。`lindera build` でビルドし、実行時にそのディレクトリから読み込みます。

## 公開するもの

公開する辞書はそれぞれアーカイブ `<key>.tar.gz` で、タグ `dictionary-<key>` の専用の GitHub のリリースに添付します。アーカイブには `<key>` という名前のディレクトリが 1 つあり、辞書クレートが埋め込む 9 つのファイル（`char_def.bin`、`matrix.mtx`、`dict.trie`、`dict.valsidx`、`dict.vals`、`dict.wordsidx`、`dict.words`、`unk.bin`、`metadata.json`）と、クレートの `NOTICE.txt` が入っています。crates.io に公開した辞書クレートがアーカイブを固定しているので、リリースは置き換えも削除もしません。

鍵（key）は、辞書の入力を表す名前です。

```text
<名前>-<ソースのアーカイブの版>-fmt<辞書フォーマットバージョン>-r<改訂番号>
```

たとえば `ipadic-2.7.0-20250920-fmt3-r1` は、`mecab-ipadic-2.7.0-20250920.tar.gz` から辞書フォーマットバージョン 3 でビルドした、最初の IPADIC の辞書です。

## 固定

各辞書クレートは、`metadata.json` の隣の `prebuilt.json` で、公開済みの辞書を固定します。

```json
{
  "key": "ipadic-2.7.0-20250920-fmt3-r1",
  "format_version": 3,
  "base_url": "https://github.com/lindera/lindera/releases/download",
  "path": "dictionary-ipadic-2.7.0-20250920-fmt3-r1/ipadic-2.7.0-20250920-fmt3-r1.tar.gz",
  "sha256": null
}
```

`sha256` はアーカイブの SHA-256 です。`null` の間は、辞書をソースからビルドします。新しい鍵は、こうして辞書の公開を待ちます。URL は、`base_url` と `path` を `/` でつないだものです。

## 鍵を変えるとき

ソースからビルドした辞書が公開済みのものと変わるときは、その辞書に新しい鍵、つまり新しいリリースが要ります。

| 変更 | 新しい鍵 |
| --- | --- |
| 新しいソースのアーカイブ（`build.rs` の新しい `file_name` と `sha256`） | その版と、改訂番号 1 |
| 新しい辞書フォーマットバージョン（`DICTIONARY_FORMAT_VERSION`） | 7 つの辞書すべての `fmt<新しい版>` と `format_version`、改訂番号 1 |
| ビルドしたファイルを変えるそのほかの変更（ビルダー、ビルダーが出力をそのまま書く依存クレート、クレートの `metadata.json` や `context_id_freq.txt`） | 次の改訂番号 |

この変更は `Dictionaries` のワークフロー（`.github/workflows/dictionaries.yml`）が見つけます。`lindera-dictionary`、辞書クレート、`Cargo.lock` を変える pull request と `main` への push で、7 つの辞書を Linux でソースからビルドし、公開する形のアーカイブにまとめ、アーカイブと各ファイルの SHA-256 をジョブの summary に出します。`prebuilt.json` が SHA-256 を固定していれば、ビルドした各ファイルを公開済みのものと比べます。比べ方はバイト単位で、`metadata.json` だけは `lindera_version` を除きます。違いがあればジョブは失敗し、新しい改訂番号を求めます。

固定した辞書のフォーマットバージョンが違うビルドは、ソースからビルドします。そのため、フォーマットを変える pull request も、これまでどおりビルドしてテストでき、そのことは警告で示されます。

## 辞書を公開する

1. 変更を入れる pull request で、変わる辞書の `prebuilt.json` に新しい鍵を書き（フォーマットを変えるなら `format_version` も）、`sha256` を `null` にします。公開されるまでの間、ビルドはその辞書をソースからビルドします。
2. pull request をマージしたら、`main` で `Dictionaries` のワークフローを `publish` 付きで動かします。

   ```shell
   gh workflow run dictionaries.yml --repo lindera/lindera --ref main -f publish=true
   ```

   ワークフローは辞書をもう一度ビルドし、まだリリースのない鍵ごとにリリースを作ってアーカイブを添付し、GitHub が報告する SHA-256 がアップロードしたものと同じかを確かめます。リリースを置き換えることはしません。公開済みの鍵については、そのリリースを報告し、今回ビルドしたアーカイブが違えば警告を出します。
3. 新しいアーカイブの SHA-256 を、実行の summary かリリースから取ります。

   ```shell
   gh release view dictionary-ipadic-2.7.0-20250920-fmt3-r1 --repo lindera/lindera --json assets --jq '.assets[].digest'
   ```

   リリースが前に付ける `sha256:` を除いて、pull request で `prebuilt.json` の `sha256` に書きます。その pull request の `Dictionaries` の実行が、公開済みの辞書をダウンロードし、ソースからのビルドと比べます。

## Lindera のリリース

crates.io の辞書クレートは公開済みの辞書を固定するので、クレートを公開する前に、辞書を公開して固定しておく必要があります。

1. リリースが終わるまで、辞書フォーマットを変えません。
2. すべての `prebuilt.json` が SHA-256 を固定していることと、`main` の最新の `Dictionaries` の実行が成功していることを確かめます。足りない辞書は、上の手順で公開して固定します。
3. リリースのタグを打ちます。`release.yml` はクレートを公開し、`lindera download` 用の `lindera-<名前>-<版>.zip` をソースからビルドします。`lindera download` は、入っている辞書の `metadata.json` の `lindera_version` が CLI の版と同じときだけ最新とみなすので、zip の `lindera_version` をリリースの版にするためです。

公開済みの辞書を使う最初のリリースである v7.0.0 では（[#1085](https://github.com/lindera/lindera/issues/1085)）、辞書フォーマットを凍結し、7 つの辞書を公開し、その SHA-256 を固定する pull request をマージしてから、`v7.0.0` のタグを打ちます。
