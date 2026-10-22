# アーキテクチャ

## モジュール構成

```text
lindera-crf/src/
├── lib.rs                # パブリックAPIの再エクスポート
├── bigram_table.rs       # 学習ラティスのバイグラム重みの添字
├── feature.rs            # FeatureSet, FeatureProvider
├── lattice.rs            # Edge, Node, Lattice
├── model.rs              # RawModel, MergedModel, Model trait
├── trainer.rs            # Trainer, Regularization, StopReason, TrainingStats
├── errors.rs             # エラー型
├── forward_backward.rs   # 前向き・後向きアルゴリズム
├── math.rs               # 数学ユーティリティ (logsumexp)
├── optimizers.rs         # optimizersモジュールの宣言
├── optimizers/
│   └── lbfgs.rs          # L-BFGS最適化
└── utils.rs              # ユーティリティtrait
```

## 主要コンポーネント

### FeatureProvider / FeatureSet

ラベルごとの素性セットを管理します。各`FeatureSet`は、指定されたラベルのユニグラム素性と左右のバイグラム素性を保持します（保持するのは素性IDのみで、重みは持ちません）。`FeatureProvider`はラベルIDを`FeatureSet`インスタンスにマッピングします。重みは`RawModel`側で別途保持されます（`weights`、`unigram_weight_indices`、`bigram_weight_indices`）。

### Lattice / Edge / Node

系列ラベリングのための可変長エッジを持つラティス構造です。`Edge`はラベル付きの候補スパンを表し、`Node`は特定の位置にあるエッジを集約します。`Lattice`は入力データから構築され、モデルが最適パスを探索するために使用されます。

### Trainer

設定可能な正則化を用いたL-BFGS最適化（正則化にL1の部分があるときはOWL-QN）によりCRFモデルを学習します。Trainerはラベル付きラティスの例を受け取り、前向き・後向きアルゴリズムで損失とその勾配を計算し、反復的にモデルの重みを更新します。`Trainer::train`は学習した`RawModel`を返します。`Trainer::train_with_stats`は、次の値を持つ`TrainingStats`も返します。

- `iterations`: 実際に回ったイテレーション数。`Trainer::max_iter`で設定した最大値で学習が止まったときはその値で、それより前に最適化が止まったときはそれより少なくなります。
- `evaluations`: 損失とその勾配を評価した回数。1回の評価は全ラティスの前向き・後向きの1回分で、ライン探索の試行も含みます。
- `stop_reason`: 最適化が止まった理由（`StopReason`）。

最適化は、次のうち最初に当てはまったもので止まります。

- `StopReason::Converged`: 目的関数の相対変化`|previous - current| / |previous|`が`Trainer::eta`で設定した許容値を3回続けて下回ったか、勾配がほぼ0になった。デフォルトの許容値`DEFAULT_ETA`（`5e-5`）は、MeCabの学習器の`eta`のデフォルトと同じです。MeCabはライン探索の試行を含めた評価ごとに同じ判定を行い、Linderaはイテレーションごとに行います。`Trainer::eta(0.0)`で判定を切ります。
- `StopReason::NoProgress`: あるイテレーションで目的関数が変わらなかった。数イテレーションでこれが起きたときは、モデルはほとんど学習されていません。
- `StopReason::MaxIterations`: `Trainer::max_iter`で設定した上限に達した。
- `StopReason::LineSearchFailed`: あるイテレーションのライン探索が失敗した。重みは、その学習で最良だったものです。

### Regularization

設定可能な正則化戦略：

- **L1**: L1ペナルティによるスパースモデル
- **L2**: L2ペナルティによる滑らかなモデル
- **ElasticNet**: L1とL2を設定可能な`l1_ratio`で組み合わせ

### Model (trait)

ラティスを通じて最適パスを探索するためのインターフェースです。2つの実装が提供されています：

- **RawModel**: 素性IDでインデックスされたフラットベクトルに重みを格納
- **MergedModel**: 推論に最適化され、素性の重みをrkyvでシリアライズ可能なコンパクトな表現にマージ。ラベルごとの要素型として`MergedFeatureSet`を使用

`MergedFeatureSet`は、ラベルの事前合計済みユニグラム`weight`と、`MergedModel`のバイグラム重み行列を参照する`left_id`/`right_id`接続IDを保持します。

`RawModel::unigram_weight`は、ユニグラム素性IDの並びについてこの和を計算し、`merge`はラベルごとにこれを呼びます。重みのない素性は何も足さないため、学習でラベルでなかったエントリの素性の重みも計算できます。`lindera-trainer`はユーザー辞書のエントリのコストにこれを使います。

### 前向き・後向きアルゴリズム

ラティス上でアルファ（前向き）とベータ（後向き）の値を計算します。学習時に損失と期待素性カウントの計算に使用されます。ラティス1つにつき1回のパスで、その損失と勾配への寄与の両方を求めます。argminのL-BFGSは同じ点のコストと勾配を2回の呼び出しで求めるので、最適化は直前のパラメータベクトルでの結果を保持します。

最適化を始める前に、ラティスで接するすべての辺の対について、そのバイグラム素性の重みの添字を、評価で足す順に表に記録します。バイグラム素性IDが同じラベルは、表の項目を共有します。評価はバイグラム素性ごとにマップを引く代わりにこの表から添字を読み、同じ重みを同じ順に足すので、学習した重みは変わりません。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --------- | ------ | ----------- |
| `alloc` | `no_std`向けのallocサポート | No* |
| `std` | 標準ライブラリサポート（`alloc`を含む） | No* |
| `train` | 学習機能（L-BFGS、マルチスレッド、ログ出力） | Yes |

\* `default`に直接列挙されているわけではありませんが、デフォルトで有効な`train`が`std`を有効化し、`std`がさらに`alloc`を有効化するため、実際にはデフォルトビルドで`alloc`／`std`は有効になっています。
