# ベンチマーク基準値の記録と回帰検出 / Benchmark baselines and regression detection

リファクタリング(REFACTORING_PLAN.md)の各フェーズでは、トークナイズ性能に
3% 以上の劣化がないことを確認してからマージする。本ドキュメントはその手順を定める。

> **重要**: Criterion のベンチマーク結果はマシン固有です。基準値との比較は
> **必ず同一マシン・同一電源状態で** 行ってください。CI の共有ランナー上の
> 絶対値同士の比較は参考値に留めること。

## 対象ベンチマーク

`lindera/benches/` に辞書ごとのベンチマークがある(要 `embed-*` feature):

| ベンチ | feature | 内容 |
|---|---|---|
| `bench_ipadic` | `embed-ipadic` | constructor / tokenize / tokenize_with_lattice / user_dict 等 7 種 |
| `bench_unidic` | `embed-unidic` | 同上 |
| `bench_ko_dic` | `embed-ko-dic` | 同上 |
| `bench_cc_cedict` | `embed-cc-cedict` | 同上 |
| `bench_ipadic_neologd` | `embed-ipadic-neologd` | 同上 |
| `bench_jieba` | `embed-jieba` | 同上 |

ホットパス(viterbi / segmenter / tokenizer)に触れる PR では、最低限
`bench_ipadic` の `bench-tokenize-ipadic` / `bench-tokenize-with-lattice-ipadic`
を比較すること。

## 手順

### 1. リファクタリング前: 基準値を保存

```sh
# 変更前のコミット(例: main)で実行
cargo bench -p lindera --features embed-ipadic -- --save-baseline refactor-base
```

Criterion が `target/criterion/<bench>/refactor-base/` に基準値を保存する。

### 2. リファクタリング後: 基準値と比較

```sh
# 変更後のブランチで実行(同一マシン)
cargo bench -p lindera --features embed-ipadic -- --baseline refactor-base
```

出力の `change:` 行を確認する。例:

```
bench-tokenize-ipadic   time:   [532.10 µs 535.42 µs 539.01 µs]
                        change: [-1.2% +0.3% +1.9%] (p = 0.72 > 0.05)
                        No change in performance detected.
```

### 3. 判定基準

- `change` の中央値が **+3% 以内** であること(REFACTORING_PLAN.md の完了チェックリスト)
- `Performance has regressed` と判定された場合は原因を調査し、解消するか
  正当な理由(機能追加等)を PR に明記する
- ノイズが疑われる場合は `--sample-size` を増やすか再実行して確認する

## 補足

- 辞書のダウンロードを毎回避けるには `LINDERA_DICTIONARIES_PATH` を設定する
  (例: `export LINDERA_DICTIONARIES_PATH=$HOME/.lindera`)。
  IPADIC / ko-dic / Jieba の辞書アーカイブは GitHub の各ミラーリポジトリ
  (`lindera/mecab-ipadic` 等)のタグ付きソースアーカイブと同一バイナリであり、
  lindera.dev と同じ MD5 を持つ(lindera.dev へ到達できない環境では、これを
  キャッシュディレクトリ `$LINDERA_DICTIONARIES_PATH/<version>/` に配置すればよい)。
- HTML レポートは `target/criterion/report/index.html` に生成される。
- すべての辞書を一括で回す場合は `make bench-all` を使用する。
