<?php

$finder = PhpCsFixer\Finder::create()
    ->in(__DIR__ . '/examples')
    ->in(__DIR__ . '/tests')
    ->in(__DIR__ . '/tools')
    ->name('*.php');

// Run from the repository root (`make format-lindera-php`), where composer.json
// and vendor/ live; the cache stays next to this file.
return (new PhpCsFixer\Config())
    ->setCacheFile(__DIR__ . '/.php-cs-fixer.cache')
    ->setRules([
        '@PER-CS' => true,
        'array_syntax' => ['syntax' => 'short'],
        'no_unused_imports' => true,
        'ordered_imports' => ['sort_algorithm' => 'alpha'],
        'single_quote' => true,
        'trailing_comma_in_multiline' => true,
    ])
    ->setFinder($finder);
