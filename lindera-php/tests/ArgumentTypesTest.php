<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Typed optional parameters reject a value of another type (lindera/lindera#1173).
 *
 * ext-php-rs before 0.16.3 read such a value as `null` for an `Option<T>`
 * parameter, so a mistyped argument used to select the default silently.
 * Since 0.16.3, the conversion of `Option<T>` fails for a value that `T`
 * rejects, and ext-php-rs throws the `TypeError` it throws for a required
 * parameter ("Invalid value given for argument"). lindera-php declares these
 * parameters as `Option<T>` and relies on that, so it requires ext-php-rs
 * 0.16.3 or later. `null` and an omitted argument still select the default.
 */
class ArgumentTypesTest extends TestCase
{
    /** A path that does not exist: the methods that read files reject it after taking their arguments. */
    private const MISSING = '/nonexistent/issue1173';

    private static ?Lindera\Dictionary $dictionary = null;

    private static function dictionary(): Lindera\Dictionary
    {
        return self::$dictionary ??= Lindera\Dictionary::load('embedded://ipadic');
    }

    /**
     * A call per typed optional parameter, keyed `Class::method($parameter)`,
     * that passes the value it is given to that parameter.
     *
     * @return array<string, callable(mixed): mixed>
     */
    private static function calls(): array
    {
        $missing = self::MISSING;
        $calls = [
            'Mode::decompose($penalty)' => static fn (mixed $v) => Lindera\Mode::decompose($v),
            'Penalty::__construct($kanji_penalty_length_threshold)' => static fn (mixed $v) => new Lindera\Penalty(kanji_penalty_length_threshold: $v),
            'Penalty::__construct($kanji_penalty_length_penalty)' => static fn (mixed $v) => new Lindera\Penalty(kanji_penalty_length_penalty: $v),
            'Penalty::__construct($other_penalty_length_threshold)' => static fn (mixed $v) => new Lindera\Penalty(other_penalty_length_threshold: $v),
            'Penalty::__construct($other_penalty_length_penalty)' => static fn (mixed $v) => new Lindera\Penalty(other_penalty_length_penalty: $v),
            'Metadata::__construct($name)' => static fn (mixed $v) => new Lindera\Metadata(name: $v),
            'Metadata::__construct($encoding)' => static fn (mixed $v) => new Lindera\Metadata(encoding: $v),
            'Metadata::__construct($default_word_cost)' => static fn (mixed $v) => new Lindera\Metadata(default_word_cost: $v),
            'Metadata::__construct($default_left_context_id)' => static fn (mixed $v) => new Lindera\Metadata(default_left_context_id: $v),
            'Metadata::__construct($default_right_context_id)' => static fn (mixed $v) => new Lindera\Metadata(default_right_context_id: $v),
            'Metadata::__construct($default_field_value)' => static fn (mixed $v) => new Lindera\Metadata(default_field_value: $v),
            'Metadata::__construct($flexible_csv)' => static fn (mixed $v) => new Lindera\Metadata(flexible_csv: $v),
            'Metadata::__construct($skip_invalid_cost_or_id)' => static fn (mixed $v) => new Lindera\Metadata(skip_invalid_cost_or_id: $v),
            'FieldDefinition::__construct($description)' => static fn (mixed $v) => new Lindera\FieldDefinition(0, 'surface', 'surface', $v),
            'Tokenizer::__construct($mode)' => static fn (mixed $v) => new Lindera\Tokenizer(self::dictionary(), mode: $v),
            'Tokenizer::__construct($user_dictionary)' => static fn (mixed $v) => new Lindera\Tokenizer(self::dictionary(), user_dictionary: $v),
            'Tokenizer::tokenizeNbest($unique)' => static fn (mixed $v) => (new Lindera\Tokenizer(self::dictionary()))->tokenizeNbest('東京都', 2, unique: $v),
            'Tokenizer::tokenizeNbest($cost_threshold)' => static fn (mixed $v) => (new Lindera\Tokenizer(self::dictionary()))->tokenizeNbest('東京都', 2, cost_threshold: $v),
            'Dictionary::buildUser($metadata)' => static fn (mixed $v) => Lindera\Dictionary::buildUser('ipadic', $missing, $missing, $v),
        ];
        if (class_exists(Lindera\Trainer::class)) {
            $calls += [
                'Trainer::train($lambda)' => static fn (mixed $v) => Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, lambda: $v),
                'Trainer::train($max_iter)' => static fn (mixed $v) => Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, max_iter: $v),
                'Trainer::train($max_threads)' => static fn (mixed $v) => Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, max_threads: $v),
                'Trainer::export($metadata)' => static fn (mixed $v) => Lindera\Trainer::export($missing, $missing, $v),
            ];
        }
        return $calls;
    }

    /**
     * Every optional parameter of the extension with a declared type other
     * than `mixed`, keyed as in calls().
     *
     * @return array<string, ReflectionNamedType>
     */
    private static function typedOptionalParameters(): array
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
                if (!$parameter->isOptional() || !$type instanceof ReflectionNamedType || $type->getName() === 'mixed') {
                    continue;
                }
                $parameters["{$owner}{$function->getName()}(\${$parameter->getName()})"] = $type;
            }
        }
        return $parameters;
    }

    /**
     * Values that a parameter of the given type must reject.
     *
     * @return list<mixed>
     */
    private static function wrongValues(ReflectionNamedType $type): array
    {
        return match ($type->getName()) {
            'int' => ['1', 1.5, true],
            'string' => [1, true, []],
            'bool' => [1, 'true'],
            // An int is not wrong for a float: PHP widens it.
            'float' => ['0.5', true],
            default => [
                'x',
                new stdClass(),
                $type->getName() === Lindera\Mode::class ? new Lindera\Penalty() : Lindera\Mode::normal(),
            ],
        };
    }

    private static function describe(mixed $value): string
    {
        return is_object($value) ? get_class($value) : var_export($value, true);
    }

    private static function parameterName(string $key): string
    {
        preg_match('/\(\$(\w+)\)$/', $key, $match);
        return $match[1];
    }

    public function testEveryTypedOptionalParameterHasACase(): void
    {
        $expected = array_keys(self::typedOptionalParameters());
        $actual = array_keys(self::calls());
        sort($expected);
        sort($actual);

        $this->assertSame(
            $expected,
            $actual,
            'add a case to ArgumentTypesTest::calls() for every typed optional parameter'
        );
    }

    public function testAWrongTypeThrowsTheExceptionOfARequiredParameter(): void
    {
        $calls = self::calls();
        $failures = [];
        foreach (self::typedOptionalParameters() as $key => $type) {
            if (!isset($calls[$key])) {
                continue; // reported by testEveryTypedOptionalParameterHasACase
            }
            $expected = 'Invalid value given for argument `' . self::parameterName($key) . '`';
            foreach (self::wrongValues($type) as $value) {
                $label = $key . ' <- ' . self::describe($value);
                try {
                    $calls[$key]($value);
                    $failures[] = "{$label}: accepted";
                } catch (\Throwable $e) {
                    // Exactly TypeError, as for a required parameter; not
                    // another Error such as the ValueError of the missing
                    // files, which would mean the argument was taken.
                    if ($e::class !== \TypeError::class || !str_contains($e->getMessage(), $expected)) {
                        $failures[] = "{$label}: " . $e::class . ': ' . $e->getMessage();
                    }
                }
            }
        }

        $this->assertSame([], $failures);
    }

    public function testNullIsAccepted(): void
    {
        $failures = [];
        foreach (self::calls() as $key => $call) {
            try {
                $call(null);
            } catch (\ValueError $e) {
                // Trainer and buildUser take their arguments, then reject the
                // missing files.
                if (!str_contains($e->getMessage(), 'does not exist')) {
                    $failures[] = "{$key} <- null: ValueError: " . $e->getMessage();
                }
            } catch (\Throwable $e) {
                $failures[] = "{$key} <- null: " . $e::class . ': ' . $e->getMessage();
            }
        }

        $this->assertSame([], $failures);
    }

    public function testNullSelectsTheDefault(): void
    {
        $penalty = new Lindera\Penalty(null, null, null, null);
        $this->assertSame(2, $penalty->kanji_penalty_length_threshold);
        $this->assertSame(1700, $penalty->other_penalty_length_penalty);

        $metadata = new Lindera\Metadata(name: null, flexible_csv: null);
        $this->assertSame('default', $metadata->name);
        $this->assertFalse($metadata->flexible_csv);

        $penalty = Lindera\Mode::decompose(null)->penalty;
        $this->assertSame(3000, $penalty->kanji_penalty_length_penalty);
        $this->assertSame(1700, $penalty->other_penalty_length_penalty);

        $this->assertNull((new Lindera\FieldDefinition(0, 'surface', 'surface', null))->description);
    }

    public function testFloatParameterWidensAnInt(): void
    {
        if (!class_exists(Lindera\Trainer::class)) {
            $this->markTestSkipped('the extension was built without the train feature');
        }

        // The int must pass the argument conversion and reach the trainer,
        // which then rejects the missing seed file.
        $this->expectException(\ValueError::class);
        $this->expectExceptionMessage('seed file does not exist');
        $missing = self::MISSING;
        Lindera\Trainer::train($missing, $missing, $missing, $missing, $missing, $missing, $missing, lambda: 1);
    }
}
