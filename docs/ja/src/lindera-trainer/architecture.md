# アーキテクチャ

## モジュール構成

```text
lindera-trainer/src/
├── lib.rs                # Trainer: CRF学習の実行制御。パブリックAPIの再エクスポート
├── config.rs             # TrainerConfig: 種辞書・char.def・feature.def・rewrite.defのパース
├── corpus.rs              # Corpus, Example, Word: 学習データの表現
├── feature_extractor.rs  # FeatureExtractor: 素性テンプレート解析と素性ID管理
├── feature_rewriter.rs   # DictionaryRewriter: MeCab互換の3セクション書き換え
└── model.rs               # Model, SerializableModel: 学習済みモデルの保持・シリアライズ・辞書出力
```

## 主要コンポーネント

### TrainerConfig

学習に必要な5つの入力ファイル――種辞書（`lex.csv`）、文字定義（`char.def`）、未知語定義（`unk.def`）、素性テンプレート（`feature.def`）、書き換えルール（`rewrite.def`）――をパースし、`Trainer`が利用する設定情報にまとめます。`TrainerConfig::from_readers`（またはファイルパスを直接渡せるラッパーの`from_paths`）は、種辞書から表層形と素性の語彙を抽出し、パースした素性テンプレートから`FeatureExtractor`を構築し、書き換えルールから`DictionaryRewriter`を構築し、さらに同じ入力から最小限のインメモリ`lindera_dictionary::dictionary::Dictionary`（文字定義・未知語カテゴリ・システム辞書）を組み立てます。また、`surfaces()`、`surface_features()`、`get_features()`に加え、`user_lexicon()` / `add_user_lexicon_entry()` / `load_user_lexicon_from_content()`によるユーザー辞書へのアクセス、`metadata()`も提供します。

### Corpus / Example / Word

アノテーション付き学習データを表現します。`Word`は表層形とその素性文字列（カンマ区切り）のペアです。`Example`は1つの学習用の文であり、`Vec<Word>`から構築され、各語の表層形を連結して元の文を復元します。`Corpus`は`Example`の集合です。`Corpus::from_reader`はタブ区切りの`surface<TAB>features`形式の行をパースし、空行または`EOS`のみの行を文の区切りとして扱います。

### FeatureExtractor

MeCab互換の素性テンプレートを解析し、生成された素性文字列から内部で採番される`NonZeroU32`型の素性IDへのマッピングを管理します。サポートされるテンプレートのプレースホルダは、`%F[n]` / `%F?[n]`（インデックス`n`の素性フィールド。`?`付きは値が`*`の場合にスキップ）、`%t`（文字カテゴリ）、`%w`（表層形、ユニグラムのみ）、`%u` / `%l` / `%r`（書き換え後のユニグラム／左文脈／右文脈の素性文字列全体）、およびバイグラムの左右文脈フィールド用の`%L[n]` / `%L?[n]` / `%R[n]` / `%R?[n]`です。`extract_unigram_feature_ids[_with_ctx]`、`extract_left_feature_ids[_with_ctx]`、`extract_right_feature_ids[_with_ctx]`は、パース済みテンプレートを素性の配列（バイグラム抽出の場合は表層形／ufeature／lfeature／rfeatureを保持するオプションの`TemplateContext`も併せて）に適用し、対応する素性IDを返します。初回利用時には新しいIDが割り当てられます。

### DictionaryRewriter

MeCabの3セクション形式`rewrite.def`（`[unigram rewrite]`、`[left rewrite]`、`[right rewrite]`）を実装します。各セクションは`pattern<TAB>replacement`形式のルールの並びを持ち、内部の`FeatureRewriterBuilder`によって`FeatureRewriter`というprefix trie（前方一致木）に構築されます。`DictionaryRewriter::from_reader`は3セクションすべてをパースしますが、セクションヘッダのないファイルは後方互換性のため（右文脈書き換えのみのレガシー形式として）扱います。`rewrite()`は素性文字列に3つの書き換え器すべてを適用し、`(ufeature, lfeature, rfeature)`を返します。マッチするルールがないセクションは入力をそのまま通過させます。`rewrite_cached()`は入力の素性文字列ごとに結果をメモ化します（MeCabの`rewrite2`に相当）。

### Model / SerializableModel

`Model`は`Trainer::train`が生成する学習済みモデルです。学習された`lindera_crf::RawModel`、学習に用いた`TrainerConfig`、抽出済みの素性の重みとラベル、そして学習後に`read_user_lexicon`で追加されたユーザー辞書エントリを保持します。学習されたCRFの重みは、MeCab互換の整数コストに変換されます（`tocost(weight, cost_factor) = clamp(-weight * cost_factor, -32767, 32767)`）。この際のコストファクターは、`i16`の値域を最大限活用できるよう計算されます（`calculate_cost_factor`）。`Model`は自身をシリアライズでき（`write_model`）、辞書ソースファイルを直接エクスポートすることもできます。

`SerializableModel`は、`Model::read_model`がリーダーから以前学習したモデルを読み込んだ際に返される、`rkyv`でシリアライズ可能な素のデータ型です（`lindera export` CLIコマンドで使用されます）。`Model`と同じ学習結果の情報――素性の重み、ラベル、品詞情報、連接コスト行列、未知語カテゴリ、保存されている`char.def` / `feature.def` / `rewrite.def`の内容、コストファクター、左右文脈IDマッピング――を`TrainerConfig`を持たない所有データとして保持し、辞書エクスポートファイルを生成するための独自のライターメソッド群を提供します。

#### `Model`のパブリックメソッド

| メソッド | 説明 |
| --- | --- |
| `read_user_lexicon<R: Read>(&mut self, rdr: R) -> Result<()>` | ユーザー定義辞書（種辞書と同じ`surface,left_id,right_id,cost,feature...`のCSV形式）をモデルに読み込みます。これにより、後続のエクスポート処理で推定された連接IDとコストを割り当てられるようになります。ユーザー辞書が必要な場合は、辞書を書き出す前に呼び出す必要があります。 |
| `write_model<W: Write>(&self, writer: &mut W) -> Result<()>` | 学習済みモデル全体（素性の重み、ラベル、品詞情報、連接行列、未知語カテゴリ、保存された定義ファイルの内容、コストファクター、左右IDマッピング）を`rkyv`バイナリ形式でシリアライズします。 |
| `read_model<R: Read>(reader: R) -> Result<SerializableModel>` | 関連関数。`write_model`が書き出したデータから`SerializableModel`をデシリアライズします。まず`rkyv`形式を試み、失敗した場合はレガシーのJSON形式にフォールバックします。`feature_sets`フィールドが存在しない旧形式のモデルについては、`feature_weights`から補完します。 |
| `write_dictionary<W1, W2, W3, W4>(&self, lexicon_wtr, connector_wtr, unk_handler_wtr, user_lexicon_wtr) -> Result<()>` | レキシコン、連接コスト行列、未知語辞書、ユーザー辞書を一度にまとめて書き出す便利メソッドです。 |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | マージされたCRFモデルから得た連接IDとコストを用いて、種辞書の各語彙エントリについて`lex.csv`形式のエントリ（`surface,left_id,right_id,cost,features...`）を書き出します。 |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | すべての`(right_id, left_id)`の組み合わせ（BOS/EOSを含む）を網羅する密な`matrix.def`形式の連接コスト行列を書き出します。学習中に一度も出現しなかった組み合わせには最大のペナルティコスト（`i16::MAX`）が設定され、Viterbi探索が未学習の遷移を避けるようにします。 |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | 各未知語文字カテゴリについて、学習された連接ID・コストと`unk.def`由来の素性文字列を用いて`unk.def`形式のエントリを書き出します。 |
| `get_unknown_word_cost(&self, category: usize) -> i32` | 指定した未知語カテゴリのインデックスに対して設定されているコスト（`unk.def`のコスト列由来）を返します。設定がない場合はデフォルト値`2000`を返します。 |
| `num_features(&self) -> usize` | 学習済みモデルが保持する素性の重みの数を返します。 |
| `num_labels(&self) -> usize` | 学習済みモデルにおけるラベル（語彙の表層形と未知語カテゴリの合計）の数を返します。 |
| `raw_model(&self) -> &lindera_crf::RawModel` | 高度な操作や低レベルの処理のために、内部の`lindera-crf`のraw modelへアクセスします。 |
| `write_bigram_details<L: Write, R: Write, C: Write>(&self, left_wtr, right_wtr, cost_wtr) -> Result<()>` | 辞書最適化やデバッグのために、バイグラムの連接素性とコストを記述した3つの診断用ファイル（左側素性一覧、右側素性一覧、左右素性ペアごとのコスト）を書き出します。 |
| `evaluate(&self, test_lattices: &[lindera_crf::Lattice]) -> f64` | raw modelの素性の重みの絶対値の平均を簡易的な評価スコアとして返します。引数`test_lattices`は現時点では未使用であり、保留データに対するモデルの評価はまだ行われません。 |
| `write_dictionary_buffers(&self, lexicon, connector, unk_handler, user_lexicon: &mut Vec<u8>) -> Result<()>` | ラベル、素性の重み、ユーザーエントリ数、表層形を、それぞれ生の`rkyv`バイトバッファへシリアライズします。CSVベースの`write_dictionary`に対する、より低レベルな代替エクスポート手段です。 |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 学習された左文脈IDをその素性文字列にマッピングした`left-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 学習された右文脈IDをその素性文字列にマッピングした`right-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |

#### `SerializableModel`のパブリックメソッド

`Model::read_model`が返す値に対して利用できるメソッド群で、以前に学習・シリアライズされたモデルから辞書ソースファイルを再エクスポートするために使用します（`lindera export` CLIコマンドで利用されます）。

| メソッド | 説明 |
| --- | --- |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`feature_sets`、`labels`、`pos_info`から、末尾の未知語カテゴリのラベルを除いて`lex.csv`形式のエントリを書き出します。 |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`connection_matrix`、`max_left_id`、`max_right_id`から、密な`matrix.def`形式の連接コスト行列を書き出します。 |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている各未知語カテゴリについて`unk.def`形式のエントリを書き出します。 |
| `write_char_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`char.def`の内容を書き出します。 |
| `write_feature_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`feature.def`の内容を書き出します。 |
| `write_rewrite_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`rewrite.def`の内容を書き出します。 |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`left_id_map`から`left-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`right_id_map`から`right-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |
| `update_metadata_json<W: Write>(&self, base_metadata_path: &Path, writer: &mut W) -> Result<()>` | ベースとなる`metadata.json`を読み込み、学習結果に基づく値（素性の重みの中央値から推定した`default_word_cost`、および素性・ラベル数や文脈IDの範囲・学習メタデータを含む`model_info`セクション）で更新した結果を書き出します。 |

#### `ModelMetadata`と`FeatureSetInfo`

`ModelMetadata`は学習実行に関する要約情報を保持します: `version`、`regularization`（使用した正則化コスト係数）、`iterations`（設定された最大イテレーション数）、`feature_count`、`label_count`です。`SerializableModel::metadata`に格納されます。

`FeatureSetInfo`は、学習後にマージされたCRFモデルから抽出された、ラベルごとの連接情報を保持します: `left_id`、`right_id`、`weight`です。`SerializableModel::feature_sets`には、`labels`と同じ順序で各ラベルに対応する1エントリが格納されます。
