# クイックスタート

このガイドでは、lindera-python を使用してテキストをトークナイズする方法を紹介します。

## 基本的なトークナイズ

トークナイザーの作成には `TokenizerBuilder` の使用を推奨します：

```python
from lindera import TokenizerBuilder

builder = TokenizerBuilder()
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")
tokenizer = builder.build()

tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```

期待される出力：

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## メソッドチェーン

`TokenizerBuilder` は簡潔な設定のためにメソッドチェーンをサポートしています：

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_mode("normal")
    .set_dictionary("embedded://ipadic")
    .build()
)

tokens = tokenizer.tokenize("すもももももももものうち")
for token in tokens:
    print(f"{token.surface}\t{token.get_detail(0)}")
```

## トークンプロパティへのアクセス

各トークンは以下のプロパティを公開しています：

```python
from lindera import TokenizerBuilder

tokenizer = TokenizerBuilder().set_dictionary("embedded://ipadic").build()
tokens = tokenizer.tokenize("東京タワー")

for token in tokens:
    print(f"Surface: {token.surface}")
    print(f"Byte range: {token.byte_start}..{token.byte_end}")
    print(f"Position: {token.position}")
    print(f"Word ID: {token.word_id}")
    print(f"Unknown: {token.is_unknown}")
    print(f"Details: {token.details}")
    print()
```

## N-best トークナイズ

コスト順にランク付けされた複数のトークナイズ候補を取得します：

```python
from lindera import TokenizerBuilder

tokenizer = TokenizerBuilder().set_dictionary("embedded://ipadic").build()
results = tokenizer.tokenize_nbest("すもももももももものうち", n=3)

for tokens, cost in results:
    surfaces = [t.surface for t in tokens]
    print(f"Cost {cost}: {' / '.join(surfaces)}")
```
