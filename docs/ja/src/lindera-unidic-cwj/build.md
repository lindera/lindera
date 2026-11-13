# ビルド

このページでは、ソースファイルから UniDic CWJ 辞書をビルドする方法を説明します。

## システム辞書のビルド

国立国語研究所のフルパッケージをダウンロードし、ビルダーが読む 4 つのファイルを取り出して、辞書をビルドします。軽量版の `unidic-cwj-202512.zip` には MeCab がコンパイルしたファイルしかなく、そこからはビルドできません:

```shell
% curl -L -o /tmp/unidic-cwj-202512_full.zip "https://clrd.ninjal.ac.jp/unidic_archive/2512/unidic-cwj-202512_full.zip"
% shasum -a 256 /tmp/unidic-cwj-202512_full.zip
ca02ad51c8ae238373dcb150eec014f1ad9f05f54c7416a16c2fe444fd4921b3  /tmp/unidic-cwj-202512_full.zip
% unzip /tmp/unidic-cwj-202512_full.zip lex.csv matrix.def char.def unk.def -d /tmp/unidic-cwj-202512

% lindera build \
  --src /tmp/unidic-cwj-202512 \
  --dest /tmp/lindera-unidic-cwj-202512 \
  --metadata ./lindera-unidic-cwj/metadata.json \
  --context-id-freq ./lindera-unidic-cwj/context_id_freq.txt
```

パッケージは 2.77 GiB、取り出す 4 つのファイルは合わせて 7.95 GiB で、そのうち 7.70 GiB が `matrix.def` です。ビルダーは `matrix.def` を 64 MiB ずつ読むので、ファイル全体ではなく、982 MiB の行列と 1 ブロックだけを持ちます。辞書の組み立てで確保したメモリは、macOS で最大 2.48 GiB でした。

`lindera-unidic-cwj/metadata.json` は `connection_id_mapping: true` です。ビルダーは、連接コストの行列の文脈 ID を、解析で参照される回数の多い順に付け替え、よく参照するセルをメモリ上で近くに集めます。`--context-id-freq` / `-f` には、クレートに同梱の `context_id_freq.txt` を渡します。この頻度は、`ctxfreq_dump` の例で `resources/bocchan.txt` を解析して数えたものです。渡さなくても組み立ては成功しますが、[UniDic](../lindera-unidic/build.md) と同じく、精度の低いエントリ数ベースの順位に切り替わり、警告を出します。どちらの場合も、付け替えはコストを変えないので、解析結果は同じです。

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします。簡易版のフォーマットは UniDic 2.1.2 と同じです:

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic-cwj/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

UniDic CWJ 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-unidic-cwj
```

これにより、外部辞書ファイルなしで `embedded://unidic-cwj` を辞書パスとして使用できるようになります。実行ファイルは 1.2 GB の辞書を含み、コンパイルには 11 GiB ほどのメモリを使います。
