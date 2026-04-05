# Quick Start

This guide shows how to tokenize text using lindera-ruby.

## Basic Tokenization

The recommended way to create a tokenizer is through `Lindera::TokenizerBuilder`:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_mode('normal')
builder.set_dictionary('embedded://ipadic')
tokenizer = builder.build

tokens = tokenizer.tokenize('関西国際空港限定トートバッグ')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```

Expected output:

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## Sequential Configuration

`TokenizerBuilder` is configured through sequential method calls:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_mode('normal')
builder.set_dictionary('embedded://ipadic')
tokenizer = builder.build

tokens = tokenizer.tokenize('すもももももももものうち')
tokens.each do |token|
  puts "#{token.surface}\t#{token.get_detail(0)}"
end
```

## Accessing Token Properties

Each token exposes the following properties:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
tokenizer = builder.build

tokens = tokenizer.tokenize('東京タワー')

tokens.each do |token|
  puts "Surface: #{token.surface}"
  puts "Byte range: #{token.byte_start}..#{token.byte_end}"
  puts "Position: #{token.position}"
  puts "Word ID: #{token.word_id}"
  puts "Unknown: #{token.is_unknown}"
  puts "Details: #{token.details}"
  puts
end
```

## N-best Tokenization

Retrieve multiple tokenization candidates ranked by cost:

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
tokenizer = builder.build

results = tokenizer.tokenize_nbest('すもももももももものうち', 3, false, nil)

results.each do |tokens, cost|
  surfaces = tokens.map(&:surface)
  puts "Cost #{cost}: #{surfaces.join(' / ')}"
end
```
