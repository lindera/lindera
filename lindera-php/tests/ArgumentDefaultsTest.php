<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Optional parameters carry a `null` default in the arginfo (lindera/lindera#1067).
 *
 * ext-php-rs registers an `Option<T>` parameter without a default unless the
 * method has `#[php(defaults(...))]`. PHP then rejects a named-argument call
 * that skips it ("the default value is not known"), and reflection reports no
 * default either.
 */
class ArgumentDefaultsTest extends TestCase
{
    public function testEveryOptionalParameterDefaultsToNull(): void
    {
        $missing = [];
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
                if (!$parameter->isOptional()) {
                    continue;
                }
                if (!$parameter->isDefaultValueAvailable() || $parameter->getDefaultValue() !== null) {
                    $missing[] = "{$owner}{$function->getName()}(\${$parameter->getName()})";
                }
            }
        }

        $this->assertSame(
            [],
            $missing,
            'optional parameters without a null default; add #[php(defaults(... = None))]'
        );
    }

    public function testPenaltyTakesANamedArgumentAfterSkippedOnes(): void
    {
        $penalty = new Lindera\Penalty(other_penalty_length_threshold: 5);

        $this->assertSame(2, $penalty->kanji_penalty_length_threshold);
        $this->assertSame(3000, $penalty->kanji_penalty_length_penalty);
        $this->assertSame(5, $penalty->other_penalty_length_threshold);
        $this->assertSame(1700, $penalty->other_penalty_length_penalty);
    }

    public function testMetadataTakesANamedArgumentAfterSkippedOnes(): void
    {
        $metadata = new Lindera\Metadata(encoding: 'EUC-JP');

        $this->assertSame('default', $metadata->name);
        $this->assertSame('EUC-JP', $metadata->encoding);
    }

    public function testTokenizeNbestTakesANamedArgumentAfterASkippedOne(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $results = $builder->build()->tokenizeNbest('東京都', 2, cost_threshold: 100000);

        $this->assertNotEmpty($results);
    }

    public function testTrainerTrainTakesANamedArgumentAfterSkippedOnes(): void
    {
        if (!class_exists(Lindera\Trainer::class)) {
            $this->markTestSkipped('the extension was built without the train feature');
        }

        // Only the argument passing is under test: the call must reach the
        // trainer, which then rejects the missing seed file.
        $this->expectException(\ValueError::class);
        $this->expectExceptionMessage('seed file does not exist');
        Lindera\Trainer::train(
            '/nonexistent/seed.csv',
            '/nonexistent/corpus.txt',
            '/nonexistent/char.def',
            '/nonexistent/unk.def',
            '/nonexistent/feature.def',
            '/nonexistent/rewrite.def',
            '/nonexistent/model.dat',
            max_iter: 1,
        );
    }
}
