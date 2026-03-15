# Tutorial

This tutorial walks you through the basic usage of the Lindera CLI, from installation to advanced text processing.

## 1. Install the CLI

Install Lindera CLI with the embedded IPADIC dictionary:

```shell
% cargo install lindera-cli --features=embed-ipadic
```

Verify the installation:

```shell
% lindera --help
```

## 2. Basic tokenization with embedded dictionary

Tokenize Japanese text using the embedded IPADIC dictionary:

```shell
% echo "東京は日本の首都です。" | lindera tokenize \
  --dict embedded://ipadic
```

Expected output:

```text
東京    名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
日本    名詞,固有名詞,地域,国,*,*,日本,ニホン,ニホン
の      助詞,連体化,*,*,*,*,の,ノ,ノ
首都    名詞,一般,*,*,*,*,首都,シュト,シュト
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

## 3. Try different output formats

### Wakati format (word segmentation only)

```shell
% echo "東京は日本の首都です。" | lindera tokenize \
  --dict embedded://ipadic \
  --output wakati
```

Expected output:

```text
東京 は 日本 の 首都 です 。
```

### JSON format (detailed information)

```shell
% echo "東京は日本の首都です。" | lindera tokenize \
  --dict embedded://ipadic \
  --output json
```

This produces a JSON array with detailed token information including byte offsets, part-of-speech tags, readings, and more.

## 4. Use decompose mode

Decompose mode splits compound nouns into their constituent parts:

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize \
  --dict embedded://ipadic \
  --mode decompose
```

Expected output:

```text
関西    名詞,固有名詞,地域,一般,*,*,関西,カンサイ,カンサイ
国際    名詞,一般,*,*,*,*,国際,コクサイ,コクサイ
空港    名詞,一般,*,*,*,*,空港,クウコウ,クーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
EOS
```

Compare with normal mode, where "関西国際空港" remains as a single token.

## 5. Apply character and token filters

Use Unicode normalization and keep only common nouns:

```shell
% echo "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。" | lindera tokenize \
  --dict embedded://ipadic \
  --char-filter 'unicode_normalize:{"kind":"nfkc"}' \
  --token-filter 'japanese_keep_tags:{"tags":["名詞,一般","名詞,固有名詞,組織"]}'
```

Expected output:

```text
Lindera 名詞,固有名詞,組織,*,*,*,*,*,*
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
エンジン        名詞,一般,*,*,*,*,エンジン,エンジン,エンジン
EOS
```

The Unicode normalization converts full-width characters to half-width, and the token filter keeps only tokens matching the specified part-of-speech tags.

You can also combine multiple filters:

```shell
% echo "すもももももももものうち" | lindera tokenize \
  --dict embedded://ipadic \
  --token-filter 'japanese_stop_tags:{"tags":["助詞","助詞,係助詞","助詞,連体化"]}'
```

## 6. Use user dictionary

Create a CSV file with custom word entries (e.g., `my_dict.csv`):

```csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
```

Tokenize with the user dictionary:

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict embedded://ipadic \
  --user-dict ./my_dict.csv
```

Without the user dictionary, "東京スカイツリー" would be split into multiple tokens. With the user dictionary, it is recognized as a single token.

For pre-built user dictionary examples, see:

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict embedded://ipadic \
  --user-dict ./resources/user_dict/ipadic_simple_userdic.csv
```

Expected output:

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```
