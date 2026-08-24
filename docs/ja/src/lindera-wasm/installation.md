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

辞書を埋め込んだパッケージを自分で公開する際の推奨命名規則は以下の通りです：

```text
lindera-wasm
lindera-wasm-{dict}
```

例：

- `lindera-wasm`
- `lindera-wasm-ipadic`
- `lindera-wasm-unidic`
- `lindera-wasm-cjk`

公開前にパッケージ名を設定するには、生成された `pkg/package.json` の `name` フィールドを編集します。

> [!NOTE]
> 本プロジェクトのリリースワークフロー（`.github/workflows/release.yml`）が実際にビルドして npm に公開しているのは、`embed-*` feature を一切使わずに `web` ターゲットでビルドした `lindera-wasm` という汎用パッケージのみです。`lindera-wasm-ipadic` のような辞書名付きのパッケージ名はどこにも公開されていません。これは、対応する `embed-*` feature（上記の[利用可能な Feature フラグ](#利用可能な-feature-フラグ上級者向け)を参照）でローカルビルドし、自分でパッケージ名をリネームした場合に得られる名前の例に過ぎません。

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
