# 学習パイプライン

Lindera は、カスタム形態素解析モデルを作成するための CRF ベースの辞書学習機能を提供します。この機能には `train` feature フラグが必要です。

## 概要

学習パイプラインは3つのステージで構成されます：

```text
lindera train --> model.dat --> lindera export --> dictionary files --> lindera build --> compiled dictionary
```

1. **Train**: アノテーション付きコーパスと種辞書から CRF の重みを学習し、バイナリモデルファイルを生成します。
2. **Export**: 学習済みモデルを Lindera 辞書ソースファイルに変換します。
3. **Build**: ソースファイルを Lindera が実行時に読み込めるバイナリ辞書にコンパイルします。

### 学習のしくみ

`lindera train` は、MeCab の学習器と同じく、コーパスの文ごとにラティスを 1 つ作ります。ラティスには、各位置から始まる種辞書の語と、ランタイムが `char.def` と `unk.def` からその位置に作る未知語が入ります（[文字定義](#3-文字定義-chardef)と[未知語定義](#4-未知語定義-unkdef)を参照）。CRF は、アノテーションされたパス（正解のトークン）が最もコストの低いパスになるように重みを学習します。正解の各トークンには、次のうち最初に当てはまるもののラベルを付けます。

1. 範囲（span）と素性文字列全体が同じ種辞書の行のうち、ファイル順で最初の行。MeCab の学習器は先頭の `eval-size` 列だけを比べますが、Lindera はすべての列が一致する必要があるため、読みなどの列が種辞書の行と異なるトークンは次の段階に進みます。
2. 範囲が同じ未知語候補の `unk.def` エントリのうち、`*` 以外の素性の列がすべて正解トークンの列と一致する、ファイル順で最初のエントリ。
3. 正解の素性文字列から作る仮想ラベル。MeCab の学習器が仮想ノードを追加するのと同じです。素性文字列が同じ正解トークンは 1 つの仮想ラベルを共有します。仮想ラベルは学習されますが、エクスポートされません。

それ以外の候補はすべて負例になります。ラティスを作り終えると、`lindera train` は正解トークンのラベルの付き方を出力します。`resources/training/` のファイルでは次のとおりです。

```text
Lattice statistics: gold_lexicon=14 gold_unknown=0 gold_virtual=0 virtual_labels=0 negative_lexicon=0 negative_unknown=17
```

`gold_lexicon`・`gold_unknown`・`gold_virtual` は上の 3 つの段階それぞれでラベルを付けた正解トークンの数、`virtual_labels` は仮想ラベルの数、`negative_lexicon` と `negative_unknown` は種辞書と未知語候補それぞれから作った負例の辺の数です。

どの正解トークンのラベルにもならなかった種辞書の行は、負例としてしか学習されません。そのため `lindera train` は、種辞書の語である正解トークンが 1 つもないとき、または表層を持つ種辞書の行がある正解トークンのうち種辞書の語であるものが半分に満たないときに、標準エラーに警告を出します。コーパスが正解トークンに種辞書の行の素性文字列を付けていない場合がこれに当たります。警告には、表層を持つ種辞書の行はあるものの素性文字列が一致する行がない最初の正解トークンと、その表層を持つ種辞書の行のうち最初の 5 行の素性文字列が並びます。学習はそのまま続きます。たとえば `resources/training/corpus.txt` の素性をすべて先頭の 6 列に切り詰め、原形と読みを除くと、次のようになります。

```text
Warning: 0 of the 14 gold tokens are seed words, and 14 are not although seed rows have their surface. A gold token is a seed word only when a seed row has its surface and its whole feature string, as the seed lexicon writes it; otherwise the seed rows with its surface are only wrong candidates at its position. The first such gold token, and the seed rows with its surface:
  gold token: 外国	名詞,一般,*,*,*,*
  seed row:   外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
```

表層を持つ種辞書の行がない正解トークンは警告の判定に数えないので、コーパスの一部しか覆わない種辞書では警告は出ません。

その後、CRF を L-BFGS（正則化に L1 の部分があるときは OWL-QN）で学習します。学習は、次のうち最初に当てはまったもので止まります。

- `converged`: 目的関数の相対変化 `|previous - current| / |previous|` が `eta` を 3 回続けて下回った。`eta` のデフォルトは MeCab の学習器と同じです。MeCab はライン探索の試行を含めた評価ごとに同じ判定を行い、Lindera はイテレーションごとに行います。
- `no progress`: あるイテレーションで目的関数が変わらなかった。数イテレーションでこれが起きたときは、モデルはほとんど学習されていないので、`lindera train` は警告を出します。
- あるイテレーションのライン探索が失敗した。モデルはその学習で最良の重みを使い、警告が出ます。
- 目的関数が収束する前に `max-iterations` に達した。

`lindera train` は、どれで止まったかを、イテレーション数と評価回数とともに出力します。1 回の評価は、全ラティスの前向き・後向きの 1 回分です。`resources/training/` のファイルでは次のとおりです。

```text
L-BFGS stopped after 9 iterations and 11 evaluations of the loss and its gradient: converged
```

## 必要な入力ファイル

### 1. 種辞書 (seed.csv)

MeCab CSV 形式のベース語彙辞書です。

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

各行の構成: `surface,left_id,right_id,cost,pos,pos_detail1,pos_detail2,pos_detail3,inflection_type,inflection_form,base_form,reading,pronunciation`

種辞書では `left_id`、`right_id`、`cost` フィールドは 0 に設定されます。学習器が CRF モデルから適切な値を計算します。

種辞書は、`lindera build` が `lex.csv` を読むのと同じく CSV として読みます。

- フィールドは引用符で囲めます。mecab-ko-dic のカンマの表層形がその例です: `",",1792,3558,788,SC,*,*,*,*,*,*,*`。
- `#` で始まる行はコメントではなく、1 つの行として読みます。UniDic の `#` の表層形がその例です。空行と、フィールドが 5 つ未満の行は読み飛ばします。
- 行の素性は、4 つ目のフィールドより後ろのテキストを、引用符も含めて書かれたまま使います。`lindera export` はそれをそのまま書き出し、`,`・`"`・改行を含む表層形は引用符で囲みます。
- 複数の行にまたがる引用付きのフィールドは、その行を示すエラーになります。閉じていない引用符である可能性が高く、そのままでは後ろの行を飲み込んでしまうためです。
- 行の順序はそのまま保たれ、その順序がラベルの ID になります。

書き換え規則・素性テンプレート・未知語の照合で素性の列を読むとき、`lindera train` は素性を、上の CSV の読み方ではなく、MeCab の学習器と同じく（`tokenizeCSV`）列に分けます。コーパスの素性も同じく分けます。

- 引用符で囲んだ列は 1 つの列で、`""` は `"` と読みます。mecab-jieba の `4S店` の定義 `"authorized full-service car dealership (offering sales, spare parts, after-sales service and customer support)"` は 1 つの列です。
- 列の前の空白とタブは読み飛ばします。CC-CEDICT の `COVID-19, the coronavirus disease identified in 2019/` は、`COVID-19` と `the coronavirus disease identified in 2019/` の 2 つの列です。
- 閉じる引用符から次の `,` までのテキストは捨てます。`"bamboo" radical in Chinese characters (Kangxi radical 118)/` は `bamboo` という列です。
- 素性の末尾の `,` の後ろに空の列はできません。

### 2. 学習コーパス (corpus.txt)

タブ区切り形式のアノテーション付きテキストデータです。各行は `surface<TAB>pos_info` で、文は `EOS` で区切られます。

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
EOS
```

正解トークンが種辞書の語になるのは、その表層と素性文字列全体を持つ種辞書の行があるときだけです。素性文字列は、引用符も含めて種辞書に書かれたとおりに比べます。上の例では、各素性は同じ表層の種辞書の行の素性です。読みなど、どれか 1 列でも素性が異なる正解トークンは未知語か仮想ラベルとして学習され、その表層を持つ種辞書の行はその位置の負例になります。正解トークンの多くがこうなっているときは、`lindera train` が警告を出します（[学習のしくみ](#学習のしくみ)を参照）。

学習の品質はこのコーパスの量と質に大きく依存します。

### 3. 文字定義 (char.def)

文字タイプのカテゴリと Unicode コードポイント範囲を定義します。

```text
# Category definition: category_name compatibility_flag continuity_flag length
DEFAULT 0 1 0
SPACE 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# Character range mapping
0x0020 SPACE             # Space (reserved for SPACE)
0x0009 SPACE             # Tab
0x000A SPACE             # Line feed
0x000B SPACE             # Vertical tab
0x000D SPACE             # Carriage return
0x3041..0x3096 HIRAGANA  # Hiragana
0x30A1..0x30F6 KATAKANA  # Katakana
0x4E00..0x9FAF KANJI     # Kanji
0x0030..0x0039 NUMERIC   # Numbers
0x0041..0x005A ALPHA     # Uppercase letters
0x0061..0x007A ALPHA     # Lowercase letters
```

パラメータは、文字の既定カテゴリが作る未知語を制御します。辞書語が始まる位置でも未知語を作るか（`INVOKE`）、どの文字も直前の文字とカテゴリを共有する連続を 1 つのトークンにするか（`GROUP`）、どの長さまで短い候補を作るか（`LENGTH`）を指定します。

範囲行には複数のカテゴリを書くことができ（例: `0x4E00 KANJINUMERIC KANJI`）、少なくとも 1 つのカテゴリが必要です。同じコードポイントを複数の行が覆う場合は、MeCab と同じく最後の行がそのカテゴリを決めます。その行の先頭のカテゴリが文字の既定カテゴリです。未知語候補を作るのは既定カテゴリだけで、素性テンプレートの `%t` もこれを使います。

MeCab と同じく、範囲行が指せるのはそれより前の行で定義したカテゴリだけで、各カテゴリの定義は 1 回だけです。`lindera train` と `lindera build` は、これに反する `char.def` を、行とカテゴリを示すエラーで拒否します。たとえば、`KATAKANA` を定義する行より前に、`char.def` の 1 行目として `0x30FC KATAKANA` を挿入して `lindera train` を実行すると、次のようになります：

```text
Error: LinderaError { kind: Args, source: failed to parse char.def: LinderaError(kind=Content, source=char.def: line 1: category KATAKANA is not defined; a range line can only name a category defined on an earlier line) }
```

`SPACE` カテゴリと、`unk.def` のその行は残してください。MeCab は、すべての `char.def` に `DEFAULT` と `SPACE` を求めます。また、すべてのカテゴリに `unk.def` の行が 1 つ以上必要です（次の節を参照）。Segmenter は `SPACE` の文字を空白として扱い、デフォルトでは MeCab と同じく読み飛ばして出力しません。`char.def` に `SPACE` カテゴリがない辞書は、`--keep-whitespace` を指定しないと使えず、`lindera tokenize` は `SPACE category is not defined in the dictionary (char.def)` で失敗します。

学習は、ランタイムと同じコード（`Lattice::unknown_word_candidates`）で未知語候補を作ります。そのため、ビルドした辞書が作る候補に対して重みを学習します。

- 各位置では、文字の既定カテゴリが候補を作ります。`INVOKE` が 0 で、その位置から種辞書の語が始まる場合は作りません。
- `GROUP` が 1 なら、どの文字も直前の文字とカテゴリを共有する連続を 1 つの候補にします。
- `LENGTH` が n なら、どの文字も先頭の文字とカテゴリを共有する間、1〜n 文字の候補も作ります（候補ラダー）。上の例の `KANJI 0 0 2` では、種辞書の語が始まらない漢字の位置に、漢字 1 文字と 2 文字の候補ができます。
- カテゴリの `unk.def` のエントリそれぞれが、これらすべての範囲の候補を持ちます。

グルーピングの長さは、ランタイムのデフォルトと同じく無制限です。Rust API の `Trainer::max_grouping_len` で、ランタイムと同じ意味（先頭の 1 文字を除いた文字数）の上限を設定できます。CLI にはこのオプションはありません。

### 4. 未知語定義 (unk.def)

文字カテゴリごとに、そのカテゴリが作る未知語を定義します。各行は `category,left_id,right_id,cost,feature...` で、`category` は `char.def` のカテゴリ名です。種辞書と同じく ID とコストは 0 にしておきます。学習した値はエクスポートで書き出されます。

```csv
# Unknown word definitions
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
SPACE,0,0,0,記号,空白,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

- 各行がそれぞれ 1 つのエントリで、1 つのカテゴリが複数のエントリを持てます。未知語がとりうる品詞ごとに 1 行です。たとえば mecab-ipadic の `unk.def` は 11 カテゴリに 40 行あり、そのうち 6 行が `KANJI`（`名詞,一般`・`名詞,サ変接続`・4 種類の `名詞,固有名詞`）です。
- 各エントリはそれぞれ 1 つの CRF ラベルになり、その `%t` はエントリのカテゴリの ID です。カテゴリの候補はエントリごとに作られるため、どのエントリがどこに合うかを学習します。
- 行の順序は保たれます。`lindera export` はエントリごとに 1 行を同じ順序で書き出します。
- `#` で始まる行と空行は読み飛ばします。フィールドが 5 つ未満（カテゴリ、2 つの ID、コスト、1 列以上の素性）の行はエラーになります。

```text
Error: LinderaError { kind: Args, source: unk.def line 1: expected at least 5 fields (category, left id, right id, cost, feature), got 4 }
```

- MeCab と同じく、各行は `char.def` で定義したカテゴリを指し、`char.def` の各カテゴリには 1 行以上が必要です。`lindera train` と `lindera build` は、これに反する `unk.def` を拒否します。エラーはカテゴリを示し、`char.def` が定義していないカテゴリの行では、コメント行と空行も数えた行番号も示します。たとえば、`char.def` に `SYMBOL` カテゴリがないまま、上の `unk.def` の 9 行目に `SYMBOL,0,0,0,記号,一般,*,*,*,*,*,*,*` を追加して `lindera train` を実行すると、次のようになります：

```text
Error: LinderaError { kind: Args, source: failed to check unk.def: LinderaError(kind=Content, source=unk.def: line 9: category SYMBOL is not defined in char.def) }
```

### 5. 素性テンプレート (feature.def)

CRF モデルが学習に使用する情報を定義する MeCab 互換の素性抽出パターンです。

```text
# Unigram features (word-level)
UNIGRAM U00:%F[0]           # POS
UNIGRAM U01:%F[0],%F?[1]    # POS + POS detail (%F?[n] = optional, skipped if *)
UNIGRAM U02:%F[6]           # Base form
UNIGRAM U03:%w              # Surface form

# Bigram features (context combination)
BIGRAM B00:%L[0]/%R[0]      # Left POS / Right POS
BIGRAM B01:%L[0],%L[1]/%R[0],%R[1]  # Left POS detail / Right POS detail
```

テンプレート変数：

| 変数 | 説明 |
| --- | --- |
| `%F[n]` / `%F?[n]` | インデックス n の素性フィールド（`?` = オプション、値が `*` の場合はスキップ） |
| `%L[n]` | 左文脈の素性フィールド（rewrite.def の左セクションから） |
| `%R[n]` | 右文脈の素性フィールド（rewrite.def の右セクションから） |
| `%w` | 単語の表層形 |
| `%u` | ユニグラム書き換え後の素性文字列 |
| `%l` | 左書き換え後の素性文字列 |
| `%r` | 右書き換え後の素性文字列 |

`%F[n]`・`%L[n]`・`%R[n]` のフィールドは、書き換え後の素性を[種辞書](#1-種辞書-seedcsv)で説明したとおりに分けた列です。`%u`・`%l`・`%r` は、`rewrite.def` が書き出したままの書き換え後の素性で、`,` か `"` を含む列は引用符で囲まれています。

素性の最後の列より後ろを読むテンプレートは、`?` の有無にかかわらず、MeCab の学習器と同じく、その素性からは何も作りません。たとえば `UNIGRAM U06:%F[6]` は、5 列の `unk.def` の素性 `n,*,*,*,*` には何も作りません。バイグラムのテンプレートは、`%L[n]` と `%R[n]` のどちらかが最後の列より後ろを読むと何も作りません。

### 6. 素性書き換えルール (rewrite.def)

MeCab 互換の3セクション形式による素性正規化ルールです。セクションは空行で区切られます。

```text
# Section 1: Unigram rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助動詞,*,*,*,特殊・デス  助動詞
*  *

# Section 2: Left context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *

# Section 3: Right context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *
```

各行は `pattern<TAB>replacement` です。パターンはワイルドカードとして `*` を使用し、前方一致で照合されます。各セクションで最初に一致したルールが適用されます。ユニグラム、左文脈、右文脈に対して異なるルールを独立して適用できるため、スパース性を低減するきめ細かい素性正規化が可能です。

パターン・置換・素性は[種辞書](#1-種辞書-seedcsv)で説明したとおりに列に分けるので、`,` を含む引用付きの列は 1 つの列です。MeCab と同じく、書き換えた列は `,` でつなぎ、`,` か `"` を含む列は引用符で囲み、中の `"` を二重にします。たとえば左の規則 `* $5` は、素性 `n,dian4,店,店,"shop, store",1` を `"shop, store"` に書き換えます。

## 学習パラメータ

| パラメータ | 説明 | デフォルト |
| --- | --- | --- |
| `lambda` | 正則化係数（過学習の制御）。有限の 0 以上の値。`l2` では MeCab の `-c C` が `1 / C` に当たる | 1.0 |
| `regularization` | 正則化の種類: `l1`、`l2`、`elasticnet` のいずれか。大文字・小文字は区別しない（`elastic_net`・`elastic-net` も Elastic Net を選ぶ） | `l2` |
| `elastic-net-l1-ratio` | Elastic Net 正則化の L1 比率（0.0-1.0、`--regularization elasticnet` の場合のみ使用） | 0.5 |
| `max-iterations` | 最大学習イテレーション数 | 100 |
| `eta` | 収束判定の許容値（[学習のしくみ](#学習のしくみ)を参照）。`0` で判定を切ります | 0.00005 |
| `max-threads` | 並列処理スレッド数 | CPU コア数 |

デフォルトは MeCab の学習器と同じです。L2 正則化の `lambda` 1.0 は、MeCab の `-c 1.0` の目的関数に当たります。[#1110](https://github.com/lindera/lindera/issues/1110) の mecab-ipadic の交差検証データでは、MeCab の学習器と同じスコアになり、ほぼ同じイテレーション数（81〜87）で収束します（[#1261](https://github.com/lindera/lindera/issues/1261)）。5 分割の平均で、境界 F1 は 0.9613、span + 品詞（4 列）F1 は 0.9398 です。v6 のデフォルトの L1・`lambda` 0.01 は、すべての分割で `max-iterations` に達し、0.9572 と 0.9344 でした。L1 は多くの重みを 0 にし、`lindera export` は重みの等しいラベルに 1 つの文脈 ID を与えるので、L1 の `matrix.def` は小さくなります。このデータでは L2 の 18.9 MB に対して 8.7 MB で、L2 ではビルドした辞書が 3.8 % 大きくなります。このデータでの学習時間は MeCab の学習器の約 2 倍で、その約 3 分の 2 は最適化の外、つまり最適化を始める前（[#1263](https://github.com/lindera/lindera/issues/1263)）と終えた後にかかります。

## CLI の使用

### Train

```bash
lindera train \
    --seed seed.csv \
    --corpus corpus.txt \
    --char-def char.def \
    --unk-def unk.def \
    --feature-def feature.def \
    --rewrite-def rewrite.def \
    --max-iterations 100 \
    --max-threads 4 \
    --output model.dat
```

### Export

学習済みモデルを辞書ソースファイルに変換します：

```bash
lindera export \
    --model model.dat \
    --metadata metadata.json \
    --output ./dict-source
```

以下のファイルが生成されます：

| ファイル | 説明 |
| --- | --- |
| `lex.csv` | 学習済みコスト付きレキシコン |
| `matrix.def` | すべての文脈 ID と BOS/EOS 用の 0 を覆う連接コスト行列。正則化でバイグラムの重みが 1 つも残らない場合も同じ。学習した重みがない連接のコストは、学習中や MeCab と同じく 0 |
| `unk.def` | 未知語定義。入力の `unk.def` のエントリごとに 1 行を入力と同じ順序で書き、カテゴリ・学習した ID とコスト・素性を持つ |
| `char.def` | 文字定義 |
| `feature.def` | 素性テンプレート |
| `rewrite.def` | 素性書き換えルール |
| `left-id.def` | 左文脈 ID マッピング |
| `right-id.def` | 右文脈 ID マッピング |
| `metadata.json` | 辞書メタデータ。`model_info.connection_matrix_size` は `matrix.def` の大きさ（行（右文脈 ID）× 列（左文脈 ID）） |

たとえば上の `unk.def` で、最初の `KANJI` の行の後に 2 つ目の `KANJI` の行 `KANJI,0,0,0,名詞,サ変接続,*,*,*,*,*,*,*` を加えると、エクスポートした `unk.def` は両方の行を入力と同じ順序で持ちます。

```csv
DEFAULT,1,1,31698,名詞,一般,*,*,*,*,*,*,*
SPACE,10,7,0,記号,空白,*,*,*,*,*,*,*
HIRAGANA,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KATAKANA,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KANJI,1,1,31698,名詞,一般,*,*,*,*,*,*,*
KANJI,3,3,4179,名詞,サ変接続,*,*,*,*,*,*,*
ALPHA,11,10,10957,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,11,10,10957,名詞,数,*,*,*,*,*,*,*
```

### Build

エクスポートされたソースファイルをバイナリ辞書にコンパイルします：

```bash
lindera build \
    --src ./dict-source \
    --dest ./dict-compiled \
    --metadata ./dict-source/metadata.json
```

`lindera build` は、MeCab の `mecab-dict-index` と同じく、`lex.csv` の各行と `unk.def` の各行の文脈 ID が `matrix.def` の範囲内にあることを確認します（[文脈 ID の確認](../lindera-cli/commands.md#文脈-id-の確認)を参照）。`lindera export` が書き出したファイルはこの確認を通ります。

### Tokenize

ビルドした辞書でテキストを解析します：

```bash
echo "これは外国人参政権です。" | lindera tokenize -d ./dict-compiled
```

上の入力ファイルに `resources/training/` のファイルを使うと、出力は次のとおりです。

```text
これ	名詞,代名詞,一般,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

`これは 外国人参政権です。` のように入力に空白があっても、空白は読み飛ばされ、出力は同じです。`--keep-whitespace` を指定すると、空白は `SPACE` の未知語として出力されます。

```text
これ	名詞,代名詞,一般,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
 	記号,空白,*,*,*,*,*,*,*
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

## 出力モデルの形式

学習済みモデルは高速読み込みのために `rkyv` バイナリ形式でシリアライズされます。以下を含みます：

- CRF で学習された素性の重み
- ラベルセット（種辞書の行、続いて `unk.def` のエントリ）
- 品詞情報
- ファイル順の `unk.def` のエントリと、`char.def`・`feature.def`・`rewrite.def` の内容
- 学習メタデータ（形式バージョン、正則化、イテレーション数、素性・ラベル数）

形式バージョンは `2.0.0`（`lindera_trainer::model::MODEL_FORMAT_VERSION`）で、モデルの構造や意味が変わるたびに変わります。`lindera export` は、別のバージョンで書かれた `model.dat`（7.0.0 より前に書かれたものはすべて該当）を、「re-run `lindera train` to regenerate it」で終わるエラーで拒否します。そうしたモデルは現在のバージョンで学習し直してください。

出力はバイト単位で再現可能です。同じ入力・同じフラグで 2 回学習すると
`model.dat` のバイト列が一致するため、チェックサムの取得や差分比較ができます。
これは `--max-threads` の値によらず成り立ちます。勾配と損失は学習データの
固定分割を固定順で加算するため、スレッド数は所要時間だけを変え、
結果は変えません。

## API の使用

`lindera-trainer` の完全な API については、[Lindera Trainer アーキテクチャ](../lindera-trainer/architecture.md)と[APIリファレンス](../lindera-trainer/api_reference.md)を参照してください。

```rust,no_run
use std::fs::File;
use lindera_trainer::{Corpus, Trainer, TrainerConfig};

// Load configuration from files
let seed_file = File::open("resources/training/seed.csv")?;
let char_file = File::open("resources/training/char.def")?;
let unk_file = File::open("resources/training/unk.def")?;
let feature_file = File::open("resources/training/feature.def")?;
let rewrite_file = File::open("resources/training/rewrite.def")?;

let config = TrainerConfig::from_readers(
    seed_file,
    char_file,
    unk_file,
    feature_file,
    rewrite_file
)?;

// Initialize and configure trainer
let trainer = Trainer::new(config)?
    .max_iter(100)
    .num_threads(4);

// Load corpus
let corpus_file = File::open("resources/training/corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// Execute training
let model = trainer.train(corpus)?;

// Save model (binary format)
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Output in Lindera dictionary format. output_user.csv には read_user_lexicon で
// 読み込んだエントリが入力順で書かれる（呼ばなければ空になる）。
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

## 推奨コーパス仕様

実用的なアプリケーション向けの辞書を生成するための推奨事項：

### コーパスサイズ

| レベル | 文数 | 用途 |
| --- | --- | --- |
| 最小 | 100以上 | 基本的な動作確認 |
| 推奨 | 1,000以上 | 実用的なアプリケーション |
| 理想 | 10,000以上 | 商用品質 |

### 品質ガイドライン

- **語彙の多様性**: さまざまな品詞のバランスの取れた分布、活用形・接尾語のカバー、専門用語・固有名詞の適切な含有。
- **一貫性**: コーパス全体で分析基準を一貫して適用すること。
- **検証**: 形態素解析結果を手動で検証すること。エラー率を5%以下に維持すること。
