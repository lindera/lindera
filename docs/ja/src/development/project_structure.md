# プロジェクト構成

Lindera は複数のクレートで構成される Cargo ワークスペースとして組織されています。

## ディレクトリ構成

```text
lindera/
├── lindera-crf/            # CRF engine (pure Rust, no_std)
├── lindera-dictionary/     # Dictionary base library
├── lindera/                # Core morphological analysis library
├── lindera-cli/            # CLI tool
├── lindera-ipadic/         # IPADIC dictionary (Japanese)
├── lindera-ipadic-neologd/ # IPADIC NEologd dictionary (Japanese)
├── lindera-unidic/         # UniDic dictionary (Japanese)
├── lindera-ko-dic/         # ko-dic dictionary (Korean)
├── lindera-cc-cedict/      # CC-CEDICT dictionary (Chinese)
├── lindera-jieba/          # Jieba dictionary (Chinese)
├── lindera-python/         # Python bindings (PyO3)
├── lindera-wasm/           # WebAssembly bindings (wasm-bindgen)
├── resources/              # Test resources and sample data
├── docs/                   # Documentation (mdBook)
└── examples/               # Example code
```

## クレートの説明

### コアクレート

#### `lindera-crf`

条件付き確率場（CRF）の pure Rust 実装です。`no_std` 環境をサポートします。高速なゼロコピーシリアライゼーションに `rkyv` を使用します。辞書学習で使用される統計学習エンジンを提供します。

#### `lindera-dictionary`

辞書のベースライブラリです。辞書の読み込み、ビルド、クエリ機能を提供します。`train` feature を有効にすると、カスタム辞書作成のための CRF 学習パイプラインも提供します。

`src/trainer/` 配下の主要モジュール：

| モジュール | 役割 |
| --- | --- |
| `config.rs` | 設定管理（種辞書、char.def、feature.def、rewrite.def） |
| `corpus.rs` | 学習コーパスの処理 |
| `feature_extractor.rs` | 素性テンプレートの解析と素性 ID 管理 |
| `feature_rewriter.rs` | MeCab 互換の素性書き換え（3セクション形式） |
| `model.rs` | 学習済みモデルの保存、シリアライゼーション、辞書出力 |

#### `lindera`

メインの形態素解析ライブラリです。辞書クレートを統合し、`Tokenizer`、`Segmenter`、文字フィルタ、トークンフィルタを提供します。

#### `lindera-cli`

トークナイズ、辞書学習、エクスポート、ビルドのためのコマンドラインインターフェースです。デフォルトで `train` feature が有効です。

### 辞書クレート

各辞書クレートには、特定の言語と辞書ソースのビルド済み辞書データが含まれます。

| クレート | 言語 | 辞書ソース |
| --- | --- | --- |
| `lindera-ipadic` | 日本語 | IPADIC |
| `lindera-ipadic-neologd` | 日本語 | IPADIC NEologd（拡張語彙） |
| `lindera-unidic` | 日本語 | UniDic |
| `lindera-ko-dic` | 韓国語 | ko-dic |
| `lindera-cc-cedict` | 中国語 | CC-CEDICT |
| `lindera-jieba` | 中国語 | Jieba |

### バインディング

#### `lindera-python`

[PyO3](https://pyo3.rs/) で構築された Python バインディングです。Lindera のトークナイザー API を Python アプリケーションに公開します。

#### `lindera-wasm`

[wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) で構築された WebAssembly バインディングです。ブラウザと Node.js でのトークナイズを可能にします。

### その他のディレクトリ

#### `resources/`

テストスイートで使用されるサンプル辞書、ユーザー辞書、テストコーパスなどのテストリソースです。

#### `docs/`

[mdBook](https://rust-lang.github.io/mdBook/) で構築されたユーザー向けドキュメントです。目次は `docs/src/SUMMARY.md` で定義されています。日本語翻訳は `docs/ja/` 配下にあります。

#### `examples/`

一般的な使用パターンを示す実行可能なサンプルプログラムです。以下のコマンドで実行できます：

```bash
cargo run --features=embed-ipadic --example=<example_name>
```
