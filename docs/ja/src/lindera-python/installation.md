# インストール

## 前提条件

- **Python 3.10 以降**（3.14 まで）
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **maturin** -- Rust ベースの Python 拡張をビルドするための Python パッケージ

maturin を pip でインストールします：

```bash
pip install maturin
```

## 開発ビルド

lindera-python を開発モードでビルドしてインストールします：

```bash
cd lindera-python
maturin develop
```

または、プロジェクトの Makefile を使用します：

```bash
make python-develop
```

### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります。デフォルトで有効になっています：

```bash
maturin develop --features train
```

### 辞書埋め込みビルド

辞書をバイナリに直接埋め込むことで、実行時に外部辞書ファイルが不要になります：

```bash
maturin develop --features embed-ipadic
```

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 有効 |
| `embed-ipadic` | 日本語辞書（IPADIC）の埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）の埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）の埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）の埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）の埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）の埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書の埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
maturin develop --features "train,embed-ipadic,embed-ko-dic"
```

## インストールの確認

インストール後、Python で lindera が利用可能であることを確認します：

```python
import lindera

print(lindera.version())
```
