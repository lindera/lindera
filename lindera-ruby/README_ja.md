# lindera-ruby

[Lindera](https://github.com/lindera/lindera) の Ruby バインディング。CJK テキスト向け形態素解析エンジン。

## 概要

lindera-ruby は、Lindera 形態素解析エンジンへの Ruby インターフェースを提供し、日本語・韓国語・中国語のテキスト解析に対応しています。

- **多言語対応**: 日本語（IPADIC、IPADIC-NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）
- **文字フィルタ**: マッピング、正規表現、Unicode 正規化、日本語踊り字処理によるテキスト前処理
- **トークンフィルタ**: 小文字化、長さフィルタリング、ストップワード、日本語固有フィルタなどの後処理フィルタ
- **柔軟な設定**: トークナイズモードやペナルティ設定のカスタマイズ
- **メタデータ対応**: 辞書スキーマとメタデータの完全な管理
- **Training & Export**（オプション）: コーパスデータからカスタム形態素解析モデルを学習

## 動作要件

- Ruby >= 3.1
- Rust >= 1.85

## 辞書

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) から入手できます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、ローカルパスに展開してください。

## インストール

```bash
cd lindera-ruby
bundle install
bundle exec rake compile
```

## 使い方

```ruby
require "lindera"

# Load dictionary from a local path (download from GitHub Releases)
dictionary = Lindera.load_dictionary("/path/to/ipadic")

# Create a tokenizer
tokenizer = Lindera::Tokenizer.new(dictionary, "normal", nil)

# Tokenize text
tokens = tokenizer.tokenize("関西国際空港")
tokens.each do |token|
  puts "#{token.surface}: #{token.details&.join(', ')}"
end
```

### TokenizerBuilder の使用

```ruby
require "lindera"

builder = Lindera::TokenizerBuilder.new
builder.set_mode("normal")
builder.set_dictionary("/path/to/ipadic")

# Add filters
builder.append_character_filter("unicode_normalize", { "kind" => "nfkc" })
builder.append_token_filter("lowercase", nil)

tokenizer = builder.build
tokens = tokenizer.tokenize("テスト")
```

## テスト

```bash
bundle exec rake compile
bundle exec rake test
```

## ライセンス

MIT
