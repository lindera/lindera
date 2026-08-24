# v5 から v6 への移行

Lindera v6.0.0 では、ビルド済みユーザー辞書の再ビルドが必要になり、
トークナイズ結果が 2 点（Decompose モードの精度修正と、新しいデフォルト
有効な未知語機能）で変わり、JavaScript バインディングのトークン型が変わり、
`lindera-dictionary` の一部 Rust API が改名されます。ほとんどのユーザーは
辞書の再ビルドだけで済みますが、`lindera_dictionary::viterbi`/`mode` の型を
直接使っている場合、JavaScript を使っている場合、未知語（辞書外の文字列）に
対する厳密な旧出力に依存している場合は、追加で確認すべき点があります。

**v5.2 以前**からアップグレードする場合は、後述のシステム辞書フォーマット
変更も対象になります。この変更は v5.3.0 で既にリリース済みのため、v5.3.0
からのアップグレードではシステム辞書の再ビルドは不要です。ただしユーザー
辞書の再ビルドは、これとは別の理由で必要です。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| **ビルド済みユーザー辞書 `.bin` が読み込めなくなった** | `.bin` からユーザー辞書を読み込むすべてのユーザー | CSV から `lindera build --user` で再ビルド |
| **学習済み `model.dat` が読み込めなくなった** | 以前のビルドで生成した `model.dat` を再利用しているユーザー | `lindera train` を再実行 |
| Decompose モードの長さペナルティが非 3 バイト文字に対して正確になった | 1・2・4 バイトの UTF-8 文字を含むテキストに対する `Mode::Decompose` の出力 | 対応不要（正確性修正）。出力を厳密に固定している場合は Decompose 出力を再確認 |
| 未知語の候補ラダー（length ladder）がデフォルトで有効に | 辞書外テキストに対する Normal・Decompose 両モードの出力 | v5 と同一の出力が必要な場合は `unknown_word_ladder(false)` / `--disable-unknown-word-ladder` を設定 |
| `max_grouping_len` オプションの新設（`--max-grouping-len`、config キー、builder/worker setter） | MeCab の `max-grouping-size` 相当の上限を使いたいユーザー | 対応不要（デフォルトは従来どおり無制限） |
| JS バインディングがプレーンオブジェクトを返すようになり `Token` クラスが廃止 | Node.js・WASM ユーザー | `token.getDetail(i)` を `token.details[i]` に置換。WASM ユーザーはフィールド名の camelCase 化にも対応 |
| **npm・PyPI のパッケージ名が変更**（`lindera-nodejs` → `lindera`、`lindera-python` → `lindera`、WASM は `lindera-wasm` に統合） | npm・PyPI からバインディングをインストールしているユーザー | 依存関係と import のパッケージ名を更新（[下記](#npmpypicratesio-のパッケージ名変更)参照） |
| `lindera_dictionary::viterbi`/`mode` の一部項目が改名・削除、`Lattice::set_text`/`set_text_nbest` に引数 2 つ追加 | `Lattice`/`Edge`/`Penalty`/`Mode` を直接利用するコード（`lindera` クレートの `Segmenter`/`Tokenizer`/`Worker` API 利用者は対象外） | 下記の表に従い呼び出し箇所を更新 |
| ビルド済み辞書フォーマットがバージョン 2（`dict.da` → `dict.trie` + `dict.valsidx`）— **v5.3.0 でリリース済み** | **v5.2 以前**で作成した自前ビルドのシステム辞書 | `lindera build` で再ビルド、または `lindera download` で再取得 |
| `loadDictionaryFromBytes()` が 9 引数 — **v5.3.0 でリリース済み** | **v5.2 以前**からアップグレードする、バイトデータから辞書を読み込む WASM ユーザー | `dictDa` の代わりに `dictTrie` と `dictValsIdx` を渡す |
| OPFS の `DictionaryFiles` が `dictDa` を `dictTrie` + `dictValsIdx` に置き換え — **v5.3.0 でリリース済み** | **v5.2 以前**からアップグレードする、`opfs` ヘルパーを使う WASM ユーザー | OPFS にキャッシュ済みの辞書を再ダウンロード |

## npm・PyPI・crates.io のパッケージ名変更

v6.0.0 から、公開パッケージ名がサフィックス付きの名前から素の名前
（bare name）に統一されます:

| レジストリ | 旧パッケージ名 | 新パッケージ名 |
| --- | --- | --- |
| npm（Node.js 本体） | `lindera-nodejs` | `lindera` |
| npm（プラットフォーム別） | `lindera-nodejs-<platform>`（例: `lindera-nodejs-darwin-arm64`） | `lindera-<platform>`（例: `lindera-darwin-arm64`） |
| npm（WASM） | `lindera-wasm-web` / `lindera-wasm-bundler` | `lindera-wasm`（単一パッケージ） |
| PyPI | `lindera-python` | `lindera` |

RubyGems の `lindera` と、crates.io のコアクレート（`lindera`・
`lindera-dictionary` など）は変更ありません。

### Node.js（npm）

`package.json` の依存関係と `require`/`import` のパッケージ名を更新してください:

```javascript
// v5
const { TokenizerBuilder } = require("lindera-nodejs");

// v6
const { TokenizerBuilder } = require("lindera");
```

プラットフォーム別パッケージ（`optionalDependencies` 経由で自動的に
解決されます）も `lindera-nodejs-<platform>` から `lindera-<platform>` に
変わりますが、通常は明示的に依存指定していないため対応は不要です。

旧 `lindera-nodejs` 系パッケージは npm 上で deprecated としてマークされます。
公開済みの旧バージョンはそのまま残ります。

### Python（PyPI）

インストールコマンドと依存関係を更新してください:

```bash
# v5
pip install lindera-python

# v6
pip install lindera
```

**Python の import 名は変わりません** — 以前から `import lindera` であり、
コードの変更は不要です。変わるのは PyPI 上の配布名だけです。

移行を助けるため、`lindera-python` の最終リリースとして、新しい `lindera`
パッケージに依存するだけの移行スタブ（transition stub）が PyPI に公開されます。
`pip install lindera-python` は当面動作しますが、依存関係は `lindera` に
切り替えてください。

### WASM（npm）

`lindera-wasm-web` と `lindera-wasm-bundler` の 2 パッケージは、
`wasm-pack --target web` でビルドされた単一の `lindera-wasm` パッケージに
統合されます。

`lindera-wasm-web` を使っていた場合は、パッケージ名の置き換えのみです:

```javascript
// v5
import __wbg_init, { TokenizerBuilder } from "lindera-wasm-web";
await __wbg_init();

// v6
import init, { TokenizerBuilder } from "lindera-wasm";
await init();
```

`lindera-wasm-bundler` を使っていた場合も、同じ `lindera-wasm` パッケージを
インストールします。ただし `web` ターゲットのビルドでは初期化が自動では
行われないため、使用前にデフォルトエクスポートの非同期初期化関数を必ず
呼び出してください:

```javascript
// v5（bundler ターゲット: 初期化はバンドラーが処理）
import { TokenizerBuilder } from "lindera-wasm-bundler";

// v6（web ターゲット: 明示的な初期化が必要）
import init, { TokenizerBuilder } from "lindera-wasm";
await init();
```

モダンなバンドラー（Vite、Webpack 5 の `asyncWebAssembly` など）は
`web` ターゲットのビルドをそのまま扱えます。設定例は
[ブラウザでの使用](./lindera-wasm/browser_usage.md) を参照してください。

### crates.io

バインディングクレート（`lindera-python`・`lindera-nodejs`・`lindera-ruby`・
`lindera-wasm`）は crates.io への公開を終了します。公開済みの 5.3.0 までの
バージョンはそのまま残ります。これらは各言語のレジストリ（PyPI・npm・
RubyGems）経由で利用するものであり、Rust クレートとして依存する用途は
想定されていません。コアクレートは引き続き crates.io に公開されます。

## ビルド済みユーザー辞書の再ビルドが必要

Lindera v6 では `daachorse` を 4.x から 5.0 に更新しており、ユーザー辞書が
内部に持つ Aho-Corasick オートマトンのシリアライズ形式が変わりました。
そのため、v5 系でビルドした `.bin` ファイルはロードに失敗します:

```text
LinderaError(kind=Deserialize, source=InvalidAutomatonError: invalid serialized automaton)
```

システム辞書と異なり、ユーザー辞書の `.bin` には `format_version` が無いため
バージョン検査ができず、より親切なメッセージを出せません。上記の
デシリアライズエラーとして表面化します。

CSV から v6 で再ビルドしてください:

```sh
lindera build --user \
  --src ./user_dict.csv \
  --dest ./build \
  --metadata ./lindera-ipadic/metadata.json
```

`.bin` ではなく `.csv` からユーザー辞書を読み込んでいる場合は、ロード時に
コンパイルされるため対応は不要です。

## 学習済みモデルファイルの再生成が必要

`lindera train` が書き出す `model.dat` がバイト単位で再現可能になりました。
同じ入力・同じフラグで 2 回学習するとバイト列が一致するため、
チェックサムの取得・キャッシュ・差分比較ができます。

これを実現するため、連接行列と未知語カテゴリをハッシュマップではなく
順序付きマップとして保持するようにしました。rkyv は archived `HashMap` を
ソース走査順にレイアウトし、その順序はインスタンスごとにシードされるためです。
ディスク上のレイアウトが変わるので、以前のビルドが書き出した `model.dat` は
読み込みに失敗します。

```text
failed to deserialize model: ... re-run `lindera train` to regenerate it.
```

`model.dat` は `lindera train` と `lindera export` の間の中間ファイルであり、
配布物ではありません。`lindera train` を再実行して生成し直してください。
旧モデルからすでにエクスポート済みの辞書には影響しません。また、
学習された重みはどちらでも同一です。変わったのは直列化の順序だけです。

関連する注意点が 2 つあります。

- `SerializableModel::connection_matrix` と `SerializableModel::unk_categories` の型が
  `HashMap` から `BTreeMap` に変わりました。これらの公開フィールドを直接読んでいる
  場合にのみ影響します。ライターメソッド群は変更ありません。
- 再現性は `--max-threads` の値によらず成り立ちます。勾配と損失は学習データの
  固定分割を固定順で加算するため、スレッド数は所要時間だけを変え、
  学習結果は変えません。
- `lindera export` の出力もバイト単位で再現可能になりました。`metadata.json` は
  `updated_at` タイムスタンプを持たなくなり、各エクスポートライターは決定的な
  順序でエントリを書き出します。これに伴い Rust API から
  `ModelInfo.updated_at` フィールドを削除しました（`lindera-dictionary`）。
  このキーを含む既存の `metadata.json` は引き続き読み込めます。
- `DictionaryRewriter::rewrite_cached` と `clear_cache` を削除しました。
  キャッシュは素性文字列全体をキーにしており、実辞書では行ごとにほぼ一意で
  一度もヒットせず、メモリを保持するだけだったためです。代わりに `rewrite`
  を呼んでください（`&mut` も不要になりました）。
- `FeatureExtractor::extract_*` 6 メソッドの `features` 引数を
  `&[String]` から `&[&str]` に変更しました。呼び出し側がフィールドごとに
  `String` を確保する必要がなくなります。`&[String]` を持っている場合は
  `&v.iter().map(|s| s.as_str()).collect::<Vec<_>>()` で渡せます。
- `lindera-wasm` からファイルシステム依存のエクスポート
  `loadUserDictionary` / `buildDictionary` / `buildUserDictionary` /
  `TokenizerBuilder.setUserDictionary(uri)`（と snake_case エイリアス）を
  削除しました。`wasm32-unknown-unknown` にはファイルシステムが無く、
  これらは常に実行時に失敗していました。システム辞書は
  `loadDictionaryFromBytes()`（または `embedded://`）で、ユーザー辞書は
  新設の `loadUserDictionaryFromBytes()` /
  `loadUserDictionaryBinFromBytes()` と `setUserDictionaryInstance()` で
  読み込んでください。`loadDictionary()` はファイル URI・パスを
  bytes API を案内するエラーで即座に拒否するようになりました。
- 固定分割の導入で加算順が変わったため、この変更**前**に学習した重みと
  変更後に学習した重みはバイト単位では一致しません（`--max-threads 1` を
  含むすべてのスレッド数で）。以後の成果物を比較可能にするには
  `lindera train` を再実行してください。

## 辞書フォーマットバージョン 2（v5.3.0 でリリース済み）

> この変更は v6.0.0 ではなく **v5.3.0** でリリース済みです。v5.3.0 から
> アップグレードする場合、システム辞書は既にフォーマットバージョン 2 で
> あり、そのままロードできます。この節は **v5.2 以前**からアップグレード
> する場合にのみ従ってください。

システム前方一致辞書は、シリアライズされた `daachorse` Aho-Corasick オートマトン
（`dict.da`）として保存されなくなりました。ビルダはビルド時に `crawdad` で文字単位の
ダブル配列トライを構築して `dict.trie` として書き出し、単語値への `u32` 累積和
インデックス（`dict.valsidx`）を併せて出力します。実行時にはトライをシリアライズ済み
バイト列上で直接走査するため、ロードはデシリアライズではなく O(1) のヘッダ検査で済み、
`mmap` 下では他の大きなコンポーネントと同様に遅延ページングされたままになります。

ビルド済み辞書ディレクトリは以下の 9 ファイルで構成されます:

| ファイル | 説明 |
| --- | --- |
| `metadata.json` | 辞書メタデータ（`format_version` を含むようになった） |
| `dict.trie` | 文字単位のダブル配列トライ構造（新規） |
| `dict.valsidx` | 単語値インデックス（新規） |
| `dict.vals` | 単語値データ |
| `dict.wordsidx` | 単語詳細インデックス |
| `dict.words` | 単語詳細（形態素素性） |
| `matrix.mtx` | 連接コスト行列 |
| `char_def.bin` | 文字カテゴリ定義 |
| `unk.bin` | 未知語辞書 |

`dict.trie` と `dict.valsidx` 以外のファイルは v5 から変更ありません。`dict.da` は
存在しなくなりました。

### 古い辞書のロードは明確なエラーで失敗する

ビルド済み辞書の `metadata.json` は `format_version: 2` を記録し、ローダーがこれを検証
します。v5.2 以前でビルドした辞書のロードは、ヘッダを持たないバイナリファイルを
誤読する代わりに、対処方法を含むエラーで失敗します:

```text
Dictionary 'ipadic' has format version 1, but this build of Lindera reads format version 2. To fix this, rebuild it with `lindera build`, or download a matching prebuilt dictionary with `lindera download`.
```

## トークナイズ結果の変更

上記のフォーマット変更とは異なり、以下の 2 点は同じ入力・同じ辞書でも
**Lindera が出力するトークン自体**を変える可能性があります。

### Decompose モードのペナルティ精度修正

`Mode::Decompose` の長さペナルティは、スパンの文字数を
`(終了バイト位置 - 開始バイト位置) / 3` で近似していました。これは全体が
3 バイト UTF-8 文字（一般的な漢字・かな）で構成されたテキストに対してのみ
正確です。Lindera の Viterbi ラティスは内部的に文字単位で索引されるように
なったため、ペナルティは実際の文字数を使うようになりました。この変更が
影響するのは 1 バイト（ASCII）・2 バイト・4 バイト（絵文字や一部の CJK
拡張文字など）の UTF-8 文字を含むスパンの Decompose 出力のみで、純粋な
日本語の Decompose 出力には影響しません。これは挙動選択ではなくバグ修正の
ため、旧来の（不正確な）挙動に戻すオプションはありません。

### 未知語の候補ラダー（新規・デフォルト有効）

辞書に無い文字が連続する箇所に対して、MeCab や Vibrato は候補となる未知語の
長さを `1..=LENGTH`（`LENGTH` は辞書の `char.def` にあるカテゴリごとの
フィールド）まで段階的に（梯子＝ladder のように）生成し、Viterbi 探索に
コスト最小の長さを選ばせます。Lindera は `char.def` から `LENGTH` を
パースして保存はしていましたが、実行時には一切参照しておらず、常に
1 文字の候補のみ（グルーピング対象カテゴリの場合はラン全体を覆う 1 候補のみ）
しか生成していませんでした。

v6.0.0 からは、このラダーがデフォルトで生成されるようになります。IPADIC で
影響を受けるのは `KANJI`・`HIRAGANA`・`KATAKANA`（`LENGTH=2`）カテゴリで、
それ以外の大半のカテゴリは `LENGTH=0` のため影響を受けません。実際の効果と
しては、辞書に無い漢字・かなが連続する箇所で、1 文字ずつに分割するより
複数文字でまとめたほうがコストが低い場合に、複数文字の未知語としてまとめて
トークナイズされるようになります — 例えば、未知の 2 字熟語が従来は 2 つの
未知語トークンに分かれていたのが、1 つの未知語トークンになります。

これは後述の新オプション `max_grouping_len`（ラン全体のグルーピング上限）
とは独立した、加算的な機能です。

辞書外の漢字・かな連続を含むテキストで v5 と同一の出力を再現するには、
ラダーを無効化してください:

```rust,ignore
let segmenter = Segmenter::new(mode, dictionary, None)
    .unknown_word_ladder(false);
```

```sh
lindera tokenize -d ipadic --disable-unknown-word-ladder input.txt
```

`AnalysisWorker`/`SegmentWorker` は `set_unknown_word_ladder(bool)`、
`TokenizerBuilder` は `set_segmenter_unknown_word_ladder(bool)` で同じ
切り替えができます。

### 新オプション: `max_grouping_len`（デフォルトは従来どおり）

v6 では MeCab の `max-grouping-size` と同じ意味論を追加しました。辞書外
文字のグルーピング可能なランが、先頭を除いて `max_grouping_len` 文字を
超える場合、グループ候補を出さずに 1 文字ずつの未知語を使います。
デフォルトは無制限で、これは v5 と同じ挙動です。つまり
**明示的に設定しない限り何も変わりません**:

```rust,ignore
let segmenter = Segmenter::new(mode, dictionary, None)
    .max_grouping_len(Some(24)); // MeCab 自体のデフォルト値
```

```sh
lindera tokenize -d ipadic --max-grouping-len 24 input.txt
```

`SegmentWorker`/`AnalysisWorker` の `set_max_grouping_len(Option<usize>)`、
`TokenizerBuilder` の `set_segmenter_max_grouping_len(usize)`、および
segmenter 設定の `max_grouping_len` キーでも指定できます。

## `lindera_dictionary` の Rust API 変更

以下は `lindera_dictionary::viterbi::{Lattice, Edge}` や
`lindera_dictionary::mode::{Mode, Penalty}` を直接利用するコード（独自の
ラティス検査や代替トークナイザーフロントエンドなど）にのみ影響します。
`lindera` クレートの `Segmenter`・`Tokenizer`・`SegmentWorker`・
`AnalysisWorker` API は影響を受けません — 上記の
`unknown_word_ladder`/`max_grouping_len` オプションが追加されただけで、
シグネチャの変更はありません。

| v5 | v6 | 理由 |
| --- | --- | --- |
| `Lattice::text_len()` | `Lattice::char_len()` | ラティスがバイト索引から文字索引に変わったため。改名により、バイト前提のコードは黙って誤動作せずコンパイルエラーになる |
| `Lattice::edges_at(pos)` | `Lattice::edges_at_char(pos)` | 同上（`pos` が文字位置になった） |
| `Lattice::paths_at(pos)` | `Lattice::paths_at_char(pos)` | 同上 |
| `Lattice::capacity()` / `shrink_to(n)` | 名前は同じだが単位がバイトから文字（スロット）に | バイト長は依然として `shrink_to` の有効な（安全側に倒れる）引数として使える（文はバイト数より文字数の方が少ないため） |
| `Edge::num_chars()` | 削除 | パック済み `Edge` は終了位置を保持しなくなった（常にラティススロットの添字と一致するため）ので、エッジ単体からは計算できなくなった |
| `Penalty::penalty(&Edge)` | `Penalty::penalty(&Edge, num_chars: usize)` | 呼び出し側が正確な文字数スパンを渡すようになった（前述の Decompose 精度修正を参照） |
| `Mode::penalty_cost(&Edge)` | 削除 | コードベース内に呼び出し元が無かった。必要であれば `Penalty::penalty` で再実装可能 |
| `Lattice::set_text(..., search_mode)` | `Lattice::set_text(..., search_mode, max_grouping_len, unknown_word_ladder)` | 新オプション用に引数 2 つが末尾に追加。v6 のデフォルトに合わせるなら `None` と `true`、v5 の挙動にするなら `None` と `false` を渡す。`set_text_nbest` も同様に変更 |
| — | `Lattice::take_max_char_len()`（新規） | 加算的な追加。処理した最大の文長を返してリセットする（worker の縮小判定用） |

`set_text` と `set_text_nbest` には、文が `u16::MAX` 文字未満であることを
検査する panic 契約も追加されました（エッジが開始位置を `u16` で保持する
ようになったため）。`Segmenter` を経由する経路は `MAX_SENTENCE_BYTES` で
文を分割するためすべて安全です。`Lattice` を直接駆動するコードだけが、
自分で入力を分割する必要があります。

## JavaScript バインディング: トークンがプレーンオブジェクトに

これまで両 JS バインディングは `Token` クラスのインスタンスを返していました。
これらのインスタンスは JavaScript ヒープの外にメモリを保持し、ホストが
ファイナライザを実行して初めて解放されます。napi はそのファイナライザを
イベントループへ遅延させ、wasm-bindgen は解放を呼び出し側に委ねるため、
大きな入力を yield せずに同期ループでトークナイズすると、強制 GC を
挟んでもメモリが蓄積していました。v6 では両バインディングともプレーン
オブジェクトを返すようになり、メモリは JavaScript の GC が完全に管理します。

実用上の利点: バッチループでメモリがフラットに保たれ、結果を
`JSON.stringify`・`structuredClone`・worker への転送にそのまま使えます。

### `getDetail(i)` の廃止（Node.js・WASM 共通）

プレーンオブジェクトにメソッドは無いため、配列を直接参照してください:

```javascript
// v5
const pos = token.getDetail(0);

// v6
const pos = token.details[0];
```

範囲外の読み出しは `null` ではなく `undefined` になります。

### Node.js: `tokenizeObjects` の廃止

`tokenizeObjects` は上記のメモリ問題に対する v5 限定の回避策としてのみ
存在していました。`tokenize` が同じプレーンオブジェクトを返すように
なったため、呼び出しを置き換えてください:

```javascript
// v5
const tokens = tokenizer.tokenizeObjects(text);

// v6
const tokens = tokenizer.tokenize(text);
```

`tokenizeNbest` もプレーンオブジェクトを返すようになったため、v5 で
既知の制約として記載していた蓄積の問題は解消されています。`NbestResult` も
`Token` と同様にクラスではなくなり、同じ `tokens`・`cost` プロパティを
持つプレーンオブジェクトになりました。

どちらもクラスではなくなったため、パッケージから export されなくなりました。
`Token`・`JsToken`・`NbestResult`・`JsNbestResult` はすべて `index.js` から
削除され、`require("lindera").Token` は `undefined` になります。
`instanceof` による判定もできません:

```javascript
// v5
const { Token } = require("lindera-nodejs");
if (tokens[0] instanceof Token) { /* ... */ }

// v6 — プレーンオブジェクトなので、プロパティで判定する
if (typeof tokens[0].surface === "string") { /* ... */ }
```

TypeScript 利用者へ: トークンを表すインターフェース名は `Token` になりました
（`Token` がクラスに使われていた間は `JsTokenData` という名前でした）。

### WASM: フィールド名が camelCase に

廃止された `Token` クラスは snake_case のフィールドを公開する一方、
同じクラスの `toJSON()` は既に camelCase を出力しており不整合がありました。
v6 では Node.js バインディングに揃えて camelCase に統一します:

```javascript
// v5
token.byte_start, token.byte_end, token.word_id, token.is_unknown

// v6
token.byteStart, token.byteEnd, token.wordId, token.isUnknown
```

`surface`・`position`・`details` は変更ありません。既に `toJSON()` を
呼んでその結果を読んでいたコードは、これらの名前を使っていたため
そのまま動作します。ただし `toJSON()` 自体は、トークンが既にプレーン
オブジェクトであるため廃止されました:

```javascript
// v5
const data = token.toJSON();

// v6
const data = token; // 既にプレーンオブジェクト
```

`Token` はモジュールから export されなくなったため、
`import { Token } from 'lindera-wasm-...'` は失敗します。代わりに import
すべきものはありません — `tokenize` が直接プレーンオブジェクトを返します。

WASM の `tokenizeNbest` は v5 の時点で既にプレーンオブジェクトを
返していたため、変更ありません。

## 対応が不要なケース

- **Python・Ruby・PHP バインディング**: 上記の JavaScript の変更の影響を
  受けません。これらは解放が決定的であり、JS 側の作り直しの動機となった
  メモリ問題が元々無かったため、`Token` クラスをそのまま維持しています。
  それぞれプレーンデータへの変換メソッド（`to_dict()`、`to_h`（`to_hash`）、
  `toArray()`）が追加されましたが、削除・改名されたものはありません。
- **`.csv` から読み込むユーザー辞書**: ロード時にコンパイルされるため、
  新しいフォーマットが自動的に反映されます。（ビルド済みの `.bin` は
  再ビルドが**必要**です — 冒頭の節を参照してください。）
- **`lindera` クレートの公開 API**: `Segmenter`・`Tokenizer`・`SegmentWorker`・
  `AnalysisWorker` のシグネチャは変わっていません（加算的な
  `unknown_word_ladder`/`max_grouping_len` オプションを除く）。パスや URI から
  辞書をロードするコードは、辞書自体を再ビルドまたは再ダウンロードすればそのまま
  コンパイル・動作します。`embed-*` の埋め込み辞書は各辞書クレートのビルド
  スクリプトがコンパイルするため、常に実行中のバージョンと一致します。

## アップグレードチェックリスト

全員:

- **ビルド済みユーザー辞書 `.bin` を CSV から再ビルドする**
  （`lindera build --user`）。どの v5 からのアップグレードでも必要です。
- 辞書外・非日本語テキストに対するトークナイズ結果を厳密に固定している場合は
  再確認する — Decompose ペナルティ修正と未知語ラダー（デフォルト有効）の
  両方が結果を変える可能性がある。v5 と同一の未知語出力が必要なら
  `unknown_word_ladder(false)` を設定する。

JavaScript（Node.js・WASM）:

- Node.js: `package.json` の依存を `lindera-nodejs` から `lindera` に変更し、
  `require`/`import` のパッケージ名を更新する。
- WASM: 依存を `lindera-wasm-web` / `lindera-wasm-bundler` から `lindera-wasm`
  に変更する。`lindera-wasm-bundler` を使っていた場合は、使用前に
  デフォルトエクスポートの初期化関数（`await init()`）を呼び出すコードを
  追加する。
- `token.getDetail(i)` を `token.details[i]` に置き換える。
- Node.js: `tokenizeObjects(text)` を `tokenize(text)` に置き換え、
  `Token`/`NbestResult` の import をやめる（export されなくなったため）。
  TypeScript 利用者は型名 `JsTokenData` を `Token` に変更する。
- WASM: トークンのフィールド参照を camelCase に変更し（`byteStart`・
  `byteEnd`・`wordId`・`isUnknown`）、`toJSON()` の呼び出しを削除し、
  `Token` の import をやめる。

Python:

- 依存を `lindera-python` から `lindera` に変更する（`pip install lindera`）。
  `import lindera` はそのままで、コードの変更は不要。

`lindera_dictionary` を直接利用している場合:

- 上記の Rust API 表に従い呼び出し箇所を更新する（`set_text`/`set_text_nbest`
  の追加引数 2 つを含む）。

v5.2 以前からアップグレードする場合（以下は v5.3.0 でリリース済み）:

- 自前ビルドのシステム辞書を `lindera build` で再ビルドするか、`lindera
  download` でビルド済み辞書を再取得する。
- WASM: `loadDictionaryFromBytes()` の呼び出しを 9 引数のシグネチャに更新する
  （`dictDa` の代わりに `dictTrie` と `dictValsIdx`）。
- WASM: v5 世代の辞書を OPFS から削除し、v6 のアーカイブをダウンロードする。
