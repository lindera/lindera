<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * The nesting limit on arguments converted to JSON (lindera/lindera#1062).
 *
 * Without the limit, an array nested a few thousand levels deep overflowed
 * the native stack and crashed PHP.
 */
class ArgumentDepthTest extends TestCase
{
    private const TOO_DEEP = 'nested more than 128 levels deep';

    /**
     * The three builder methods that convert a PHP value to JSON.
     *
     * @return array<string, array{callable(Lindera\TokenizerBuilder, mixed): void}>
     */
    public static function methods(): array
    {
        return [
            'setSpacePenalty' => [
                static fn (Lindera\TokenizerBuilder $builder, mixed $value) => $builder->setSpacePenalty($value),
            ],
            'appendCharacterFilter' => [
                static fn (Lindera\TokenizerBuilder $builder, mixed $value) => $builder->appendCharacterFilter('unicode_normalize', $value),
            ],
            'appendTokenFilter' => [
                static fn (Lindera\TokenizerBuilder $builder, mixed $value) => $builder->appendTokenFilter('lowercase', $value),
            ],
        ];
    }

    /** Returns ['a' => ['a' => ...]] made of `$levels` nested associative arrays. */
    private static function nestedMaps(int $levels): array
    {
        $value = [];
        for ($i = 1; $i < $levels; $i++) {
            $value = ['a' => $value];
        }
        return $value;
    }

    /** Returns [[...]] made of `$levels` nested lists. */
    private static function nestedLists(int $levels): array
    {
        $value = [];
        for ($i = 1; $i < $levels; $i++) {
            $value = [$value];
        }
        return $value;
    }

    #[DataProvider('methods')]
    public function testAccepts128Levels(callable $call): void
    {
        foreach ([self::nestedMaps(128), self::nestedLists(128)] as $value) {
            try {
                $call(new Lindera\TokenizerBuilder(), $value);
            } catch (\ValueError $e) {
                // setSpacePenalty rejects the shape, but not for being too deep.
                $this->assertStringNotContainsString(self::TOO_DEEP, $e->getMessage());
            }
        }
        $this->addToAssertionCount(1);
    }

    #[DataProvider('methods')]
    public function testRejectsDeeperValuesWithAValueError(callable $call): void
    {
        foreach ([self::nestedMaps(129), self::nestedLists(129), self::nestedMaps(10_000)] as $value) {
            try {
                $call(new Lindera\TokenizerBuilder(), $value);
                $this->fail('a value nested more than 128 levels deep was accepted');
            } catch (\ValueError $e) {
                // setSpacePenalty keeps the reason instead of its generic message.
                $this->assertStringContainsString(self::TOO_DEEP, $e->getMessage());
            }
        }
    }

    #[DataProvider('methods')]
    public function testBuilderStaysUsableAfterRejectingAnArgument(callable $call): void
    {
        $builder = new Lindera\TokenizerBuilder();
        try {
            $call($builder, self::nestedMaps(129));
            $this->fail('a value nested more than 128 levels deep was accepted');
        } catch (\ValueError $e) {
            $this->assertStringContainsString(self::TOO_DEEP, $e->getMessage());
        }
        $call($builder, null);
        $this->addToAssertionCount(1);
    }

    public function testAReferenceCycleIsStillAnUnsupportedValue(): void
    {
        // PHP needs a reference to build a cycle, and the converter rejects
        // references before any depth is reached.
        $value = ['rules' => []];
        $value['self'] = &$value;
        $this->expectException(\Exception::class);
        $this->expectExceptionMessage('Unsupported PHP value type');
        (new Lindera\TokenizerBuilder())->appendTokenFilter('lowercase', $value);
    }
}
