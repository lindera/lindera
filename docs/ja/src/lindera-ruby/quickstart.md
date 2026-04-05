# クイックスタート

このガイドでは、lindera-ruby を使用してテキストをトークナイズする方法を紹介します。

## 基本的なトークナイズ

トークナイザーの作成には `Lindera::TokenizerBuilder` の使用を推奨します：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_mode('normal')
builder.set_dictionary('/path/to/ipadic')
tokenizer = builder.build

tokens = tokenizer.tokenize('関西国際空港限定トートバッグ')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```

> **注意:** ビルド済み辞書を [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードし、展開したディレクトリのパスを指定してください。

期待される出力：

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## 逐次的な設定

`TokenizerBuilder` は逐次的なメソッド呼び出しで設定します：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_mode('normal')
builder.set_dictionary('/path/to/ipadic')
tokenizer = builder.build

tokens = tokenizer.tokenize('すもももももももものうち')
tokens.each do |token|
  puts "#{token.surface}\t#{token.get_detail(0)}"
end
```

## トークンプロパティへのアクセス

各トークンは以下のプロパティを公開しています：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/path/to/ipadic')
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

## N-best トークナイズ

コスト順にランク付けされた複数のトークナイズ候補を取得します：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/path/to/ipadic')
tokenizer = builder.build

results = tokenizer.tokenize_nbest('すもももももももものうち', 3, false, nil)

results.each do |tokens, cost|
  surfaces = tokens.map(&:surface)
  puts "Cost #{cost}: #{surfaces.join(' / ')}"
end
```
