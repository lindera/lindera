# Lindera WASM サンプル

Lindera を WebAssembly にコンパイルした、インタラクティブな形態素解析 Web アプリケーションです。

## 事前準備

- Rust ツールチェイン
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (`cargo install wasm-pack`)
- Node.js (v18以上)

以下のコマンドは、特に記載がない限りリポジトリのルート（`lindera/`）で実行してください。

## ビルド

### 1. WASM パッケージのビルド

```bash
cd lindera-wasm
wasm-pack build --release --target=web
cp js/opfs.js pkg/
cp js/opfs.d.ts pkg/
```

`pkg/` ディレクトリに `.wasm` ファイルと JavaScript グルーコードが生成されます。

### 2. npm 依存パッケージのインストール

```bash
cd example
npm install
```

## 実行

### 開発サーバー

`lindera-wasm/example/` から実行します:

```bash
npm start
```

ブラウザで <http://localhost:8080> を開きます。

開発サーバーには以下が設定されています:

- OPFS（Origin Private File System）アクセス用の CORS ヘッダー
- 辞書ダウンロード時の CORS 問題を回避するための GitHub Releases プロキシ

### プロダクションビルド

`lindera-wasm/example/` から実行します:

```bash
npm run build
```

出力ファイルは `dist/` ディレクトリに生成されます。

## 仕組み

この Web アプリケーションは、辞書ファイル（例: IPADIC）を GitHub Releases からダウンロードし、ブラウザの OPFS にキャッシュします。UI から辞書の管理（ダウンロード/削除）とインタラクティブな形態素解析を行えます。
