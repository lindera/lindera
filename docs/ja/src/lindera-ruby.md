# Lindera Ruby

Lindera Ruby は、[Magnus](https://github.com/matsadler/magnus) と [rb-sys](https://github.com/oxidize-rb/rb-sys) を使用して構築された Lindera 形態素解析エンジンの Ruby バインディングです。Ruby 3.1 以降をサポートし、Lindera の高性能なトークナイズ機能を Ruby エコシステムに提供します。

## 特徴

- **多言語対応**: 日本語（IPADIC、IPADIC NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）のテキストをトークナイズ
- **テキスト処理パイプライン**: 文字フィルタとトークンフィルタを組み合わせて、柔軟な前処理・後処理が可能
- **CRF ベースの辞書学習**: アノテーション付きコーパスからカスタム形態素解析モデルを学習（`train` feature が必要）
- **複数のトークナイズモード**: 解析粒度に応じた Normal モードと Decompose モード
- **N-best トークナイズ**: コスト順にランク付けされた複数のトークナイズ候補を取得
- **ユーザー辞書**: システム辞書をカスタム語彙で拡張

## 整数の引数

整数の引数は、その設定が保持できる値だけを受け付けます。`Lindera::Metadata.new` の `default_word_cost` は -32768 から 32767、文脈 ID は 0 から 65535、`Lindera::Penalty.new` のペナルティは 32 ビットの整数を受け付けます。個数や位置は 0 以上を受け付けます。該当するのは、`Lindera::Penalty.new` のしきい値、`Schema#get_field_name` と `Token#get_detail` の `index`、`Tokenizer#tokenize_nbest` の `n`、`Lindera.train` の `max_iter` と `max_threads` です。`Tokenizer#tokenize_nbest` の `cost_threshold` は 64 ビットの整数を受け付けます。範囲外の値は、引数の名前と範囲を含む `RangeError` を投げます。末尾より先の位置は、これまでどおり `nil` を返します。

```ruby
require 'lindera'

begin
  Lindera::Metadata.new(nil, nil, 100_000, nil, nil, nil, nil, nil)
rescue RangeError => e
  puts "#{e.class}: #{e.message}"
  # RangeError: default_word_cost must be an integer between -32768 and 32767, got 100000
end

tokenizer = Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ipadic'))

begin
  tokenizer.tokenize_nbest('東京都', -1, nil, nil)
rescue RangeError => e
  puts "#{e.class}: #{e.message}"
  # RangeError: n must be an integer between 0 and 18446744073709551615, got -1
end
```

## ドキュメント

- [インストール](./lindera-ruby/installation.md) -- 前提条件、ビルド手順、feature フラグ
- [クイックスタート](./lindera-ruby/quickstart.md) -- 最小限の使用例
- [Tokenizer API](./lindera-ruby/tokenizer_api.md) -- `TokenizerBuilder`、`Tokenizer`、`Token` クラスリファレンス
- [辞書管理](./lindera-ruby/dictionary_management.md) -- 辞書の読み込み、ビルド、管理
- [テキスト処理パイプライン](./lindera-ruby/text_processing_pipeline.md) -- 文字フィルタとトークンフィルタ
- [学習](./lindera-ruby/training.md) -- カスタム CRF モデルの学習と辞書のエクスポート
