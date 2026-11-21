<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * The batch methods of Tokenizer (lindera/lindera#1402): every text is
 * tokenized as a call of its own, on several threads, and the results come in
 * the order of the texts, the same as the single calls'.
 */
class TokenizeBatchTest extends TestCase
{
    private static function tokenizer(): Lindera\Tokenizer
    {
        return new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ipadic'));
    }

    /**
     * The first 60 lines of resources/bocchan.txt (about 50 KB, enough input
     * for a batch on several threads; all of it would take more than PHP's
     * default memory limit as token arrays) and an empty text.
     *
     * @return list<string>
     */
    private static function texts(): array
    {
        $text = file_get_contents(__DIR__ . '/../../resources/bocchan.txt');
        self::assertIsString($text);
        return [...array_slice(explode("\n", $text), 0, 60), ''];
    }

    /**
     * @param list<Lindera\Token> $tokens
     * @return list<array<string, mixed>>
     */
    private static function fields(array $tokens): array
    {
        return array_map(static fn(Lindera\Token $t) => $t->toArray(), $tokens);
    }

    /**
     * @return array<string, array{?int}>
     */
    public static function threadCounts(): array
    {
        return ['default' => [null], 'one' => [1], 'two' => [2], 'eight' => [8]];
    }

    #[DataProvider('threadCounts')]
    public function testTokenizeBatchMatchesTokenize(?int $maxThreads): void
    {
        $tokenizer = self::tokenizer();
        $texts = self::texts();
        $expected = array_map(static fn(string $text) => self::fields($tokenizer->tokenize($text)), $texts);
        $batch = $tokenizer->tokenizeBatch($texts, $maxThreads);
        $this->assertSame($expected, array_map(static fn(array $tokens) => self::fields($tokens), $batch));
    }

    public function testTokenizeSurfacesBatchMatchesTokenizeSurfaces(): void
    {
        $tokenizer = self::tokenizer();
        $texts = self::texts();
        $expected = array_map(static fn(string $text) => $tokenizer->tokenizeSurfaces($text), $texts);
        $this->assertSame($expected, $tokenizer->tokenizeSurfacesBatch($texts, 4));
        $this->assertSame($expected, $tokenizer->tokenizeSurfacesBatch($texts));
    }

    public function testTokenizeNbestBatchMatchesTokenizeNbest(): void
    {
        $tokenizer = self::tokenizer();
        $texts = array_slice(self::texts(), 0, 60);
        $results = static fn(array $nbest) => array_map(
            static fn(Lindera\NbestResult $r) => [self::fields($r->tokens), $r->cost],
            $nbest
        );
        $expected = array_map(static fn(string $text) => $results($tokenizer->tokenizeNbest($text, 3, true, 5000)), $texts);
        foreach ([null, 4] as $maxThreads) {
            $batch = $tokenizer->tokenizeNbestBatch($texts, 3, true, 5000, $maxThreads);
            $this->assertSame($expected, array_map($results, $batch), 'max_threads ' . var_export($maxThreads, true));
        }
    }

    public function testTokenizeBatchTakesAnEmptyArray(): void
    {
        $this->assertSame([], self::tokenizer()->tokenizeBatch([]));
    }

    public function testTokenizeBatchRejectsAString(): void
    {
        $this->expectException(TypeError::class);
        self::tokenizer()->tokenizeBatch('東京');
    }
}
