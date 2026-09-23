<?php

declare(strict_types=1);

namespace Lindera\Tests;

use Lindera\Tools\StubGenerator;
use PhpToken;
use PHPUnit\Framework\TestCase;
use ReflectionExtension;

/**
 * Guards the extension name and the committed stub file.
 *
 * The stub file is generated from the compiled extension, so the strongest
 * check is simply "regenerating it yields the committed bytes". The other
 * tests do not go through the generator, so a generator bug that dropped a
 * class would still be caught.
 */
final class StubsTest extends TestCase
{
    private const STUB = __DIR__ . '/../stubs/lindera.stubs.php';

    public function testExtensionRegistersAsLindera(): void
    {
        // PIE's extension-name, php -m, extension=lindera and lindera.so all
        // use this name; the crate name lindera-php must not leak through.
        self::assertTrue(extension_loaded('lindera'));
        self::assertFalse(extension_loaded('lindera-php'));
        self::assertSame('lindera', (new ReflectionExtension('lindera'))->getName());
    }

    public function testStubFileMatchesTheCompiledExtension(): void
    {
        self::assertStringEqualsFile(
            self::STUB,
            (new StubGenerator())->generate(),
            'lindera-php/stubs/lindera.stubs.php is out of date; run `make stubs-lindera-php`.',
        );
    }

    public function testStubFileParses(): void
    {
        // TOKEN_PARSE makes the tokenizer throw ParseError on invalid PHP.
        $tokens = PhpToken::tokenize((string) file_get_contents(self::STUB), TOKEN_PARSE);

        self::assertNotEmpty($tokens);
        self::assertSame('<?php', trim($tokens[0]->text));
    }

    public function testStubDeclaresEveryClassAndFunction(): void
    {
        $extension = new ReflectionExtension('lindera');
        $code = (string) file_get_contents(self::STUB);

        self::assertStringContainsString("\nnamespace Lindera;\n", $code);

        $classes = $extension->getClassNames();
        self::assertNotEmpty($classes);
        foreach ($classes as $class) {
            $short = substr($class, strrpos($class, '\\') + 1);
            self::assertMatchesRegularExpression(
                '/^(?:(?:final|abstract) )?class ' . preg_quote($short, '/') . '\b/m',
                $code,
                "class {$class} is missing from the stub file",
            );
        }

        // The extension registers classes only today; any function added
        // later must show up in the stubs as well.
        foreach (array_keys($extension->getFunctions()) as $function) {
            $short = substr($function, strrpos($function, '\\') + 1);
            self::assertMatchesRegularExpression(
                '/^function ' . preg_quote($short, '/') . '\(/m',
                $code,
                "function {$function} is missing from the stub file",
            );
        }
    }
}
