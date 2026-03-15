# アーキテクチャ

## モジュール構成

```text
lindera-dictionary/src/
├── lib.rs               # パブリックAPI
├── dictionary.rs        # Dictionary, UserDictionary
├── builder.rs           # DictionaryBuilder
├── loader.rs            # DictionaryLoader trait, FSDictionaryLoader
├── viterbi.rs           # Lattice, Edge, Viterbiセグメンテーション
├── nbest.rs             # NBestGenerator (Forward-DP Backward-A*)
├── mode.rs              # Mode (Normal/Decompose), Penalty
├── error.rs             # LinderaError, LinderaErrorKind
├── compress.rs          # 辞書圧縮
├── decompress.rs        # 辞書展開
├── assets.rs            # ダウンロードとファイル管理
├── dictionary/
│   ├── character_definition.rs    # 文字種定義
│   ├── connection_cost_matrix.rs  # 連接コスト行列
│   ├── prefix_dictionary.rs       # ダブル配列トライ辞書
│   ├── unknown_dictionary.rs      # 未知語処理
│   ├── metadata.rs                # 辞書メタデータ
│   └── schema.rs                  # スキーマ定義
└── trainer/             # (train feature)
    ├── config.rs        # TrainerConfig
    ├── corpus.rs        # Corpus, Example, Word
    ├── feature_extractor.rs  # 素性テンプレート解析
    ├── feature_rewriter.rs   # MeCab互換リライトルール
    └── model.rs         # 学習済みモデル, tocost()
```

## 主要コンポーネント

### Dictionary / UserDictionary

コンパイル済み辞書データを保持する主要データ構造です。`Dictionary`は文字種定義、連接コスト行列、前方一致辞書（ダブル配列トライ）、および未知語辞書を含みます。`UserDictionary`を使用すると、システム辞書の上にカスタム語彙を追加できます。

### DictionaryBuilder

ソースCSVファイルから辞書をビルドするためのFluent APIです。MeCab形式の辞書ソースを、実行時に使用されるバイナリ形式にコンパイルします。

### DictionaryLoader / FSDictionaryLoader

`DictionaryLoader`はコンパイル済み辞書を読み込むためのtraitです。`FSDictionaryLoader`はファイルシステムベースの実装で、ディレクトリから辞書ファイルを読み込みます。オプションでメモリマップドファイルをサポートします。

### Viterbi (Lattice, Edge)

入力テキストから候補トークンのラティスを構築し、Viterbiアルゴリズムを使用して最適なセグメンテーションパスを探索します。ラティス内の各`Edge`は、関連するコスト（単語コスト + 連接コスト）を持つ候補トークンを表します。

### NBestGenerator

Forward-DP Backward-A*アルゴリズムを使用してN-bestセグメンテーションパスを生成します。これにより、アプリケーションは単一の最適パスを超えた代替セグメンテーションを検討できます。

### Mode

トークナイゼーションの動作を制御します：

- **Normal**: 最適なViterbiパスを使用した標準的なトークナイゼーション
- **Decompose**: 設定可能な`Penalty`閾値に基づいて複合名詞をさらに分割

### Trainer (train feature)

`lindera-crf`を使用したCRFベースの辞書学習パイプラインです。学習ワークフローには以下が含まれます：

1. **TrainerConfig**: 種辞書、`char.def`、`feature.def`、`rewrite.def`を解析
2. **Corpus**: ラベル付き例として学習データを管理
3. **FeatureExtractor**: 素性テンプレートを解析し、素性IDを割り当て
4. **DictionaryRewriter**: MeCab互換の3セクションリライトルールを適用
5. **Model**: 学習結果を保持し、`tocost(weight, cost_factor)`によるコスト変換で辞書ファイルを出力

## Featureフラグ

| Feature | 説明 | デフォルト |
| --------- | ------ | ----------- |
| `compress` | 辞書圧縮サポート | Yes |
| `mmap` | メモリマップドファイルサポート | Yes |
| `build_rs` | 辞書ソースのHTTPダウンロード | No |
| `train` | CRFベースの学習（lindera-crfに依存） | No |
