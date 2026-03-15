# テキスト処理パイプライン

Lindera Python は、トークナイズ前に文字フィルタを適用し、トークナイズ後にトークンフィルタを適用する、組み合わせ可能なテキスト処理パイプラインをサポートしています。フィルタは `TokenizerBuilder` に追加され、追加された順序で実行されます。

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

## 文字フィルタ

文字フィルタはトークナイズ前に入力テキストを変換します。

### unicode_normalize

入力テキストに Unicode 正規化を適用します。

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("unicode_normalize", {"kind": "nfkc"})
    .build()
)
```

サポートされる正規化形式: `"nfc"`、`"nfkc"`、`"nfd"`、`"nfkd"`。

### mapping

マッピングテーブルに従って文字や文字列を置換します。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("mapping", {
        "mapping": {
            "\u30fc": "-",
            "\uff5e": "~",
        }
    })
    .build()
)
```

### japanese_iteration_mark

日本語の踊り字（繰り返し記号）を完全な形に展開します。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_character_filter("japanese_iteration_mark", {
        "normalize_kanji": True,
        "normalize_kana": True,
    })
    .build()
)
```

## トークンフィルタ

トークンフィルタはトークナイズ後にトークンを変換または除去します。

### lowercase

トークンの表層形を小文字に変換します。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("lowercase", {})
    .build()
)
```

### japanese_base_form

辞書の形態素情報を使用して、活用形を基本形（辞書形）に置換します。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_base_form", {})
    .build()
)
```

### japanese_stop_tags

指定されたタグに一致する品詞のトークンを除去します。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_stop_tags", {
        "tags": ["助詞", "助動詞"],
    })
    .build()
)
```

### japanese_keep_tags

指定されたタグに一致する品詞のトークンのみを保持します。その他のトークンはすべて除去されます。

```python
tokenizer = (
    TokenizerBuilder()
    .set_dictionary("embedded://ipadic")
    .append_token_filter("japanese_keep_tags", {
        "tags": ["名詞"],
    })
    .build()
)
```

## パイプラインの完全な例

以下の例では、複数の文字フィルタとトークンフィルタを1つのパイプラインに組み合わせています：

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_mode("normal")
    .set_dictionary("embedded://ipadic")
    # Preprocessing
    .append_character_filter("unicode_normalize", {"kind": "nfkc"})
    .append_character_filter("japanese_iteration_mark", {
        "normalize_kanji": True,
        "normalize_kana": True,
    })
    # Postprocessing
    .append_token_filter("japanese_base_form", {})
    .append_token_filter("japanese_stop_tags", {
        "tags": ["助詞", "助動詞", "記号"],
    })
    .append_token_filter("lowercase", {})
    .build()
)

tokens = tokenizer.tokenize("Ｌｉｎｄｅｒａは形態素解析を行うライブラリです。")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```

このパイプラインでは：

1. `unicode_normalize` が全角文字を半角に変換（NFKC 正規化）
2. `japanese_iteration_mark` が踊り字を展開
3. `japanese_base_form` が活用形のトークンを基本形に変換
4. `japanese_stop_tags` が助詞、助動詞、記号を除去
5. `lowercase` がアルファベットを小文字に正規化
