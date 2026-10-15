<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * `Lindera\Trainer::export()` writes the files `lindera export` writes
 * (#1169).
 */
class TrainerTest extends TestCase
{
    /**
     * The files `lindera export` writes besides `metadata.json`.
     */
    private const EXPORTED_FILES = [
        'lex.csv',
        'matrix.def',
        'unk.def',
        'char.def',
        'feature.def',
        'rewrite.def',
        'left-id.def',
        'right-id.def',
    ];

    /**
     * The training input files and their contents.
     */
    private const INPUTS = [
        'seed.csv' => "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            . "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n",
        'char.def' => "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n\n0x3041..0x3096 HIRAGANA\n",
        'unk.def' => "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            . "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n",
        'feature.def' => "UNIGRAM U00:%F[0]\nBIGRAM B00:%L[0]/%R[0]\n",
        'rewrite.def' => "名詞,一般\tNOUN\n",
        'corpus.txt' => "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            . "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
            . "EOS\n",
    ];

    protected function setUp(): void
    {
        if (!class_exists(Lindera\Trainer::class)) {
            $this->markTestSkipped('train feature not available');
        }
    }

    /**
     * Writes the training inputs to a new temporary directory, trains a model
     * from them, passes the directory and the model file to `$fn` and removes
     * the directory, with what `$fn` wrote in it, afterwards.
     */
    private static function withModel(callable $fn): void
    {
        $dir = sys_get_temp_dir() . '/lindera-trainer-' . bin2hex(random_bytes(8));
        mkdir($dir);
        try {
            foreach (self::INPUTS as $name => $content) {
                file_put_contents($dir . '/' . $name, $content);
            }
            $model = $dir . '/model.dat';
            Lindera\Trainer::train(
                $dir . '/seed.csv',
                $dir . '/corpus.txt',
                $dir . '/char.def',
                $dir . '/unk.def',
                $dir . '/feature.def',
                $dir . '/rewrite.def',
                $model,
                0.01,
                5,
                1,
            );
            $fn($dir, $model);
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

    /**
     * Returns the names of the files in `$dir`, sorted.
     *
     * @return list<string>
     */
    private static function fileNames(string $dir): array
    {
        $names = array_values(array_diff(scandir($dir) ?: [], ['.', '..']));
        sort($names);
        return $names;
    }

    /**
     * Returns `$names` sorted.
     *
     * @param list<string> $names
     * @return list<string>
     */
    private static function sorted(array $names): array
    {
        sort($names);
        return $names;
    }

    public function testExportWritesTheFilesOfLinderaExport(): void
    {
        self::withModel(function (string $dir, string $model): void {
            $output = $dir . '/exported';
            Lindera\Trainer::export($model, $output);

            $this->assertSame(self::sorted(self::EXPORTED_FILES), self::fileNames($output));
            foreach (self::EXPORTED_FILES as $name) {
                $this->assertGreaterThan(0, filesize($output . '/' . $name), $name);
            }
            // The definition files are those the model was trained with.
            foreach (['char.def', 'feature.def', 'rewrite.def'] as $name) {
                $this->assertSame(
                    file_get_contents($dir . '/' . $name),
                    file_get_contents($output . '/' . $name),
                    $name,
                );
            }
        });
    }

    public function testExportWithMetadataWritesMetadataJsonToo(): void
    {
        self::withModel(function (string $dir, string $model): void {
            $metadata = $dir . '/metadata.json';
            file_put_contents($metadata, '{"name": "test-dict", "version": "1.0.0", "encoding": "utf-8"}');
            $output = $dir . '/exported';
            Lindera\Trainer::export($model, $output, $metadata);

            $this->assertSame(
                self::sorted([...self::EXPORTED_FILES, 'metadata.json']),
                self::fileNames($output),
            );
        });
    }
}
