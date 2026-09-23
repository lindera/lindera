<?php

declare(strict_types=1);

/**
 * PHPUnit bootstrap.
 *
 * The extension is loaded with `php -d extension=...`, so all this has to do
 * is fail loudly when it was not, rather than letting every test report a
 * confusing "class not found".
 */
// The module registers itself as `lindera` (see get_module in src/lib.rs): the
// same name PIE, `php -m` and `extension=lindera` use.
if (!extension_loaded('lindera')) {
    fwrite(
        STDERR,
        "The lindera extension is not loaded.\n"
        . "Run the tests through `make test-lindera-php`, or pass the built library:\n"
        . "  php -d extension=/path/to/liblindera_php.so vendor/bin/phpunit -c lindera-php/phpunit.xml.dist\n"
    );
    exit(1);
}

// composer.json lives at the repository root (Packagist reads it there), so
// Composer installs into <root>/vendor rather than next to this directory.
require __DIR__ . '/../../vendor/autoload.php';
