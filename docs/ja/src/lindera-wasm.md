# Lindera WASM

Lindera WASM は、[wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) を使用して構築された Lindera 形態素解析エンジンの WebAssembly バインディングです。Web ブラウザ、Node.js、バンドラー環境で日本語、韓国語、中国語のテキストトークナイズを直接実行できます。

## 配布フォーマット

Lindera WASM は [wasm-pack](https://rustwasm.github.io/wasm-pack/) を通じて複数の配布フォーマットをサポートしています：

| ターゲット | 用途 | モジュールシステム |
| --- | --- | --- |
| `web` | ブラウザ ESM | ES Modules |
| `bundler` | Webpack、Vite、Rollup | ES Modules（バンドラー解決） |

## 辞書パッケージ

各パッケージはオフライン使用のために特定の辞書を埋め込みます：

| Feature フラグ | 辞書 | 言語 |
| --- | --- | --- |
| （なし） | 埋め込み辞書なし | -- |
| `embed-ipadic` | IPADIC | 日本語 |
| `embed-unidic` | UniDic | 日本語 |
| `embed-ko-dic` | ko-dic | 韓国語 |
| `embed-cc-cedict` | CC-CEDICT | 中国語 |
| `embed-jieba` | Jieba | 中国語 |
| `embed-cjk` | IPADIC + ko-dic + Jieba | CJK |

## セクション

- [インストール](./lindera-wasm/installation.md) -- lindera-wasm パッケージのビルドとインストール
- [クイックスタート](./lindera-wasm/quickstart.md) -- 最小限の動作例
- [Tokenizer API](./lindera-wasm/tokenizer_api.md) -- JavaScript/TypeScript の完全な API リファレンス
- [辞書管理](./lindera-wasm/dictionary_management.md) -- 辞書の読み込みとビルド
- [ブラウザでの使用](./lindera-wasm/browser_usage.md) -- Web アプリケーションとの統合
