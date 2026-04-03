# インストール

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)（stable ツールチェーン）
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)（v0.10 以降）

## wasm-pack によるビルド

ターゲット環境に合わせて WASM パッケージをビルドします。辞書を埋め込むには、少なくとも1つの辞書 feature フラグを有効にする必要があります。

### Web（ブラウザ向け ES Modules）

```bash
wasm-pack build --target web --features embed-ipadic
```

### バンドラー（Webpack、Vite、Rollup）

```bash
wasm-pack build --target bundler --features embed-ipadic
```

出力は `lindera-wasm` クレート内の `pkg/` ディレクトリに書き込まれます。

## 利用可能な Feature フラグ

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

ビルド済みパッケージが npm に公開されている場合：

```bash
npm install lindera-wasm-web-ipadic
```

または yarn で：

```bash
yarn add lindera-wasm-web-ipadic
```
