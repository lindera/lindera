<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * The penalty of decompose mode: TokenizerBuilder::setMode() and the
 * Tokenizer constructor take a Lindera\Penalty (lindera/lindera#1215).
 */
class DecomposePenaltyTest extends TestCase
{
    private const TEXT = '関西国際空港限定トートバッグ';

    private const DEFAULT_SURFACES = ['関西', '国際', '空港', '限定', 'トートバッグ'];

    // Without length penalties, decompose mode keeps the compound whole, as
    // normal mode does.
    private const NO_LENGTH_PENALTY_SURFACES = ['関西国際空港', '限定', 'トートバッグ'];

    private static function noLengthPenalty(): Lindera\Penalty
    {
        return new Lindera\Penalty(kanji_penalty_length_penalty: 0, other_penalty_length_penalty: 0);
    }

    /**
     * @param list<Lindera\Token> $tokens
     * @return list<string>
     */
    private static function surfaces(array $tokens): array
    {
        return array_map(static fn (Lindera\Token $token): string => $token->surface, $tokens);
    }

    /**
     * @return list<string>
     */
    private static function builderSurfaces(?Lindera\Penalty $penalty): array
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $builder->setMode('decompose', $penalty);
        return self::surfaces($builder->build()->tokenize(self::TEXT));
    }

    /**
     * @return list<string>
     */
    private static function constructorSurfaces(?Lindera\Penalty $penalty): array
    {
        $dictionary = Lindera\Dictionary::load('embedded://ipadic');
        $tokenizer = new Lindera\Tokenizer($dictionary, 'decompose', penalty: $penalty);
        return self::surfaces($tokenizer->tokenize(self::TEXT));
    }

    /**
     * @return array<string, array{?Lindera\Penalty, list<string>}>
     */
    public static function penalties(): array
    {
        return [
            'null' => [null, self::DEFAULT_SURFACES],
            'default penalty' => [new Lindera\Penalty(), self::DEFAULT_SURFACES],
            'no length penalty' => [self::noLengthPenalty(), self::NO_LENGTH_PENALTY_SURFACES],
        ];
    }

    /**
     * @param list<string> $expected
     */
    #[DataProvider('penalties')]
    public function testSetModePenaltyReachesTheSegmenter(?Lindera\Penalty $penalty, array $expected): void
    {
        $this->assertSame($expected, self::builderSurfaces($penalty));
    }

    /**
     * @param list<string> $expected
     */
    #[DataProvider('penalties')]
    public function testConstructorPenaltyReachesTheSegmenter(?Lindera\Penalty $penalty, array $expected): void
    {
        $this->assertSame($expected, self::constructorSurfaces($penalty));
    }

    public function testSetModeWithoutAPenaltyUsesTheDefault(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $builder->setMode('decompose');
        $this->assertSame(self::DEFAULT_SURFACES, self::surfaces($builder->build()->tokenize(self::TEXT)));
    }

    public function testSetModeRejectsAPenaltyWithNormalMode(): void
    {
        $this->expectException(ValueError::class);
        $this->expectExceptionMessage('decompose');
        (new Lindera\TokenizerBuilder())->setMode('normal', self::noLengthPenalty());
    }

    public function testConstructorRejectsAPenaltyWithNormalMode(): void
    {
        $this->expectException(ValueError::class);
        $this->expectExceptionMessage('decompose');
        new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ipadic'), penalty: self::noLengthPenalty());
    }
}
