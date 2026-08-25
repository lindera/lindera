# インストール

## cargo経由でインストール

cargo経由でバイナリをインストールできます：

```shell
% cargo install lindera-cli
```

## GitHub Releasesからダウンロード

または、以下のリリースページからビルド済みバイナリをダウンロードすることもできます：

- [https://github.com/lindera/lindera/releases](https://github.com/lindera/lindera/releases)

## 辞書の入手

Lindera はバイナリに辞書を同梱していません。最も簡単な入手方法は `download` サブコマンドで、CLI と同じバージョンのビルド済み辞書を取得して OS 標準のアプリケーションデータディレクトリにインストールします：

```shell
% lindera download ipadic
```

ダウンロード後は辞書名で参照できます：

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dict ipadic
```

利用可能な辞書名と保存場所については[コマンド](commands.md#download)を参照してください。

または、[GitHub Releases](https://github.com/lindera/lindera/releases) ページからビルド済み辞書を手動でダウンロードすることもできます：

```shell
# 例: IPADIC 辞書のダウンロードと展開
% curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
% unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

CLI 使用時に展開した辞書パスを指定します（アーカイブには `lindera-ipadic` ディレクトリが含まれる点に注意）：

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dict /path/to/ipadic/lindera-ipadic
```

## ソースからビルド

### 辞書なしでビルド（デフォルト）

辞書を埋め込まず、トークナイザーとトレーナーのみを含むバイナリをビルドします：

```shell
% cargo build --release
```

### 全機能を含めてビルド

```shell
% cargo build --release --all-features
```

### 辞書埋め込みビルド（上級者向け）

辞書をバイナリに直接埋め込みたい上級者向けに、`embed-*` feature フラグを使用できます。実行時の外部辞書ファイルが不要になりますが、バイナリサイズが増加します。

#### IPADIC（日本語辞書）

```shell
% cargo build --release --features=embed-ipadic
```

#### IPADIC NEologd（日本語辞書）

```shell
% cargo build --release --features=embed-ipadic-neologd
```

#### UniDic（日本語辞書）

```shell
% cargo build --release --features=embed-unidic
```

#### SudachiDict（日本語辞書）

```shell
% cargo build --release --features=embed-sudachidict
```

#### ko-dic（韓国語辞書）

```shell
% cargo build --release --features=embed-ko-dic
```

#### CC-CEDICT（中国語辞書）

```shell
% cargo build --release --features=embed-cc-cedict
```

#### Jieba（中国語辞書）

```shell
% cargo build --release --features=embed-jieba
```

> [!TIP]
> `embed-*` feature フラグ付きでビルドした後、`embedded://` スキームで埋め込み辞書をロードできます：
>
> ```shell
> % echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。
