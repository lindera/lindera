<?php

/**
 * Tokenization with user dictionary example.
 *
 * Usage:
 *   cargo build -p lindera-php --features embed-ipadic
 *   php -d extension=target/debug/liblindera_php.so lindera-php/examples/tokenize_with_userdict.php
 */

$projectRoot = dirname(__DIR__);

// Load the dictionary
$dictionary = Lindera\Dictionary::load('embedded://ipadic');

$metadata = $dictionary->metadata();

// Load the user dictionary
$userDictionaryPath = $projectRoot . '/resources/ipadic_simple_userdic.csv';
$userDictionary = Lindera\Dictionary::loadUser($userDictionaryPath, $metadata);

// Create a tokenizer
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDictionary);

$text = '関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う';
echo "text: {$text}\n\n";

// Tokenize the text
$tokens = $tokenizer->tokenize($text);

foreach ($tokens as $token) {
    echo $token->surface . "\n";
}
