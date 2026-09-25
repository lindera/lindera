<?php

/**
 * Stubs for the lindera PHP extension (Packagist: lindera/lindera).
 *
 * Generated from the compiled extension by lindera-php/tools/generate-stubs.php;
 * do not edit by hand. Point PHPStan (stubFiles) or Psalm (stubs) at this file.
 * Never include it at runtime next to the real extension: the classes would be
 * declared twice.
 *
 * @generated
 */

namespace Lindera;

class Dictionary implements \Stringable
{
    /** Not constructible from PHP: instances come from the extension. */
    private function __construct() {}

    public function __toString(): string {}

    public static function build(string $input_dir, string $output_dir, Metadata $metadata): void {}

    public static function buildUser(string $_kind, string $input_file, string $output_dir, ?Metadata $metadata = null): void {}

    public static function load(string $uri): Dictionary {}

    public static function loadUser(string $uri, Metadata $metadata): UserDictionary {}

    public function metadata(): Metadata {}

    public function metadataEncoding(): string {}

    public function metadataName(): string {}

    public static function version(): string {}
}

class FieldDefinition implements \Stringable
{
    /** @var string|null */
    public $description;

    /** @var string */
    public $field_type;

    /** @var int */
    public $index;

    /** @var string */
    public $name;

    public function __construct(int $index, string $name, string $field_type, ?string $description = null) {}

    public function __toString(): string {}
}

class FieldType implements \Stringable
{
    /** @var string */
    public $value;

    public function __construct(string $value) {}

    public function __toString(): string {}
}

class Metadata implements \Stringable
{
    /** @var string */
    public $default_field_value;

    /** @var int */
    public $default_left_context_id;

    /** @var int */
    public $default_right_context_id;

    /** @var int */
    public $default_word_cost;

    /** @var list<string> */
    public $dictionary_schema_fields;

    /** @var string */
    public $encoding;

    /** @var bool */
    public $flexible_csv;

    /** @var string */
    public $name;

    /** @var bool */
    public $normalize_details;

    /** @var bool */
    public $skip_invalid_cost_or_id;

    /** @var list<string> */
    public $user_dictionary_schema_fields;

    public function __construct(?string $name = null, ?string $encoding = null, ?int $default_word_cost = null, ?int $default_left_context_id = null, ?int $default_right_context_id = null, ?string $default_field_value = null, ?bool $flexible_csv = null, ?bool $skip_invalid_cost_or_id = null, ?bool $normalize_details = null) {}

    public function __toString(): string {}

    public static function createDefault(): Metadata {}

    public static function fromJsonFile(string $path): Metadata {}

    public function toArray(): array {}
}

class Mode implements \Stringable
{
    /** @var string */
    public $name;

    public function __construct(?string $mode = null) {}

    public function __toString(): string {}

    public function isDecompose(): bool {}

    public function isNormal(): bool {}
}

class NbestResult implements \Stringable
{
    /** @var int */
    public $cost;

    /** @var list<Token> */
    public $tokens;

    /** Not constructible from PHP: instances come from the extension. */
    private function __construct() {}

    public function __toString(): string {}
}

class Penalty implements \Stringable
{
    /** @var int */
    public $kanji_penalty_length_penalty;

    /** @var int */
    public $kanji_penalty_length_threshold;

    /** @var int */
    public $other_penalty_length_penalty;

    /** @var int */
    public $other_penalty_length_threshold;

    public function __construct(?int $kanji_penalty_length_threshold = null, ?int $kanji_penalty_length_penalty = null, ?int $other_penalty_length_threshold = null, ?int $other_penalty_length_penalty = null) {}

    public function __toString(): string {}
}

class Schema implements \Stringable
{
    /** @var list<string> */
    public $fields;

    public function __construct(array $fields) {}

    public function __toString(): string {}

    public static function createDefault(): Schema {}

    public function fieldCount(): int {}

    public function getAllFields(): array {}

    public function getCustomFields(): array {}

    public function getFieldByName(string $name): ?FieldDefinition {}

    public function getFieldIndex(string $field_name): int {}

    public function getFieldName(int $index): ?string {}

    public function validateRecord(array $record): void {}
}

class Token implements \Stringable
{
    /** @var int */
    public $byte_end;

    /** @var int */
    public $byte_start;

    /** @var list<string> */
    public $details;

    /** @var bool */
    public $is_unknown;

    /** @var int */
    public $position;

    /** @var string */
    public $surface;

    /** @var int */
    public $word_id;

    /** Not constructible from PHP: instances come from the extension. */
    private function __construct() {}

    public function __toString(): string {}

    public function getDetail(int $index): ?string {}

    public function toArray(): array {}
}

class Tokenizer
{
    public function __construct(Dictionary $dictionary, ?string $mode = null, ?UserDictionary $user_dictionary = null) {}

    public function tokenize(string $text): array {}

    public function tokenizeNbest(string $text, int $n, ?bool $unique = null, ?int $cost_threshold = null): array {}

    public function tokenizeSurfaces(string $text): array {}
}

class TokenizerBuilder
{
    public function __construct() {}

    public function appendCharacterFilter(string $kind, mixed $args = null): void {}

    public function appendTokenFilter(string $kind, mixed $args = null): void {}

    public function build(): Tokenizer {}

    public function fromFile(string $file_path): void {}

    public function setDictionary(string $path): void {}

    public function setKeepWhitespace(bool $keep_whitespace): void {}

    public function setMode(string $mode): void {}

    public function setSpacePenalty(mixed $value): void {}

    public function setUserDictionary(string $uri): void {}
}

class Trainer
{
    /** Not constructible from PHP: instances come from the extension. */
    private function __construct() {}

    public static function export(string $model, string $output, ?string $metadata = null): void {}

    public static function train(string $seed, string $corpus, string $char_def, string $unk_def, string $feature_def, string $rewrite_def, string $output, ?float $lambda = null, ?int $max_iter = null, ?int $max_threads = null): void {}
}

class UserDictionary implements \Stringable
{
    /** Not constructible from PHP: instances come from the extension. */
    private function __construct() {}

    public function __toString(): string {}
}
