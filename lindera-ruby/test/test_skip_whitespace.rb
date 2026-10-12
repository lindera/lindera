# frozen_string_literal: true

require_relative 'test_helper'

# Whitespace skipping: TokenizerBuilder#set_skip_whitespace (#1137).
class TestSkipWhitespace < Minitest::Test
  # With whitespace skipped (the default for IPADIC), 'が' connects to the word
  # before the space and reads as a particle (助詞), as in MeCab; with the
  # whitespace node in the lattice it reads as a conjunction (接続詞).
  TEXT = 'Google が 新しい'

  # Builds an IPADIC tokenizer after calling set_skip_whitespace with each of
  # `values` in turn, and returns the part of speech of 'が' in TEXT.
  def pos_of_ga(*values)
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    values.each { |value| builder.set_skip_whitespace(value) }
    token = builder.build.tokenize(TEXT).find { |t| t.surface == 'が' }
    refute_nil token, 'no token with surface が'
    token.details[0]
  end

  def test_set_skip_whitespace_returns_nil
    builder = Lindera::TokenizerBuilder.new
    [true, false, nil].each do |value|
      assert_nil builder.set_skip_whitespace(value), "value #{value.inspect}"
    end
  end

  def test_set_skip_whitespace_turns_skipping_off_and_on
    assert_equal '助詞', pos_of_ga
    assert_equal '助詞', pos_of_ga(true)
    assert_equal '接続詞', pos_of_ga(false)
    # The last call wins.
    assert_equal '接続詞', pos_of_ga(true, false)
  end

  def test_set_skip_whitespace_nil_restores_the_dictionary_default
    # The default for IPADIC skips whitespace.
    assert_equal '助詞', pos_of_ga(false, nil)
  end

  def test_set_skip_whitespace_has_no_effect_with_kept_whitespace
    surfaces = lambda do |*values|
      builder = Lindera::TokenizerBuilder.new
      builder.set_dictionary('embedded://ipadic')
      builder.set_keep_whitespace(true)
      values.each { |value| builder.set_skip_whitespace(value) }
      builder.build.tokenize(TEXT).map(&:surface)
    end
    kept = surfaces.call
    assert_includes kept, ' '
    assert_equal kept, surfaces.call(true)
  end
end
