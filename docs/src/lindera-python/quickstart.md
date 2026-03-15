# Quick Start

This guide shows how to tokenize text using lindera-python.

## Basic Tokenization

The recommended way to create a tokenizer is through `TokenizerBuilder`:

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

Expected output:

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## Method Chaining

`TokenizerBuilder` supports method chaining for concise configuration:

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

## Accessing Token Properties

Each token exposes the following properties:

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

## N-best Tokenization

Retrieve multiple tokenization candidates ranked by cost:

```python
from lindera import TokenizerBuilder

tokenizer = TokenizerBuilder().set_dictionary("embedded://ipadic").build()
results = tokenizer.tokenize_nbest("すもももももももものうち", n=3)

for tokens, cost in results:
    surfaces = [t.surface for t in tokens]
    print(f"Cost {cost}: {' / '.join(surfaces)}")
```
