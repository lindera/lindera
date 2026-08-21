# frozen_string_literal: true

require_relative 'test_helper'

class TestToken < Minitest::Test
  def setup
    dictionary = Lindera.load_dictionary('embedded://ipadic')
    tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', nil)
    @tokens = tokenizer.tokenize('関西国際空港')
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

  # #952: to_h returns plain data with natural Ruby types.
  def test_token_to_h
    require 'json'

    token = @tokens[0]
    hash = token.to_h

    assert_kind_of Hash, hash
    assert_equal token.surface, hash[:surface]
    assert_kind_of Integer, hash[:byte_start]
    assert_equal token.byte_start, hash[:byte_start]
    assert_equal token.byte_end, hash[:byte_end]
    assert_equal token.position, hash[:position]
    assert_equal token.word_id, hash[:word_id]
    assert_equal token.unknown?, hash[:is_unknown]
    assert_kind_of Array, hash[:details]
    assert_equal token.details, hash[:details]

    # Serializes without a custom encoder.
    assert_equal hash[:surface], JSON.parse(JSON.generate(hash))['surface']
  end

  # to_hash is the canonical name; to_h is the idiomatic alias.
  def test_token_to_hash_alias
    token = @tokens[0]
    assert_equal token.to_h, token.to_hash
  end

  def test_token_inspect
    token = @tokens[0]
    inspect_str = token.inspect
    assert inspect_str.include?('Lindera::Token')
    assert inspect_str.include?(token.surface)
  end
end
