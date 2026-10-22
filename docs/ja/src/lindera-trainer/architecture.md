# アーキテクチャ

## モジュール構成

```text
lindera-trainer/src/
├── lib.rs                # Trainer: CRF学習の実行制御。パブリックAPIの再エクスポート
├── config.rs             # TrainerConfig: 種辞書・char.def・unk.def・feature.def・rewrite.defのパース
├── corpus.rs              # Corpus, Example, Word: 学習データの表現
├── feature_extractor.rs  # FeatureExtractor: 素性テンプレート解析と素性ID管理
├── feature_rewriter.rs   # DictionaryRewriter: MeCab互換の3セクション書き換え
└── model.rs               # Model, SerializableModel: 学習済みモデルの保持・シリアライズ・辞書出力
```

## 主要コンポーネント

### TrainerConfig

学習に必要な5つの入力ファイル――種辞書（`lex.csv`）、文字定義（`char.def`）、未知語定義（`unk.def`）、素性テンプレート（`feature.def`）、書き換えルール（`rewrite.def`）――をパースし、`Trainer`が利用する設定情報にまとめます。`TrainerConfig::from_readers`（またはファイルパスを直接渡せるラッパーの`from_paths`）は、種辞書から表層形と素性の語彙を抽出し、パースした素性テンプレートから`FeatureExtractor`を構築し、書き換えルールから`DictionaryRewriter`を構築し、さらに種辞書・文字定義・未知語定義をもとにインメモリの`lindera_dictionary::dictionary::Dictionary`を組み立てます。`char.def`は辞書ビルダー（`CharacterDefinitionBuilder::build_from_str`）でパースするため、学習時の文字カテゴリはビルドされる辞書と同じになります。`%t`素性に使う各文字の既定カテゴリは、ビルダーの`build_default_category_table`から得ます。この`Dictionary`（`dict()`）は学習用ラティスの材料を持ちます。`prefix_dictionary`（`system_lexicon()`）は種辞書で、種辞書の表層形を収めたprefix dictionaryの単語IDは行番号です。`unknown_dictionary`（`unk_handler()`）は、辞書ビルダーの`parse_unk`で読んだ`unk.def`のエントリを、ファイル順に1行1エントリで持ちます。これらのコストと文脈IDは学習では使わず、`Dictionary`の連接行列はプレースホルダーです。`TrainerConfig`は、種辞書の素性文字列と、`unk.def`のエントリを`(カテゴリ, 素性文字列)`の組としてファイル順にも保持し、ラベルとエクスポートする`unk.def`はこれらから作ります。`unk.def`では、`#`で始まる行と空行を読み飛ばし、フィールドが5つ未満の行と、文脈IDやコストが数値でない行は、その行を示すエラーになります。`from_readers`は、`mecab-dict-index`や`lindera build`と同じく、それより前の行で定義していないカテゴリを指す`char.def`の範囲行、2回定義したカテゴリ、`char.def`にないカテゴリを指す`unk.def`の行、`unk.def`の行がない`char.def`のカテゴリも拒否します（`lindera_dictionary::dictionary::unknown_dictionary::check_unknown_word_categories`）。そのため、こうした入力は、エクスポートしたファイルのビルドではなく、学習の前に失敗します。また、`surfaces()`、`surface_features()`、`get_features()`に加え、`user_lexicon()` / `add_user_lexicon_entry()` / `load_user_lexicon_from_content()`によるユーザー辞書へのアクセス、`metadata()`も提供します。

### Corpus / Example / Word

アノテーション付き学習データを表現します。`Word`は表層形とその素性文字列（カンマ区切り）のペアです。`Example`は1つの学習用の文であり、`Vec<Word>`から構築され、各語の表層形を連結して元の文を復元します。`Corpus`は`Example`の集合です。`Corpus::from_reader`はタブ区切りの`surface<TAB>features`形式の行をパースし、空行または`EOS`のみの行を文の区切りとして扱います。

### FeatureExtractor

MeCab互換の素性テンプレートを解析し、生成された素性文字列から内部で採番される`NonZeroU32`型の素性IDへのマッピングを管理します。サポートされるテンプレートのプレースホルダは、`%F[n]` / `%F?[n]`（インデックス`n`の素性フィールド。`?`付きは値が`*`の場合にスキップ）、`%t`（文字カテゴリ）、`%w`（表層形、ユニグラムのみ）、`%u` / `%l` / `%r`（書き換え後のユニグラム／左文脈／右文脈の素性文字列全体）、およびバイグラムの左右文脈フィールド用の`%L[n]` / `%L?[n]` / `%R[n]` / `%R?[n]`です。`%F[n]`・`%L[n]`・`%R[n]`が最後の列より後ろを読むテンプレートは、`?`の有無にかかわらず、MeCabの学習器と同じく素性を作りません。`extract_unigram_feature_ids[_with_ctx]`、`extract_left_feature_ids[_with_ctx]`、`extract_right_feature_ids[_with_ctx]`は、パース済みテンプレートを素性の配列（バイグラム抽出の場合は表層形／ufeature／lfeature／rfeatureを保持するオプションの`TemplateContext`も併せて）に適用し、対応する素性IDを返します。初回利用時には新しいIDが割り当てられます。

### DictionaryRewriter

MeCabの3セクション形式`rewrite.def`（`[unigram rewrite]`、`[left rewrite]`、`[right rewrite]`）を実装します。各セクションは`pattern<TAB>replacement`形式のルールの並びを持ち、内部の`FeatureRewriterBuilder`によって`FeatureRewriter`というprefix trie（前方一致木）に構築されます。`DictionaryRewriter::from_reader`は3セクションすべてをパースしますが、セクションヘッダのないファイルは後方互換性のため（右文脈書き換えのみのレガシー形式として）扱います。`rewrite()`は素性文字列に3つの書き換え器すべてを適用し、`(ufeature, lfeature, rfeature)`を返します。マッチするルールがないセクションは入力をそのまま通過させます。MeCabと同じく、素性と各ルールのパターン・置換はMeCabの`tokenizeCSV`の規則（クレート内部の`feature_csv::split_feature`）で列に分けるので、`,`を含む引用付きの列は1つの列です。`rewrite()`は書き換えた列を`,`でつなぎ、`,`か`"`を含む列は引用符で囲みます。学習器は、素性テンプレートのために3つの結果を同じく列に分けます。（かつての`rewrite_cached()`は素性文字列全体をキーにメモ化していましたが、実辞書の素性は行ごとにほぼ一意でキャッシュが一度もヒットしないため削除されました — #975。）

### Trainer

`Trainer::new`はCRFのラベルを作ります。まず種辞書の行ごとに行順で1つ（ラベル`行番号 + 1`。単語IDが行番号である学習用辞書での一致を、そのままラベルに対応付けるため）、続いて`unk.def`のエントリごとにファイル順で1つ作り、その`%t`はエントリのカテゴリのIDです。`Trainer::train`は、MeCabの学習器と同じく、例文ごとに`lindera_crf::Lattice`を1つ作ります。

- 各位置の候補は、その位置から始まる種辞書の語と、ランタイムがその位置に作る未知語です。未知語は`lindera_dictionary::viterbi::Lattice::unknown_word_candidates`が列挙します。文字の既定カテゴリの未知語を、その`INVOKE`・`GROUP`・`LENGTH`の設定で、カテゴリの`unk.def`のエントリそれぞれについて作ります。
- 正解の各トークンには、範囲と素性文字列全体が同じ最初の種辞書の行のラベルを付けます。なければ、範囲が同じ未知語候補の`unk.def`エントリのうち、`*`以外の素性の列がすべて正解トークンの列と一致するファイル順で最初のエントリのラベルを付けます（どちらの列も`rewrite()`と同じく分けます）。それもなければ、MeCabの学習器が仮想ノードを追加するのと同じく、正解の素性文字列から作る仮想ラベル（`%t`は0、`%w`なし、素性文字列ごとに1つ）を付けます。仮想ラベルはほかのラベルの後に並び、学習されますが、エクスポートされることはありません。
- それ以外の候補はすべて負例の辺になります。

ラティスを作り終えると、`Lattice statistics:`の行を出力します。各方法でラベルを付けた正解トークンの数（`gold_lexicon`・`gold_unknown`・`gold_virtual`）、仮想ラベルの数（`virtual_labels`）、種辞書と未知語候補それぞれから作った負例の辺の数（`negative_lexicon`・`negative_unknown`）です。`Trainer::max_grouping_len`は、ランタイムと同じ意味（先頭の1文字を除いた文字数。デフォルトの`None`は無制限）で未知語のグルーピングの長さに上限を設けます。ほかのビルダーメソッドは、正則化・イテレーション数の上限・収束判定の許容値（`Trainer::eta`。デフォルトは`DEFAULT_ETA`の`5e-5`。[lindera-crfのアーキテクチャ](../lindera-crf/architecture.md#trainer)を参照）・スレッド数を設定します。学習の後に、最適化が止まった理由を、イテレーション数と損失・勾配の評価回数とともに出力します。あるイテレーションで目的関数が変わらなかったときと、ライン探索が失敗したときは、警告も出力します。

### Model / SerializableModel

`Model`は`Trainer::train`が生成する学習済みモデルです。学習された`lindera_crf::RawModel`、学習に用いた`TrainerConfig`、抽出済みの素性の重みとラベル、仮想ラベルの素性文字列、そして学習後に`read_user_lexicon`で追加されたユーザー辞書エントリを保持します。学習されたCRFの重みは、MeCab互換の整数コストに変換されます（`tocost(weight, cost_factor) = clamp(round(-weight * cost_factor), i16::MIN, i16::MAX)`、すなわち`[-32768, 32767]`の範囲にクランプされます）。この際のコストファクターは、`i16`の値域を最大限活用できるよう計算されます（`calculate_cost_factor`）。`Model`は自身をシリアライズでき（`write_model`）、辞書ソースファイルを直接エクスポートすることもできます。

`SerializableModel`は、`Model::read_model`がリーダーから以前学習したモデルを読み込んだ際に返される、`rkyv`でシリアライズ可能な素のデータ型です（`lindera export` CLIコマンドで使用されます）。`Model`と同じ学習結果の情報――素性の重み、ラベル、品詞情報、連接コスト行列、`unk.def`のエントリ（`unk_entries`。ファイル順の`(カテゴリ, 素性文字列)`の組）、保存されている`char.def` / `feature.def` / `rewrite.def`の内容、コストファクター、左右文脈IDマッピング――を`TrainerConfig`を持たない所有データとして保持し、辞書エクスポートファイルを生成するための独自のライターメソッド群を提供します。

#### `Model`のパブリックメソッド

| メソッド | 説明 |
| --- | --- |
| `read_user_lexicon<R: Read>(&mut self, rdr: R) -> Result<()>` | ユーザー定義辞書（種辞書と同じ`surface,left_id,right_id,cost,feature...`のCSV形式）をモデルに読み込みます。これにより、エントリが与えていないパラメータを`write_dictionary`が埋められるようになります。MeCabの`mecab-dict-index`と同じく、コストと連接IDは別々に埋めます。空のコストは計算し、連接IDのどちらかが空か負の数なら、両方を推定します。パラメータが`0,0,0`のエントリは、Vibratoの学習器と同じく3つとも埋めます（`mecab-dict-index`はそのまま使います）。空でも範囲内の数でもない連接IDやコストは、そのエントリを示すエラーになり、そのときはエントリを1つも追加しません。`mecab-dict-index`はそうした値を`atoi`で数として読みますが、`lindera build`は拒否します。ユーザー辞書が必要な場合は、辞書を書き出す前に呼び出す必要があります。 |
| `write_model<W: Write>(&self, writer: &mut W) -> Result<()>` | 学習済みモデル全体（素性の重み、ラベル、品詞情報、連接行列、`unk.def`のエントリ、保存された定義ファイルの内容、コストファクター、左右IDマッピング）を`rkyv`バイナリ形式でシリアライズします。`metadata.version`は`model::MODEL_FORMAT_VERSION`（`2.0.0`）です。保存する行列の大きさは、重みが残ったかどうかにかかわらず、モデルのすべての文脈IDを覆います。出力はバイト単位で再現可能です。連接行列は順序付きマップ、`unk.def`のエントリは`Vec`であるため、バイト列はモデルの内容だけで決まります。 |
| `read_model<R: Read>(reader: R) -> Result<SerializableModel>` | 関連関数。`write_model`が書き出したデータから`SerializableModel`をデシリアライズします。まず`rkyv`形式を試み、ペイロードが実際にJSONオブジェクトで始まる場合にのみレガシーのJSON形式へフォールバックします。いずれでもない場合は、形式の不一致として再学習を促すエラーを返します。どちらの形式でも、`metadata.version`が`MODEL_FORMAT_VERSION`でないモデルは同じエラーで拒否するため、7.0.0より前に書かれたモデルはすべて再学習が必要です。`feature_sets`フィールドが存在しない旧形式のモデルについては、`feature_weights`から補完します。 |
| `write_dictionary<W1, W2, W3, W4>(&self, lexicon_wtr, connector_wtr, unk_handler_wtr, user_lexicon_wtr) -> Result<()>` | レキシコン、連接コスト行列、未知語辞書、ユーザー辞書を一度にまとめて書き出す便利メソッドです。ユーザー辞書は`read_user_lexicon`で読み込んだエントリを入力順で書き出します。連接IDのないエントリには、素性がそのエントリに最も近い種辞書の行に`lex.csv`が付ける連接IDを割り当てます。コストのないエントリには、MeCabの`calcCost`と同じく、そのエントリのunigram素性の学習済みの重みの和を、`lex.csv`のコストファクターで`tocost`により変換したコストを割り当てます。素性は種辞書の行と同じく、書き換えた素性、先頭の文字の既定カテゴリ（`%t`）、表層形（`%w`。MeCabはunigramテンプレートの`%w`を受け付けません）から作ります。そのため、素性と先頭の文字のカテゴリが種辞書のある行と同じエントリは、`lex.csv`がその行に付けるコストを受け取ります。モデルに重みのない素性は何も足しません。エントリが与えたパラメータはそのまま書き出します。最も近い行は、エントリと同じ位置で一致する列ごとに、前の列ほど高い点を足した合計が最も高い行で、同点なら最初の行です。どの種辞書の行とも共通の列を持たない素性の、連接IDのないエントリは、MeCabの`mecab-dict-index`と同じくエラーになり、そのときは何も書き出しません。 |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | マージされたCRFモデルから得た連接IDとコストを用いて、種辞書の各語彙エントリについて`lex.csv`形式のエントリ（`surface,left_id,right_id,cost,features...`）を書き出します。 |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | マージしたモデルの文脈IDのすべての`(right_id, left_id)`の組み合わせ（BOS/EOS用の0を含む）を網羅する密な`matrix.def`形式の連接コスト行列を書き出します。各組み合わせのコストは、学習した重みを`tocost`で変換したものです。学習した重みがない組み合わせ（バイグラム素性が学習中に隣り合わなかったか、正則化で重みが0になったもの）は0になります。これは学習中のその遷移のコストであり、MeCabの`mecab-dict-gen`が書き出す値でもあります。`write_model`が書き出すモデルについて`SerializableModel::write_connection_costs`が書き出す行列と同じです。 |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | 学習に使った`unk.def`のエントリごとに、ファイル順で`unk.def`の行を1行ずつ書き出します。各行は、エントリのカテゴリ名と素性文字列、学習された連接IDとコストを持ちます。 |
| `get_unknown_word_cost(&self, category: usize) -> i32` | `char.def`のカテゴリのインデックスに対する入力のコストを返します。学習した値ではなく`unk.def`のコスト列の値で、エントリが複数あるカテゴリでは最後の行のコストです。設定がない場合はデフォルト値`2000`を返します。 |
| `num_features(&self) -> usize` | 学習済みモデルが保持する素性の重みの数を返します。 |
| `num_labels(&self) -> usize` | 学習済みモデルのエクスポートされるラベル（種辞書の行と`unk.def`のエントリの合計）の数を返します。仮想ラベルは数えません。 |
| `raw_model(&self) -> &lindera_crf::RawModel` | 高度な操作や低レベルの処理のために、内部の`lindera-crf`のraw modelへアクセスします。 |
| `write_bigram_details<L: Write, R: Write, C: Write>(&self, left_wtr, right_wtr, cost_wtr) -> Result<()>` | バイグラム素性とコストを記述した3つの診断用ファイル（左側素性一覧、右側素性一覧、生のバイグラム素性ペアごとのコスト行。id 0 は`BOS`/`EOS`、名前は素性抽出器由来）を書き出します。3ファイルとも同じモデルに対してバイト単位で再現可能です。 |
| `evaluate(&self, test_lattices: &[lindera_crf::Lattice]) -> f64` | raw modelの素性の重みの絶対値の平均を簡易的な評価スコアとして返します。引数`test_lattices`は現時点では未使用であり、保留データに対するモデルの評価はまだ行われません。 |
| `write_dictionary_buffers(&self, lexicon, connector, unk_handler, user_lexicon: &mut Vec<u8>) -> Result<()>` | ラベル、素性の重み、ユーザーエントリ数、表層形を、それぞれ生の`rkyv`バイトバッファへシリアライズします。CSVベースの`write_dictionary`に対する、より低レベルな代替エクスポート手段です。 |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | `left-id.def`を書き出します。先頭行は`0 BOS/EOS`で、続いてマージ済みモデルの左文脈ID（`write_dictionary`が`lex.csv`・`unk.def`・`matrix.def`に書き出すID）ごとに、そのIDを持つ最初のラベルの左素性を書き出します。`write_model`が書き出すモデルについて`SerializableModel::write_left_id_def`が書き出すファイルと同じです（両者は1つの対応づけを使います）。 |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | `right-id.def`を書き出します。`write_left_id_def`と同じく、先頭行の`0 BOS/EOS`に続いて、マージ済みモデルの右文脈IDごとに、そのIDを持つ最初のラベルの右素性を書き出します。`write_model`が書き出すモデルについて`SerializableModel::write_right_id_def`が書き出すファイルと同じです。 |

#### `SerializableModel`のパブリックメソッド

`Model::read_model`が返す値に対して利用できるメソッド群で、以前に学習・シリアライズされたモデルから辞書ソースファイルを再エクスポートするために使用します（`lindera export` CLIコマンドと、Python・Node.js・Ruby・PHPバインディングのエクスポート関数で利用されます）。

| メソッド | 説明 |
| --- | --- |
| `export(&self, output_dir: &Path, base_metadata: Option<&Path>) -> Result<Vec<PathBuf>>` | 以下のメソッドで、辞書ソースファイルを`output_dir`に書き出します（`output_dir`がなければ作成します）: `lex.csv`、`matrix.def`、`unk.def`、`char.def`、`feature.def`、`rewrite.def`、`left-id.def`、`right-id.def`と、`base_metadata`を指定したときは`metadata.json`（`update_metadata_json`）です。書き出したファイルのパスをこの順序で返します。`lindera export`とバインディングのエクスポート関数はこのメソッドを呼ぶため、同じファイルを書き出します。`metadata.json`を最初に作るため、ベースの`metadata.json`を読めないときはファイルを1つも書き出さずに失敗します。エラーはファイルを示します。 |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`feature_sets`、`labels`、`pos_info`から、種辞書の行の後に続く`unk.def`のエントリのラベルを除いて`lex.csv`形式のエントリを書き出します。 |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`connection_matrix`、`max_left_id`、`max_right_id`から、密な`matrix.def`形式の連接コスト行列を書き出します。大きさは`max_right_id + 1`行×`max_left_id + 1`列で、学習でバイグラムの重みが1つも残らなかった場合もモデルのすべての文脈IDを覆います。`connection_matrix`にないセル（学習した重みがない連接）は、`Model::write_connection_costs`と同じく0になります。 |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`unk_entries`のエントリごとに、ファイル順で`unk.def`の行を1行ずつ書き出します。各行は、カテゴリ名、学習された連接IDとコスト、素性文字列を持ちます。 |
| `write_char_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`char.def`の内容を書き出します。 |
| `write_feature_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`feature.def`の内容を書き出します。 |
| `write_rewrite_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 元の学習入力からそのまま保存されている`rewrite.def`の内容を書き出します。 |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`left_id_map`から`left-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | 保存されている`right_id_map`から`right-id.def`を書き出します。先頭行は`0 BOS/EOS`です。 |
| `update_metadata_json<W: Write>(&self, base_metadata_path: &Path, writer: &mut W) -> Result<()>` | ベースとなる`metadata.json`を読み込み、学習結果に基づく値（素性の重みの中央値から推定した`default_word_cost`、および素性・ラベル数や文脈IDの範囲・`connection_matrix_size`としての`matrix.def`の大きさ（`{行数}x{列数}`）・学習メタデータを含む`model_info`セクション）で更新した結果を書き出します。タイムスタンプは含まれず、同じモデルに対してバイト単位で再現可能です。 |

#### `ModelMetadata`と`FeatureSetInfo`

`ModelMetadata`は学習実行に関する要約情報を保持します: `version`（モデルの形式バージョン`MODEL_FORMAT_VERSION`）、`regularization`（使用した正則化コスト係数）、`iterations`（実際に回ったL-BFGSのイテレーション数。`Trainer::max_iter`で設定した最大値以下。`model_info.training_iterations`としてエクスポートされます）、`feature_count`、`label_count`です。`SerializableModel::metadata`に格納されます。

`FeatureSetInfo`は、学習後にマージされたCRFモデルから抽出された、ラベルごとの連接情報を保持します: `left_id`、`right_id`、`weight`です。`SerializableModel::feature_sets`には、CRFのラベルごとに1エントリが格納されます。`labels`の各要素に対応するエントリが同じ順序で並び、その後にエクスポートされない仮想ラベルのエントリが続きます。
