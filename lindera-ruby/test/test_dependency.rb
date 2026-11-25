# frozen_string_literal: true

require 'json'
require_relative 'test_helper'

# The token filter japanese_dependency (#1438). The model is the one of the
# tests of lindera-analysis: a chunker over the part of speech and a parser
# that makes every bunsetsu depend on the next.
class TestDependency < Minitest::Test
  MODEL = File.expand_path('../../resources/dependency/test_model_ipadic.bin', __dir__)
  TEXT = '彼は本を読んだ。'

  # A builder with the embedded IPADIC.
  def builder
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    builder
  end

  # A tokenizer with the filter.
  def tokenizer
    builder = self.builder
    builder.append_token_filter('japanese_dependency', { 'model' => MODEL })
    builder.build
  end

  def test_a_token_has_its_dependency
    tokens = tokenizer.tokenize(TEXT)
    written = tokens.map do |token|
      dependency = token.dependency
      [token.surface, dependency.sentence, dependency.bunsetsu, dependency.head, dependency.role,
       dependency.score]
    end

    assert_equal [
      ['彼', 0, 0, 1, 'head', 1.0],
      ['は', 0, 0, 1, 'function', 1.0],
      ['本', 0, 1, 2, 'head', 0.0],
      ['を', 0, 1, 2, 'function', 0.0],
      ['読ん', 0, 2, nil, 'head', 0.0],
      ['だ', 0, 2, nil, 'function', 0.0],
      ['。', 0, 2, nil, 'other', 0.0]
    ], written
    assert_instance_of Lindera::TokenDependency, tokens[0].dependency
    assert_instance_of Float, tokens[0].dependency.score
  end

  def test_a_token_has_no_dependency_without_the_filter
    tokens = builder.build.tokenize(TEXT)

    assert_equal [nil] * 7, tokens.map(&:dependency)
    assert tokens[0].to_h.key?(:dependency)
    assert_nil tokens[0].to_h[:dependency]
  end

  def test_to_h_holds_the_dependency
    tokens = tokenizer.tokenize(TEXT)
    first = { sentence: 0, bunsetsu: 0, head: 1, role: 'head', score: 1.0 }
    last = { sentence: 0, bunsetsu: 2, head: nil, role: 'other', score: 0.0 }

    assert_equal first, tokens[0].dependency.to_h
    assert_equal first, tokens[0].dependency.to_hash
    assert_equal first, tokens[0].to_h[:dependency]
    assert_equal last, tokens[6].to_h[:dependency]
    # The Hash of a token is plain data.
    assert_equal last, JSON.parse(JSON.generate(tokens[6].to_h), symbolize_names: true)[:dependency]
    assert_equal "#<Lindera::TokenDependency sentence=0, bunsetsu=0, head=1, role='head', score=1>",
                 tokens[0].dependency.inspect
    assert_includes tokens[6].dependency.inspect, 'head=nil'
  end

  def test_the_batch_and_the_nbest_results_carry_it
    tokenizer = self.tokenizer
    expected = tokenizer.tokenize(TEXT).map(&:to_h)

    assert_equal expected, tokenizer.tokenize_batch([TEXT, TEXT])[1].map(&:to_h)
    tokens, _cost = tokenizer.tokenize_nbest(TEXT, 1, false, nil)[0]

    assert_equal expected, tokens.map(&:to_h)
  end

  def test_sentences_are_numbered
    tokens = tokenizer.tokenize('彼は本を読んだ。本を読んだ。')

    assert_equal(([0] * 7) + ([1] * 5), tokens.map { |token| token.dependency.sentence })
    # The bunsetsu of the second sentence start at 0 again.
    assert_equal '本', tokens[7].surface
    assert_equal 0, tokens[7].dependency.bunsetsu
  end

  def test_a_model_that_cannot_be_read_is_an_error_of_build
    builder = self.builder
    builder.append_token_filter('japanese_dependency', { 'model' => 'no/such/model.bin' })
    error = assert_raises(StandardError) { builder.build }

    assert_includes error.message, 'no/such/model.bin'
  end

  def test_tokens_removed_by_an_earlier_filter_are_refused
    builder = self.builder
    builder.append_token_filter('japanese_stop_tags', { 'tags' => ['助詞,係助詞'] })
    builder.append_token_filter('japanese_dependency', { 'model' => MODEL })
    error = assert_raises(StandardError) { builder.build.tokenize(TEXT) }

    assert_includes error.message, 'removed or joined'
  end
end
