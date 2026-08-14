<?php

// Per-call benchmark for lindera-php (interleaved A/B harness member).
//
// Prints one TSV line per workload: "<workload>\t<min microseconds/call>".
// With --verify, prints the token surfaces instead (for the byte-identical
// correctness gate). Run with the extension loaded, e.g.:
//   php -d extension=$BENCH_PHP_EXT scripts/benchmarks/bench_php.php
// See scripts/benchmarks/README.md for the protocol.

$text = getenv('BENCH_TEXT') ?: 'すもももももももものうち';
$calls = (int) (getenv('BENCH_CALLS') ?: '2000');
$inner = (int) (getenv('BENCH_INNER') ?: '10');

function usPerCall(callable $fn, string $text, int $calls): float
{
    $start = hrtime(true);
    for ($i = 0; $i < $calls; $i++) {
        $fn($text);
    }

    return (hrtime(true) - $start) / $calls / 1000.0;
}

function minUs(callable $fn, string $text, int $calls, int $inner): float
{
    $best = INF;
    for ($i = 0; $i < $inner; $i++) {
        $best = min($best, usPerCall($fn, $text, $calls));
    }

    return $best;
}

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$tokenizer = $builder->build();
$hasSurfaces = method_exists($tokenizer, 'tokenizeSurfaces');

if (in_array('--verify', $argv, true)) {
    $surfaces = array_map(static fn ($t) => $t->surface, $tokenizer->tokenize($text));
    echo "tokenize\t" . implode("\t", $surfaces) . "\n";
    if ($hasSurfaces) {
        echo "surfaces\t" . implode("\t", $tokenizer->tokenizeSurfaces($text)) . "\n";
    }
} else {
    printf("tokenize\t%.3f\n", minUs(static fn ($t) => $tokenizer->tokenize($t), $text, $calls, $inner));
    if ($hasSurfaces) {
        printf("surfaces\t%.3f\n", minUs(static fn ($t) => $tokenizer->tokenizeSurfaces($t), $text, $calls, $inner));
    }
}
