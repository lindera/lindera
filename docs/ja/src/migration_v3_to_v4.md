# v3 から v4 への移行

Lindera v4.0.0 は、v3 系で意図的に先送りしていた破壊的変更をまとめて取り込んだメジャー
リリースです。個々の変更は小さいものですが、安心してアップグレードできるよう、すべての変更を
本ガイドに列挙します。

破壊的変更は、v3.0.7 と v4 の公開サーフェスを `cargo public-api` で機械的に差分比較して検証
しています。

## 概要

| 変更 | 対象 | 対応 |
| --- | --- | --- |
| デフォルトスキーマのフィールド名が `pos_detail_*` に統一 | Python, Node.js, Ruby, PHP | `middle_pos` / `small_pos` / `fine_pos` を `pos_detail_1` / `pos_detail_2` / `pos_detail_3` に変更 |
| `Token.details` が常にリスト | Python, Node.js, Ruby | `null` / `None` / `nil` の処理を削除 |
| バインディングの `Segmenter` を削除 | Python, WASM | 代わりに tokenizer を使用 |
| `LINDERA_CACHE` 環境変数を削除 | Rust ビルド, CLI | `LINDERA_DICTIONARIES_PATH` を使用 |
| ユーザー辞書のバイナリ形式が変更 | すべて（ビルド済み `.bin`） | ユーザー辞書を CSV から再ビルド |
| `lindera-dictionary` の viterbi 内部をカプセル化 | Rust クレート利用者 | 新しいアクセサを使用 |

最上位の `lindera` クレートの公開 Rust API は v3.0.7 と v4 で変わりません。

## デフォルト辞書スキーマのフィールド名

`Schema.create_default()`（およびデフォルト辞書スキーマ）は、3 つの品詞詳細フィールドを
`middle_pos` / `small_pos` / `fine_pos` ではなく `pos_detail_1` / `pos_detail_2` /
`pos_detail_3`（インデックス 5, 6, 7）と命名するようになりました。これにより、すべての
バインディングが、すでに `pos_detail_*` を使用していたコアの
`lindera::dictionary::Schema::default()` と一致します。

対象は Python・Node.js・Ruby・PHP バインディングです。WASM バインディングはすでに
`pos_detail_*` を使用していたため変更ありません。

Python の例:

```python
schema = Schema.create_default()
# v3: schema.fields[5] == "middle_pos"
# v4: schema.fields[5] == "pos_detail_1"

# v3
index = schema.get_field_index("middle_pos")
# v4
index = schema.get_field_index("pos_detail_1")
```

これらのフィールド名を文字列で参照している箇所（ルックアップ、カスタムスキーマ、シリアライズ
された設定など）があれば、`pos_detail_*` 形式に更新してください。

## `Token.details` は常にリスト

`Token.details` は常に文字列のリストになり、`null` / `None` / `nil` を返さなくなりました。
詳細を持たないトークンは空リストで表現されます。以前は Python・Node.js・Ruby バインディングが
（実際には常に値が入っているにもかかわらず）nullable 型でラップしていました。PHP と WASM
バインディングはもともと非 nullable でした。

Python では型が `list[str] | None` から `list[str]` に変わります:

```python
# v3 — 型の都合で null チェックが必要だった
if token.details is not None:
    pos = token.details[0]

# v4 — details は常にリスト
pos = token.details[0]
```

Node.js では `Array<string> | null` から `Array<string>` に、Ruby では `Array | nil` から
`Array` に変わります。`null` / `nil` チェックを削除してください。

## バインディングの `Segmenter` を削除

使われていなかった `Segmenter` ラッパーをバインディングから削除しました。コンストラクタが無く
使用できないもので、形態素分割は常に tokenizer から利用できます。

- Python: `lindera.segmenter` サブモジュールと `lindera.segmenter.Segmenter` が無くなりました。
- WASM: `Segmenter` クラスのエクスポートが無くなりました。

代わりに tokenizer でトークン化してください:

```python
from lindera import Tokenizer, TokenizerBuilder

tokenizer = TokenizerBuilder().build()
tokens = tokenizer.tokenize("関西国際空港")
```

## `LINDERA_CACHE` 環境変数を削除

非推奨だったビルド時環境変数 `LINDERA_CACHE` を削除しました。数リリース前から正式に
サポートされている `LINDERA_DICTIONARIES_PATH` を使用してください:

```sh
# v3（非推奨）
export LINDERA_CACHE=/path/to/dicts

# v4
export LINDERA_DICTIONARIES_PATH=/path/to/dicts
```

## ユーザー辞書のバイナリ形式が変更

ユーザー辞書は、システム辞書と同じ 8-bit のバリアント数エンコーディングを使うようになりました
（1 表層あたり最大 255 バリアント、以前は 31）。そのため、v3 でビルドしたユーザー辞書の `.bin`
ファイルは v4 では誤ってデコードされます。形式バージョンのガードが無いため、失敗はサイレントで
す（トークンは生成されますが、誤った詳細になります）。

v4 でユーザー辞書を CSV から再ビルドしてください:

```sh
lindera build --user \
  --src user_dict.csv \
  --dest ./build \
  --metadata lindera-ipadic/metadata.json
```

ユーザー辞書をビルド済み `.bin` ではなく `.csv` から読み込んでいる場合は、読み込み時に再ビルド
されるため対応は不要です。

## Rust ライブラリ: `lindera-dictionary` の viterbi 内部

これは `lindera-dictionary` クレートを直接利用している場合にのみ影響します。`lindera` クレートの
API は変わりません。

内部の viterbi 構造体は公開フィールドを持たなくなりました。代わりにアクセサを使用してください:

```rust
// v3 — フィールドへ直接アクセス
let id = word_id.id;
let cost = word_entry.word_cost;

// v4 — アクセサ
let id = word_id.id();
let cost = word_entry.word_cost();
```

`lindera_dictionary::viterbi` のその他の変更:

- `EdgeType` を削除しました。
- `WordEntry` に `new()` / `word_cost()` / `word_id()` を、`WordId` に `id()` を追加しました。
- `WordEntry::serialize` / `WordEntry::deserialize` / `WordEntry::SERIALIZED_LEN` を非公開に
  しました。
- `util::read_aligned_file` と `embedded_dictionary!` マクロを追加しました。

この一覧は、`lindera-dictionary` クレートの v3.0.7 と v4 リリース間で機械生成された
完全な `cargo public-api` 差分から作成しています。

## アップグレードチェックリスト

- `middle_pos` / `small_pos` / `fine_pos` を `pos_detail_1` / `pos_detail_2` /
  `pos_detail_3` に置き換える（Python, Node.js, Ruby, PHP）。
- `Token.details` の `null` / `None` / `nil` チェックを削除する（Python, Node.js, Ruby）。
- バインディングの `Segmenter` の使用を tokenizer に置き換える（Python, WASM）。
- `LINDERA_CACHE` を `LINDERA_DICTIONARIES_PATH` に置き換える。
- ビルド済みユーザー辞書 `.bin` を CSV から再ビルドする。
- `lindera-dictionary` の viterbi フィールド直接アクセスを新しいアクセサに切り替える（Rust）。
