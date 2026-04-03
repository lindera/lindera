# インストール

## 前提条件

- **Node.js 18 以降**（LTS バージョン推奨）
- **Rust ツールチェーン** -- [rustup](https://rustup.rs/) 経由でインストール
- **NAPI-RS CLI** -- Rust で Node.js ネイティブアドオンをビルドするための CLI ツール

NAPI-RS CLI をグローバルにインストールします：

```bash
npm install -g @napi-rs/cli
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

### 辞書埋め込みビルド

辞書をバイナリに直接埋め込むことで、実行時に外部辞書ファイルが不要になります：

```bash
npm run build -- --features embed-ipadic
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
npm run build -- --features "train,embed-ipadic,embed-ko-dic"
```

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
