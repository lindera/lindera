# インストール

## PyPI からのインストール

ビルド済みホイールが [PyPI](https://pypi.org/project/lindera-python/) で公開されています：

```bash
pip install lindera-python
```

> [!NOTE]
> PyPI パッケージには辞書が含まれていません。下記の[辞書の入手](#辞書の入手)を参照してください。

## 辞書の入手

Lindera はパッケージに辞書を同梱していません。ビルド済み辞書を別途入手する必要があります。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手できます。辞書アーカイブをダウンロードしてローカルディレクトリに展開してください：

```bash
# 例: IPADIC 辞書のダウンロードと展開
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## ソースからのビルド

特定の feature フラグを有効にする必要がある場合など、ソースからビルドするには以下の前提条件が必要です：

- **Python 3.10 以降**（3.14 まで）
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **maturin** -- Rust ベースの Python 拡張をビルドするための Python パッケージ

maturin を pip でインストールします：

```bash
pip install maturin
```

### 開発ビルド

lindera-python を開発モードでビルドしてインストールします：

```bash
cd lindera-python
maturin develop
```

または、プロジェクトの Makefile を使用します：

```bash
make python-develop
```

#### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります。デフォルトで有効になっています：

```bash
maturin develop --features train
```

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 有効 |
| `embed-ipadic` | 日本語辞書（IPADIC）をバイナリに埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）をバイナリに埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）をバイナリに埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）をバイナリに埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）をバイナリに埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）をバイナリに埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書をバイナリに埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
maturin develop --features "train,embed-ipadic,embed-ko-dic"
```

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```python
> dictionary = load_dictionary("embedded://ipadic")
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## インストールの確認

インストール後、Python で lindera が利用可能であることを確認します：

```python
import lindera

print(lindera.version())
```
