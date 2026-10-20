<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * Lindera\Mode, the counterpart of the Rust lindera::mode::Mode
 * (lindera/lindera#1218).
 *
 * Equality is checked with `==` itself: PHPUnit's assertEquals() compares
 * an object's array cast, which does not hold the getter properties.
 */
class ModeTest extends TestCase
{
    private static function shortKatakanaThreshold(): Lindera\Penalty
    {
        return new Lindera\Penalty(other_penalty_length_threshold: 4, other_penalty_length_penalty: 3000);
    }

    private function assertDefaultPenalty(?Lindera\Penalty $penalty): void
    {
        $this->assertNotNull($penalty);
        $this->assertSame(2, $penalty->kanji_penalty_length_threshold);
        $this->assertSame(3000, $penalty->kanji_penalty_length_penalty);
        $this->assertSame(7, $penalty->other_penalty_length_threshold);
        $this->assertSame(1700, $penalty->other_penalty_length_penalty);
    }

    public function testNormal(): void
    {
        $mode = Lindera\Mode::normal();
        $this->assertSame('normal', $mode->name);
        $this->assertSame('normal', (string) $mode);
        $this->assertTrue($mode->isNormal());
        $this->assertFalse($mode->isDecompose());
        $this->assertNull($mode->penalty);
    }

    public function testDecomposeWithTheDefaultPenalty(): void
    {
        foreach ([Lindera\Mode::decompose(), Lindera\Mode::decompose(null)] as $mode) {
            $this->assertSame('decompose', $mode->name);
            $this->assertSame('decompose', (string) $mode);
            $this->assertTrue($mode->isDecompose());
            $this->assertFalse($mode->isNormal());
            $this->assertDefaultPenalty($mode->penalty);
        }
    }

    public function testDecomposeKeepsThePenalty(): void
    {
        $penalty = Lindera\Mode::decompose(self::shortKatakanaThreshold())->penalty;
        $this->assertNotNull($penalty);
        $this->assertSame(2, $penalty->kanji_penalty_length_threshold);
        $this->assertSame(3000, $penalty->kanji_penalty_length_penalty);
        $this->assertSame(4, $penalty->other_penalty_length_threshold);
        $this->assertSame(3000, $penalty->other_penalty_length_penalty);
    }

    public function testFromStr(): void
    {
        $this->assertTrue(Lindera\Mode::fromStr('normal') == Lindera\Mode::normal());
        $this->assertTrue(Lindera\Mode::fromStr('decompose') == Lindera\Mode::decompose());
    }

    /**
     * @return array<string, array{string}>
     */
    public static function otherNames(): array
    {
        return [
            'Normal' => ['Normal'],
            'Decompose' => ['Decompose'],
            'DECOMPOSE' => ['DECOMPOSE'],
            'empty' => [''],
            'invalid' => ['invalid'],
        ];
    }

    /**
     * Case sensitive, as the Rust Mode parses; the ValueError that
     * setMode('invalid') threw in v6.
     */
    #[DataProvider('otherNames')]
    public function testFromStrRejectsOtherNames(string $name): void
    {
        $this->expectException(ValueError::class);
        $this->expectExceptionMessage('Invalid mode');
        Lindera\Mode::fromStr($name);
    }

    public function testFromStrTakesAString(): void
    {
        $this->expectException(TypeError::class);
        Lindera\Mode::fromStr(0);
    }

    public function testDecomposeRejectsAValueThatIsNotAPenalty(): void
    {
        $this->expectException(TypeError::class);
        Lindera\Mode::decompose(Lindera\Mode::normal());
    }

    public function testEqualityComparesTheNameAndThePenalty(): void
    {
        $this->assertTrue(Lindera\Mode::normal() == Lindera\Mode::normal());
        $this->assertTrue(Lindera\Mode::decompose() == Lindera\Mode::decompose(new Lindera\Penalty()));
        $this->assertTrue(
            Lindera\Mode::decompose(self::shortKatakanaThreshold()) == Lindera\Mode::decompose(self::shortKatakanaThreshold())
        );
        $this->assertFalse(Lindera\Mode::decompose() == Lindera\Mode::decompose(self::shortKatakanaThreshold()));
        $this->assertFalse(Lindera\Mode::normal() == Lindera\Mode::decompose());
        $this->assertFalse(Lindera\Mode::normal() === Lindera\Mode::normal());
    }

    public function testAModeIsNotEqualToAnotherObject(): void
    {
        $this->assertFalse(Lindera\Mode::normal() == new Lindera\Penalty());
        $this->assertFalse(Lindera\Mode::normal() == new stdClass());
    }

    public function testAModeHasNoPublicConstructor(): void
    {
        $this->expectException(Throwable::class);
        $this->expectExceptionMessage('instantiate');
        new Lindera\Mode();
    }
}
