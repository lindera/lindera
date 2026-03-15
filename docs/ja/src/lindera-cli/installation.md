# インストール

## cargo経由でインストール

cargo経由でバイナリをインストールできます：

```shell
% cargo install lindera-cli
```

## GitHub Releasesからダウンロード

または、以下のリリースページからビルド済みバイナリをダウンロードすることもできます：

- [https://github.com/lindera/lindera/releases](https://github.com/lindera/lindera/releases)

## ソースからビルド

### IPADIC（日本語辞書）を含めてビルド

"ipadic" 機能フラグを使用すると、LinderaにIPADICを含めることができます。

```shell
% cargo build --release --features=embed-ipadic
```

### UniDic（日本語辞書）を含めてビルド

"unidic" 機能フラグを使用すると、LinderaにUniDicを含めることができます。

```shell
% cargo build --release --features=embed-unidic
```

### ko-dic（韓国語辞書）を含めてビルド

"ko-dic" 機能フラグを使用すると、Linderaにko-dicを含めることができます。

```shell
% cargo build --release --features=embed-ko-dic
```

### CC-CEDICT（中国語辞書）を含めてビルド

"cc-cedict" 機能フラグを使用すると、LinderaにCC-CEDICTを含めることができます。

```shell
% cargo build --release --features=embed-cc-cedict
```

### Jieba（中国語辞書）を含めてビルド

"jieba" 機能フラグを使用すると、LinderaにJiebaを含めることができます。

```shell
% cargo build --release --features=embed-jieba
```

### 辞書なしでビルド

Linderaのバイナリサイズを削減するには、機能フラグを省略します。
これにより、辞書が含まれなくなるため、トークナイザーとトレーナーのみを含むバイナリになります。

```shell
% cargo build --release
```

### 全機能を含めてビルド

```shell
% cargo build --release --all-features
```
