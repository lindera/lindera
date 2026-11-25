<?php

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * The token filter japanese_dependency (lindera/lindera#1438). The model is
 * the one of the tests of lindera-analysis: a chunker over the part of speech
 * and a parser that makes every bunsetsu depend on the next.
 */
class DependencyTest extends TestCase
{
    private const MODEL = __DIR__ . '/../../resources/dependency/test_model_ipadic.bin';

    private const TEXT = '彼は本を読んだ。';

    private static function builder(): Lindera\TokenizerBuilder
    {
        $builder = new Lindera\TokenizerBuilder();
        $builder->setDictionary('embedded://ipadic');
        return $builder;
    }

    private static function tokenizer(): Lindera\Tokenizer
    {
        $builder = self::builder();
        $builder->appendTokenFilter('japanese_dependency', ['model' => self::MODEL]);
        return $builder->build();
    }

    /**
     * @param list<Lindera\Token> $tokens
     * @return list<array<string, mixed>>
     */
    private static function arrays(array $tokens): array
    {
        return array_map(static fn(Lindera\Token $token) => $token->toArray(), $tokens);
    }

    public function testATokenHasItsDependency(): void
    {
        $tokens = self::tokenizer()->tokenize(self::TEXT);
        $written = array_map(
            static fn(Lindera\Token $token) => [
                $token->surface,
                $token->dependency->sentence,
                $token->dependency->bunsetsu,
                $token->dependency->head,
                $token->dependency->role,
                $token->dependency->score,
            ],
            $tokens,
        );

        $this->assertSame([
            ['彼', 0, 0, 1, 'head', 1.0],
            ['は', 0, 0, 1, 'function', 1.0],
            ['本', 0, 1, 2, 'head', 0.0],
            ['を', 0, 1, 2, 'function', 0.0],
            ['読ん', 0, 2, null, 'head', 0.0],
            ['だ', 0, 2, null, 'function', 0.0],
            ['。', 0, 2, null, 'other', 0.0],
        ], $written);
        $this->assertInstanceOf(Lindera\TokenDependency::class, $tokens[0]->dependency);
    }

    public function testATokenHasNoDependencyWithoutTheFilter(): void
    {
        $tokens = self::builder()->build()->tokenize(self::TEXT);

        foreach ($tokens as $token) {
            $this->assertNull($token->dependency);
        }
        $array = $tokens[0]->toArray();
        $this->assertArrayHasKey('dependency', $array);
        $this->assertNull($array['dependency']);
    }

    public function testToArrayHoldsTheDependency(): void
    {
        $tokens = self::tokenizer()->tokenize(self::TEXT);
        $first = ['sentence' => 0, 'bunsetsu' => 0, 'head' => 1, 'role' => 'head', 'score' => 1.0];
        $last = ['sentence' => 0, 'bunsetsu' => 2, 'head' => null, 'role' => 'other', 'score' => 0.0];

        $this->assertSame($first, $tokens[0]->dependency->toArray());
        $this->assertSame($first, $tokens[0]->toArray()['dependency']);
        $this->assertSame($last, $tokens[6]->toArray()['dependency']);
        // The array of a token is plain data.
        $json = json_encode($tokens[6]->toArray(), JSON_THROW_ON_ERROR | JSON_PRESERVE_ZERO_FRACTION);
        $this->assertSame($last, json_decode($json, true, 8, JSON_THROW_ON_ERROR)['dependency']);
        $this->assertSame(
            "TokenDependency(sentence=0, bunsetsu=0, head=1, role='head', score=1)",
            (string) $tokens[0]->dependency,
        );
        $this->assertStringContainsString('head=null', (string) $tokens[6]->dependency);
    }

    public function testTheBatchAndTheNbestResultsCarryIt(): void
    {
        $tokenizer = self::tokenizer();
        $expected = self::arrays($tokenizer->tokenize(self::TEXT));

        $this->assertSame($expected, self::arrays($tokenizer->tokenizeBatch([self::TEXT, self::TEXT])[1]));
        $this->assertSame($expected, self::arrays($tokenizer->tokenizeNbest(self::TEXT, 1)[0]->tokens));
    }

    public function testSentencesAreNumbered(): void
    {
        $tokens = self::tokenizer()->tokenize('彼は本を読んだ。本を読んだ。');
        $sentences = array_map(static fn(Lindera\Token $token) => $token->dependency->sentence, $tokens);

        $this->assertSame([0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1], $sentences);
        // The bunsetsu of the second sentence start at 0 again.
        $this->assertSame('本', $tokens[7]->surface);
        $this->assertSame(0, $tokens[7]->dependency->bunsetsu);
    }

    public function testAModelThatCannotBeReadIsAnErrorOfBuild(): void
    {
        $builder = self::builder();
        $builder->appendTokenFilter('japanese_dependency', ['model' => 'no/such/model.bin']);

        $this->expectExceptionMessageMatches('~no/such/model\.bin~');
        $builder->build();
    }

    public function testTokensRemovedByAnEarlierFilterAreRefused(): void
    {
        $builder = self::builder();
        $builder->appendTokenFilter('japanese_stop_tags', ['tags' => ['助詞,係助詞']]);
        $builder->appendTokenFilter('japanese_dependency', ['model' => self::MODEL]);
        $tokenizer = $builder->build();

        $this->expectExceptionMessageMatches('/removed or joined/');
        $tokenizer->tokenize(self::TEXT);
    }
}
