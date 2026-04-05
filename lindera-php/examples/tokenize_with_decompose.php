<?php

/**
 * Tokenization with decompose mode example.
 *
 * Usage:
 *   cargo build -p lindera-php --features embed-ipadic
 *   php -d extension=target/debug/liblindera_php.so lindera-php/examples/tokenize_with_decompose.php
 */

// Load the dictionary
$dictionary = Lindera\Dictionary::load('embedded://ipadic');

// Create a tokenizer with decompose mode
$tokenizer = new Lindera\Tokenizer($dictionary, 'decompose');

$text = '関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う';
echo "text: {$text}\n\n";

// Tokenize the text
$tokens = $tokenizer->tokenize($text);

foreach ($tokens as $token) {
    echo $token->surface . "\n";
}
