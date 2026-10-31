<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Building a system dictionary (`Lindera\Dictionary::build`) whose metadata
 * enables `connection_id_mapping`, with and without a context-ID frequency
 * histogram.
 */
class BuildDictionaryTest extends TestCase
{
    /**
     * The source of a dictionary with a 4x4 connection matrix, whose
     * metadata.json enables connection_id_mapping and whose one entry has
     * the context IDs 2, with a histogram that counts 1, 3 and 2 for IDs 1,
     * 2 and 3.
     */
    private const FIXTURE = __DIR__ . '/../../resources/connection_id_mapping';

    /**
     * Builds the fixture into a new temporary directory with `$context_id_freq`,
     * and returns the `context_id_map` of the built metadata.json; the
     * directory is removed afterwards.
     *
     * @return array<string, list<int>>
     */
    private static function builtContextIdMap(?string $context_id_freq = null): array
    {
        $dir = sys_get_temp_dir() . '/lindera-build-' . bin2hex(random_bytes(8));
        try {
            $metadata = Lindera\Metadata::fromJsonFile(self::FIXTURE . '/metadata.json');
            Lindera\Dictionary::build(self::FIXTURE . '/source', $dir, $metadata, $context_id_freq);
            $built = json_decode((string) file_get_contents($dir . '/metadata.json'), true, flags: JSON_THROW_ON_ERROR);
            return $built['context_id_map'];
        } finally {
            self::removeTree($dir);
        }
    }

    /**
     * Removes `$path` and, if it is a directory, everything in it.
     */
    private static function removeTree(string $path): void
    {
        if (is_dir($path) && !is_link($path)) {
            foreach (scandir($path) ?: [] as $entry) {
                if ($entry !== '.' && $entry !== '..') {
                    self::removeTree($path . '/' . $entry);
                }
            }
            rmdir($path);
        } else {
            @unlink($path);
        }
    }

    public function testBuildKeepsConnectionIdMappingOfTheMetadata(): void
    {
        // Without a histogram, the IDs other than 0 are ranked by the count of
        // dictionary entries, ties by ID: 2 (the entry's), then 1 and 3.
        $this->assertSame(
            ['left' => [0, 2, 1, 3], 'right' => [0, 2, 1, 3]],
            self::builtContextIdMap(),
        );
    }

    public function testBuildRanksContextIdsByTheHistogram(): void
    {
        $this->assertSame(
            ['left' => [0, 3, 1, 2], 'right' => [0, 3, 1, 2]],
            self::builtContextIdMap(self::FIXTURE . '/context_id_freq.txt'),
        );
    }
}
