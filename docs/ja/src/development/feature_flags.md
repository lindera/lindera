# Feature フラグ

Lindera は Cargo の feature フラグを使用して、オプション機能と辞書の埋め込みを制御します。

## コア Feature

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `mmap` | メモリマップドファイルサポート | 有効 |
| `train` | CRF ベースの辞書学習（`lindera-crf` に依存） | CLI のみ |

- `mmap` はメインの `lindera` クレートでデフォルトで有効です。
- `train` はデフォルトでは `lindera-cli` でのみ有効です。ライブラリとして使用する場合は `--features train` で明示的に有効にしてください。

## 外部辞書の使用（推奨）

推奨される方法は、ビルド済み辞書を外部ファイルとして使用することです。[GitHub Releases](https://github.com/lindera/lindera/releases) から辞書をダウンロードし、実行時にそのパスを指定してください：

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

この使用方法では、追加の feature フラグは不要です。

## 辞書埋め込み Feature（上級者向け）

これらの feature はビルド済み辞書をバイナリに直接埋め込み、実行時に外部辞書ファイルを不要にします。自己完結型バイナリが必要な上級者向けの機能です。

| Feature | 辞書 | 言語 |
| --- | --- | --- |
| `embed-ipadic` | IPADIC | 日本語 |
| `embed-ipadic-neologd` | IPADIC NEologd | 日本語 |
| `embed-unidic` | UniDic | 日本語 |
| `embed-ko-dic` | ko-dic | 韓国語 |
| `embed-cc-cedict` | CC-CEDICT | 中国語 |
| `embed-jieba` | Jieba | 中国語 |

いずれもデフォルトでは無効です。必要に応じて有効にしてください：

```toml
[dependencies]
lindera = { version = "2.3.2", features = ["embed-ipadic"] }
```

埋め込みを有効にした場合、以下のように辞書を読み込めます：

```rust
let dictionary = load_dictionary("embedded://ipadic")?;
```

### 組み合わせ Feature

多言語アプリケーション向けに、複数の辞書を一度に有効にするメタ Feature です。

| Feature | 含まれる辞書 |
| --- | --- |
| `embed-cjk` | IPADIC + ko-dic + Jieba |
| `embed-cjk2` | UniDic + ko-dic + Jieba |
| `embed-cjk3` | IPADIC NEologd + ko-dic + Jieba |

### Feature フラグの組み合わせ

複数の feature フラグを組み合わせることができます。例えば、日本語と韓国語の辞書を両方埋め込む場合：

```toml
[dependencies]
lindera = { version = "2.3.2", features = ["embed-ipadic", "embed-ko-dic"] }
```

またはコマンドラインから：

```bash
cargo build --features embed-ipadic,embed-ko-dic
```

### 注意事項

- 辞書の埋め込みはバイナリサイズを大幅に増加させます。実際に必要な辞書のみを埋め込んでください。
- `train` feature は `lindera-crf` への依存を追加し、コンパイル時間が増加します。トークナイズのみのユースケースでは不要です。
- `mmap` feature はメモリマップドによる辞書読み込みを有効にし、ディスクから読み込む大規模辞書のメモリ使用量を削減します。埋め込み辞書には影響しません。
