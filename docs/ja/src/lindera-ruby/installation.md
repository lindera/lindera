# インストール

> [!NOTE]
> lindera-ruby はまだ RubyGems に公開されていません。ソースからビルドする必要があります。

## 前提条件

- **Ruby 3.1 以降**
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **Bundler** -- Ruby の依存関係管理ツール（`gem install bundler`）

## 辞書の入手

Lindera はパッケージに辞書を同梱していません。ビルド済み辞書を別途入手する必要があります。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手できます。辞書アーカイブをダウンロードしてローカルディレクトリに展開してください：

```bash
# 例: IPADIC 辞書のダウンロードと展開
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## 開発ビルド

lindera-ruby を開発モードでビルドしてインストールします：

```bash
cd lindera-ruby
bundle install
bundle exec rake compile
```

または、プロジェクトの Makefile を使用します：

```bash
make ruby-develop
```

### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります：

```bash
LINDERA_FEATURES="train" bundle exec rake compile
```

## Feature フラグ

Feature は環境変数 `LINDERA_FEATURES` にカンマ区切りリストで指定します。

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 無効 |
| `embed-ipadic` | 日本語辞書（IPADIC）をバイナリに埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）をバイナリに埋め込み | 無効 |
| `embed-ipadic-neologd` | 日本語辞書（IPADIC NEologd）をバイナリに埋め込み | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）をバイナリに埋め込み | 無効 |
| `embed-cc-cedict` | 中国語辞書（CC-CEDICT）をバイナリに埋め込み | 無効 |
| `embed-jieba` | 中国語辞書（Jieba）をバイナリに埋め込み | 無効 |
| `embed-cjk` | 全 CJK 辞書をバイナリに埋め込み（IPADIC、ko-dic、Jieba） | 無効 |

複数の feature を組み合わせることができます：

```bash
LINDERA_FEATURES="train,embed-ipadic,embed-ko-dic" bundle exec rake compile
```

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```ruby
> dictionary = Lindera.load_dictionary("embedded://ipadic")
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## インストールの確認

インストール後、Ruby で lindera が利用可能であることを確認します：

```ruby
require 'lindera'

puts Lindera.version
```
