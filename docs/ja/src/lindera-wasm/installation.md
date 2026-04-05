# インストール

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)（stable ツールチェーン）
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)（v0.10 以降）

## 辞書の入手

Lindera WASM はデフォルトで辞書を同梱しません。ブラウザ環境では、OPFS（Origin Private File System）API を使用して辞書を実行時にダウンロードする方法を推奨します。

### GitHub Releases からのダウンロード

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) ページから入手できます。ブラウザ環境では、OPFS ヘルパーを使用して辞書をダウンロードしてキャッシュします：

```javascript
import { downloadDictionary, hasDictionary } from 'lindera-wasm-web/opfs';

if (!await hasDictionary("ipadic")) {
    await downloadDictionary(
        "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
        "ipadic",
    );
}
```

詳細は [OPFS 辞書ストレージ](./opfs.md) を参照してください。

## wasm-pack によるビルド

ターゲット環境に合わせて WASM パッケージをビルドします：

### Web（ブラウザ向け ES Modules）

```bash
wasm-pack build --target web
```

### バンドラー（Webpack、Vite、Rollup）

```bash
wasm-pack build --target bundler
```

出力は `lindera-wasm` クレート内の `pkg/` ディレクトリに書き込まれます。

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

npm に公開する際の推奨命名規則は以下の通りです：

```text
lindera-wasm-{target}
lindera-wasm-{target}-{dict}
```

例：

- `lindera-wasm-web`
- `lindera-wasm-web-ipadic`
- `lindera-wasm-bundler-unidic`
- `lindera-wasm-web-cjk`

公開前にパッケージ名を設定するには、生成された `pkg/package.json` の `name` フィールドを編集します。

## npm からのインストール

ビルド済みパッケージが npm で公開されています：

```bash
npm install lindera-wasm-web
```

または yarn で：

```bash
yarn add lindera-wasm-web
```

> [!NOTE]
> npm パッケージには辞書が含まれていません。OPFS ヘルパーを使用して辞書を実行時にダウンロードしてください。詳細は [OPFS 辞書ストレージ](./opfs.md) を参照してください。
