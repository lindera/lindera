# v7.0 から v7.1 への移行

このガイドは、v7.1.0 の破壊的変更をまとめたものです。Rust の API に 3 つあります。マイナーの版に入れたのは、メンテナーの決定です（[#1440](https://github.com/lindera/lindera/issues/1440)、[#1446](https://github.com/lindera/lindera/issues/1446)、[#1466](https://github.com/lindera/lindera/issues/1466)）。トークナイザーが返したトークンを読むコードは、影響を受けません。

## 概要

| 変更 | 影響を受ける人 | 対応 |
| --- | --- | --- |
| **`Token` が係り受け解析の結果を持つようになり、`#[non_exhaustive]` になった** | `Token` を構造体式で作る、または全部の欄を分解する Rust のコード | `Token::new` でトークンを作り、違う欄は後から設定する |
| **`Dictionary::save_to_path` を削除した** | これを呼んでいた Rust のコード（実行すると panic していた） | 辞書のディレクトリをコピーする |
| **`Metadata` に欄 `skip_invalid_encoding` が増えた** | `Metadata` を、全部の欄を書いた構造体式で作る Rust のコード | `skip_invalid_encoding: false` を足すか、式の最後に `..Default::default()` を書く |

トークナイザーが返したトークンを読むコードは、影響を受けません。`surface`、`byte_start`、`details()`、`as_value()` などの欄とメソッドは、これまでどおりです。

v7.1 は、文節の係り受け解析（[Lindera Dependency](./lindera-dependency.md)）を足します。`Token` の変更は、それに伴うものです。v7.1 が足すほかのものは、既存のコードの変更を必要としません。利用者が気づきうる点は、[変わらないこと](#変わらないこと)に挙げます。

v7.1 は、5 つのコマンドの名前を非推奨にもします。動かなくなるものはありません。[辞書のコマンドは `lindera dictionary` の下に](#辞書のコマンドは-lindera-dictionary-の下に)を参照してください。

## `Token` は `Token::new` で作る

`Token` に、係り受け解析の結果を持つ欄が加わりました。`Token::dependency()` で読み、係り受けを解析するトークンフィルターを使わなければ `None` です（[#1438](https://github.com/lindera/lindera/issues/1438)）。欄を足すと、この型の構造体式がすべて壊れます。そこで、この型を `#[non_exhaustive]` にしました。今後は、欄を足してもコードが壊れません。

`#[non_exhaustive]` の構造体は、その型のクレートの外では、構造体式で作れません。欄のパターンには `..` が必要です。該当するのは、トークンを手で作るコードで、多くは自作のトークンフィルターのテストです。

変更前:

```rust
let token = Token {
    surface: Cow::Borrowed("東京都"),
    byte_start: 0,
    byte_end: 9,
    position: 0,
    position_length: 2,
    word_id: WordId::new(LexType::System, 0),
    dictionary: &dictionary,
    user_dictionary: None,
    details: Some(vec![Cow::Borrowed("名詞"), Cow::Borrowed("固有名詞")]),
};
```

変更後:

```rust
let token = Token::new(
    Cow::Borrowed("東京都"),
    0,
    9,
    0,
    WordId::new(LexType::System, 0),
    &dictionary,
    None,
)
.with_position_length(2)
.with_details(vec![Cow::Borrowed("名詞"), Cow::Borrowed("固有名詞")]);
```

- `Token::new` の引数は、表層、バイトの位置（始まりと終わり）、位置、単語の ID、辞書、ユーザー辞書の順です。`position_length` は 1 になり、詳細と係り受けは空のままです。
- `with_position_length` と `with_details` は新しいメソッドで、構造体式で設定していた 2 つの欄を設定します。欄はこれまでどおり公開なので、`token.details = Some(...)` のように代入もできます。
- 全部の欄を並べていたパターンは、末尾に `..` を付けます。`let Token { surface, byte_start, .. } = token;`

該当する箇所は、コンパイラが示します。構造体式は `cannot create non-exhaustive struct using struct expression`（E0639）、パターンは ``..` required with struct marked as non-exhaustive``（E0638）です。

## `Dictionary::save_to_path` を削除した

`lindera_dictionary::dictionary::Dictionary::save_to_path` を削除しました（[#1446](https://github.com/lindera/lindera/issues/1446)）。v1.0.0 から API にありましたが、実装されたことがありません。渡されたディレクトリを作ったあと、どの呼び出しでも panic していました。これを呼ぶコードは、コンパイルできなくなります。

```text
error[E0599]: no method named `save_to_path` found for struct `lindera_dictionary::dictionary::Dictionary` in the current scope
```

代わりのメソッドはありません。Lindera は、読み込んだ辞書を書き出しません。辞書はファイルの入ったディレクトリで、ディレクトリをコピーすれば同じ辞書になります。ソースから作るには [`lindera dictionary build`](./lindera-cli/commands.md) を、公開されている辞書を入れるには [`lindera dictionary download`](./lindera-cli/commands.md) を使います。

## `Metadata` に欄が増えた

`lindera_dictionary::dictionary::metadata::Metadata` に、欄 `skip_invalid_encoding: bool` が増えました（[#1466](https://github.com/lindera/lindera/issues/1466)）。JUMAN 辞書に必要な設定です。これがあると、ビルダーは、語彙の CSV ファイルの行が辞書の文字コードのテキストとして読めないときに、その行を飛ばして警告します（[`build` コマンド](./lindera-cli/commands.md#文字コードとして読めない行)を参照）。`Metadata` は `#[non_exhaustive]` ではないので、全部の欄を書いた構造体式は、コンパイルできなくなります。

```text
error[E0063]: missing field `skip_invalid_encoding` in initializer of `lindera_dictionary::dictionary::metadata::Metadata`
```

`skip_invalid_encoding: false` を足してください。これまでのビルドと同じ動きです。または、式の最後に `..Default::default()` を書いてください。こう書くと、今後に欄が増えても影響を受けません。`metadata.json` を読むコード、`Metadata::default()` から始めるコード、`Metadata::new` を呼ぶコードは、影響を受けません。この欄の無い `metadata.json` は `false` として読まれ、`false` のときは、書き出すファイルにこの欄は入りません。

## 辞書のコマンドは `lindera dictionary` の下に

これは非推奨で、破壊的変更ではありません。v7.0 のコマンドラインは、これまでどおり動きます。

コマンドラインは、先に対象を、その後に操作を書く形になりました（[#1482](https://github.com/lindera/lindera/issues/1482)、[#1483](https://github.com/lindera/lindera/issues/1483)）。係り受け解析の操作は `lindera dependency` に、辞書の操作は `lindera dictionary` にあります。辞書の操作は、これまで `lindera` 自身のサブコマンドでした。

| v7.0 | v7.1 |
| --- | --- |
| `lindera list` | `lindera dictionary list` |
| `lindera download` | `lindera dictionary download` |
| `lindera build` | `lindera dictionary build` |
| `lindera train` | `lindera dictionary train` |
| `lindera export` | `lindera dictionary export` |

オプション・標準出力・終了コードは変わりません。`lindera tokenize` の名前はそのままです。

v7.0 の名前は今も動きます。新しいコマンドと同じことをする前に、標準エラー出力へ 1 行を書きます。

```text
Warning: `lindera build` is deprecated and will be removed in v8.0.0; use `lindera dictionary build` instead
```

`lindera --help` の一覧には出ず、v8.0.0 で削除します。スクリプトが 5 つのどれかを呼んでいるなら、名前を変えてください。変えるまでは、標準エラー出力に 1 行増えるだけで、ほかは変わりません。

これらのコマンドを案内するメッセージは、新しい名前を書きます。たとえば、辞書のディレクトリが空のときの `lindera dictionary list` の案内（`run 'lindera dictionary download <name>' to download one`）や、形式の版が違う辞書のエラー（``rebuild it with `lindera dictionary build` ``）です。ライブラリが同じ文を書くので、バインディングにも同じ文が出ます。

[コマンド](./lindera-cli/commands.md#dictionary) を参照してください。

## 変わらないこと

- `Token::as_value` と `lindera tokenize -o json` が書く JSON は、係り受けを持たないトークンでは、キーが増えません。
- `Token::details` は、これまでどおり辞書の欄を持ちます。係り受け解析の結果は、そこには書きません。
- トークンは、ポインター 1 つ分だけ大きくなります。
- `lindera` コマンドとバインディングは、既定のビルドに `dependency` feature を持ちます。トークンフィルター `japanese_dependency` がパッケージに入り、`lindera-dependency` クレートに依存します（このクレート自身は依存を持ちません）。コマンドは 134 kB 大きくなります。`default-features = false` で、入っていないビルドになります。ライブラリの `lindera` と `lindera-analysis` は、feature を指定したときだけ持ちます。
- モデルは、パッケージにも入っておらず、既定では埋め込みもしません。IPADIC のモデルは、`lindera dependency download ipadic` で取得します。UniDic のモデルは、`unidic`・`unidic-cwj`・`unidic-csj` で取得します。
- 10 個目の辞書として、[JUMAN 辞書](./lindera-jumandic.md)（辞書名 `jumandic`、クレート `lindera-jumandic`、feature `embed-jumandic`）が増えました。mecab-jumandic 7.0-20130310 から作ります（[#1466](https://github.com/lindera/lindera/issues/1466)）。既定のビルドは、これを埋め込みません。`lindera dictionary download jumandic` で取得できます。この辞書の係り受け解析のモデルは、公開していません。CaboCha が配布する JUMAN のモデルを、この辞書用のモデルに[変換できます](./lindera-dependency/cabocha_models.md)。
