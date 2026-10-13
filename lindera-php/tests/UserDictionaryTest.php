<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Loading a user dictionary for a loaded system dictionary
 * (`Lindera\Dictionary::loadUserFor`).
 */
class UserDictionaryTest extends TestCase
{
    /**
     * Line 1 lies inside IPADIC's 1316 x 1316 connection matrix; line 2 has
     * the context IDs 60000, far outside it.
     */
    private const OUT_OF_RANGE_CSV
        = "東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー\n"
        . "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n";

    private const SIMPLE_CSV = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n";

    /**
     * Writes `$content` to `userdic.csv` in a new temporary directory, passes
     * its path to `$fn` and removes the directory afterwards.
     */
    private static function withCsv(string $content, callable $fn): mixed
    {
        $dir = sys_get_temp_dir() . '/lindera-userdic-' . bin2hex(random_bytes(8));
        mkdir($dir);
        $csv = $dir . '/userdic.csv';
        try {
            file_put_contents($csv, $content);
            return $fn($csv);
        } finally {
            @unlink($csv);
            rmdir($dir);
        }
    }

    /**
     * Returns the surfaces of the tokens of `$text`.
     *
     * @return list<string>
     */
    private static function surfaces(Lindera\Tokenizer $tokenizer, string $text): array
    {
        return array_map(static fn($token) => $token->surface, $tokenizer->tokenize($text));
    }

    public function testLoadUserForNamesTheCsvLineAndSurface(): void
    {
        $dictionary = Lindera\Dictionary::load('embedded://ipadic');
        $message = self::withCsv(self::OUT_OF_RANGE_CSV, static function (string $csv) use ($dictionary): string {
            try {
                Lindera\Dictionary::loadUserFor($csv, $dictionary);
            } catch (\ValueError $e) {
                return $e->getMessage();
            }
            self::fail('loaded a user dictionary with a row outside the matrix');
        });

        $this->assertStringContainsString("line 2 (surface: '東京スカイツリー')", $message);
        $this->assertStringContainsString('left context id 60000 is out of range', $message);
    }

    public function testLoadUserForLoadsACsvInsideTheMatrix(): void
    {
        $dictionary = Lindera\Dictionary::load('embedded://ipadic');
        $baseline = self::surfaces(new Lindera\Tokenizer($dictionary, 'normal'), '東京スカイツリー');
        $this->assertGreaterThan(1, count($baseline), 'fixture assumption: ' . implode(' / ', $baseline));

        $userDictionary = self::withCsv(
            self::SIMPLE_CSV,
            static fn(string $csv) => Lindera\Dictionary::loadUserFor($csv, $dictionary),
        );
        $tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDictionary);
        $this->assertSame(['東京スカイツリー'], self::surfaces($tokenizer, '東京スカイツリー'));
    }
}
