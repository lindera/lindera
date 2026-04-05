# テキスト処理パイプライン

Lindera PHP は、トークナイズ前に文字フィルタを適用し、トークナイズ後にトークンフィルタを適用する、組み合わせ可能なテキスト処理パイプラインをサポートしています。フィルタは `TokenizerBuilder` に追加され、追加された順序で実行されます。

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

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$tokenizer = $builder->build();
```

サポートされる正規化形式: `"nfc"`、`"nfkc"`、`"nfd"`、`"nfkd"`。

### mapping

マッピングテーブルに従って文字や文字列を置換します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('mapping', [
    'mapping' => [
        'リンデラ' => 'lindera',
    ],
]);
$tokenizer = $builder->build();
```

### japanese_iteration_mark

日本語の踊り字（繰り返し記号）を完全な形に展開します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('japanese_iteration_mark', [
    'normalize_kanji' => 'true',
    'normalize_kana' => 'true',
]);
$tokenizer = $builder->build();
```

## トークンフィルタ

トークンフィルタはトークナイズ後にトークンを変換または除去します。

### lowercase

トークンの表層形を小文字に変換します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('lowercase');
$tokenizer = $builder->build();
```

### japanese_base_form

辞書の形態素情報を使用して、活用形を基本形（辞書形）に置換します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_base_form', []);
$tokenizer = $builder->build();
```

### japanese_katakana_stem

カタカナ語の語尾の長音記号を除去してステミングを行います。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_katakana_stem', ['min' => 3]);
$tokenizer = $builder->build();
```

### japanese_stop_tags

指定されたタグに一致する品詞のトークンを除去します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => ['助詞', '助動詞', '記号'],
]);
$tokenizer = $builder->build();
```

### japanese_keep_tags

指定されたタグに一致する品詞のトークンのみを保持します。その他のトークンはすべて除去されます。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_keep_tags', [
    'tags' => ['名詞'],
]);
$tokenizer = $builder->build();
```

## パイプラインの完全な例

以下の例では、複数の文字フィルタとトークンフィルタを1つのパイプラインに組み合わせています：

```php
<?php

$builder = new Lindera\TokenizerBuilder();

// Set mode and dictionary
$builder->setMode('normal');
$builder->setDictionary('embedded://ipadic');

// Preprocessing
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$builder->appendCharacterFilter(
    'japanese_iteration_mark',
    ['normalize_kanji' => 'true', 'normalize_kana' => 'true']
);
$builder->appendCharacterFilter('mapping', ['mapping' => ['リンデラ' => 'lindera']]);

// Postprocessing
$builder->appendTokenFilter('japanese_katakana_stem', ['min' => 3]);
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => [
        '接続詞',
        '助詞',
        '助詞,格助詞',
        '助詞,格助詞,一般',
        '助詞,係助詞',
        '助詞,副助詞',
        '助詞,終助詞',
        '助詞,連体化',
        '助動詞',
        '記号',
        '記号,一般',
        '記号,読点',
        '記号,句点',
        '記号,空白',
    ],
]);
$builder->appendTokenFilter('lowercase');

// Build the tokenizer
$tokenizer = $builder->build();

// Tokenize
$text = 'Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。';
$tokens = $tokenizer->tokenize($text);

foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```

このパイプラインでは：

1. `unicode_normalize` が全角文字を半角に変換（NFKC 正規化）
2. `japanese_iteration_mark` が踊り字を展開
3. `mapping` が指定された文字列を置換
4. `japanese_katakana_stem` がカタカナ語をステミング
5. `japanese_stop_tags` が助詞、助動詞、記号を除去
6. `lowercase` がアルファベットを小文字に正規化
