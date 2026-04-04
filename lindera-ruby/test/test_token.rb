# frozen_string_literal: true

require_relative "test_helper"

class TestToken < Minitest::Test
  def setup
    dictionary = Lindera.load_dictionary("embedded://ipadic")
    tokenizer = Lindera::Tokenizer.new(dictionary, "normal", nil)
    @tokens = tokenizer.tokenize("関西国際空港")
  end

  def test_token_attributes
    token = @tokens[0]

    assert_kind_of Lindera::Token, token
    assert_kind_of String, token.surface
    assert_kind_of Integer, token.byte_start
    assert_kind_of Integer, token.byte_end
    assert_kind_of Integer, token.position
    assert_kind_of Integer, token.word_id
  end

  def test_token_details
    token = @tokens[0]
    details = token.details

    assert_kind_of Array, details
    assert details.length.positive?

    first_detail = token.get_detail(0)
    assert_equal details[0], first_detail
  end

  def test_get_detail_out_of_bounds
    token = @tokens[0]
    assert_nil token.get_detail(9999)
  end

  def test_token_to_s
    token = @tokens[0]
    assert_equal token.surface, token.to_s
  end

  def test_token_inspect
    token = @tokens[0]
    inspect_str = token.inspect
    assert inspect_str.include?("Lindera::Token")
    assert inspect_str.include?(token.surface)
  end
end
