# 学習データ

文節の区切りは文節で学習し、解析器は文節の係り受けで学習します。どちらも、動かすときに使う辞書の形態素の上で学習します。`train` feature の裏にある `lindera dependency convert` は、そのためのデータを、語の係り受けと文節の境界を持つコーパスから作ります。対象は、最後の列に `BunsetuBILabel` を持つ CoNLL-U のファイルで、UD Japanese-GSD と UD Japanese-PUD がこの形です。

## コーパスを変換する

```shell
cargo run --release -p lindera-cli --features embed-ipadic -- dependency convert \
    --dict embedded://ipadic \
    --input ja_gsd-ud-train.conllu \
    --output train.lattice
```

`--dict` には、Lindera 用にビルドした辞書のディレクトリも渡せます。その場合は `train` feature だけで足ります。

[スキーマ](./head_and_function_word.md)は、その辞書のものを使います。`convert` は、辞書の名前を辞書のメタデータから読みます。スキーマがあるのは `ipadic`・`unidic`・`unidic-cwj`・`unidic-csj` です。UniDic では、辞書のディレクトリを渡します。たとえば、[`lindera download`](../lindera-cli/commands.md) が入れたディレクトリです。

```shell
dictionary=$(lindera download unidic-cwj)
cargo run --release -p lindera-cli -- dependency convert \
    --dict "$dictionary" \
    --input ja_gsd-ud-train.conllu \
    --output train.lattice
```

スキーマの無い辞書は断ります。

```text
Error: LinderaError { kind: Content, source: /path/to/ko-dic: there is no schema for the dictionary ko-dic; there are schemas for ipadic, unidic, unidic-cwj, unidic-csj }
```

学習するコマンドは辞書を読みません。[`train-chunker`](./chunking.md#モデルを学習する)・[`train-parser`](./parsing.md#モデルを学習する)・`instances` には、辞書の名前を `--dictionary-name` で渡します。省くと `ipadic` です。

コマンドは、文ごとに次を行います。

1. **語の係り受けを、文節の係り受けに直す。** `BunsetuBILabel=B` の語が文節を始め、文節の外に係る語が、文節の係り先を決めます。並列は、京大コーパスと CaboCha の形に戻します。並列の各要素は次の要素に係り、係りの種類は `P` です。
2. **文節を、辞書の形態素に合わせる。** 文章を、解析器を使うときと同じ、その辞書と Lindera の既定の設定で分かち書きします。文節の境界が形態素の途中に来たら、次に合う境界まで文節をつなぎます。つないだ文節は、CaboCha の `tools/kc2cabocha.pl` と同じく、最後の文節の係り先を引き継ぎます。
3. 根が 1 つで最後の文節であり、係り先がすべて右にあり、係りが交差しない文だけを残し、**それ以外の文は落とす。**

## 出力

出力は[ラティス形式](./architecture.md#ラティス形式)で、文節の行は係り先と係りの種類だけを持ちます。

```text
* 0 6D 
ただし	接続詞,*,*,*,*,*,ただし,タダシ,タダシ
、	記号,読点,*,*,*,*,、,、,、
* 1 2D 
50	名詞,数,*,*,*,*,*,*,*
周年	名詞,接尾,助数詞,*,*,*,周年,シュウネン,シューネン
ソング	名詞,一般,*,*,*,*,ソング,ソング,ソング
に	助詞,格助詞,一般,*,*,*,に,ニ,ニ
* 2 6D 
変更	名詞,サ変接続,*,*,*,*,変更,ヘンコウ,ヘンコー
後	名詞,接尾,副詞可能,*,*,*,後,ゴ,ゴ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
```

主辞と機能語の位置は書き出しません。データを読む側が[スキーマ](./head_and_function_word.md)から計算するので、規則を変えても、古い位置がファイルに残りません。

## 報告

報告は標準エラーに出ます。UD Japanese-GSD の train を IPADIC で変換したときの報告です。

```text
ja_gsd-ud-train.conllu
sentences kept: 7024 of 7050 (99.63%)
sentences dropped: 26
  bunsetsu that cannot be made: 0
  root: 0
  arc to the left: 3
  crossing arcs: 24
  text that does not match: 0
bunsetsu with more than one word that leaves it: 0
arcs of type P: 517 of 49764 (1.04%)
bunsetsu boundaries on a morpheme boundary: 49372 of 49764 (99.21%)
sentences with merged bunsetsu: 372 of 7024 (5.30%)
bunsetsu merged: 392 of 56788 (0.69%)
bunsetsu written: 56396
arcs of type P written: 489 of 49372 (0.99%)
morphemes written: 162456
head words on the content head of the corpus: 54449 of 56396 (96.55%)
  275: 名詞-一般 is the head word, to the right of 名詞-一般
  240: 名詞-接尾-一般 is the head word, to the right of 名詞-一般
```

- 2 つの原因で落ちた文は、両方に数えます。
- 種類 `P` の係りは、文節を形態素に合わせる前に数え、書き出したものでもう一度数えます。つないだ文節は、最後の文節の係りを引き継ぎます。
- 最後の数行は、辞書のスキーマの主辞を、コーパスが印を付けた内容の主辞（`BunsetuPositionType`）と比べ、食い違いの多い 10 種類を品詞で並べます。ここでは 2 行だけを示しています。[主辞と機能語](./head_and_function_word.md)を参照してください。

## コーパス

コーパスはリポジトリに入っていません。[`scripts/dependency/fetch_corpora.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) が、この章の数字を測った commit のコーパスを取得し、`convert_corpora.sh` が変換します。

| コーパス | Commit | ファイル | 残った文 | 文節 | 形態素 |
| --- | --- | --- | --- | --- | --- |
| [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD) | c662dc9 | train | 7,024 of 7,050 | 56,396 | 162,456 |
| | | dev | 506 of 507 | 4,147 | 11,934 |
| | | test | 542 of 543 | 4,509 | 12,591 |
| [UD Japanese-PUD](https://github.com/UniversalDependencies/UD_Japanese-PUD) | cf69c5c | test | 994 of 1,000 | 9,843 | 27,728 |

この表は IPADIC のものです。残った文は、どの辞書でも同じです（文を落とす理由が、辞書に関係しないため）。文節と形態素の数は辞書で変わります。UD Japanese-GSD は UniDic の短単位で分かち書きされているので、UniDic の辞書では、文節の境界が形態素の途中に来ることが減り、併合される文節も減ります（`convert_corpora.sh --dictionary unidic-cwj`）。

| 辞書 | ファイル | 形態素の境界に合った文節の境界 | 併合された文節 | 文節 | 形態素 |
| --- | --- | --- | --- | --- | --- |
| IPADIC | GSD train | 99.21% (49,372 / 49,764) | 392 | 56,396 | 162,456 |
| | GSD dev | 99.16% (3,641 / 3,672) | 31 | 4,147 | 11,934 |
| | GSD test | 99.10% (3,967 / 4,003) | 36 | 4,509 | 12,591 |
| | PUD | 99.45% (8,849 / 8,898) | 49 | 9,843 | 27,728 |
| `unidic` | GSD train | 99.87% (49,700 / 49,764) | 64 | 56,724 | 172,976 |
| | GSD dev | 99.89% (3,668 / 3,672) | 4 | 4,174 | 12,709 |
| | GSD test | 99.80% (3,995 / 4,003) | 8 | 4,537 | 13,294 |
| | PUD | 99.98% (8,896 / 8,898) | 2 | 9,890 | 29,571 |
| `unidic-cwj` | GSD train | 99.95% (49,739 / 49,764) | 25 | 56,763 | 172,196 |
| | GSD dev | 99.95% (3,670 / 3,672) | 2 | 4,176 | 12,657 |
| | GSD test | 99.98% (4,002 / 4,003) | 1 | 4,544 | 13,254 |
| | PUD | 99.97% (8,895 / 8,898) | 3 | 9,889 | 29,413 |
| `unidic-csj` | GSD train | 99.94% (49,735 / 49,764) | 29 | 56,759 | 173,184 |
| | GSD dev | 99.95% (3,670 / 3,672) | 2 | 4,176 | 12,723 |
| | GSD test | 99.95% (4,001 / 4,003) | 2 | 4,543 | 13,299 |
| | PUD | 99.97% (8,895 / 8,898) | 3 | 9,889 | 29,545 |

3 つの UniDic の辞書は、同じ形態素を返しません。GSD train の文のうち、`unidic` と `unidic-cwj` で分かち書きが同じ文は 85.88%、品詞まで同じ文は 66.06% で、`unidic-cwj` と `unidic-csj` では 88.10% と 73.22% です（`compare_dictionaries.py`）。そのため、学習データは 1 つの辞書のためのもので、モデルも同じです。

## コーパスのライセンス

UD Japanese-GSD のライセンスは CC BY-SA 4.0、UD Japanese-PUD は CC BY-SA 3.0 です（各リポジトリの `LICENSE.txt`）。以下は、このプロジェクトでの扱い方であり、法的な助言ではありません。

- コーパスと、コーパスから変換したデータは、リポジトリにコミットしません。
- 変換したデータは、コーパスの注釈を改変したものです。共有するときは、コーパスのライセンスで共有し、コーパスの名前と、データを変換したことを明記してください。
- 変換したデータで学習したモデルは、コーパスとその版、ライセンスを明記して公開します。継承（share-alike）の条件が学習済みのモデルに及ぶかを判断した裁判例は、ありません。このプロジェクトは、重みを持つだけで注釈を含まない自分のモデルを、コーパスの改変物ではないとみなし、MIT ライセンスで配布します。[モデルのライセンス](./model_file.md#モデルのライセンス)を見てください。
