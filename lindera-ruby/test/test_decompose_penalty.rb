# frozen_string_literal: true

require_relative 'test_helper'

# The penalty of decompose mode: TokenizerBuilder#set_mode and Tokenizer.new
# take a Lindera::Mode, which holds the penalty (#1215, #1218).
class TestDecomposePenalty < Minitest::Test
  TEXT = '関西国際空港限定トートバッグ'

  NORMAL_SURFACES = %w[関西国際空港 限定 トートバッグ].freeze

  DEFAULT_SURFACES = %w[関西 国際 空港 限定 トートバッグ].freeze

  # Without length penalties, decompose mode keeps the compound whole, as
  # normal mode does.
  NO_LENGTH_PENALTY_SURFACES = NORMAL_SURFACES

  # With a katakana threshold of 4, the output differs from both normal
  # mode's and the default penalty's, so a decompose mode that loses its
  # penalty, or is read as normal mode, fails.
  SHORT_KATAKANA_SURFACES = %w[関西 国際 空港 限定 トー トバッグ].freeze

  def no_length_penalty
    Lindera::Penalty.new(nil, 0, nil, 0)
  end

  def short_katakana_threshold
    Lindera::Penalty.new(nil, nil, 4, 3000)
  end

  # Pairs of a mode and the surfaces it gives for TEXT.
  def modes
    [
      [Lindera::Mode.normal, NORMAL_SURFACES],
      [Lindera::Mode.decompose, DEFAULT_SURFACES],
      [Lindera::Mode.decompose(nil), DEFAULT_SURFACES],
      [Lindera::Mode.decompose(Lindera::Penalty.new(nil, nil, nil, nil)), DEFAULT_SURFACES],
      [Lindera::Mode.decompose(no_length_penalty), NO_LENGTH_PENALTY_SURFACES],
      [Lindera::Mode.decompose(short_katakana_threshold), SHORT_KATAKANA_SURFACES],
      [Lindera::Mode.from_str('decompose'), DEFAULT_SURFACES]
    ]
  end

  # Returns the surfaces of TEXT for a tokenizer built with set_mode(*args).
  def builder_surfaces(*args)
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    builder.set_mode(*args)
    builder.build.tokenize(TEXT).map(&:surface)
  end

  # Returns the surfaces of TEXT for a Tokenizer built directly from IPADIC.
  def constructor_surfaces(*args, **kwargs)
    dictionary = Lindera.load_dictionary('embedded://ipadic')
    Lindera::Tokenizer.new(dictionary, *args, **kwargs).tokenize(TEXT).map(&:surface)
  end

  def test_set_mode_penalty_reaches_the_segmenter
    modes.each do |mode, expected|
      assert_equal expected, builder_surfaces(mode), mode.inspect
    end
  end

  def test_constructor_penalty_reaches_the_segmenter
    modes.each do |mode, expected|
      assert_equal expected, constructor_surfaces(mode), mode.inspect
      assert_equal expected, constructor_surfaces(mode, nil, space_penalty: nil), mode.inspect
    end
  end

  def test_one_mode_serves_several_tokenizers
    mode = Lindera::Mode.decompose(short_katakana_threshold)
    assert_equal SHORT_KATAKANA_SURFACES, builder_surfaces(mode)
    assert_equal SHORT_KATAKANA_SURFACES, constructor_surfaces(mode)
    assert_equal SHORT_KATAKANA_SURFACES, builder_surfaces(mode)
  end

  def test_constructor_defaults_to_normal_mode
    assert_equal NORMAL_SURFACES, constructor_surfaces
    assert_equal NORMAL_SURFACES, constructor_surfaces(nil)
    assert_equal NORMAL_SURFACES, constructor_surfaces(nil, nil)
  end

  def test_a_value_that_is_not_a_mode_is_rejected
    dictionary = Lindera.load_dictionary('embedded://ipadic')
    ['decompose', 'normal', :decompose, 0, Lindera::Penalty.new(nil, nil, nil, nil)].each do |value|
      assert_raises(TypeError, value.inspect) { Lindera::TokenizerBuilder.new.set_mode(value) }
      assert_raises(TypeError, value.inspect) { Lindera::Tokenizer.new(dictionary, value) }
    end
    assert_raises(TypeError) { Lindera::TokenizerBuilder.new.set_mode(nil) }
  end

  def test_the_penalty_argument_is_gone
    # The penalty goes in the Mode (#1218).
    builder = Lindera::TokenizerBuilder.new
    assert_raises(ArgumentError) { builder.set_mode }
    assert_raises(ArgumentError) { builder.set_mode(Lindera::Mode.decompose, no_length_penalty) }
    assert_raises(ArgumentError) { builder.set_mode(Lindera::Mode.decompose, penalty: no_length_penalty) }

    dictionary = Lindera.load_dictionary('embedded://ipadic')
    assert_raises(ArgumentError) { Lindera::Tokenizer.new(dictionary, penalty: no_length_penalty) }
  end
end
