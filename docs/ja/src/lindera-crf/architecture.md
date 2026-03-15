# アーキテクチャ

## モジュール構成

```text
lindera-crf/src/
├── lib.rs                # パブリックAPIの再エクスポート
├── feature.rs            # FeatureSet, FeatureProvider
├── lattice.rs            # Edge, Node, Lattice
├── model.rs              # RawModel, MergedModel, Model trait
├── trainer.rs            # Trainer, Regularization enum
├── errors.rs             # エラー型
├── forward_backward.rs   # 前向き・後向きアルゴリズム
├── math.rs               # 数学ユーティリティ (logsumexp)
├── optimizers/
│   └── lbfgs.rs          # L-BFGS最適化
└── utils.rs              # ユーティリティtrait
```

## 主要コンポーネント

### FeatureProvider / FeatureSet

ラベルごとの素性セットを管理します。各`FeatureSet`は、指定されたラベルのユニグラム素性と左右のバイグラム素性を保持します。`FeatureProvider`は`FeatureSet`インスタンスを集約し、素性IDから重みへのマッピングを行います。

### Lattice / Edge / Node

系列ラベリングのための可変長エッジを持つラティス構造です。`Edge`はラベル付きの候補スパンを表し、`Node`は特定の位置にあるエッジを集約します。`Lattice`は入力データから構築され、モデルが最適パスを探索するために使用されます。

### Trainer

設定可能な正則化を用いたL-BFGS最適化によりCRFモデルを学習します。Trainerはラベル付きラティスの例を受け取り、前向き・後向きアルゴリズムで勾配を計算し、反復的にモデルの重みを更新します。

### Regularization

設定可能な正則化戦略：

- **L1**: L1ペナルティによるスパースモデル
- **L2**: L2ペナルティによる滑らかなモデル
- **ElasticNet**: L1とL2を設定可能な`l1_ratio`で組み合わせ

### Model (trait)

ラティスを通じて最適パスを探索するためのインターフェースです。2つの実装が提供されています：

- **RawModel**: 素性IDでインデックスされたフラットベクトルに重みを格納
- **MergedModel**: 推論に最適化され、素性の重みをrkyvでシリアライズ可能なコンパクトな表現にマージ

### 前向き・後向きアルゴリズム

ラティス上でアルファ（前向き）とベータ（後向き）の値を計算します。学習時に期待素性カウントと勾配の計算に使用されます。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --------- | ------ | ----------- |
| `alloc` | `no_std`向けのallocサポート | No |
| `std` | 標準ライブラリサポート（`alloc`を含む） | No |
| `train` | 学習機能（L-BFGS、マルチスレッド、ログ出力） | Yes |
