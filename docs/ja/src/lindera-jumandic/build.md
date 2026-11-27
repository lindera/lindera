# ビルド

このページでは、ソースファイルから JUMAN 辞書をビルドする方法を説明します。

## システム辞書のビルド

ソースは、MeCab のリポジトリの `mecab-jumandic/` です。クレートがビルドに使う commit のアーカイブをダウンロードし、辞書をビルドします:

```shell
# MeCab のリポジトリのダウンロードと、mecab-jumandic の展開
% curl -L -o /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz \
  "https://github.com/taku910/mecab/archive/61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz"
% tar zxvf /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz -C /tmp \
  mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic

# 辞書のビルド
% lindera build \
  --src /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic \
  --dest /tmp/lindera-jumandic-7.0-20130310 \
  --metadata ./lindera-jumandic/metadata.json
```

アーカイブは 45 MiB で、SHA-256 は `7ad44f987ae0b7fd345c72a4b67e14dbe0f4bd1669d247a8aaeb0f15218a3fd1` です。クレートのビルドスクリプトは、この値で照合します。ビルドした辞書は 125 MB で、751,179 語です。

## 飛ばされる 6 行

ビルドは、6 行について警告を出します:

```text
[WARN  lindera_dictionary::builder::prefix_dictionary] Skipping a lexicon entry: /tmp/mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1/mecab-jumandic/AuxV.csv: line 588: it is not UTF-8 (invalid utf-8: invalid UTF-8 in field 0 near byte index 3)
```

`AuxV.csv` の 588〜593 行目は、6 つの助動詞（無活用型）で、表層・原形・読みの最後の文字が、3 バイトのうち 2 バイトで切れています。mecab-jumandic 7.0-20130310 の公式の配布物も、同じバイト列です。辞書が作られたときに壊れた行で、元の語が何だったかは分かりません。MeCab は行をバイト列として受け入れるので、この 6 語は、どのテキストにも現れない表層の語として MeCab の辞書に入ります。

UTF-8 の辞書は、UTF-8 として読めない行からは作られません。ビルドが失敗します。文字コードの指定（`encoding`）の誤りには、これで気づけます。この辞書の `metadata.json` は、`"skip_invalid_encoding": true` で、そういう行を飛ばすように指定しています。

`skip_invalid_encoding` は、指定しなければ `false` です。自作の辞書の `metadata.json` で指定するのは、ソースにそういう行があると分かっているときだけにしてください。指定すると、`encoding` を誤ったときに、失敗せずに、警告を出して行を落とすようになります。UTF-8 以外の文字コードでは、その文字コードの文字にならないバイト列のある行を飛ばします。指定しなければ、そのバイト列は U+FFFD に置き換わります。

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします:

```shell
% lindera build \
  --src ./resources/user_dict/jumandic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-jumandic/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

JUMAN 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-jumandic
```

これにより、外部辞書ファイルなしで `embedded://jumandic` を辞書パスとして使用できるようになります。`lindera-jumandic` のビルドスクリプトが、上のアーカイブをダウンロードして辞書をビルドします。

## 学習

この形式の辞書を `lindera train` で学習することは、試していません。JUMAN の品詞の学習用コーパスが、ここには無いためです。
