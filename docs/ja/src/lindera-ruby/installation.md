# インストール

## 前提条件

- **Ruby 3.1 以降**
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **Bundler** -- Ruby の依存関係管理ツール（`gem install bundler`）

## 開発ビルド

lindera-ruby を開発モードでビルドしてインストールします：

```bash
cd lindera-ruby
bundle install
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
```

または、プロジェクトの Makefile を使用します：

```bash
make ruby-develop
```

### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります：

```bash
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

### 辞書埋め込みビルド

辞書をバイナリに直接埋め込むことで、実行時に外部辞書ファイルが不要になります：

```bash
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
```

## Feature フラグ

Feature は環境変数 `LINDERA_FEATURES` にカンマ区切りリストで指定します。

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 無効 |
| `embed-ipadic` | 日本語辞書（IPADIC）の埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）の埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）の埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）の埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）の埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）の埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書の埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
LINDERA_FEATURES="train,embed-ipadic,embed-ko-dic" bundle exec rake compile
```

## インストールの確認

インストール後、Ruby で lindera が利用可能であることを確認します：

```ruby
require 'lindera'

puts Lindera.version
```
