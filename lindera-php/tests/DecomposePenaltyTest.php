<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * The penalty of decompose mode: TokenizerBuilder::setMode() and the
 * Tokenizer constructor take a Lindera\Mode, which holds the penalty
 * (lindera/lindera#1215, #1218).
 */
class DecomposePenaltyTest extends TestCase
{
    private const TEXT = '関西国際空港限定トートバッグ';

    private const NORMAL_SURFACES = ['関西国際空港', '限定', 'トートバッグ'];

    private const DEFAULT_SURFACES = ['関西', '国際', '空港', '限定', 'トートバッグ'];

    // Without length penalties, decompose mode keeps the compound whole, as
    // normal mode does.
    private const NO_LENGTH_PENALTY_SURFACES = self::NORMAL_SURFACES;

    // With a katakana threshold of 4, the output differs from both normal
    // mode's and the default penalty's, so a decompose mode that loses its
    // penalty, or is read as normal mode, fails.
    private const SHORT_KATAKANA_SURFACES = ['関西', '国際', '空港', '限定', 'トー', 'トバッグ'];

    private static function noLengthPenalty(): Lindera\Penalty
    {
        return new Lindera\Penalty(kanji_penalty_length_penalty: 0, other_penalty_length_penalty: 0);
    }

    private static function shortKatakanaThreshold(): Lindera\Penalty
    {
        return new Lindera\Penalty(other_penalty_length_threshold: 4, other_penalty_length_penalty: 3000);
    }

    /**
     * @param list<Lindera\Token> $tokens
     * @return list<string>
     */
    private static function surfaces(array $tokens): array
    {
        return array_map(static fn(Lindera\Token $token): string => $token->surface, $tokens);
    }

    /**
     * @return list<string>
     */
    private static function builderSurfaces(Lindera\Mode $mode): array
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $builder->setMode($mode);
        return self::surfaces($builder->build()->tokenize(self::TEXT));
    }

    /**
     * @return list<string>
     */
    private static function constructorSurfaces(mixed ...$args): array
    {
        $dictionary = Lindera\Dictionary::load('embedded://ipadic');
        $tokenizer = new Lindera\Tokenizer($dictionary, ...$args);
        return self::surfaces($tokenizer->tokenize(self::TEXT));
    }

    /**
     * @return array<string, array{Lindera\Mode, list<string>}>
     */
    public static function modes(): array
    {
        return [
            'normal' => [Lindera\Mode::normal(), self::NORMAL_SURFACES],
            'decompose' => [Lindera\Mode::decompose(), self::DEFAULT_SURFACES],
            'decompose with null' => [Lindera\Mode::decompose(null), self::DEFAULT_SURFACES],
            'decompose with the default penalty' => [Lindera\Mode::decompose(new Lindera\Penalty()), self::DEFAULT_SURFACES],
            'decompose without length penalties' => [Lindera\Mode::decompose(self::noLengthPenalty()), self::NO_LENGTH_PENALTY_SURFACES],
            'decompose with a short katakana threshold' => [Lindera\Mode::decompose(penalty: self::shortKatakanaThreshold()), self::SHORT_KATAKANA_SURFACES],
            'fromStr decompose' => [Lindera\Mode::fromStr('decompose'), self::DEFAULT_SURFACES],
        ];
    }

    /**
     * @param list<string> $expected
     */
    #[DataProvider('modes')]
    public function testSetModePenaltyReachesTheSegmenter(Lindera\Mode $mode, array $expected): void
    {
        $this->assertSame($expected, self::builderSurfaces($mode));
    }

    /**
     * @param list<string> $expected
     */
    #[DataProvider('modes')]
    public function testConstructorPenaltyReachesTheSegmenter(Lindera\Mode $mode, array $expected): void
    {
        $this->assertSame($expected, self::constructorSurfaces($mode));
        $this->assertSame($expected, self::constructorSurfaces(mode: $mode));
    }

    public function testOneModeServesSeveralTokenizers(): void
    {
        $mode = Lindera\Mode::decompose(self::shortKatakanaThreshold());
        $this->assertSame(self::SHORT_KATAKANA_SURFACES, self::builderSurfaces($mode));
        $this->assertSame(self::SHORT_KATAKANA_SURFACES, self::constructorSurfaces($mode));
        $this->assertSame(self::SHORT_KATAKANA_SURFACES, self::builderSurfaces($mode));
    }

    public function testConstructorDefaultsToNormalMode(): void
    {
        $this->assertSame(self::NORMAL_SURFACES, self::constructorSurfaces());
        $this->assertSame(self::NORMAL_SURFACES, self::constructorSurfaces(null));
        $this->assertSame(self::NORMAL_SURFACES, self::constructorSurfaces(mode: null));
    }

    /**
     * @return array<string, array{mixed}>
     */
    public static function valuesThatAreNotAMode(): array
    {
        return [
            'decompose' => ['decompose'],
            'normal' => ['normal'],
            'int' => [0],
            'stdClass' => [new stdClass()],
            'Penalty' => [new Lindera\Penalty()],
        ];
    }

    #[DataProvider('valuesThatAreNotAMode')]
    public function testSetModeRejectsAValueThatIsNotAMode(mixed $value): void
    {
        $this->expectException(TypeError::class);
        (new Lindera\TokenizerBuilder())->setMode($value);
    }

    public function testSetModeRejectsNull(): void
    {
        $this->expectException(TypeError::class);
        (new Lindera\TokenizerBuilder())->setMode(null);
    }

    #[DataProvider('valuesThatAreNotAMode')]
    public function testConstructorRejectsAValueThatIsNotAMode(mixed $value): void
    {
        $this->expectException(TypeError::class);
        new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ipadic'), $value);
    }

    public function testSetModeTakesNoPenalty(): void
    {
        // The penalty goes in the Mode (#1218).
        $this->expectException(ArgumentCountError::class);
        (new Lindera\TokenizerBuilder())->setMode(Lindera\Mode::decompose(), self::noLengthPenalty());
    }

    public function testConstructorTakesNoPenalty(): void
    {
        $this->expectException(Error::class);
        $this->expectExceptionMessage('penalty');
        new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ipadic'), penalty: self::noLengthPenalty());
    }
}
