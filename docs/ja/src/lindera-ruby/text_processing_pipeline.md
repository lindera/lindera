# テキスト処理パイプライン

Lindera Ruby は、トークナイズ前に文字フィルタを適用し、トークナイズ後にトークンフィルタを適用する、組み合わせ可能なテキスト処理パイプラインをサポートしています。フィルタは `Lindera::TokenizerBuilder` に追加され、追加された順序で実行されます。

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

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_character_filter('unicode_normalize', { 'kind' => 'nfkc' })
tokenizer = builder.build
```

サポートされる正規化形式: `"nfc"`、`"nfkc"`、`"nfd"`、`"nfkd"`。

### mapping

マッピングテーブルに従って文字や文字列を置換します。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_character_filter('mapping', {
  'mapping' => {
    "\u30fc" => '-',
    "\uff5e" => '~'
  }
})
tokenizer = builder.build
```

### japanese_iteration_mark

日本語の踊り字（繰り返し記号）を完全な形に展開します。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_character_filter('japanese_iteration_mark', {
  'normalize_kanji' => 'true',
  'normalize_kana' => 'true'
})
tokenizer = builder.build
```

## トークンフィルタ

トークンフィルタはトークナイズ後にトークンを変換または除去します。

### lowercase

トークンの表層形を小文字に変換します。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('lowercase', nil)
tokenizer = builder.build
```

### japanese_base_form

辞書の形態素情報を使用して、活用形を基本形（辞書形）に置換します。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_base_form', nil)
tokenizer = builder.build
```

### japanese_stop_tags

指定されたタグに一致する品詞のトークンを除去します。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_stop_tags', {
  'tags' => ['助詞', '助動詞']
})
tokenizer = builder.build
```

### japanese_keep_tags

指定されたタグに一致する品詞のトークンのみを保持します。その他のトークンはすべて除去されます。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_keep_tags', {
  'tags' => ['名詞']
})
tokenizer = builder.build
```

### japanese_katakana_stem

カタカナ語の末尾にある長音記号を除去します。最小文字数を指定できます。

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_katakana_stem', { 'min' => 3 })
tokenizer = builder.build
```

## パイプラインの完全な例

以下の例では、複数の文字フィルタとトークンフィルタを1つのパイプラインに組み合わせています：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_mode('normal')
builder.set_dictionary('embedded://ipadic')

# Preprocessing
builder.append_character_filter('unicode_normalize', { 'kind' => 'nfkc' })
builder.append_character_filter('japanese_iteration_mark', {
  'normalize_kanji' => 'true',
  'normalize_kana' => 'true'
})

# Postprocessing
builder.append_token_filter('japanese_base_form', nil)
builder.append_token_filter('japanese_stop_tags', {
  'tags' => ['助詞', '助動詞', '記号']
})
builder.append_token_filter('lowercase', nil)

tokenizer = builder.build

tokens = tokenizer.tokenize('Ｌｉｎｄｅｒａは形態素解析を行うライブラリです。')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```

このパイプラインでは：

1. `unicode_normalize` が全角文字を半角に変換（NFKC 正規化）
2. `japanese_iteration_mark` が踊り字を展開
3. `japanese_base_form` が活用形のトークンを基本形に変換
4. `japanese_stop_tags` が助詞、助動詞、記号を除去
5. `lowercase` がアルファベットを小文字に正規化
