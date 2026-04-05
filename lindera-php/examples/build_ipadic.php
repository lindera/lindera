<?php

/**
 * Build IPADIC dictionary from source example.
 *
 * Usage:
 *   cargo build -p lindera-php --features embed-ipadic
 *   php -d extension=target/debug/liblindera_php.so lindera-php/examples/build_ipadic.php
 */

$projectRoot = dirname(__DIR__);

$url = "https://lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz";
$filename = "/tmp/mecab-ipadic-2.7.0-20070801.tar.gz";

// Download dictionary source file
$version = Lindera\Dictionary::version();
$context = stream_context_create([
    "http" => [
        "header" => "User-Agent: lindera-php/{$version}\r\n"
    ]
]);

echo "Downloading {$url}...\n";
file_put_contents($filename, file_get_contents($url, false, $context));

// Extract the dictionary source file
echo "Extracting...\n";
$phar = new PharData($filename);
$phar->extractTo("/tmp/", null, true);

$sourcePath = "/tmp/mecab-ipadic-2.7.0-20070801";
$destinationPath = "/tmp/lindera-ipadic-2.7.0-20070801";
$metadataPath = $projectRoot . "/resources/ipadic_metadata.json";

$metadata = Lindera\Metadata::fromJsonFile($metadataPath);

// Build dictionary
echo "Building dictionary...\n";
Lindera\Dictionary::build($sourcePath, $destinationPath, $metadata);

// List all files in the destination directory
echo "\nFiles created in {$destinationPath}:\n";
$iterator = new RecursiveIteratorIterator(
    new RecursiveDirectoryIterator($destinationPath, RecursiveDirectoryIterator::SKIP_DOTS)
);
foreach ($iterator as $file) {
    $relPath = substr($file->getPathname(), strlen($destinationPath) + 1);
    $size = number_format($file->getSize());
    echo "  {$relPath} ({$size} bytes)\n";
}
echo "\n";
