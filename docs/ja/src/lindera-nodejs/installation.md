# インストール

## npm からのインストール

ビルド済みパッケージが npm で公開予定です：

```bash
npm install lindera
```

> [!NOTE]
> npm パッケージには辞書が含まれていません。下記の[辞書の入手](#辞書の入手)を参照してください。
> ブラウザ/WASM での利用には [lindera-wasm](../lindera-wasm/installation.md) を参照してください。

## ソースからのビルド

### 前提条件

- **Node.js 18 以降**（LTS バージョン推奨）
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **NAPI-RS CLI** -- Rust で Node.js ネイティブアドオンをビルドするための CLI ツール

NAPI-RS CLI をグローバルにインストールします：

```bash
npm install -g @napi-rs/cli
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

lindera-nodejs を開発モードでビルドします：

```bash
cd lindera-nodejs
npm install
npm run build
```

または、プロジェクトの Makefile を使用します：

```bash
make nodejs-develop
```

### 学習機能付きビルド

`train` feature を有効にすると、CRF ベースの辞書学習機能が利用可能になります。デフォルトで有効になっています：

```bash
npm run build -- --features train
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
npm run build -- --features "train,embed-ipadic,embed-ko-dic"
```

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```javascript
> const dictionary = loadDictionary("embedded://ipadic");
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## インストールの確認

インストール後、Node.js で lindera が利用可能であることを確認します：

```javascript
const lindera = require("lindera");

console.log(lindera.version());
```

または ES modules を使用する場合：

```javascript
import { version } from "lindera";

console.log(version());
```
