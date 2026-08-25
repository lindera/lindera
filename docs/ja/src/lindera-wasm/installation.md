# インストール

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)（stable ツールチェーン）
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)（v0.10 以降）

## 辞書の入手

Lindera WASM はデフォルトで辞書を同梱しません。ブラウザ環境では、OPFS（Origin Private File System）API を使用して辞書を実行時にダウンロードする方法を推奨します。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手できます。ブラウザ環境では、OPFS ヘルパーを使用して辞書をダウンロードしてキャッシュします：

```javascript
import { downloadDictionary, hasDictionary } from 'lindera-wasm/opfs';

if (!await hasDictionary("ipadic")) {
    await downloadDictionary(
        "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
        "ipadic",
    );
}
```

詳細は [OPFS 辞書ストレージ](./opfs.md) を参照してください。

## wasm-pack によるビルド

公開されている npm パッケージと同じ構成でビルドするには、`web` ターゲットを使用します：

```bash
wasm-pack build --target web
```

出力は `lindera-wasm` クレート内の `pkg/` ディレクトリに書き込まれます。

`web` ターゲットのビルドは、ブラウザからネイティブ ES モジュールとして直接利用できるほか、モダンなバンドラー（Vite、Webpack 5 の `asyncWebAssembly` など）からもそのまま利用できます。バンドラー専用のビルドを別途用意する必要はありません。

## なぜ web ターゲットのみなのか（bundler ではなく）

wasm-pack のデフォルトの `--target` は `bundler` ですが、公開されている `lindera-wasm` パッケージは意図的に `--target web` でビルドしています。ターゲット名はやや誤解を招きやすいため、それぞれの実態を説明します：

- `--target bundler`（wasm-pack のデフォルト）は、`.wasm` ファイルを ES モジュールとして直接 `import` する JavaScript を生成します。これはまだ標準化が完了していない WebAssembly ESM integration 提案に依存しており、実際にこの出力を解釈できるのは事実上 `asyncWebAssembly` experiment を有効にした Webpack だけです。このデフォルトは Webpack が支配的だった時代の名残であり、「bundler 汎用」という名前に反して実態は Webpack 専用に近い出力です。たとえば Vite で利用するには追加プラグイン（`vite-plugin-wasm` と top-level-await 対応）が必要です。
- `--target web` は標準技術のみで構成されたコードを生成します。`.wasm` ファイルは `new URL('lindera_wasm_bg.wasm', import.meta.url)` で解決され、明示的な非同期 init 関数の中で `fetch` + `WebAssembly.instantiateStreaming` により読み込まれます。ビルドステップなしでブラウザのネイティブ ES モジュールとして動作するほか、モダンなバンドラー（Vite、Webpack 5、Rollup）は `new URL(..., import.meta.url)` パターンを認識して `.wasm` ファイルをアセットとして配置します。唯一のコストは、API を使う前に一度 `await __wbg_init()` を呼ぶ必要があることです。

まとめると、bundler ターゲットは Webpack に限ればゼロコンフィグですが、web ターゲットは明示的な init 呼び出し 1 回のコストでどこでも動きます。v5 までは両方のビルド（`lindera-wasm-web` と `lindera-wasm-bundler`）を公開していましたが、v6 からは可搬性の高い単一ビルドに統合しました。`lindera-wasm-bundler` からの移行は [v5 から v6 への移行ガイド](../migration_v5_to_v6.md)を参照してください。

## 利用可能な Feature フラグ（上級者向け）

辞書を WASM バイナリに直接埋め込みたい上級者向けに、以下の feature フラグが利用できます。バイナリサイズが大幅に増加しますが、実行時の辞書ダウンロードが不要になります。

| Feature | 辞書 | 言語 |
| --- | --- | --- |
| `embed-ipadic` | IPADIC | 日本語 |
| `embed-unidic` | UniDic | 日本語 |
| `embed-ko-dic` | ko-dic | 韓国語 |
| `embed-cc-cedict` | CC-CEDICT | 中国語 |
| `embed-jieba` | Jieba | 中国語 |
| `embed-cjk` | IPADIC + ko-dic + Jieba | CJK（全言語） |

複数の feature フラグを有効にして複数の辞書を組み合わせることができます：

```bash
wasm-pack build --target web --features embed-ipadic,embed-ko-dic
```

## npm パッケージの命名規則

スコープなしの `lindera-wasm-*` プレフィックスは Lindera プロジェクトの名前空間です。辞書を埋め込んだ独自ビルドを npm に公開する場合は、公式パッケージとの誤認を避けるため、必ず自分のスコープ配下の名前で公開してください：

```text
@your-scope/lindera-wasm-{dict}
```

例：

- `@your-scope/lindera-wasm-ipadic`
- `@your-scope/lindera-wasm-unidic`
- `@your-scope/lindera-wasm-cjk`

公開前にパッケージ名を設定するには、生成された `pkg/package.json` の `name` フィールドを編集します。なお、辞書を埋め込んだパッケージは辞書の再配布にあたるため、各辞書のライセンス条件（帰属表示など）にも従ってください。

> [!NOTE]
> 本プロジェクトのリリースワークフロー（`.github/workflows/release.yml`）が実際にビルドして npm に公開しているのは、`embed-*` feature を一切使わずに `web` ターゲットでビルドした `lindera-wasm` という汎用パッケージのみです。`lindera-wasm-ipadic` のようなスコープなしの辞書名付きパッケージ名は、v3 以前のリリースの名残としてプロジェクトが確保しているもので、現在は更新されていません。第三者がこれらの名前で公開することはできません（しないでください）。本ドキュメント内でのこれらの名前は、対応する `embed-*` feature（上記の[利用可能な Feature フラグ](#利用可能な-feature-フラグ上級者向け)を参照）でローカルビルドした場合の説明用の例に過ぎません。

## npm からのインストール

ビルド済みパッケージが npm で公開されています：

```bash
npm install lindera-wasm
```

または yarn で：

```bash
yarn add lindera-wasm
```

> [!NOTE]
> npm パッケージには辞書が含まれていません。OPFS ヘルパーを使用して辞書を実行時にダウンロードしてください。詳細は [OPFS 辞書ストレージ](./opfs.md) を参照してください。
