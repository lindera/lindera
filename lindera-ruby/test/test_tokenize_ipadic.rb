# frozen_string_literal: true

require_relative "test_helper"

class TestTokenizeIpadic < Minitest::Test
  def setup
    @dictionary = Lindera.load_dictionary("embedded://ipadic")
  end

  def test_basic_tokenize
    tokenizer = Lindera::Tokenizer.new(@dictionary, "normal", nil)
    tokens = tokenizer.tokenize("すもももももももものうち")

    assert_equal 7, tokens.length
    assert_equal "すもも", tokens[0].surface
    assert_equal "も", tokens[1].surface
    assert_equal "もも", tokens[2].surface
    assert_equal "も", tokens[3].surface
    assert_equal "もも", tokens[4].surface
    assert_equal "の", tokens[5].surface
    assert_equal "うち", tokens[6].surface
  end

  def test_tokenize_with_builder
    builder = Lindera::TokenizerBuilder.new
    builder.set_mode("normal")
    builder.set_dictionary("embedded://ipadic")
    tokenizer = builder.build

    tokens = tokenizer.tokenize("関西国際空港")
    assert tokens.length.positive?
    assert_equal "関西国際空港", tokens.map(&:surface).join
  end

  def test_tokenize_nbest
    tokenizer = Lindera::Tokenizer.new(@dictionary, "normal", nil)
    results = tokenizer.tokenize_nbest("すもももももももものうち", 3, false, nil)

    assert results.length.positive?
    results.each do |pair|
      tokens = pair[0]
      cost = pair[1]
      assert_kind_of Array, tokens
      assert_kind_of Integer, cost
    end
  end
end
