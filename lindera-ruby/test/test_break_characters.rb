# frozen_string_literal: true

require_relative 'test_helper'

# Break characters: TokenizerBuilder#set_break_characters (#1353).
class TestBreakCharacters < Minitest::Test
  # 'レノン・ジョン' is not in IPADIC. With '・' as a break character (the
  # default), the unknown word stops at it; with none, the katakana run, '・'
  # included, is one unknown word, as in MeCab.
  TEXT = 'レノン・ジョン'
  SPLIT = %w[レノン ・ ジョン].freeze
  WHOLE = ['レノン・ジョン'].freeze

  # Builds an IPADIC tokenizer after calling set_break_characters with each of
  # `values` in turn, and returns the surfaces of TEXT.
  def surfaces(*values)
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    values.each { |value| builder.set_break_characters(value) }
    builder.build.tokenize(TEXT).map(&:surface)
  end

  def test_set_break_characters_returns_nil
    builder = Lindera::TokenizerBuilder.new
    ['・', '', nil].each do |value|
      assert_nil builder.set_break_characters(value), "value #{value.inspect}"
    end
  end

  def test_set_break_characters_sets_the_characters
    assert_equal SPLIT, surfaces
    assert_equal SPLIT, surfaces('・')
    assert_equal WHOLE, surfaces('')
    # Another set replaces the default.
    assert_equal WHOLE, surfaces('ー')
    # The last call wins.
    assert_equal WHOLE, surfaces('・', '')
  end

  def test_set_break_characters_nil_restores_the_default
    assert_equal SPLIT, surfaces('', nil)
  end

  def test_set_break_characters_rejects_non_strings
    [0, true, ['・']].each do |value|
      assert_raises(TypeError, "value #{value.inspect}") do
        Lindera::TokenizerBuilder.new.set_break_characters(value)
      end
    end
  end
end
