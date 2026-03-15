# Feature フラグ

Lindera は Cargo の feature フラグを使用して、オプション機能と辞書の埋め込みを制御します。

## コア Feature

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `compress` | 辞書の圧縮サポート | 有効 |
| `mmap` | メモリマップドファイルサポート | 有効 |
| `train` | CRF ベースの辞書学習（`lindera-crf` に依存） | CLI のみ |

- `compress` と `mmap` はメインの `lindera` クレートでデフォルトで有効です。
- `train` はデフォルトでは `lindera-cli` でのみ有効です。ライブラリとして使用する場合は `--features train` で明示的に有効にしてください。

## 辞書埋め込み Feature

これらの feature はビルド済み辞書をバイナリに直接埋め込み、実行時に外部辞書ファイルを不要にします。

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
lindera = { version = "2.1.1", features = ["embed-ipadic"] }
```

## 組み合わせ Feature

多言語アプリケーション向けに、複数の辞書を一度に有効にするメタ Feature です。

| Feature | 含まれる辞書 |
| --- | --- |
| `embed-cjk` | IPADIC + ko-dic + Jieba |
| `embed-cjk2` | 代替の CJK 辞書の組み合わせ |
| `embed-cjk3` | 代替の CJK 辞書の組み合わせ |

## Feature フラグの組み合わせ

複数の feature フラグを組み合わせることができます。例えば、日本語と韓国語の辞書を両方埋め込む場合：

```toml
[dependencies]
lindera = { version = "2.1.1", features = ["embed-ipadic", "embed-ko-dic"] }
```

またはコマンドラインから：

```bash
cargo build --features embed-ipadic,embed-ko-dic
```

### 注意事項

- 辞書の埋め込みはバイナリサイズを大幅に増加させます。実際に必要な辞書のみを埋め込んでください。
- `train` feature は `lindera-crf` への依存を追加し、コンパイル時間が増加します。トークナイズのみのユースケースでは不要です。
- `mmap` feature はメモリマップドによる辞書読み込みを有効にし、ディスクから読み込む大規模辞書のメモリ使用量を削減します。埋め込み辞書には影響しません。
