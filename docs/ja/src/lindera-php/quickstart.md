# クイックスタート

このガイドでは、lindera-php を使用してテキストをトークナイズする方法を紹介します。

## 基本的なトークナイズ

辞書を読み込み、トークナイザーを作成してテキストをトークナイズします：

```php
<?php

// Load the dictionary
$dictionary = Lindera\Dictionary::load('embedded://ipadic');

// Create a tokenizer
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');

// Tokenize the text
$tokens = $tokenizer->tokenize('関西国際空港限定トートバッグ');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```

期待される出力：

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## TokenizerBuilder の使用

`TokenizerBuilder` を使用すると、より柔軟にトークナイザーを設定できます：

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setMode('normal');
$builder->setDictionary('embedded://ipadic');
$tokenizer = $builder->build();

$tokens = $tokenizer->tokenize('すもももももももものうち');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . $token->getDetail(0) . "\n";
}
```

## トークンプロパティへのアクセス

各トークンは以下のプロパティを公開しています：

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');
$tokens = $tokenizer->tokenize('東京タワー');

foreach ($tokens as $token) {
    echo "Surface: {$token->surface}\n";
    echo "Byte range: {$token->byte_start}..{$token->byte_end}\n";
    echo "Position: {$token->position}\n";
    echo "Word ID: {$token->word_id}\n";
    echo "Unknown: " . ($token->is_unknown ? 'true' : 'false') . "\n";
    echo "Details: " . implode(',', $token->details) . "\n";
    echo "\n";
}
```

## N-best トークナイズ

コスト順にランク付けされた複数のトークナイズ候補を取得します：

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$tokenizer = $builder->build();

$results = $tokenizer->tokenizeNbest('東京都', 3);
foreach ($results as $result) {
    $surfaces = array_map(fn($t) => $t->surface, $result->tokens);
    echo "Cost {$result->cost}: " . implode(' / ', $surfaces) . "\n";
}
```

## Decompose モード

Decompose モードでは、複合語をより小さな単位に分解します：

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'decompose');

$tokens = $tokenizer->tokenize('関西国際空港限定トートバッグ');
foreach ($tokens as $token) {
    echo $token->surface . "\n";
}
```
