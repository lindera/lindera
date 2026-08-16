# アーキテクチャ

## モジュール構成

```text
lindera-dictionary/src/
├── lib.rs               # パブリックAPI
├── dictionary.rs        # Dictionary, UserDictionary
├── builder.rs           # DictionaryBuilder
├── loader.rs            # DictionaryLoader trait, FSDictionaryLoader
├── viterbi.rs           # Lattice, Edge, Viterbiセグメンテーション
├── nbest.rs             # NBestGenerator (Forward-DP Backward-A*)
├── mode.rs              # Mode (Normal/Decompose), Penalty
├── error.rs             # LinderaError, LinderaErrorKind
├── assets.rs            # ダウンロードとファイル管理
├── macros.rs            # 辞書クレート共通の embedded_dictionary! マクロ
├── dictionary/
│   ├── character_definition.rs    # 文字種定義
│   ├── connection_cost_matrix.rs  # 連接コスト行列
│   ├── context_id_map.rs          # ContextIdMap（連接コストのコンテキストID再割り当て）
│   ├── prefix_dictionary.rs       # ダブル配列トライ辞書
│   ├── unknown_dictionary.rs      # 未知語処理
│   ├── metadata.rs                # 辞書メタデータ
│   └── schema.rs                  # スキーマ定義
```

## 主要コンポーネント

### Dictionary / UserDictionary

コンパイル済み辞書データを保持する主要データ構造です。`Dictionary`は文字種定義、連接コスト行列、前方一致辞書（ダブル配列トライ）、および未知語辞書を含みます。`UserDictionary`を使用すると、システム辞書の上にカスタム語彙を追加できます。

### DictionaryBuilder

ソースCSVファイルから辞書をビルドするためのFluent APIです。MeCab形式の辞書ソースを、実行時に使用されるバイナリ形式にコンパイルします。4つのビルドステージ（メタデータ、未知語辞書、前方一致辞書、連接コスト行列）は、wasm以外のターゲットではスコープ付きスレッド上で並行実行され、OSスレッドを持たないwasmでは逐次ビルドにフォールバックします。並行実行パスでは4ステージすべての作業データを同時に保持するため、ピークメモリ使用量が大きくなります。

### DictionaryLoader / FSDictionaryLoader

`DictionaryLoader`はコンパイル済み辞書を読み込むためのtraitです。`FSDictionaryLoader`はファイルシステムベースの実装で、ディレクトリから辞書ファイルを読み込みます。オプションでメモリマップドファイルをサポートします。

### 埋め込み辞書マクロ

`embedded_dictionary!`マクロ（`lindera-dictionary/src/macros.rs`、`#[macro_export]`）は、各辞書クレートがコンパイル済み辞書をバイナリに埋め込むために必要なボイラープレートを生成します。具体的には、`include_bytes!`経由で辞書コンポーネントを読み込む`load()`関数と、`DictionaryLoader`を実装するローダー構造体です。各辞書クレート（`lindera-ipadic`、`lindera-ipadic-neologd`、`lindera-unidic`、`lindera-ko-dic`、`lindera-cc-cedict`、`lindera-jieba`）の`embedded.rs`は、読み込みロジックを重複実装する代わりにこのマクロを呼び出します。

### Viterbi (Lattice, Edge)

入力テキストから候補トークンのラティスを構築し、Viterbiアルゴリズムを使用して最適なセグメンテーションパスを探索します。ラティス内の各`Edge`は、関連するコスト（単語コスト + 連接コスト）を持つ候補トークンを表します。

### NBestGenerator

Forward-DP Backward-A*アルゴリズムを使用してN-bestセグメンテーションパスを生成します。これにより、アプリケーションは単一の最適パスを超えた代替セグメンテーションを検討できます。

### Mode

トークナイゼーションの動作を制御します：

- **Normal**: 最適なViterbiパスを使用した標準的なトークナイゼーション
- **Decompose**: 設定可能な`Penalty`閾値に基づいて複合名詞をさらに分割

### 辞書フォーマットバージョン

ビルド済み辞書ディレクトリは、自身が書かれたオンディスクレイアウトを `metadata.json` の `format_version` として記録します。値は `DictionaryBuilder::build_metadata` が `DICTIONARY_FORMAT_VERSION`（`lindera-dictionary/src/dictionary/metadata.rs`）から刻印します。いま何を書いたのかを正しく主張できるのはビルダだけなので、**ソース** `metadata.json` 側の値は無視されます。辞書ディレクトリのロード時には記録されたバージョンを検証し、一致しなければ対処方法を含むエラーを返します。

この検証が重要なのは、ほとんどの成果物が自前のヘッダを持たないためです。`matrix.mtx`・`dict.vals`・`dict.words` は生の配列なので、古いレイアウトの辞書でも長さが辻褄の合う範囲なら**エラーにならず異常な値として解釈されます**。各辞書クレートに手書きでチェックインされている**ソース** `metadata.json` はビルドの**入力**を記述するもので、`format_version` を持たず、フォーマット検証の対象にもなりません。

ビルド済み成果物のバイト列が変わる変更では必ず `DICTIONARY_FORMAT_VERSION` を上げてください。シリアライズ形式をそのまま書き出している依存クレートの更新（`dict.da` の `daachorse`、`char_def.bin` / `unk.bin` の `rkyv`）も対象で、本クレートのコードが 1 行も変わらないため見落としやすい箇所です。ビルドキャッシュがフォーマットバージョンをキーに含めているのは、まさにこのためです。

### コンテキストIDリマッピング

辞書メタデータ（`lindera-dictionary/src/dictionary/metadata.rs`）は、`connection_id_mapping: bool`フラグとオプションの`context_id_map: Option<ContextIdMap>`を持ちます。辞書クレート（例: `lindera-unidic`）が`connection_id_mapping`を有効にすると、`DictionaryBuilder`はビルド時にアクセス頻度に基づいて連接行列の左右コンテキストIDを再割り当てし、頻繁に使用される連接コストのセルが近くに集まるようにしてキャッシュ局所性を高めます。`DictionaryBuilder::with_context_id_freq`は、IDのランク付けに使用するバンドル済み頻度ヒストグラムを（任意で）アタッチするためのメソッドで、再割り当て自体は`lindera-dictionary/src/builder/context_id_remap.rs`で計算されます。`ContextIdMap`（`lindera-dictionary/src/dictionary/context_id_map.rs`）は結果として得られる`left`/`right`の置換を保持し、同じマッピングを後から再適用できるようにビルド済みの`metadata.json`に永続化されます。ユーザー辞書は常に元の（再割り当てされていない）ID空間でコンパイルされるため、`UserDictionary::remap_context_ids`（`lindera-dictionary/src/dictionary.rs`）は永続化された`ContextIdMap`を使って、ユーザー辞書のコンテキストIDを、それが紐付くシステム辞書と同じ空間に再割り当てします。この再割り当ては全単射（bijective）な付け替えであるため、トークナイゼーションの出力は変わらず、ルックアップの局所性のみが変化します。

### 学習

CRFベースの辞書学習パイプラインは、本クレートのランタイム型の上に構築された独立クレート `lindera-trainer` に含まれます。詳細は学習パイプラインのドキュメントを参照してください。

## Featureフラグ

| Feature | 説明 | デフォルト |
| --------- | ------ | ----------- |
| `mmap` | ファイルシステム辞書読み込みのためのメモリマップドファイルサポート（単語リストファイルと接続コスト行列は遅延読み込みされ、全体展開されるのはトライのみ） | Yes |
| `build_rs` | 辞書ソースのHTTPダウンロード | No |
| `ctxfreq` | 実験的機能: 連接行列のアクセス頻度プロファイリングを計測し、コンテキストID頻度リマップの構築に使用 | No |
