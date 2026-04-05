# Text Processing Pipeline

Lindera Ruby supports a composable text processing pipeline that applies character filters before tokenization and token filters after tokenization. Filters are added to the `TokenizerBuilder` and executed in the order they are appended.

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

## Character Filters

Character filters transform the input text before tokenization.

### unicode_normalize

Applies Unicode normalization to the input text.

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_character_filter('unicode_normalize', { 'kind' => 'nfkc' })
tokenizer = builder.build
```

Supported normalization forms: `"nfc"`, `"nfkc"`, `"nfd"`, `"nfkd"`.

### mapping

Replaces characters or strings according to a mapping table.

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

Resolves Japanese iteration marks (odoriji) into their full forms.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_character_filter('japanese_iteration_mark', {
  'normalize_kanji' => 'true',
  'normalize_kana' => 'true'
})
tokenizer = builder.build
```

## Token Filters

Token filters transform or remove tokens after tokenization.

### lowercase

Converts token surface forms to lowercase.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('lowercase', nil)
tokenizer = builder.build
```

### japanese_base_form

Replaces inflected forms with their base (dictionary) form using the morphological details from the dictionary.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_base_form', nil)
tokenizer = builder.build
```

### japanese_stop_tags

Removes tokens whose part-of-speech matches any of the specified tags.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_stop_tags', {
  'tags' => ['助詞', '助動詞']
})
tokenizer = builder.build
```

### japanese_keep_tags

Keeps only tokens whose part-of-speech matches one of the specified tags. All other tokens are removed.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_keep_tags', {
  'tags' => ['名詞']
})
tokenizer = builder.build
```

### japanese_katakana_stem

Removes trailing prolonged sound marks from katakana tokens that exceed a minimum length.

```ruby
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.append_token_filter('japanese_katakana_stem', { 'min' => 3 })
tokenizer = builder.build
```

## Complete Pipeline Example

The following example combines multiple character filters and token filters into a single pipeline:

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

In this pipeline:

1. `unicode_normalize` converts full-width characters to half-width (NFKC normalization)
2. `japanese_iteration_mark` resolves iteration marks
3. `japanese_base_form` converts inflected tokens to base form
4. `japanese_stop_tags` removes particles, auxiliary verbs, and symbols
5. `lowercase` normalizes alphabetic characters to lowercase
