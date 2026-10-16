# frozen_string_literal: true

require_relative 'test_helper'

# The penalty of decompose mode: TokenizerBuilder#set_mode and Tokenizer.new
# take a Lindera::Penalty with the keyword penalty: (#1215).
class TestDecomposePenalty < Minitest::Test
  TEXT = '関西国際空港限定トートバッグ'

  DEFAULT_SURFACES = %w[関西 国際 空港 限定 トートバッグ].freeze

  # Without length penalties, decompose mode keeps the compound whole, as
  # normal mode does.
  NO_LENGTH_PENALTY_SURFACES = %w[関西国際空港 限定 トートバッグ].freeze

  def no_length_penalty
    Lindera::Penalty.new(nil, 0, nil, 0)
  end

  # Returns the surfaces of TEXT for a tokenizer built with set_mode(*args).
  def builder_surfaces(*args, **kwargs)
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    builder.set_mode(*args, **kwargs)
    builder.build.tokenize(TEXT).map(&:surface)
  end

  # Returns the surfaces of TEXT for a Tokenizer built directly from IPADIC.
  def constructor_surfaces(*args, **kwargs)
    dictionary = Lindera.load_dictionary('embedded://ipadic')
    Lindera::Tokenizer.new(dictionary, *args, **kwargs).tokenize(TEXT).map(&:surface)
  end

  def test_set_mode_penalty_reaches_the_segmenter
    assert_equal DEFAULT_SURFACES, builder_surfaces('decompose')
    assert_equal DEFAULT_SURFACES, builder_surfaces('decompose', penalty: nil)
    assert_equal DEFAULT_SURFACES, builder_surfaces('decompose', penalty: Lindera::Penalty.new(nil, nil, nil, nil))
    assert_equal NO_LENGTH_PENALTY_SURFACES, builder_surfaces('decompose', penalty: no_length_penalty)
  end

  def test_constructor_penalty_reaches_the_segmenter
    assert_equal DEFAULT_SURFACES, constructor_surfaces('decompose')
    assert_equal DEFAULT_SURFACES, constructor_surfaces('decompose', nil, penalty: nil)
    assert_equal NO_LENGTH_PENALTY_SURFACES, constructor_surfaces('decompose', penalty: no_length_penalty)
    assert_equal NO_LENGTH_PENALTY_SURFACES,
                 constructor_surfaces('decompose', nil, space_penalty: nil, penalty: no_length_penalty)
  end

  def test_a_penalty_with_normal_mode_is_rejected
    error = assert_raises(RuntimeError) { Lindera::TokenizerBuilder.new.set_mode('normal', penalty: no_length_penalty) }
    assert_match(/decompose/, error.message)

    dictionary = Lindera.load_dictionary('embedded://ipadic')
    error = assert_raises(RuntimeError) { Lindera::Tokenizer.new(dictionary, penalty: no_length_penalty) }
    assert_match(/decompose/, error.message)
  end

  def test_a_penalty_of_another_type_is_rejected
    dictionary = Lindera.load_dictionary('embedded://ipadic')
    [0, 'penalty', { 'kanji_penalty_length_penalty' => 0 }].each do |value|
      assert_raises(TypeError, value.inspect) { Lindera::TokenizerBuilder.new.set_mode('decompose', penalty: value) }
      assert_raises(TypeError, value.inspect) { Lindera::Tokenizer.new(dictionary, 'decompose', penalty: value) }
    end
  end

  def test_set_mode_rejects_a_positional_penalty_and_unknown_keywords
    builder = Lindera::TokenizerBuilder.new
    assert_raises(ArgumentError) { builder.set_mode }
    assert_raises(ArgumentError) { builder.set_mode('decompose', no_length_penalty) }
    assert_raises(ArgumentError) { builder.set_mode('decompose', mode_penalty: no_length_penalty) }
  end
end
