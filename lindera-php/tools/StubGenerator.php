<?php

declare(strict_types=1);

namespace Lindera\Tools;

use ReflectionClass;
use ReflectionClassConstant;
use ReflectionException;
use ReflectionExtension;
use ReflectionFunction;
use ReflectionFunctionAbstract;
use ReflectionIntersectionType;
use ReflectionMethod;
use ReflectionNamedType;
use ReflectionParameter;
use ReflectionProperty;
use ReflectionType;
use ReflectionUnionType;

/**
 * Renders PHP stub declarations for a loaded extension from its reflection data.
 *
 * The stubs describe every class, constant, property, method and function the
 * extension registers, in a form IDEs, PHPStan and Psalm can read. They are
 * generated from the compiled extension rather than written by hand so that
 * the committed file cannot drift from the Rust source: `StubsTest` regenerates
 * them and compares the result with `stubs/lindera.stubs.php`.
 *
 * Output is deterministic: functions are sorted by name, classes by depth in
 * the inheritance tree and then by name, members by name within each class.
 */
final class StubGenerator
{
    /**
     * PHPDoc types of the public properties, keyed by class then property.
     *
     * ext-php-rs registers `#[php(getter)]` properties without a native type,
     * so reflection cannot tell `int` from `string`. The types are taken from
     * the return types of the getters in `src/*.rs` and written as `@var`
     * tags. Generation fails when a property is missing here or an entry names
     * a property that no longer exists, so this table cannot drift from the
     * extension.
     *
     * @var array<string, array<string, string>>
     */
    private const PROPERTY_TYPES = [
        'Lindera\FieldDefinition' => [
            'description' => 'string|null',
            'field_type' => 'string',
            'index' => 'int',
            'name' => 'string',
        ],
        'Lindera\FieldType' => [
            'value' => 'string',
        ],
        'Lindera\Metadata' => [
            'default_field_value' => 'string',
            'default_left_context_id' => 'int',
            'default_right_context_id' => 'int',
            'default_word_cost' => 'int',
            'dictionary_schema_fields' => 'list<string>',
            'encoding' => 'string',
            'flexible_csv' => 'bool',
            'name' => 'string',
            'normalize_details' => 'bool',
            'skip_invalid_cost_or_id' => 'bool',
            'user_dictionary_schema_fields' => 'list<string>',
        ],
        'Lindera\Mode' => [
            'name' => 'string',
        ],
        'Lindera\NbestResult' => [
            'cost' => 'int',
            'tokens' => 'list<Token>',
        ],
        'Lindera\Penalty' => [
            'kanji_penalty_length_penalty' => 'int',
            'kanji_penalty_length_threshold' => 'int',
            'other_penalty_length_penalty' => 'int',
            'other_penalty_length_threshold' => 'int',
        ],
        'Lindera\Schema' => [
            'fields' => 'list<string>',
        ],
        'Lindera\Token' => [
            'byte_end' => 'int',
            'byte_start' => 'int',
            'details' => 'list<string>',
            'is_unknown' => 'bool',
            'position' => 'int',
            'surface' => 'string',
            'word_id' => 'int',
        ],
    ];

    /**
     * The message ext-php-rs throws from the placeholder constructor it gives
     * classes that PHP code is not meant to construct.
     */
    private const NOT_CONSTRUCTIBLE = 'You cannot instantiate this class from PHP.';

    /** @var string The namespace whose prefix is dropped from type names. */
    private string $namespace;

    /**
     * @param string $extension The extension name as `php -m` prints it.
     * @param string $namespace The namespace every symbol lives in, without a leading backslash.
     */
    public function __construct(
        private string $extension = 'lindera',
        string $namespace = 'Lindera',
    ) {
        $this->namespace = trim($namespace, '\\');
    }

    /**
     * Renders the complete stub file.
     *
     * @return string PHP source, starting with `<?php`.
     *
     * @throws ReflectionException When the extension is not loaded.
     */
    public function generate(): string
    {
        $extension = new ReflectionExtension($this->extension);

        $out = "<?php\n\n";
        $out .= "/**\n";
        $out .= " * Stubs for the {$extension->getName()} PHP extension (Packagist: lindera/lindera).\n";
        $out .= " *\n";
        $out .= " * Generated from the compiled extension by lindera-php/tools/generate-stubs.php;\n";
        $out .= " * do not edit by hand. Point PHPStan (stubFiles) or Psalm (stubs) at this file.\n";
        $out .= " * Never include it at runtime next to the real extension: the classes would be\n";
        $out .= " * declared twice.\n";
        $out .= " *\n";
        $out .= " * @generated\n";
        $out .= " */\n\n";
        $out .= "namespace {$this->namespace};\n";

        $functions = $extension->getFunctions();
        ksort($functions, SORT_STRING);
        foreach ($functions as $function) {
            $out .= "\n" . $this->renderFunction($function);
        }

        $classes = $extension->getClasses();
        foreach (array_keys(self::PROPERTY_TYPES) as $class) {
            if (!isset($classes[$class])) {
                throw new \LogicException("PROPERTY_TYPES lists {$class}, which the extension no longer registers.");
            }
        }
        foreach ($this->sortClasses($classes) as $class) {
            $out .= "\n" . $this->renderClass($class);
        }

        return $out;
    }

    /**
     * Orders classes so that a parent always precedes its subclasses.
     *
     * @param array<string, ReflectionClass<object>> $classes
     *
     * @return list<ReflectionClass<object>>
     */
    private function sortClasses(array $classes): array
    {
        $depth = static function (ReflectionClass $class): int {
            $n = 0;
            while (($parent = $class->getParentClass()) !== false) {
                $n++;
                $class = $parent;
            }

            return $n;
        };

        $list = array_values($classes);
        usort($list, static fn(ReflectionClass $a, ReflectionClass $b): int => [$depth($a), $a->getName()] <=> [$depth($b), $b->getName()]);

        return $list;
    }

    /**
     * @param ReflectionClass<object> $class
     */
    private function renderClass(ReflectionClass $class): string
    {
        $name = $class->getName();
        $out = $this->renderDocComment($class->getDocComment(), '');

        $keyword = $class->isInterface() ? 'interface' : ($class->isTrait() ? 'trait' : 'class');
        $modifiers = '';
        if ($keyword === 'class') {
            if ($class->isFinal()) {
                $modifiers = 'final ';
            } elseif ($class->isAbstract()) {
                $modifiers = 'abstract ';
            }
        }
        $out .= "{$modifiers}{$keyword} " . $this->shortName($name);

        $parent = $class->getParentClass();
        if ($parent !== false) {
            $out .= ' extends ' . $this->className($parent->getName());
        }

        $interfaces = $class->getInterfaceNames();
        if ($parent !== false) {
            $interfaces = array_values(array_diff($interfaces, $parent->getInterfaceNames()));
        }
        sort($interfaces, SORT_STRING);
        if ($interfaces !== []) {
            $out .= ($class->isInterface() ? ' extends ' : ' implements ')
                . implode(', ', array_map(fn(string $i): string => $this->className($i), $interfaces));
        }
        $out .= "\n{\n";

        $members = [];
        $constructible = $this->isConstructibleFromPhp($class);

        $constants = $class->getReflectionConstants();
        usort($constants, static fn(ReflectionClassConstant $a, ReflectionClassConstant $b): int => strcmp($a->getName(), $b->getName()));
        foreach ($constants as $constant) {
            if ($constant->getDeclaringClass()->getName() !== $name) {
                continue;
            }
            $members[] = $this->renderDocComment($constant->getDocComment(), '    ')
                . '    ' . implode(' ', \Reflection::getModifierNames($constant->getModifiers()))
                . ' const ' . $constant->getName() . ' = ' . $this->literal($constant->getValue()) . ";\n";
        }

        $properties = $class->getProperties();
        usort($properties, static fn(ReflectionProperty $a, ReflectionProperty $b): int => strcmp($a->getName(), $b->getName()));
        $typed = self::PROPERTY_TYPES[$name] ?? [];
        foreach ($properties as $property) {
            if ($property->getDeclaringClass()->getName() !== $name) {
                continue;
            }
            $members[] = $this->renderProperty($property, $typed);
            unset($typed[$property->getName()]);
        }
        if ($typed !== []) {
            throw new \LogicException(
                "PROPERTY_TYPES lists {$name}::\$" . implode(', $', array_keys($typed)) . ', which the extension no longer registers.',
            );
        }

        $methods = $class->getMethods();
        usort($methods, static fn(ReflectionMethod $a, ReflectionMethod $b): int => strcmp($a->getName(), $b->getName()));
        foreach ($methods as $method) {
            if ($method->getDeclaringClass()->getName() !== $name) {
                continue;
            }
            $members[] = $this->renderMethod($method, $constructible);
        }

        $out .= implode("\n", $members);
        $out .= "}\n";

        return $out;
    }

    /**
     * Reports whether `new Class()` works, or throws ext-php-rs's placeholder error.
     *
     * ext-php-rs gives every class without a `#[php(constructor)]` a public
     * zero-argument `__construct` that only throws. Reflection cannot tell it
     * from a real constructor, so the generator tries it. Only classes whose
     * constructor takes no required arguments are probed; the probe has no
     * side effects for any lindera class.
     *
     * @param ReflectionClass<object> $class
     */
    private function isConstructibleFromPhp(ReflectionClass $class): bool
    {
        $constructor = $class->getConstructor();
        if ($constructor === null || $constructor->getNumberOfRequiredParameters() > 0 || !$class->isInstantiable()) {
            return true;
        }
        try {
            $class->newInstance();

            return true;
        } catch (\Throwable $e) {
            return $e->getMessage() !== self::NOT_CONSTRUCTIBLE;
        }
    }

    /**
     * @param array<string, string> $types PHPDoc types for this class's properties.
     */
    private function renderProperty(ReflectionProperty $property, array $types): string
    {
        $name = $property->getName();
        $class = $property->getDeclaringClass()->getName();
        $parts = \Reflection::getModifierNames($property->getModifiers());
        $type = $property->getType();
        $doc = $property->getDocComment();
        if ($type !== null) {
            $parts[] = $this->typeName($type);
        } elseif (isset($types[$name])) {
            $doc = "/** @var {$types[$name]} */";
        } elseif ($property->isPublic()) {
            throw new \LogicException(
                "{$class}::\${$name} has no native type; add it to StubGenerator::PROPERTY_TYPES.",
            );
        }
        $parts[] = '$' . $name;

        $line = '    ' . implode(' ', $parts);
        // An untyped property defaults to null implicitly; only spell out
        // other defaults.
        if ($property->hasDefaultValue() && !$property->isReadOnly() && $property->getDefaultValue() !== null) {
            $line .= ' = ' . $this->literal($property->getDefaultValue());
        }

        return $this->renderDocComment($doc, '    ') . $line . ";\n";
    }

    /**
     * @param bool $constructible Whether `new` works on the declaring class.
     */
    private function renderMethod(ReflectionMethod $method, bool $constructible): string
    {
        $modifiers = \Reflection::getModifierNames($method->getModifiers());
        $doc = $method->getDocComment();
        if ($method->isConstructor() && !$constructible) {
            // Declared private so that static analysis rejects `new`, which
            // is what the runtime does (it throws).
            $modifiers = array_values(array_diff($modifiers, ['public', 'protected']));
            array_unshift($modifiers, 'private');
            $doc = '/** Not constructible from PHP: instances come from the extension. */';
        }
        $line = '    ' . implode(' ', $modifiers) . ' function ' . $method->getName()
            . $this->renderSignature($method);
        $line .= $method->isAbstract() ? ";\n" : " {}\n";

        return $this->renderDocComment($doc, '    ') . $line;
    }

    /**
     * Renders a constant, default or literal value as PHP source.
     */
    private function literal(mixed $value): string
    {
        if ($value === null) {
            return 'null';
        }
        if (is_bool($value)) {
            return $value ? 'true' : 'false';
        }

        return var_export($value, true);
    }

    private function renderFunction(ReflectionFunction $function): string
    {
        return $this->renderDocComment($function->getDocComment(), '')
            . 'function ' . $this->shortName($function->getName()) . $this->renderSignature($function) . " {}\n";
    }

    /**
     * Renders `(params): returnType`.
     */
    private function renderSignature(ReflectionFunctionAbstract $function): string
    {
        $params = array_map(fn(ReflectionParameter $p): string => $this->renderParameter($p), $function->getParameters());
        $out = '(' . implode(', ', $params) . ')';

        $return = $function->getReturnType();
        if ($return !== null) {
            $out .= ': ' . $this->typeName($return);
        }

        return $out;
    }

    private function renderParameter(ReflectionParameter $parameter): string
    {
        $out = '';
        $type = $parameter->getType();
        if ($type !== null) {
            $out .= $this->typeName($type) . ' ';
        }
        if ($parameter->isPassedByReference()) {
            $out .= '&';
        }
        if ($parameter->isVariadic()) {
            $out .= '...';
        }
        $out .= '$' . $parameter->getName();

        if ($parameter->isOptional() && !$parameter->isVariadic()) {
            // Internal functions expose defaults only when the arginfo declares
            // them; fall back to null, which is what an omitted optional
            // argument means for every lindera signature (they map to Rust `Option`).
            $default = $parameter->isDefaultValueAvailable() ? $parameter->getDefaultValue() : null;
            $out .= ' = ' . $this->literal($default);
        }

        return $out;
    }

    /**
     * Renders a type as it should appear inside the stub namespace.
     */
    private function typeName(ReflectionType $type): string
    {
        if ($type instanceof ReflectionNamedType) {
            $name = $type->getName();
            $rendered = $type->isBuiltin() ? $name : $this->className($name);
            if ($type->allowsNull() && $name !== 'mixed' && $name !== 'null') {
                $rendered = '?' . $rendered;
            }

            return $rendered;
        }
        if ($type instanceof ReflectionUnionType) {
            return implode('|', array_map(fn(ReflectionType $t): string => $this->typeName($t), $type->getTypes()));
        }
        if ($type instanceof ReflectionIntersectionType) {
            return implode('&', array_map(fn(ReflectionType $t): string => $this->typeName($t), $type->getTypes()));
        }

        return (string) $type;
    }

    /**
     * Renders a class name relative to the stub namespace.
     *
     * Classes inside the namespace are written unqualified; everything else is
     * fully qualified with a leading backslash.
     */
    private function className(string $name): string
    {
        $prefix = $this->namespace . '\\';
        if (str_starts_with($name, $prefix)) {
            return substr($name, strlen($prefix));
        }

        return '\\' . $name;
    }

    private function shortName(string $name): string
    {
        $pos = strrpos($name, '\\');

        return $pos === false ? $name : substr($name, $pos + 1);
    }

    /**
     * Re-indents a reflection doc comment, or renders nothing when there is none.
     *
     * @param string|false $comment
     */
    private function renderDocComment($comment, string $indent): string
    {
        if ($comment === false || $comment === '') {
            return '';
        }
        $lines = preg_split('/\R/', trim($comment)) ?: [];

        return implode("\n", array_map(static fn(string $l): string => $indent . ltrim($l), $lines)) . "\n";
    }
}
