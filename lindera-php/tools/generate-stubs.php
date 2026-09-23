#!/usr/bin/env php
<?php

declare(strict_types=1);

/**
 * Prints stubs for the loaded lindera extension to stdout.
 *
 * Usage (from the repository root, after `cargo build -p lindera-php`):
 *
 *   php -n -d extension=target/debug/liblindera_php.so lindera-php/tools/generate-stubs.php \
 *       > lindera-php/stubs/lindera.stubs.php
 *
 * `make stubs-lindera-php` does exactly that. The committed stub file must
 * match this output; `StubsTest` checks that.
 */

require __DIR__ . '/StubGenerator.php';

if (!extension_loaded('lindera')) {
    fwrite(STDERR, "The lindera extension is not loaded; pass it with -d extension=/path/to/liblindera_php.so\n");
    exit(1);
}

echo (new Lindera\Tools\StubGenerator())->generate();
