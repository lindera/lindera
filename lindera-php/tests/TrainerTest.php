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
     * Writes the training inputs to a new temporary directory, passes the
     * directory to `$fn` and removes the directory, with what `$fn` wrote in
     * it, afterwards.
     */
    private static function withInputs(callable $fn): void
    {
        $dir = sys_get_temp_dir() . '/lindera-trainer-' . bin2hex(random_bytes(8));
        mkdir($dir);
        try {
            foreach (self::INPUTS as $name => $content) {
                file_put_contents($dir . '/' . $name, $content);
            }
            $fn($dir);
        } finally {
            self::removeTree($dir);
        }
    }

    /**
     * Trains a model from the inputs in `$dir` into `$model`, with `$lambda`,
     * 5 iterations and 1 thread.
     */
    private static function train(string $dir, string $model, ?float $lambda): void
    {
        Lindera\Trainer::train(
            $dir . '/seed.csv',
            $dir . '/corpus.txt',
            $dir . '/char.def',
            $dir . '/unk.def',
            $dir . '/feature.def',
            $dir . '/rewrite.def',
            $model,
            $lambda,
            5,
            1,
        );
    }

    /**
     * Writes the training inputs to a new temporary directory, trains a model
     * from them, passes the directory and the model file to `$fn` and removes
     * the directory, with what `$fn` wrote in it, afterwards.
     */
    private static function withModel(callable $fn): void
    {
        self::withInputs(function (string $dir) use ($fn): void {
            $model = $dir . '/model.dat';
            self::train($dir, $model, 0.01);
            $fn($dir, $model);
        });
    }

    /**
     * A `null` lambda trains with 1.0, the L2 coefficient of MeCab's
     * `-c 1.0` (#1261): the model is byte-identical to one trained with 1.0.
     */
    public function testTrainWithNullLambdaUsesOne(): void
    {
        self::withInputs(function (string $dir): void {
            self::train($dir, $dir . '/default.dat', null);
            self::train($dir, $dir . '/explicit.dat', 1.0);

            $this->assertGreaterThan(0, filesize($dir . '/default.dat'));
            $this->assertSame(
                file_get_contents($dir . '/default.dat'),
                file_get_contents($dir . '/explicit.dat'),
            );
        });
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

    /**
     * The training files of the repository, on which every setting of
     * testTrainTakesTheRegularizationTypeTheRatioAndEta trains a different
     * model: the default converges before 100 iterations, so `eta: 0`
     * changes where it stops (#1262).
     */
    private const TRAINING = __DIR__ . '/../../resources/training';

    /**
     * A path that does not exist: the settings are checked before any file.
     */
    private const MISSING = '/nonexistent/issue1262';

    /**
     * Trains a model from TRAINING into `$dir/$name` with at most 100
     * iterations, 1 thread and the named `$settings`, and returns it.
     *
     * @param array<string, mixed> $settings
     */
    private static function trainResources(string $dir, string $name, array $settings = []): string
    {
        $model = $dir . '/' . $name;
        Lindera\Trainer::train(
            self::TRAINING . '/seed.csv',
            self::TRAINING . '/corpus.txt',
            self::TRAINING . '/char.def',
            self::TRAINING . '/unk.def',
            self::TRAINING . '/feature.def',
            self::TRAINING . '/rewrite.def',
            $model,
            ...['max_iter' => 100, 'max_threads' => 1, ...$settings],
        );
        return (string) file_get_contents($model);
    }

    /**
     * A zero effective L1 coefficient trains instead of panicking in OWL-QN.
     */
    public function testTrainWithZeroL1Coefficient(): void
    {
        self::withInputs(function (string $dir): void {
            $cases = [['l1', 0.0, 0.5], ['elasticnet', 0.0, 0.5], ['elasticnet', 1.0, 0.0]];
            foreach ($cases as $i => [$kind, $lambda, $ratio]) {
                $model = self::trainResources($dir, "zero-l1-{$i}.dat", [
                    'regularization' => $kind,
                    'lambda' => $lambda,
                    'elastic_net_l1_ratio' => $ratio,
                ]);
                $this->assertNotEmpty($model);
            }
        });
    }

    /**
     * `$regularization`, `$elastic_net_l1_ratio` and `$eta` reach the
     * trainer, with the CLI's defaults: giving the defaults trains the
     * default model, a ratio is ignored without `elasticnet`, and each other
     * setting trains a different model.
     */
    public function testTrainTakesTheRegularizationTypeTheRatioAndEta(): void
    {
        self::withInputs(function (string $dir): void {
            $default = self::trainResources($dir, 'default.dat');
            $this->assertSame($default, self::trainResources($dir, 'explicit.dat', [
                'regularization' => 'l2',
                'elastic_net_l1_ratio' => 0.5,
                'eta' => 5e-5,
            ]));
            $this->assertSame($default, self::trainResources($dir, 'l2-ratio.dat', [
                'regularization' => 'l2',
                'elastic_net_l1_ratio' => 0.2,
            ]));
            $models = [
                'default' => $default,
                'l1' => self::trainResources($dir, 'l1.dat', ['regularization' => 'l1']),
                'elasticnet 0.2' => self::trainResources($dir, 'en02.dat', [
                    'regularization' => 'elasticnet',
                    'elastic_net_l1_ratio' => 0.2,
                ]),
                'elasticnet 0.8' => self::trainResources($dir, 'en08.dat', [
                    'regularization' => 'elasticnet',
                    'elastic_net_l1_ratio' => 0.8,
                ]),
                'eta 0' => self::trainResources($dir, 'eta0.dat', ['eta' => 0.0]),
            ];
            $counts = array_count_values(array_map('md5', $models));
            $same = array_keys(array_filter($models, static fn(string $model): bool => $counts[md5($model)] > 1));
            $this->assertSame([], $same, 'these settings trained the same model');
        });
    }

    /**
     * Calls `train()` with the missing input files and the named `$settings`.
     *
     * @param array<string, mixed> $settings
     */
    private static function trainMissing(array $settings): void
    {
        $missing = self::MISSING;
        Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, ...$settings);
    }

    /**
     * A setting the trainer would reject when training starts throws a
     * `ValueError` that names it, before any file is read.
     */
    public function testTrainRejectsASettingBeforeReadingAnyFile(): void
    {
        $cases = [
            [['regularization' => 'l3'], "regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'"],
            [['lambda' => -1.0], 'lambda must be a finite number >= 0, got -1'],
            [['lambda' => NAN], 'lambda must be a finite number >= 0, got NaN'],
            [['lambda' => INF], 'lambda must be a finite number >= 0, got Infinity'],
            [['eta' => -1.0], 'eta must be a finite number >= 0, got -1'],
            [['eta' => NAN], 'eta must be a finite number >= 0, got NaN'],
            [['eta' => INF], 'eta must be a finite number >= 0, got Infinity'],
            [['elastic_net_l1_ratio' => -0.1], 'elastic_net_l1_ratio must be a number between 0 and 1, got -0.1'],
            [['elastic_net_l1_ratio' => 1.5], 'elastic_net_l1_ratio must be a number between 0 and 1, got 1.5'],
            [['elastic_net_l1_ratio' => NAN], 'elastic_net_l1_ratio must be a number between 0 and 1, got NaN'],
        ];
        foreach ($cases as [$settings, $message]) {
            try {
                self::trainMissing($settings);
                $this->fail('accepted ' . var_export($settings, true));
            } catch (ValueError $e) {
                $this->assertSame($message, $e->getMessage());
            }
        }
    }

    /**
     * The bounds and the other spellings pass the checks, so the missing
     * seed file is what fails.
     */
    public function testTrainAcceptsTheBoundsOfEachSetting(): void
    {
        foreach ([
            ['lambda' => 0.0],
            ['eta' => 0.0],
            ['elastic_net_l1_ratio' => 0.0],
            ['elastic_net_l1_ratio' => 1.0],
            ['regularization' => 'ElasticNet'],
            ['regularization' => 'elastic-net'],
        ] as $settings) {
            try {
                self::trainMissing($settings);
                $this->fail('trained from missing files with ' . var_export($settings, true));
            } catch (ValueError $e) {
                $this->assertStringContainsString('seed file does not exist', $e->getMessage());
            }
        }
    }

    /**
     * A child that makes its standard output non-blocking, fills it until a
     * write fails with EAGAIN, and then trains from the directory and into
     * the file it is given. It exits with 3 if 64 MiB went through without
     * EAGAIN, so that a stream that blocks cannot pass the test.
     */
    private const FULL_STDOUT_CHILD = <<<'PHP'
        [, $training, $output] = $argv;
        stream_set_blocking(STDOUT, false);
        $chunk = str_repeat('x', 65536);
        $filled = false;
        for ($i = 0; $i < 1024 && !$filled; $i++) {
            $written = @fwrite(STDOUT, $chunk);
            $filled = $written === false || $written === 0;
        }
        if (!$filled) {
            exit(3);
        }
        Lindera\Trainer::train(
            "$training/seed.csv",
            "$training/corpus.txt",
            "$training/char.def",
            "$training/unk.def",
            "$training/feature.def",
            "$training/rewrite.def",
            $output,
            max_iter: 5,
            max_threads: 1,
        );
        PHP;

    /**
     * The training and the binding write their progress to standard output
     * without panicking when it is full and non-blocking: they wait until
     * it takes the lines, as on a blocking stream (#1312). This process
     * reads nothing for 3 seconds while the child trains. The child loads
     * the extension this process loaded, found in `/proc/self/maps`.
     */
    public function testTrainWaitsForAFullStdout(): void
    {
        $maps = @file_get_contents('/proc/self/maps');
        if ($maps === false || preg_match('/\s(\/\S*liblindera_php\.so)$/m', $maps, $match) !== 1) {
            $this->markTestSkipped('needs /proc/self/maps to find the extension');
        }
        self::withInputs(function (string $dir) use ($match): void {
            $output = $dir . '/model.dat';
            $command = [
                PHP_BINARY, '-n', '-d', 'extension=' . $match[1],
                '-r', self::FULL_STDOUT_CHILD, '--', self::TRAINING, $output,
            ];
            $process = proc_open($command, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes);
            $this->assertIsResource($process);
            sleep(3);
            $stdout = (string) stream_get_contents($pipes[1]);
            $stderr = (string) stream_get_contents($pipes[2]);
            fclose($pipes[1]);
            fclose($pipes[2]);
            $status = proc_close($process);
            $report = 'stdout: ' . substr($stdout, -2000) . "\nstderr: " . substr($stderr, -2000);

            $this->assertSame(0, $status, $report);
            // The binding's first and last lines, right after the bytes that
            // filled the stream.
            $this->assertMatchesRegularExpression('/x+Training with \d+ examples\.\.\.\n/', $stdout, $report);
            $this->assertStringEndsWith("Model saved to $output\n", $stdout, $report);
            $this->assertFileExists($output);
        });
    }
}
