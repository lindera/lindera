# lindera-python

[Lindera](https://github.com/lindera/lindera) の Python バインディング。日本語形態素解析エンジン。

## 概要

lindera-python は、Lindera 3.0.0 形態素解析エンジンへの包括的な Python インターフェースを提供し、日本語・韓国語・中国語のテキスト解析に対応しています。主な機能は以下の通りです:

- **多言語対応**: 日本語（IPADIC、IPADIC-NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）
- **文字フィルタ**: マッピング、正規表現、Unicode 正規化、日本語踊り字処理によるテキスト前処理
- **トークンフィルタ**: 小文字化、長さフィルタリング、ストップワード、日本語固有フィルタなどの後処理フィルタ
- **柔軟な設定**: トークナイズモードやペナルティ設定のカスタマイズ
- **メタデータ対応**: 辞書スキーマとメタデータの完全な管理

## 機能

### コアコンポーネント

- **TokenizerBuilder**: カスタマイズされたトークナイザを構築するための Fluent API
- **Tokenizer**: フィルタリング機能を統合した高性能テキストトークナイズ
- **CharacterFilter**: テキスト正規化のための前処理フィルタ
- **TokenFilter**: トークン精緻化のための後処理フィルタ
- **Metadata & Schema**: 辞書構造と設定の管理
- **Training & Export**（オプション）: コーパスデータからカスタム形態素解析モデルを学習

### 対応辞書

- **日本語**: IPADIC、IPADIC-NEologd、UniDic
- **韓国語**: ko-dic
- **中国語**: CC-CEDICT、Jieba
- **カスタム**: ユーザー辞書対応

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) から入手できます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、展開したパスを指定して読み込みます。

### フィルタの種類

**文字フィルタ:**

- マッピングフィルタ（文字置換）
- 正規表現フィルタ（パターンベースの置換）
- Unicode 正規化（NFKC など）
- 日本語踊り字の正規化

**トークンフィルタ:**

- テキストの大文字・小文字変換
- 長さフィルタリング（最小/最大文字数）
- ストップワードフィルタリング
- 日本語固有フィルタ（基本形、読みなど）
- 韓国語固有フィルタ

## プロジェクト依存関係のインストール

- pyenv : <https://github.com/pyenv/pyenv?tab=readme-ov-file#installation>
- Poetry : <https://python-poetry.org/docs/#installation>
- Rust : <https://www.rust-lang.org/tools/install>

## Python のインストール

```shell
# Install Python
% pyenv install 3.13.5
```

## リポジトリのセットアップと仮想環境の有効化

```shell
# Clone lindera project repository
% git clone git@github.com:lindera/lindera.git
% cd lindera

# Create Python virtual environment and initialize
% make init

# Activate Python virtual environment
% source .venv/bin/activate
```

## 仮想環境への lindera-python のインストール

このコマンドは開発設定（デバッグビルド）でライブラリをビルドします。

```shell
(.venv) % make python-develop
```

## クイックスタート

### 基本的なトークナイズ

```python
from lindera.dictionary import load_dictionary
from lindera.tokenizer import Tokenizer

# Load dictionary from a local path (download from GitHub Releases)
dictionary = load_dictionary("/path/to/ipadic")

# Create a tokenizer
tokenizer = Tokenizer(dictionary, mode="normal")

# Tokenize Japanese text
text = "すもももももももものうち"
tokens = tokenizer.tokenize(text)

for token in tokens:
    print(f"Text: {token.surface}, Position: {token.byte_start}-{token.byte_end}")
```

### 文字フィルタの使用

```python
from lindera import TokenizerBuilder

# Create tokenizer builder
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("/path/to/ipadic")

# Add character filters
builder.append_character_filter("mapping", {"mapping": {"ー": "-"}})
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})

# Build tokenizer with filters
tokenizer = builder.build()
text = "テストー１２３"
tokens = tokenizer.tokenize(text)  # Will apply filters automatically
```

### トークンフィルタの使用

```python
from lindera import TokenizerBuilder

# Create tokenizer builder
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("/path/to/ipadic")

# Add token filters
builder.append_token_filter("lowercase")
builder.append_token_filter("length", {"min": 2, "max": 10})
builder.append_token_filter("japanese_stop_tags", {"tags": ["助詞", "助動詞"]})

# Build tokenizer with filters
tokenizer = builder.build()
tokens = tokenizer.tokenize("テキストの解析")
```

### 統合パイプライン

```python
from lindera import TokenizerBuilder

# Build tokenizer with integrated filters
builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("/path/to/ipadic")

# Add character filters
builder.append_character_filter("mapping", {"mapping": {"ー": "-"}})
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})

# Add token filters  
builder.append_token_filter("lowercase")
builder.append_token_filter("japanese_base_form")

# Build and use
tokenizer = builder.build()
tokens = tokenizer.tokenize("コーヒーショップ")
```

### メタデータの操作

```python
from lindera import Metadata

# Get metadata for a specific dictionary
metadata = Metadata.load("/path/to/ipadic")
print(f"Dictionary: {metadata.dictionary_name}")
print(f"Version: {metadata.dictionary_version}")

# Access schema information
schema = metadata.dictionary_schema
print(f"Schema has {len(schema.fields)} fields")
print(f"Fields: {schema.fields[:5]}")  # First 5 fields
```

## 応用的な使い方

### フィルタ設定の例

文字フィルタとトークンフィルタは、辞書型の引数で設定を受け取ります:

```python
from lindera import TokenizerBuilder

builder = TokenizerBuilder()
builder.set_dictionary("/path/to/ipadic")

# Character filters with dict configuration
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})
builder.append_character_filter("japanese_iteration_mark", {
    "normalize_kanji": "true",
    "normalize_kana": "true"
})
builder.append_character_filter("mapping", {
    "mapping": {"リンデラ": "lindera", "トウキョウ": "東京"}
})

# Token filters with dict configuration  
builder.append_token_filter("japanese_katakana_stem", {"min": 3})
builder.append_token_filter("length", {"min": 2, "max": 10})
builder.append_token_filter("japanese_stop_tags", {
    "tags": ["助詞", "助動詞", "記号"]
})

# Filters without configuration can omit the dict
builder.append_token_filter("lowercase")
builder.append_token_filter("japanese_base_form")

tokenizer = builder.build()
```

`examples/` ディレクトリに包括的な使用例があります:

- `tokenize.py`: 基本的なトークナイズ
- `tokenize_with_filters.py`: 文字フィルタとトークンフィルタの使用
- `tokenize_with_userdict.py`: カスタムユーザー辞書
- `train_and_export.py`: カスタム辞書の学習とエクスポート（`train` feature が必要）
- 多言語トークナイズ
- 高度な設定オプション

## 辞書サポート

### 日本語

- **IPADIC**: 標準的な日本語辞書。一般的なテキストに適しています
- **UniDic**: 詳細な形態素情報を持つ学術辞書

### 韓国語

- **ko-dic**: 形態素解析のための標準韓国語辞書

### 中国語

- **CC-CEDICT**: コミュニティが管理する中国語-英語辞書

### カスタム辞書

- ドメイン固有の用語に対応するユーザー辞書サポート
- CSV 形式で簡単にカスタマイズ可能

## 辞書の学習（実験的）

lindera-python は、`train` feature を有効にしてビルドすることで、アノテーション済みコーパスデータからカスタム形態素解析モデルの学習をサポートします。

### 学習サポート付きでビルド

```shell
# Install with training support
(.venv) % maturin develop --features train
```

### モデルの学習

```python
import lindera.trainer

# Train a model from corpus
lindera.trainer.train(
    seed="path/to/seed.csv",           # Seed lexicon
    corpus="path/to/corpus.txt",       # Training corpus
    char_def="path/to/char.def",       # Character definitions
    unk_def="path/to/unk.def",         # Unknown word definitions
    feature_def="path/to/feature.def", # Feature templates
    rewrite_def="path/to/rewrite.def", # Rewrite rules
    output="model.dat",                # Output model file
    lambda_=0.01,                      # L1 regularization
    max_iter=100,                      # Max iterations
    max_threads=None                   # Auto-detect CPU cores
)
```

### 辞書ファイルのエクスポート

```python
# Export trained model to dictionary files
lindera.trainer.export(
    model="model.dat",              # Trained model
    output="exported_dict/",        # Output directory
    metadata="metadata.json"        # Optional metadata file
)
```

以下のファイルが生成されます:

- `lex.csv`: 語彙ファイル
- `matrix.def`: 連接コスト行列
- `unk.def`: 未知語定義
- `char.def`: 文字定義
- `metadata.json`: 辞書メタデータ（指定した場合）

完全な使用例は `examples/train_and_export.py` を参照してください。

## API リファレンス

### コアクラス

- `TokenizerBuilder`: トークナイザ設定のための Fluent ビルダー
- `Tokenizer`: メインのトークナイズエンジン
- `Token`: テキスト、位置、言語的特徴を持つ個々のトークン
- `CharacterFilter`: テキスト前処理フィルタ
- `TokenFilter`: トークン後処理フィルタ
- `Metadata`: 辞書メタデータと設定
- `Schema`: 辞書スキーマ定義

### 学習関数（`train` feature が必要）

- `train()`: コーパスから形態素解析モデルを学習
- `export()`: 学習済みモデルを辞書ファイルにエクスポート

包括的な API 使用例は `test_basic.py` ファイルを参照してください。
