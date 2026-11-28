# 評価

`train` feature の裏にある `lindera dependency evaluate` は、出力を正解と比べます。どちらも[ラティス形式](./architecture.md#ラティス形式)で、同じ文が同じ順に並んでいる必要があります。

```shell
cargo run --release -p lindera-cli -- dependency evaluate \
    --gold dev.lattice \
    --system output.lattice
```

CaboCha の `cabocha-system-eval` と J.DepP と同じものを数えるので、数字を並べて比べられます。割合には、必ず分子と分母を添えます。`--json` を付けると、同じ内容を JSON で書き出します。

## 何を数えるか

- **文節の区切り**: 文節についての精度、再現率、F。出力の文節は、正解に、同じ形態素から始まる同じ長さの文節があれば正解です。
- **係り受け**: 区切りが正解と同じ文（解析器に正解の文節を与えたときは、すべての文）について、係り先が合っている文節を数えます。レベル 0 はすべての文節、レベル 1 は各文の最後を除いた文節、レベル 2 は最後の 2 つを除いた文節です。レベル 1 は J.DepP が報告する数字で、ここでも見出しの数字にします。
- **一貫**（end to end）: 正解の文節のうち、各文の最後を除いたものについて、出力に同じ範囲の文節があり、その係り先が、正解の係り先と同じ範囲であるものを数えます。

係りの種類（`D`、`P`）は比べません。2 つのファイルで形態素が違う文は、どの数にも入れずに、番号を添えて報告します。正解に文節がない文は数えません。

## 例

[文節の区切り](./chunking.md)の既定の設定の出力を、UD Japanese-GSD の dev で評価した結果です。区切りの出力は、どの文節も次の文節に係るとして書かれているので、係り受けの行は、区切りが合った文についての、その基準の値です。

```text
sentences: 506
left out, different morphemes: 0
not counted, no bunsetsu: 0
chunking precision: 97.02% (4009/4132)
chunking recall: 96.67% (4009/4147)
chunking F: 96.85%
chunking sentences: 85.38% (432/506)
dependency level 0: 68.55% (2308/3367)
dependency level 1: 63.92% (1876/2935)
dependency level 2: 57.84% (1453/2512)
dependency sentences: 16.90% (73/432)
end to end: 60.12% (2189/3641)
end to end sentences: 14.43% (73/506)
```

正解の文節を与えた「どの文節も次の文節に係る」は、どの解析器も超えなければならない基準です。レベル 1 で、dev は 64.10% (2,334 / 3,641)、test は 62.77% (2,490 / 3,967)、UD Japanese-PUD は 60.87% (5,386 / 8,849) です。

## 2 つの出力を比べる

`--compare` を付けると、2 つ目の出力も評価し、2 つを対にします。文の区切り、一貫の係り、文のそれぞれについて、両方で正解、1 つ目だけ正解、2 つ目だけ正解、どちらも不正解を数えます。p 値は、食い違う 2 つの数に対する McNemar の正確検定（両側）のものです。区切りの対は、係り先が何であっても、2 つの区切りを比べられます。

```shell
cargo run --release -p lindera-cli -- dependency evaluate \
    --gold dev.lattice --system first.lattice --compare second.lattice
```

train の 10 分割での、2 つの区切りの比較です（1 つ目が CaboCha のテンプレート、2 つ目が既定のテンプレート）。

```text
paired segmentations: both 5908, first only 74, second only 107, neither 935, McNemar p = 0.0171
paired arcs: both 29235, first only 117, second only 164, neither 19856, McNemar p = 0.0060
paired sentences: both 887, first only 4, second only 12, neither 6121, McNemar p = 0.0768
```

## どのデータで何を確かめるか

dev と test は小さく、係りは 3,641 本と 3,967 本です。90% 前後の精度では、1 つの数字の 95% 区間が約 ±1 ポイントになるので、1 ポイント未満の差は、この 2 つだけでは見分けられません。

- **素性と設定は、train の交差検証で比べます。** 約 49,000 本の係りが得られ（約 ±0.3 ポイント）、上の対の検定も使います。
- **dev は、設定を選ぶのに使います。**
- **test と UD Japanese-PUD は、報告する結果にだけ使います。**

## 交差検証

`lindera dependency split` は、分割を書き出します。入力の文 `i`（0 から数える）は、fold `i mod k` のテストのファイルと、ほかのすべての fold の学習のファイルに入ります。そのため、どこでコマンドを流しても同じ分割になります。

```shell
cargo run --release -p lindera-cli -- dependency split \
    --input train.lattice --folds 10 --output-dir folds
```

これは、`fold0.test.lattice` と `fold0.train.lattice` から `fold9.test.lattice` と `fold9.train.lattice` までを書き出します。UD Japanese-GSD の train では、各テストのファイルは 702 文か 703 文です。

[`scripts/dependency/chunker_experiments.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) は、これらのコマンドで交差検証を行う例です。学習のファイルごとに学習し、テストのファイルを区切り、出力をつないで、1 つのファイルとして評価します。
