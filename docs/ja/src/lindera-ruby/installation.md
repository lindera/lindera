# インストール

v6.1.0 以降の lindera-ruby は、[RubyGems](https://rubygems.org/gems/lindera) から `lindera` としてインストールできます。ソース gem のため、`gem install` の実行時に同梱の Rust ソースからネイティブ拡張をコンパイルします（数分かかります）。

## 前提条件

- **Ruby 3.1 以降**と、ネイティブ gem のビルドに必要なツール（C コンパイラと `make`）
- **Rust ツールチェーン 1.88 以降** -- [rustup](https://rustup.rs/) 経由でインストール
- **libclang** -- Debian/Ubuntu は `libclang-dev`、macOS は Xcode コマンドラインツール
- **Bundler** -- ソースからビルドする場合のみ（`gem install bundler`）

## RubyGems からのインストール

```bash
gem install lindera
```

または `Gemfile` に追加します：

```ruby
gem "lindera"
```

> [!NOTE]
> lindera v6.1.0 以降を使用してください。v6.1.0 より前に公開された `lindera` gem は、インストール時のコンパイルに失敗します。それらのバージョンは、以下の手順でソースからビルドしてください。

gem は [crates.io](https://crates.io/crates/lindera) 上の同じバージョンの `lindera` と `lindera-binding-core` に対してコンパイルされます（この 2 つはバージョンを完全に固定しています）。gem には `Cargo.lock` が含まれないため、それ以外の依存（ほかの `lindera-*` クレートを含む）は、インストール時に互換性のある最新のリリースに解決されます。デフォルトの feature（`train`）のみを有効にし、辞書は埋め込みません。辞書は[辞書の入手](#辞書の入手)を参照してください。その他の [Feature フラグ](#feature-フラグ)を有効にするには、インストール時に `LINDERA_FEATURES` を指定します：

```bash
LINDERA_FEATURES="embed-ipadic" gem install lindera
```

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
make build-lindera-ruby
```

`make test-lindera-ruby` を実行すると、Rust のユニットテストと Ruby の minitest
スイートの両方が実行されます。

### gem のビルド

`bundle exec rake build`（または `make package-lindera-ruby`）は、ソース gem を
`lindera-ruby/pkg/` に出力します。`cargo package` が正規化したクレートを同梱するため、
gem はこのリポジトリの外でも、crates.io 上の同じバージョンの `lindera` と
`lindera-binding-core` に対してコンパイルできます。`gem build` を直接実行しないで
ください。クレート自身の `Cargo.toml` は Cargo ワークスペースの中でしか解決できません。

`make test-lindera-ruby-gem`（Docker が必要）は、gem をビルドし、クリーンな `ruby`
コンテナで `gem install` を実行して、テキストをトークナイズします。このテストは
`lindera` クレートをチェックアウトからコンパイルするため、crates.io に未公開の
バージョンでも実行できます。

### 学習機能付きビルド

`train` feature は、CRF ベースの辞書学習機能を有効にします。デフォルトで有効です：

```bash
LINDERA_FEATURES="train" bundle exec rake compile
```

## Feature フラグ

Feature は環境変数 `LINDERA_FEATURES` にカンマ区切りリストで指定します。`bundle exec rake compile` と `gem install` のどちらでも使えます。

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `train` | CRF 学習機能 | 有効 |
| `embed-ipadic` | 日本語辞書（IPADIC）をバイナリに埋め込み | 無効 |
| `embed-unidic` | 日本語辞書（UniDic）をバイナリに埋め込み | 無効 |
| `embed-sudachidict` | 日本語辞書（SudachiDict）をバイナリに埋め込み | 無効 |
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
