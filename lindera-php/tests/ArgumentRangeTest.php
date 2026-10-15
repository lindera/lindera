<?php

declare(strict_types=1);

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * Integer parameters reject a value outside the range of the setting they
 * set (lindera/lindera#1209).
 *
 * PHP passes every `int` as a 64-bit integer, and the core keeps most of
 * these settings in a narrower type, such as the 16-bit default word cost or
 * the unsigned number of N-best results. A value the setting cannot hold
 * throws a `ValueError` that names the parameter and the accepted range,
 * instead of wrapping into that type (100000 used to become -31072).
 */
class ArgumentRangeTest extends TestCase
{
    /** The largest `usize` and `u64`, which no PHP int exceeds. */
    private const UNSIGNED_MAX = '18446744073709551615';

    /** A path that does not exist: Trainer::train rejects it after checking its integers. */
    private const MISSING = '/nonexistent/issue1209';

    /**
     * Integer parameters whose whole PHP range is valid, so they need no case.
     */
    private const FULL_RANGE = [
        'Tokenizer::tokenizeNbest($cost_threshold)',
    ];

    private static ?Lindera\Dictionary $dictionary = null;

    private static function dictionary(): Lindera\Dictionary
    {
        return self::$dictionary ??= Lindera\Dictionary::load('embedded://ipadic');
    }

    /**
     * A case per integer parameter, keyed `Class::method($parameter)`: a call
     * that passes the value it is given to that parameter, and the smallest
     * and largest value the parameter accepts.
     *
     * @return array<string, array{callable(int): mixed, int, int|string}>
     */
    private static function cases(): array
    {
        $unsigned = self::UNSIGNED_MAX;
        $cases = [
            'Metadata::__construct($default_word_cost)' => [static fn (int $v) => new Lindera\Metadata(default_word_cost: $v), -32768, 32767],
            'Metadata::__construct($default_left_context_id)' => [static fn (int $v) => new Lindera\Metadata(default_left_context_id: $v), 0, 65535],
            'Metadata::__construct($default_right_context_id)' => [static fn (int $v) => new Lindera\Metadata(default_right_context_id: $v), 0, 65535],
            'Penalty::__construct($kanji_penalty_length_threshold)' => [static fn (int $v) => new Lindera\Penalty(kanji_penalty_length_threshold: $v), 0, $unsigned],
            'Penalty::__construct($kanji_penalty_length_penalty)' => [static fn (int $v) => new Lindera\Penalty(kanji_penalty_length_penalty: $v), -2147483648, 2147483647],
            'Penalty::__construct($other_penalty_length_threshold)' => [static fn (int $v) => new Lindera\Penalty(other_penalty_length_threshold: $v), 0, $unsigned],
            'Penalty::__construct($other_penalty_length_penalty)' => [static fn (int $v) => new Lindera\Penalty(other_penalty_length_penalty: $v), -2147483648, 2147483647],
            'FieldDefinition::__construct($index)' => [static fn (int $v) => new Lindera\FieldDefinition($v, 'surface', 'surface'), 0, $unsigned],
            'Schema::getFieldName($index)' => [static fn (int $v) => Lindera\Schema::createDefault()->getFieldName($v), 0, $unsigned],
            'Token::getDetail($index)' => [static fn (int $v) => (new Lindera\Tokenizer(self::dictionary()))->tokenize('東京')[0]->getDetail($v), 0, $unsigned],
            'Tokenizer::tokenizeNbest($n)' => [static fn (int $v) => (new Lindera\Tokenizer(self::dictionary()))->tokenizeNbest('東京', $v), 0, $unsigned],
        ];
        if (class_exists(Lindera\Trainer::class)) {
            $missing = self::MISSING;
            $cases += [
                'Trainer::train($max_iter)' => [static fn (int $v) => Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, max_iter: $v), 0, $unsigned],
                'Trainer::train($max_threads)' => [static fn (int $v) => Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, max_threads: $v), 0, $unsigned],
            ];
        }
        return $cases;
    }

    /**
     * Every `int` parameter of the extension, keyed as in cases().
     *
     * @return list<string>
     */
    private static function intParameters(): array
    {
        $parameters = [];
        $extension = new ReflectionExtension('lindera');
        $functions = array_values($extension->getFunctions());
        foreach ($extension->getClasses() as $class) {
            foreach ($class->getMethods() as $method) {
                $functions[] = $method;
            }
        }
        foreach ($functions as $function) {
            $owner = $function instanceof ReflectionMethod
                ? $function->getDeclaringClass()->getShortName() . '::'
                : '';
            foreach ($function->getParameters() as $parameter) {
                $type = $parameter->getType();
                if ($type instanceof ReflectionNamedType && $type->getName() === 'int') {
                    $parameters[] = "{$owner}{$function->getName()}(\${$parameter->getName()})";
                }
            }
        }
        return $parameters;
    }

    /**
     * @return array<string, array{string}>
     */
    public static function parameters(): array
    {
        $keys = array_keys(self::cases());
        return array_combine($keys, array_map(static fn (string $key) => [$key], $keys));
    }

    private static function parameterName(string $key): string
    {
        preg_match('/\(\$(\w+)\)$/', $key, $match);
        return $match[1];
    }

    /**
     * Calls the case with a value and returns what it threw, or null.
     */
    private static function thrown(string $key, int $value): ?\Throwable
    {
        try {
            self::cases()[$key][0]($value);
            return null;
        } catch (\Throwable $e) {
            return $e;
        }
    }

    private function assertRejected(string $key, int $value): void
    {
        [, $min, $max] = self::cases()[$key];
        $error = self::thrown($key, $value);
        $this->assertNotNull($error, "{$key} accepted {$value}");
        $this->assertSame(ValueError::class, $error::class, $error->getMessage());
        $this->assertSame(
            self::parameterName($key) . " must be an integer between {$min} and {$max}, got {$value}",
            $error->getMessage()
        );
    }

    public function testEveryIntParameterHasACase(): void
    {
        $expected = array_values(array_diff(self::intParameters(), self::FULL_RANGE));
        $actual = array_keys(self::cases());
        sort($expected);
        sort($actual);
        $this->assertSame(
            $expected,
            $actual,
            'add a case to ArgumentRangeTest::cases() for every int parameter, or list it in FULL_RANGE'
        );
    }

    #[DataProvider('parameters')]
    public function testAValueBelowTheRangeThrows(string $key): void
    {
        $this->assertRejected($key, self::cases()[$key][1] - 1);
    }

    #[DataProvider('parameters')]
    public function testAValueAboveTheRangeThrows(string $key): void
    {
        $max = self::cases()[$key][2];
        if (!is_int($max)) {
            $this->markTestSkipped("{$key} accepts every non-negative PHP int");
        }
        $this->assertRejected($key, $max + 1);
    }

    #[DataProvider('parameters')]
    public function testTheBoundsAreAccepted(string $key): void
    {
        [, $min, $max] = self::cases()[$key];
        foreach ([$min, is_int($max) ? $max : PHP_INT_MAX] as $value) {
            $error = self::thrown($key, $value);
            // Trainer::train still throws for its missing files, after taking
            // the value.
            $this->assertFalse(
                $error !== null && str_contains($error->getMessage(), 'must be an integer between'),
                "{$key} rejected {$value}: " . $error?->getMessage()
            );
        }
    }

    public function testTheValueIsKeptUnchanged(): void
    {
        $metadata = new Lindera\Metadata(default_word_cost: -32768, default_left_context_id: 65535);
        $this->assertSame(-32768, $metadata->default_word_cost);
        $this->assertSame(65535, $metadata->default_left_context_id);
        $this->assertSame(PHP_INT_MAX, (new Lindera\FieldDefinition(PHP_INT_MAX, 'x', 'custom'))->index);
        $this->assertNull(Lindera\Schema::createDefault()->getFieldName(PHP_INT_MAX));
    }
}
