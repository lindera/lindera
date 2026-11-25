# v7.0 から v7.1 への移行

このガイドは、v7.1.0 の破壊的変更をまとめたものです。Rust の API に 1 つあります。マイナーの版に入れたのは、メンテナーの決定です（[#1440](https://github.com/lindera/lindera/issues/1440)）。トークナイザーが返したトークンを読むコードは、影響を受けません。

## 概要

| 変更 | 影響を受ける人 | 対応 |
| --- | --- | --- |
| **`Token` が係り受け解析の結果を持つようになり、`#[non_exhaustive]` になった** | `Token` を構造体式で作る、または全部の欄を分解する Rust のコード | `Token::new` でトークンを作り、違う欄は後から設定する |

トークナイザーが返したトークンを読むコードは、影響を受けません。`surface`、`byte_start`、`details()`、`as_value()` などの欄とメソッドは、これまでどおりです。

v7.1 は、文節の係り受け解析（[Lindera Dependency](./lindera-dependency.md)）を足します。この変更は、それに伴うものです。v7.1 が足すほかのものは、既存のコードの変更を必要としません。利用者が気づきうる点は、[変わらないこと](#変わらないこと)に挙げます。

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

## 変わらないこと

- `Token::as_value` と `lindera tokenize -o json` が書く JSON は、係り受けを持たないトークンでは、キーが増えません。
- `Token::details` は、これまでどおり辞書の欄を持ちます。係り受け解析の結果は、そこには書きません。
- トークンは、ポインター 1 つ分だけ大きくなります。
- `lindera` コマンドとバインディングは、既定のビルドに `dependency` feature を持ちます。トークンフィルター `japanese_dependency` がパッケージに入り、`lindera-dependency` クレートに依存します（このクレート自身は依存を持ちません）。コマンドは 134 kB 大きくなります。`default-features = false` で、入っていないビルドになります。ライブラリの `lindera` と `lindera-analysis` は、feature を指定したときだけ持ちます。
- モデルは、パッケージにも入っておらず、既定では埋め込みもしません。IPADIC のモデルは、`lindera download dependency-ipadic` で取得します。
