<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

class LinderaTest extends TestCase
{
    public function testVersion(): void
    {
        $version = Lindera\Dictionary::version();
        $this->assertIsString($version);
        $this->assertNotEmpty($version);
    }

    public function testModeNormal(): void
    {
        $mode = new Lindera\Mode('normal');
        $this->assertTrue($mode->isNormal());
        $this->assertFalse($mode->isDecompose());
        $this->assertEquals('normal', $mode->name);
    }

    public function testModeDecompose(): void
    {
        $mode = new Lindera\Mode('decompose');
        $this->assertTrue($mode->isDecompose());
        $this->assertFalse($mode->isNormal());
    }

    public function testModeDefault(): void
    {
        $mode = new Lindera\Mode();
        $this->assertTrue($mode->isNormal());
    }

    public function testModeInvalid(): void
    {
        $this->expectException(\ValueError::class);
        new Lindera\Mode('invalid');
    }

    public function testPenaltyDefault(): void
    {
        $penalty = new Lindera\Penalty();
        $this->assertEquals(2, $penalty->kanji_penalty_length_threshold);
        $this->assertEquals(3000, $penalty->kanji_penalty_length_penalty);
        $this->assertEquals(7, $penalty->other_penalty_length_threshold);
        $this->assertEquals(1700, $penalty->other_penalty_length_penalty);
    }

    public function testPenaltyCustom(): void
    {
        $penalty = new Lindera\Penalty(5, 4000, 10, 2000);
        $this->assertEquals(5, $penalty->kanji_penalty_length_threshold);
        $this->assertEquals(4000, $penalty->kanji_penalty_length_penalty);
        $this->assertEquals(10, $penalty->other_penalty_length_threshold);
        $this->assertEquals(2000, $penalty->other_penalty_length_penalty);
    }

    public function testSchemaDefault(): void
    {
        $schema = Lindera\Schema::createDefault();
        $this->assertEquals(13, $schema->fieldCount());
        $fields = $schema->fields;
        $this->assertEquals('surface', $fields[0]);
        $this->assertEquals('pronunciation', $fields[12]);
    }

    public function testSchemaGetFieldIndex(): void
    {
        $schema = Lindera\Schema::createDefault();
        $this->assertEquals(0, $schema->getFieldIndex('surface'));
        $this->assertEquals(3, $schema->getFieldIndex('cost'));
        $this->assertEquals(-1, $schema->getFieldIndex('nonexistent'));
    }

    public function testSchemaGetFieldByName(): void
    {
        $schema = Lindera\Schema::createDefault();
        $field = $schema->getFieldByName('surface');
        $this->assertNotNull($field);
        $this->assertEquals(0, $field->index);
        $this->assertEquals('surface', $field->name);
        $this->assertEquals('surface', $field->field_type);
    }

    public function testSchemaCustomFields(): void
    {
        $schema = Lindera\Schema::createDefault();
        $custom = $schema->getCustomFields();
        $this->assertEquals(9, count($custom));
        $this->assertEquals('major_pos', $custom[0]);
    }

    public function testMetadataDefault(): void
    {
        $metadata = Lindera\Metadata::createDefault();
        $this->assertEquals('default', $metadata->name);
        $this->assertEquals('UTF-8', $metadata->encoding);
        $this->assertEquals(-10000, $metadata->default_word_cost);
    }

    public function testMetadataCustom(): void
    {
        $metadata = new Lindera\Metadata('test', 'EUC-JP', -5000);
        $this->assertEquals('test', $metadata->name);
        $this->assertEquals('EUC-JP', $metadata->encoding);
        $this->assertEquals(-5000, $metadata->default_word_cost);
    }

    public function testFieldType(): void
    {
        $ft = new Lindera\FieldType('surface');
        $this->assertEquals('surface', $ft->value);

        $ft2 = new Lindera\FieldType('custom');
        $this->assertEquals('custom', $ft2->value);
    }

    public function testFieldTypeInvalid(): void
    {
        $this->expectException(\ValueError::class);
        new Lindera\FieldType('invalid');
    }

    public function testTokenizerBuilderBuild(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $tokenizer = $builder->build();
        $this->assertInstanceOf(Lindera\Tokenizer::class, $tokenizer);
    }

    public function testTokenize(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $tokenizer = $builder->build();

        $tokens = $tokenizer->tokenize('関西国際空港');
        $this->assertIsArray($tokens);
        $this->assertGreaterThan(0, count($tokens));

        $token = $tokens[0];
        $this->assertInstanceOf(Lindera\Token::class, $token);
        $this->assertIsString($token->surface);
        $this->assertNotEmpty($token->surface);
        $this->assertIsInt($token->byte_start);
        $this->assertIsInt($token->byte_end);
        $this->assertIsInt($token->position);
        $this->assertIsInt($token->word_id);
        $this->assertIsBool($token->is_unknown);
        $this->assertIsArray($token->details);
    }

    public function testTokenGetDetail(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $tokenizer = $builder->build();

        $tokens = $tokenizer->tokenize('東京');
        $this->assertGreaterThan(0, count($tokens));

        $token = $tokens[0];
        $detail0 = $token->getDetail(0);
        $this->assertIsString($detail0);

        $detailNull = $token->getDetail(100);
        $this->assertNull($detailNull);
    }

    public function testTokenizerBuilderSetMode(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $builder->setMode('normal');
        $tokenizer = $builder->build();

        $tokens = $tokenizer->tokenize('関西国際空港');
        $this->assertGreaterThan(0, count($tokens));
    }

    public function testTokenizerBuilderDecomposeMode(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $builder->setMode('decompose');
        $tokenizer = $builder->build();

        $tokens = $tokenizer->tokenize('関西国際空港');
        $this->assertGreaterThan(0, count($tokens));
    }

    public function testTokenizerDirect(): void
    {
        $dict = Lindera\Dictionary::load('embedded://ipadic');
        $tokenizer = new Lindera\Tokenizer($dict, 'normal');

        $tokens = $tokenizer->tokenize('すもももももももものうち');
        $this->assertGreaterThan(0, count($tokens));

        // Check that surfaces join back to original text
        $surfaces = array_map(fn($t) => $t->surface, $tokens);
        $this->assertEquals('すもももももももものうち', implode('', $surfaces));
    }

    public function testTokenizeNbest(): void
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        $tokenizer = $builder->build();

        $results = $tokenizer->tokenizeNbest('東京都', 3);
        $this->assertIsArray($results);
        $this->assertGreaterThan(0, count($results));

        $result = $results[0];
        $this->assertInstanceOf(Lindera\NbestResult::class, $result);
        $this->assertIsArray($result->tokens);
        $this->assertIsInt($result->cost);
    }

    public function testLoadDictionary(): void
    {
        $dict = Lindera\Dictionary::load('embedded://ipadic');
        $this->assertInstanceOf(Lindera\Dictionary::class, $dict);
        $this->assertNotEmpty($dict->metadataName());
        $this->assertNotEmpty($dict->metadataEncoding());
    }

    public function testDictionaryMetadata(): void
    {
        $dict = Lindera\Dictionary::load('embedded://ipadic');
        $metadata = $dict->metadata();
        $this->assertInstanceOf(Lindera\Metadata::class, $metadata);
        $this->assertNotEmpty($metadata->name);
    }

    public function testSchemaValidateRecord(): void
    {
        $schema = new Lindera\Schema(['surface', 'pos']);

        // Valid record
        $schema->validateRecord(['東京', '名詞']);
        $this->assertTrue(true); // No exception

        // Too few fields
        $this->expectException(\ValueError::class);
        $schema->validateRecord(['東京']);
    }
}
